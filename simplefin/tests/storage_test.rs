use std::str::FromStr;

use rust_decimal::Decimal;
use simplefin::models::{Account, Organization, Transaction};
use simplefin::storage::{
    AccountFilter, AccountSource, BalanceHistoryFilter, BalanceSnapshot, DataConfig, JsonStorage,
    ManualAccount, OrgFilter, Storage, TransactionFilter, unify_accounts,
};

fn test_org(id: &str, name: &str) -> Organization {
    Organization {
        sfin_url: format!("https://example.com/{id}"),
        domain: Some(format!("{id}.com")),
        name: Some(name.to_string()),
        url: None,
        id: Some(id.to_string()),
    }
}

fn test_account(id: &str, name: &str, org: &Organization) -> Account {
    Account {
        org: org.clone(),
        id: id.to_string(),
        name: name.to_string(),
        currency: "USD".to_string(),
        balance: Decimal::from_str("1000.00").unwrap(),
        available_balance: Some(Decimal::from_str("950.00").unwrap()),
        balance_date: 1700000000,
        transactions: Vec::new(),
    }
}

fn test_transaction(id: &str, amount: &str, posted: i64) -> Transaction {
    Transaction {
        id: id.to_string(),
        posted,
        amount: Decimal::from_str(amount).unwrap(),
        description: format!("Transaction {id}"),
        transacted_at: None,
        pending: false,
    }
}

fn open_temp_storage() -> (tempfile::TempDir, JsonStorage) {
    let dir = tempfile::tempdir().unwrap();
    let storage = JsonStorage::open(dir.path()).unwrap();
    (dir, storage)
}

// === Upsert idempotency ===

#[test]
fn upsert_organizations_idempotent() {
    let (_dir, mut storage) = open_temp_storage();
    let org = test_org("org1", "First National");

    storage.upsert_organizations(&[org.clone()]).unwrap();
    storage.upsert_organizations(&[org.clone()]).unwrap();

    let orgs = storage.get_organizations(&OrgFilter::default()).unwrap();
    assert_eq!(orgs.len(), 1);
    assert_eq!(orgs[0].name.as_deref(), Some("First National"));
}

#[test]
fn upsert_organizations_updates_metadata() {
    let (_dir, mut storage) = open_temp_storage();
    let org1 = test_org("org1", "Old Name");
    storage.upsert_organizations(&[org1]).unwrap();

    let org1_updated = test_org("org1", "New Name");
    storage.upsert_organizations(&[org1_updated]).unwrap();

    let orgs = storage.get_organizations(&OrgFilter::default()).unwrap();
    assert_eq!(orgs.len(), 1);
    assert_eq!(orgs[0].name.as_deref(), Some("New Name"));
}

#[test]
fn upsert_accounts_idempotent() {
    let (_dir, mut storage) = open_temp_storage();
    let org = test_org("org1", "Bank");
    let account = test_account("acc1", "Checking", &org);

    storage.upsert_accounts(&[account.clone()]).unwrap();
    storage.upsert_accounts(&[account.clone()]).unwrap();

    let accounts = storage.get_accounts(&AccountFilter::default()).unwrap();
    assert_eq!(accounts.len(), 1);
    assert_eq!(accounts[0].name, "Checking");
}

#[test]
fn upsert_accounts_updates_balance() {
    let (_dir, mut storage) = open_temp_storage();
    let org = test_org("org1", "Bank");

    let mut account = test_account("acc1", "Checking", &org);
    account.balance = Decimal::from_str("1000.00").unwrap();
    storage.upsert_accounts(&[account]).unwrap();

    let mut updated = test_account("acc1", "Checking", &org);
    updated.balance = Decimal::from_str("1500.00").unwrap();
    storage.upsert_accounts(&[updated]).unwrap();

    let accounts = storage.get_accounts(&AccountFilter::default()).unwrap();
    assert_eq!(accounts.len(), 1);
    assert_eq!(accounts[0].balance, Decimal::from_str("1500.00").unwrap());
}

