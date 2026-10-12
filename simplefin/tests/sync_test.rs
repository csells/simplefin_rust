//! Collection pipeline tests driven by a fake fetcher (no network).

use std::cell::RefCell;
use std::collections::HashMap;
use std::str::FromStr;

use asupersync::runtime::RuntimeBuilder;
use rust_decimal::Decimal;
use simplefin::models::{Account, AccountSet, Organization, Transaction};
use simplefin::storage::{
    AccountFilter, BalanceHistoryFilter, JsonStorage, MemoryFiles, Storage, TransactionFilter,
};
use simplefin::sync::{AccountFetcher, CollectEvent, CollectReport, collect_data};
use simplefin::{AccountQueryParams, Anomaly};

const NOW: i64 = 1_700_100_000;

fn org() -> Organization {
    Organization {
        sfin_url: "https://example.com/bank".into(),
        domain: None,
        name: Some("Example Bank".into()),
        url: None,
        id: Some("org-1".into()),
    }
}

fn account(id: &str, balance: &str, txns: Vec<Transaction>) -> Account {
    Account {
        org: org(),
        id: id.into(),
        name: format!("Checking ({id})"),
        currency: "USD".into(),
        balance: Decimal::from_str(balance).unwrap(),
        available_balance: None,
        balance_date: NOW,
        transactions: txns,
    }
}

fn txn(id: &str, posted: i64, pending: bool) -> Transaction {
    Transaction {
        id: id.into(),
        posted,
        amount: Decimal::from_str("-10.00").unwrap(),
        description: format!("Purchase {id}"),
        transacted_at: None,
        pending,
        payee: None,
        memo: None,
    }
}

fn set(accounts: Vec<Account>, messages: &[&str]) -> AccountSet {
    AccountSet {
        server_messages: messages.iter().map(|m| m.to_string()).collect(),
        accounts,
    }
}

/// Records the start date requested for each per-account fetch.
struct FakeFetcher {
    balances: AccountSet,
    per_account: HashMap<String, AccountSet>,
    requested_starts: RefCell<Vec<(String, Option<i64>)>>,
}

impl FakeFetcher {
    fn new(balances: AccountSet) -> Self {
        Self {
            balances,
            per_account: HashMap::new(),
            requested_starts: RefCell::new(Vec::new()),
        }
    }

    fn with_account(mut self, id: &str, result: AccountSet) -> Self {
        self.per_account.insert(id.into(), result);
        self
    }
}

impl AccountFetcher for FakeFetcher {
    async fn fetch_accounts(&self, params: &AccountQueryParams) -> simplefin::Result<AccountSet> {
        if params.balances_only {
            return Ok(self.balances.clone());
        }
        let id = params.account_ids.as_ref().unwrap()[0].clone();
        assert!(
            params.include_pending,
            "per-account fetch must include pending"
        );
        self.requested_starts
            .borrow_mut()
            .push((id.clone(), params.start_date));
        Ok(self
            .per_account
            .get(&id)
            .cloned()
            .unwrap_or_else(|| set(Vec::new(), &[])))
    }
}

fn run<S: Storage>(
    fetcher: &FakeFetcher,
    storage: &mut S,
    now: i64,
) -> (CollectReport, Vec<CollectEvent>) {
    let rt = RuntimeBuilder::current_thread().build().unwrap();
    rt.block_on(async {
        let mut events = Vec::new();
        let report = collect_data(fetcher, storage, now, &mut |e: &CollectEvent| {
            events.push(e.clone())
        })
        .await
        .unwrap();
        (report, events)
    })
}

fn temp_storage() -> (tempfile::TempDir, JsonStorage) {
    let dir = tempfile::tempdir().unwrap();
    let storage = JsonStorage::open(dir.path()).unwrap();
    (dir, storage)
}