#[test]
fn upsert_transactions_idempotent() {
    let (_dir, mut storage) = open_temp_storage();
    let txns = vec![
        test_transaction("tx1", "-50.00", 1700000000),
        test_transaction("tx2", "100.00", 1700001000),
    ];

    let new1 = storage.upsert_transactions("acc1", &txns).unwrap();
    assert_eq!(new1, 2);

    let new2 = storage.upsert_transactions("acc1", &txns).unwrap();
    assert_eq!(new2, 0); // all duplicates

    let _all = storage
        .get_transactions(&TransactionFilter {
            account_id: Some("acc1".to_string()),
            ..Default::default()
        })
        .unwrap();
}

#[test]
fn upsert_transactions_returns_correct_new_count() {
    let (_dir, mut storage) = open_temp_storage();

    let batch1 = vec![
        test_transaction("tx1", "-50.00", 1700000000),
        test_transaction("tx2", "100.00", 1700001000),
    ];
    let new = storage.upsert_transactions("acc1", &batch1).unwrap();
    assert_eq!(new, 2);

    let batch2 = vec![
        test_transaction("tx2", "100.00", 1700001000), // duplicate
        test_transaction("tx3", "-25.00", 1700002000), // new
    ];
    let new = storage.upsert_transactions("acc1", &batch2).unwrap();
    assert_eq!(new, 1);
}

// === Last collected tracking ===

#[test]
fn last_collected_initially_none() {
    let (_dir, storage) = open_temp_storage();
    assert_eq!(storage.last_collected("acc1").unwrap(), None);
}

#[test]
fn last_collected_tracks_per_account() {
    let (_dir, mut storage) = open_temp_storage();

    storage.set_last_collected("acc1", 1700000000).unwrap();
    storage.set_last_collected("acc2", 1700001000).unwrap();

    assert_eq!(storage.last_collected("acc1").unwrap(), Some(1700000000));
    assert_eq!(storage.last_collected("acc2").unwrap(), Some(1700001000));
    assert_eq!(storage.last_collected("acc3").unwrap(), None);
}

#[test]
fn last_collected_updates() {
    let (_dir, mut storage) = open_temp_storage();

    storage.set_last_collected("acc1", 1700000000).unwrap();
    assert_eq!(storage.last_collected("acc1").unwrap(), Some(1700000000));

    storage.set_last_collected("acc1", 1700005000).unwrap();
    assert_eq!(storage.last_collected("acc1").unwrap(), Some(1700005000));
}

// === Filter correctness ===

#[test]
fn filter_organizations_by_id() {
    let (_dir, mut storage) = open_temp_storage();
    storage
        .upsert_organizations(&[
            test_org("org1", "Bank A"),
            test_org("org2", "Bank B"),
        ])
        .unwrap();

    let filtered = storage
        .get_organizations(&OrgFilter {
            org_id: Some("org1".to_string()),
            ..Default::default()
        })
        .unwrap();
    assert_eq!(filtered.len(), 1);
    assert_eq!(filtered[0].name.as_deref(), Some("Bank A"));
}

#[test]
fn filter_organizations_by_name() {
    let (_dir, mut storage) = open_temp_storage();
    storage
        .upsert_organizations(&[
            test_org("org1", "Bank A"),
            test_org("org2", "Bank B"),
        ])
        .unwrap();

    let filtered = storage
        .get_organizations(&OrgFilter {
            name: Some("Bank B".to_string()),
            ..Default::default()
        })
        .unwrap();
    assert_eq!(filtered.len(), 1);
    assert_eq!(filtered[0].id.as_deref(), Some("org2"));
}

#[test]
fn filter_accounts_by_id() {
    let (_dir, mut storage) = open_temp_storage();
    let org = test_org("org1", "Bank");
    storage
        .upsert_accounts(&[
            test_account("acc1", "Checking", &org),
            test_account("acc2", "Savings", &org),
        ])
        .unwrap();

    let filtered = storage
        .get_accounts(&AccountFilter {
            account_id: Some("acc2".to_string()),
            ..Default::default()
        })
        .unwrap();
    assert_eq!(filtered.len(), 1);
    assert_eq!(filtered[0].name, "Savings");
}

#[test]
fn filter_accounts_by_org() {
    let (_dir, mut storage) = open_temp_storage();
    let org1 = test_org("org1", "Bank A");
    let org2 = test_org("org2", "Bank B");
    storage
        .upsert_accounts(&[
            test_account("acc1", "Checking", &org1),
            test_account("acc2", "Savings", &org2),
        ])
        .unwrap();

    let filtered = storage
        .get_accounts(&AccountFilter {
            org_id: Some("org2".to_string()),
            ..Default::default()
        })
        .unwrap();
    assert_eq!(filtered.len(), 1);
    assert_eq!(filtered[0].name, "Savings");
}

#[test]
fn filter_transactions_by_date_range() {
    let (_dir, mut storage) = open_temp_storage();
    let org = test_org("org1", "Bank");
    let account = test_account("acc1", "Checking", &org);
    storage.upsert_organizations(&[org]).unwrap();
    storage.upsert_accounts(&[account]).unwrap();
    storage
        .upsert_transactions(
            "acc1",
            &[
                test_transaction("tx1", "-50.00", 1700000000),
                test_transaction("tx2", "-25.00", 1700050000),
                test_transaction("tx3", "100.00", 1700100000),
            ],
        )
        .unwrap();

    let filtered = storage
        .get_transactions(&TransactionFilter {
            start_date: Some(1700040000),
            end_date: Some(1700090000),
            ..Default::default()
        })
        .unwrap();
    assert_eq!(filtered.len(), 1);
    assert_eq!(filtered[0].id, "tx2");
}

#[test]
fn filter_transactions_excludes_pending_by_default() {
    let (_dir, mut storage) = open_temp_storage();
    let org = test_org("org1", "Bank");
    let account = test_account("acc1", "Checking", &org);
    storage.upsert_organizations(&[org]).unwrap();
    storage.upsert_accounts(&[account]).unwrap();

    let mut pending_tx = test_transaction("tx1", "-50.00", 1700000000);
    pending_tx.pending = true;
    let settled_tx = test_transaction("tx2", "100.00", 1700001000);
    storage
        .upsert_transactions("acc1", &[pending_tx, settled_tx])
        .unwrap();

    let filtered = storage
        .get_transactions(&TransactionFilter {
            include_pending: Some(false),
            ..Default::default()
        })
        .unwrap();
    assert_eq!(filtered.len(), 1);
    assert_eq!(filtered[0].id, "tx2");
}

#[test]
fn filter_transactions_includes_pending_when_requested() {
    let (_dir, mut storage) = open_temp_storage();
    let org = test_org("org1", "Bank");
    let account = test_account("acc1", "Checking", &org);
    storage.upsert_organizations(&[org]).unwrap();
    storage.upsert_accounts(&[account]).unwrap();

    let mut pending_tx = test_transaction("tx1", "-50.00", 1700000000);
    pending_tx.pending = true;
    let settled_tx = test_transaction("tx2", "100.00", 1700001000);
    storage
        .upsert_transactions("acc1", &[pending_tx, settled_tx])
        .unwrap();

    let filtered = storage
        .get_transactions(&TransactionFilter {
            include_pending: Some(true),
            ..Default::default()
        })
        .unwrap();
    assert_eq!(filtered.len(), 2);
}

#[test]
fn transaction_with_context_has_correct_fields() {
    let (_dir, mut storage) = open_temp_storage();
    let org = test_org("org1", "First National");
    let account = test_account("acc1", "Checking", &org);
    storage.upsert_organizations(&[org]).unwrap();
    storage.upsert_accounts(&[account]).unwrap();
    storage
        .upsert_transactions("acc1", &[test_transaction("tx1", "-42.50", 1700000000)])
        .unwrap();

    let results = storage
        .get_transactions(&TransactionFilter::default())
        .unwrap();
    assert_eq!(results.len(), 1);

    let twc = &results[0];
    assert_eq!(twc.id, "tx1");
    assert_eq!(twc.account_id, "acc1");
    assert_eq!(twc.account_name, "Checking");
    assert_eq!(twc.org_name, "First National");
    assert_eq!(twc.currency, "USD");
    assert_eq!(twc.posted, 1700000000);
    assert_eq!(twc.amount, Decimal::from_str("-42.50").unwrap());
    assert_eq!(twc.description, "Transaction tx1");
    assert!(!twc.pending);
}

#[test]
fn filter_no_match_returns_empty() {
    let (_dir, mut storage) = open_temp_storage();
    let org = test_org("org1", "Bank");
    storage.upsert_organizations(&[org]).unwrap();

    let filtered = storage
        .get_organizations(&OrgFilter {
            org_id: Some("nonexistent".to_string()),
            ..Default::default()
        })
        .unwrap();
    assert!(filtered.is_empty());
}