#[test]
fn first_run_collects_accounts_transactions_and_balances() {
    let (_dir, mut storage) = temp_storage();
    let fetcher = FakeFetcher::new(set(vec![account("a1", "100.00", vec![])], &[])).with_account(
        "a1",
        set(
            vec![account(
                "a1",
                "100.00",
                vec![txn("t1", 1000, false), txn("t2", 2000, false)],
            )],
            &[],
        ),
    );

    let (report, _) = run(&fetcher, &mut storage, NOW);

    assert!(report.first_run);
    assert_eq!(report.accounts_returned, 1);
    assert_eq!(report.accounts, 1);
    assert_eq!(report.new_transactions, 2);
    assert_eq!(report.duplicates_skipped, 0);
    assert!(report.anomalies.is_empty(), "no anomalies on first run");
    assert_eq!(report.timestamp, NOW);

    assert_eq!(
        storage
            .get_accounts(&AccountFilter::default())
            .unwrap()
            .len(),
        1
    );
    assert_eq!(
        storage
            .get_organizations(&Default::default())
            .unwrap()
            .len(),
        1
    );
    let history = storage
        .get_balance_history(&BalanceHistoryFilter::default())
        .unwrap();
    assert_eq!(history.len(), 1);
    assert_eq!(history[0].timestamp, NOW);
    assert_eq!(storage.last_collected("a1").unwrap(), Some(2000));
    // First fetch with no state starts from epoch 0.
    assert_eq!(fetcher.requested_starts.borrow()[0], ("a1".into(), Some(0)));
}

#[test]
fn second_run_counts_duplicates_and_fetches_incrementally() {
    let (_dir, mut storage) = temp_storage();
    let fetcher = FakeFetcher::new(set(vec![account("a1", "100.00", vec![])], &[])).with_account(
        "a1",
        set(
            vec![account("a1", "100.00", vec![txn("t1", 1000, false)])],
            &[],
        ),
    );
    run(&fetcher, &mut storage, NOW);

    let fetcher2 = FakeFetcher::new(set(vec![account("a1", "100.00", vec![])], &[])).with_account(
        "a1",
        set(
            vec![account(
                "a1",
                "100.00",
                vec![txn("t1", 1000, false), txn("t2", 3000, false)],
            )],
            &[],
        ),
    );
    let (report, _) = run(&fetcher2, &mut storage, NOW + 60);

    assert!(!report.first_run);
    assert_eq!(report.new_transactions, 1);
    assert_eq!(report.duplicates_skipped, 1);
    assert_eq!(
        fetcher2.requested_starts.borrow()[0],
        ("a1".into(), Some(1000))
    );
    assert_eq!(storage.last_collected("a1").unwrap(), Some(3000));
    // Unchanged balance is deduped: still one snapshot.
    let history = storage
        .get_balance_history(&BalanceHistoryFilter::default())
        .unwrap();
    assert_eq!(history.len(), 1);
}

#[test]
fn pending_and_zero_posted_transactions_do_not_advance_state() {
    let (_dir, mut storage) = temp_storage();
    let fetcher = FakeFetcher::new(set(vec![account("a1", "100.00", vec![])], &[])).with_account(
        "a1",
        set(
            vec![account(
                "a1",
                "100.00",
                vec![
                    txn("t1", 1000, false),
                    txn("p1", 9000, true),
                    txn("p2", 0, true),
                ],
            )],
            &[],
        ),
    );

    let (report, _) = run(&fetcher, &mut storage, NOW);

    assert_eq!(report.new_transactions, 3);
    assert_eq!(storage.last_collected("a1").unwrap(), Some(1000));
}

#[test]
fn state_never_moves_backwards() {
    let (_dir, mut storage) = temp_storage();
    storage.set_last_collected("a1", 5000).unwrap();
    let fetcher = FakeFetcher::new(set(vec![account("a1", "100.00", vec![])], &[])).with_account(
        "a1",
        set(
            vec![account("a1", "100.00", vec![txn("old", 1000, false)])],
            &[],
        ),
    );

    run(&fetcher, &mut storage, NOW);

    assert_eq!(
        fetcher.requested_starts.borrow()[0],
        ("a1".into(), Some(5000))
    );
    assert_eq!(storage.last_collected("a1").unwrap(), Some(5000));
}

#[test]
fn corrupted_zero_state_recovers_from_stored_transactions() {
    let (_dir, mut storage) = temp_storage();
    storage
        .upsert_transactions("a1", &[txn("t1", 4000, false)])
        .unwrap();
    storage.set_last_collected("a1", 0).unwrap();
    let fetcher = FakeFetcher::new(set(vec![account("a1", "100.00", vec![])], &[]));

    let (_, events) = run(&fetcher, &mut storage, NOW);

    assert_eq!(
        fetcher.requested_starts.borrow()[0],
        ("a1".into(), Some(4000))
    );
    assert_eq!(storage.last_collected("a1").unwrap(), Some(4000));
    assert!(events.iter().any(|e| matches!(
        e,
        CollectEvent::SyncStateRecovered {
            timestamp: 4000,
            ..
        }
    )));
}