#[test]
fn empty_storage_returns_empty_collections() {
    let (_dir, storage) = open_temp_storage();
    assert!(storage.get_organizations(&OrgFilter::default()).unwrap().is_empty());
    assert!(storage.get_accounts(&AccountFilter::default()).unwrap().is_empty());
    assert!(storage.get_transactions(&TransactionFilter::default()).unwrap().is_empty());
}

#[test]
fn multiple_accounts_transactions_isolated() {
    let (_dir, mut storage) = open_temp_storage();
    let org = test_org("org1", "Bank");
    let acc1 = test_account("acc1", "Checking", &org);
    let acc2 = test_account("acc2", "Savings", &org);
    storage.upsert_organizations(&[org]).unwrap();
    storage.upsert_accounts(&[acc1, acc2]).unwrap();

    storage
        .upsert_transactions("acc1", &[test_transaction("tx1", "-50.00", 1700000000)])
        .unwrap();
    storage
        .upsert_transactions("acc2", &[test_transaction("tx2", "200.00", 1700001000)])
        .unwrap();

    let acc1_txns = storage
        .get_transactions(&TransactionFilter {
            account_id: Some("acc1".to_string()),
            ..Default::default()
        })
        .unwrap();
    assert_eq!(acc1_txns.len(), 1);
    assert_eq!(acc1_txns[0].id, "tx1");

    let acc2_txns = storage
        .get_transactions(&TransactionFilter {
            account_id: Some("acc2".to_string()),
            ..Default::default()
        })
        .unwrap();
    assert_eq!(acc2_txns.len(), 1);
    assert_eq!(acc2_txns[0].id, "tx2");

    let all_txns = storage
        .get_transactions(&TransactionFilter::default())
        .unwrap();
    assert_eq!(all_txns.len(), 2);
}

// === Manual accounts ===

#[test]
fn upsert_manual_account_creates_new() {
    let (_dir, mut storage) = open_temp_storage();
    let manual = ManualAccount {
        id: "manual-fua".to_string(),
        name: "My 401k".to_string(),
        org_name: "My Provider".to_string(),
        currency: "USD".to_string(),
        refresh_days: 1,
    };
    storage.upsert_manual_accounts(&[manual]).unwrap();

    let accounts = storage.get_manual_accounts().unwrap();
    assert_eq!(accounts.len(), 1);
    assert_eq!(accounts[0].name, "My 401k");
}

#[test]
fn upsert_manual_account_idempotent() {
    let (_dir, mut storage) = open_temp_storage();
    let manual = ManualAccount {
        id: "manual-fua".to_string(),
        name: "My 401k".to_string(),
        org_name: "My Provider".to_string(),
        currency: "USD".to_string(),
        refresh_days: 1,
    };
    storage.upsert_manual_accounts(&[manual.clone()]).unwrap();
    storage.upsert_manual_accounts(&[manual]).unwrap();

    let accounts = storage.get_manual_accounts().unwrap();
    assert_eq!(accounts.len(), 1);
}

#[test]
fn upsert_manual_account_updates_name() {
    let (_dir, mut storage) = open_temp_storage();
    let v1 = ManualAccount {
        id: "manual-fua".to_string(),
        name: "Old Name".to_string(),
        org_name: "My Provider".to_string(),
        currency: "USD".to_string(),
        refresh_days: 1,
    };
    storage.upsert_manual_accounts(&[v1]).unwrap();

    let v2 = ManualAccount {
        id: "manual-fua".to_string(),
        name: "New Name".to_string(),
        org_name: "My Provider".to_string(),
        currency: "USD".to_string(),
        refresh_days: 1,
    };
    storage.upsert_manual_accounts(&[v2]).unwrap();

    let accounts = storage.get_manual_accounts().unwrap();
    assert_eq!(accounts[0].name, "New Name");
}

// === Balance history ===

#[test]
fn record_balance_and_retrieve() {
    let (_dir, mut storage) = open_temp_storage();
    let bal = Decimal::from_str("10652.67").unwrap();
    storage.record_balance("acc1", 1700000000, bal).unwrap();

    let history = storage
        .get_balance_history(&BalanceHistoryFilter {
            account_id: Some("acc1".to_string()),
            ..Default::default()
        })
        .unwrap();
    assert_eq!(history.len(), 1);
    assert_eq!(history[0].account_id, "acc1");
    assert_eq!(history[0].timestamp, 1700000000);
    assert_eq!(history[0].balance, bal);
}

#[test]
fn balance_history_accumulates_over_time() {
    let (_dir, mut storage) = open_temp_storage();
    storage
        .record_balance("acc1", 1700000000, Decimal::from_str("1000.00").unwrap())
        .unwrap();
    storage
        .record_balance("acc1", 1700100000, Decimal::from_str("1500.00").unwrap())
        .unwrap();
    storage
        .record_balance("acc1", 1700200000, Decimal::from_str("1200.00").unwrap())
        .unwrap();

    let history = storage
        .get_balance_history(&BalanceHistoryFilter {
            account_id: Some("acc1".to_string()),
            ..Default::default()
        })
        .unwrap();
    assert_eq!(history.len(), 3);
    // Should be in chronological order
    assert!(history[0].timestamp <= history[1].timestamp);
    assert!(history[1].timestamp <= history[2].timestamp);
}

#[test]
fn balance_history_isolated_per_account() {
    let (_dir, mut storage) = open_temp_storage();
    storage
        .record_balance("acc1", 1700000000, Decimal::from_str("1000.00").unwrap())
        .unwrap();
    storage
        .record_balance("acc2", 1700000000, Decimal::from_str("5000.00").unwrap())
        .unwrap();

    let h1 = storage
        .get_balance_history(&BalanceHistoryFilter {
            account_id: Some("acc1".to_string()),
            ..Default::default()
        })
        .unwrap();
    assert_eq!(h1.len(), 1);
    assert_eq!(h1[0].balance, Decimal::from_str("1000.00").unwrap());

    let h2 = storage
        .get_balance_history(&BalanceHistoryFilter {
            account_id: Some("acc2".to_string()),
            ..Default::default()
        })
        .unwrap();
    assert_eq!(h2.len(), 1);
    assert_eq!(h2[0].balance, Decimal::from_str("5000.00").unwrap());
}

#[test]
fn balance_history_filter_by_date_range() {
    let (_dir, mut storage) = open_temp_storage();
    storage
        .record_balance("acc1", 1700000000, Decimal::from_str("1000.00").unwrap())
        .unwrap();
    storage
        .record_balance("acc1", 1700100000, Decimal::from_str("1500.00").unwrap())
        .unwrap();
    storage
        .record_balance("acc1", 1700200000, Decimal::from_str("1200.00").unwrap())
        .unwrap();

    let filtered = storage
        .get_balance_history(&BalanceHistoryFilter {
            account_id: Some("acc1".to_string()),
            start_date: Some(1700050000),
            end_date: Some(1700150000),
        })
        .unwrap();
    assert_eq!(filtered.len(), 1);
    assert_eq!(filtered[0].balance, Decimal::from_str("1500.00").unwrap());
}

#[test]
fn balance_history_all_accounts() {
    let (_dir, mut storage) = open_temp_storage();
    storage
        .record_balance("acc1", 1700000000, Decimal::from_str("1000.00").unwrap())
        .unwrap();
    storage
        .record_balance("acc2", 1700000000, Decimal::from_str("5000.00").unwrap())
        .unwrap();

    let all = storage
        .get_balance_history(&BalanceHistoryFilter::default())
        .unwrap();
    assert_eq!(all.len(), 2);
}

#[test]
fn balance_history_empty_initially() {
    let (_dir, storage) = open_temp_storage();
    let history = storage
        .get_balance_history(&BalanceHistoryFilter::default())
        .unwrap();
    assert!(history.is_empty());
}

// === Balance dedup ===

#[test]
fn record_balance_skips_duplicate() {
    let (_dir, mut storage) = open_temp_storage();
    let bal = Decimal::from_str("1000.00").unwrap();
    storage.record_balance("acc1", 1700000000, bal).unwrap();
    storage.record_balance("acc1", 1700100000, bal).unwrap();

    let history = storage
        .get_balance_history(&BalanceHistoryFilter {
            account_id: Some("acc1".to_string()),
            ..Default::default()
        })
        .unwrap();
    assert_eq!(history.len(), 1);
}