#[test]
fn detects_anomalies_against_previous_accounts_and_persists_warnings() {
    let (_dir, mut storage) = temp_storage();
    storage
        .upsert_accounts(&[
            account("a1", "1000.00", vec![]),
            account("gone", "5.00", vec![]),
        ])
        .unwrap();
    let fetcher = FakeFetcher::new(set(
        vec![account("a1", "0.00", vec![])],
        &["Connection needs attention"],
    ))
    .with_account(
        "a1",
        set(vec![account("a1", "0.00", vec![])], &["Rate limited"]),
    );

    let (report, events) = run(&fetcher, &mut storage, NOW);

    assert!(!report.first_run);
    assert!(
        report
            .anomalies
            .iter()
            .any(|a| matches!(a, Anomaly::BalanceDroppedToZero { .. }))
    );
    assert!(
        report
            .anomalies
            .iter()
            .any(|a| matches!(a, Anomaly::AccountDisappeared { .. }))
    );
    assert_eq!(
        report.bridge_messages,
        vec![
            "Connection needs attention".to_string(),
            "Checking (a1): Rate limited".to_string()
        ]
    );

    let warnings = storage.get_warnings().unwrap().unwrap();
    assert_eq!(warnings.timestamp, NOW);
    assert_eq!(warnings.anomalies.len(), report.anomalies.len());
    assert_eq!(warnings.bridge_messages, report.bridge_messages);

    assert!(events.iter().any(|e| matches!(
        e,
        CollectEvent::BridgeMessage { account_name: None, message } if message == "Connection needs attention"
    )));
    assert!(events.iter().any(|e| matches!(
        e,
        CollectEvent::BridgeMessage { account_name: Some(name), message }
            if name == "Checking (a1)" && message == "Rate limited"
    )));
    assert!(events.iter().any(|e| matches!(e, CollectEvent::Anomaly(_))));
}

#[test]
fn reports_per_account_progress_events() {
    let (_dir, mut storage) = temp_storage();
    let fetcher = FakeFetcher::new(set(vec![account("a1", "100.00", vec![])], &[])).with_account(
        "a1",
        set(
            vec![account("a1", "100.00", vec![txn("t1", 1000, false)])],
            &[],
        ),
    );

    let (_, events) = run(&fetcher, &mut storage, NOW);

    assert!(events.iter().any(|e| matches!(
        e,
        CollectEvent::AccountCollected {
            new_transactions: 1,
            existing_transactions: 0,
            ..
        }
    )));
}

#[test]
fn empty_bridge_response_collects_nothing() {
    let (_dir, mut storage) = temp_storage();
    let fetcher = FakeFetcher::new(set(Vec::new(), &["No institutions linked"]));

    let (report, events) = run(&fetcher, &mut storage, NOW);

    assert_eq!(report.accounts_returned, 0);
    assert_eq!(report.accounts, 0);
    assert_eq!(report.new_transactions, 0);
    assert_eq!(
        report.bridge_messages,
        vec!["No institutions linked".to_string()]
    );
    assert_eq!(events.len(), 1);
    // Nothing was persisted, matching the CLI's historical behavior.
    assert!(storage.get_warnings().unwrap().is_none());
    assert!(fetcher.requested_starts.borrow().is_empty());
}

#[test]
fn collects_into_memory_backed_storage() {
    let mut storage = JsonStorage::with_files(MemoryFiles::new()).unwrap();
    let fetcher = FakeFetcher::new(set(vec![account("a1", "100.00", vec![])], &[])).with_account(
        "a1",
        set(
            vec![account("a1", "100.00", vec![txn("t1", 1000, false)])],
            &[],
        ),
    );

    let (report, _) = run(&fetcher, &mut storage, NOW);

    assert_eq!(report.new_transactions, 1);
    assert_eq!(
        storage
            .get_transactions(&TransactionFilter::default())
            .unwrap()
            .len(),
        1
    );
    assert_eq!(storage.last_collected("a1").unwrap(), Some(1000));
    assert!(storage.get_warnings().unwrap().is_some());
}