#[test]
fn record_balance_records_different_values() {
    let (_dir, mut storage) = open_temp_storage();
    storage
        .record_balance("acc1", 1700000000, Decimal::from_str("1000.00").unwrap())
        .unwrap();
    storage
        .record_balance("acc1", 1700100000, Decimal::from_str("1500.00").unwrap())
        .unwrap();

    let history = storage
        .get_balance_history(&BalanceHistoryFilter {
            account_id: Some("acc1".to_string()),
            ..Default::default()
        })
        .unwrap();
    assert_eq!(history.len(), 2);
}

#[test]
fn record_balance_records_change_back_to_original() {
    let (_dir, mut storage) = open_temp_storage();
    storage
        .record_balance("acc1", 1700000000, Decimal::from_str("1000.00").unwrap())
        .unwrap();
    storage
        .record_balance("acc1", 1700100000, Decimal::from_str("1500.00").unwrap())
        .unwrap();
    storage
        .record_balance("acc1", 1700200000, Decimal::from_str("1000.00").unwrap())
        .unwrap();

    let history = storage
        .get_balance_history(&BalanceHistoryFilter {
            account_id: Some("acc1".to_string()),
            ..Default::default()
        })
        .unwrap();
    // All 3 recorded because the balance changed each time (even back to original)
    assert_eq!(history.len(), 3);
}

// === Unified accounts ===

#[test]
fn unify_merges_both_sources() {
    let org = test_org("org1", "Bank");
    let sf_account = test_account("acc1", "Checking", &org);
    let manual = ManualAccount {
        id: "manual-test".to_string(),
        name: "Test 401k".to_string(),
        org_name: "TestOrg".to_string(),
        currency: "USD".to_string(),
        refresh_days: 1,
    };
    let history = vec![BalanceSnapshot {
        account_id: "manual-test".to_string(),
        timestamp: 1700000000,
        balance: Decimal::from_str("50000.00").unwrap(),
    }];

    let unified = unify_accounts(&[sf_account], &[manual], &history);
    assert_eq!(unified.len(), 2);
    assert_eq!(unified[0].source, AccountSource::Simplefin);
    assert_eq!(unified[1].source, AccountSource::Manual);
    assert_eq!(
        unified[1].balance,
        Decimal::from_str("50000.00").unwrap()
    );
}

#[test]
fn unify_uses_latest_balance_for_manual() {
    let manual = ManualAccount {
        id: "manual-test".to_string(),
        name: "Test".to_string(),
        org_name: "Org".to_string(),
        currency: "USD".to_string(),
        refresh_days: 1,
    };
    let history = vec![
        BalanceSnapshot {
            account_id: "manual-test".to_string(),
            timestamp: 100,
            balance: Decimal::from_str("1000.00").unwrap(),
        },
        BalanceSnapshot {
            account_id: "manual-test".to_string(),
            timestamp: 200,
            balance: Decimal::from_str("2000.00").unwrap(),
        },
    ];

    let unified = unify_accounts(&[], &[manual], &history);
    assert_eq!(unified[0].balance, Decimal::from_str("2000.00").unwrap());
}

#[test]
fn unify_zero_balance_when_no_history() {
    let manual = ManualAccount {
        id: "manual-test".to_string(),
        name: "Test".to_string(),
        org_name: "Org".to_string(),
        currency: "USD".to_string(),
        refresh_days: 1,
    };

    let unified = unify_accounts(&[], &[manual], &[]);
    assert_eq!(unified[0].balance, Decimal::ZERO);
    assert!(unified[0].balance_date.is_none());
}

// === Config ===

#[test]
fn config_default_when_missing() {
    let (_dir, storage) = open_temp_storage();
    let config = storage.get_config().unwrap();
    assert!(config.excluded_account_patterns.is_empty());
    assert!(config.classification_overrides.is_empty());
}

#[test]
fn config_roundtrip() {
    let (_dir, storage) = open_temp_storage();
    let config = DataConfig {
        excluded_account_patterns: vec!["A. SMITH".to_string()],
        ..Default::default()
    };
    storage.set_config(&config).unwrap();

    let loaded = storage.get_config().unwrap();
    assert_eq!(loaded.excluded_account_patterns, vec!["A. SMITH"]);
}

#[test]
fn config_with_classification_overrides() {
    use simplefin::AccountCategory;
    use std::collections::HashMap;

    let (_dir, storage) = open_temp_storage();
    let mut overrides = HashMap::new();
    overrides.insert("acc-1".to_string(), AccountCategory::Cash);

    let config = DataConfig {
        classification_overrides: overrides,
        ..Default::default()
    };
    storage.set_config(&config).unwrap();

    let loaded = storage.get_config().unwrap();
    assert_eq!(
        loaded.classification_overrides.get("acc-1"),
        Some(&AccountCategory::Cash)
    );
}

// === Stale accounts ===

#[test]
fn stale_no_manual_accounts() {
    let (_dir, storage) = open_temp_storage();
    let stale = storage.get_stale_accounts(1_000_000).unwrap();
    assert!(stale.is_empty());
}

#[test]
fn stale_account_with_no_balance_history() {
    let (_dir, mut storage) = open_temp_storage();
    let manual = ManualAccount {
        id: "manual-test".to_string(),
        name: "Test Account".to_string(),
        org_name: "TestOrg".to_string(),
        currency: "USD".to_string(),
        refresh_days: 1,
    };
    storage.upsert_manual_accounts(&[manual]).unwrap();

    let stale = storage.get_stale_accounts(1_000_000).unwrap();
    assert_eq!(stale.len(), 1);
    assert_eq!(stale[0].id, "manual-test");
    assert!(stale[0].last_updated.is_none());
    assert!(stale[0].days_since_update.is_none());
}

#[test]
fn stale_account_recently_updated() {
    let (_dir, mut storage) = open_temp_storage();
    let manual = ManualAccount {
        id: "manual-test".to_string(),
        name: "Test".to_string(),
        org_name: "Org".to_string(),
        currency: "USD".to_string(),
        refresh_days: 7,
    };
    storage.upsert_manual_accounts(&[manual]).unwrap();
    let now = 1_000_000i64;
    // Updated 2 days ago — refresh is 7 days, so not stale
    storage
        .record_balance("manual-test", now - 2 * 86400, Decimal::from_str("100").unwrap())
        .unwrap();

    let stale = storage.get_stale_accounts(now).unwrap();
    assert!(stale.is_empty());
}

#[test]
fn stale_account_overdue() {
    let (_dir, mut storage) = open_temp_storage();
    let manual = ManualAccount {
        id: "manual-test".to_string(),
        name: "Test".to_string(),
        org_name: "Org".to_string(),
        currency: "USD".to_string(),
        refresh_days: 1,
    };
    storage.upsert_manual_accounts(&[manual]).unwrap();
    let now = 1_000_000i64;
    // Updated 3 days ago — refresh is 1 day, so stale
    storage
        .record_balance("manual-test", now - 3 * 86400, Decimal::from_str("100").unwrap())
        .unwrap();

    let stale = storage.get_stale_accounts(now).unwrap();
    assert_eq!(stale.len(), 1);
    assert_eq!(stale[0].days_since_update, Some(3));
}

#[test]
fn stale_respects_per_account_refresh_days() {
    let (_dir, mut storage) = open_temp_storage();
    let daily = ManualAccount {
        id: "daily".to_string(),
        name: "Daily".to_string(),
        org_name: "Org".to_string(),
        currency: "USD".to_string(),
        refresh_days: 1,
    };
    let monthly = ManualAccount {
        id: "monthly".to_string(),
        name: "Monthly".to_string(),
        org_name: "Org".to_string(),
        currency: "USD".to_string(),
        refresh_days: 30,
    };
    storage.upsert_manual_accounts(&[daily, monthly]).unwrap();

    let now = 1_000_000i64;
    let five_days_ago = now - 5 * 86400;
    storage
        .record_balance("daily", five_days_ago, Decimal::from_str("100").unwrap())
        .unwrap();
    storage
        .record_balance("monthly", five_days_ago, Decimal::from_str("500000").unwrap())
        .unwrap();

    let stale = storage.get_stale_accounts(now).unwrap();
    // daily is stale (5 >= 1), monthly is not (5 < 30)
    assert_eq!(stale.len(), 1);
    assert_eq!(stale[0].id, "daily");
}
