//! `FileStore` contract tests for `DiskFiles` and `MemoryFiles`, plus
//! `JsonStorage` round-trips over the in-memory store.

use std::str::FromStr;

use rust_decimal::Decimal;
use simplefin::models::{Account, Organization, Transaction};
use simplefin::storage::{
    AccountFilter, BalanceHistoryFilter, DataConfig, DiskFiles, FileStore, JsonStorage,
    MemoryFiles, Storage, TransactionFilter, WarningRecord,
};
use simplefin::{Anomaly, TransactionEdit, TransactionEdits};

// === FileStore contract, run against both implementations ===

fn assert_read_absent_is_none(files: &impl FileStore) {
    assert_eq!(files.read("missing.json").unwrap(), None);
    assert_eq!(files.read("nested/missing.json").unwrap(), None);
}

fn assert_write_then_read(files: &impl FileStore) {
    files.write("a.json", "one").unwrap();
    assert_eq!(files.read("a.json").unwrap().as_deref(), Some("one"));
    files.write("a.json", "two").unwrap();
    assert_eq!(files.read("a.json").unwrap().as_deref(), Some("two"));
}

fn assert_write_creates_parents(files: &impl FileStore) {
    files.write("deep/er/file.json", "x").unwrap();
    assert_eq!(
        files.read("deep/er/file.json").unwrap().as_deref(),
        Some("x")
    );
    assert_eq!(
        files.list("deep/er").unwrap(),
        vec!["file.json".to_string()]
    );
}

fn assert_list_direct_children_only(files: &impl FileStore) {
    assert!(
        files.list("dir").unwrap().is_empty(),
        "absent dir lists empty"
    );
    files.write("dir/b.json", "b").unwrap();
    files.write("dir/a.json", "a").unwrap();
    files.write("dir/sub/c.json", "c").unwrap();
    files.write("dirx/d.json", "d").unwrap();
    files.write("top.json", "t").unwrap();
    assert_eq!(
        files.list("dir").unwrap(),
        vec!["a.json".to_string(), "b.json".to_string()]
    );
}

fn assert_remove_is_idempotent(files: &impl FileStore) {
    files.write("gone.json", "x").unwrap();
    files.remove("gone.json").unwrap();
    assert_eq!(files.read("gone.json").unwrap(), None);
    files.remove("gone.json").unwrap();
    files.remove("never/existed.json").unwrap();
}

fn assert_contract(files: &impl FileStore) {
    assert_read_absent_is_none(files);
    assert_write_then_read(files);
    assert_write_creates_parents(files);
    assert_list_direct_children_only(files);
    assert_remove_is_idempotent(files);
}

#[test]
fn memory_files_satisfies_file_store_contract() {
    assert_contract(&MemoryFiles::new());
}

#[test]
fn disk_files_satisfies_file_store_contract() {
    let dir = tempfile::tempdir().unwrap();
    assert_contract(&DiskFiles::new(dir.path()));
}

#[test]
fn disk_files_write_leaves_no_temp_file() {
    let dir = tempfile::tempdir().unwrap();
    let files = DiskFiles::new(dir.path());
    files.write("transactions/acc.json", "[]").unwrap();
    assert_eq!(
        files.list("transactions").unwrap(),
        vec!["acc.json".to_string()]
    );
    assert!(dir.path().join("transactions").join("acc.json").is_file());
}

#[test]
fn memory_files_clones_share_one_map() {
    let files = MemoryFiles::new();
    let clone = files.clone();
    clone.write("shared.json", "yes").unwrap();
    assert_eq!(files.read("shared.json").unwrap().as_deref(), Some("yes"));
}

#[test]
fn memory_files_entries_round_trip() {
    let files = MemoryFiles::from_entries([
        ("b/x.json".to_string(), "2".to_string()),
        ("a.json".to_string(), "1".to_string()),
    ]);
    assert_eq!(files.read("b/x.json").unwrap().as_deref(), Some("2"));
    assert_eq!(
        files.entries(),
        vec![
            ("a.json".to_string(), "1".to_string()),
            ("b/x.json".to_string(), "2".to_string()),
        ]
    );
}

// === JsonStorage over DiskFiles keeps the on-disk layout ===

#[test]
fn open_keeps_on_disk_layout() {
    let dir = tempfile::tempdir().unwrap();
    let mut storage = JsonStorage::open(dir.path()).unwrap();
    assert!(dir.path().join("transactions").is_dir());
    assert!(dir.path().join("balance_history").is_dir());

    let org = org();
    storage
        .upsert_organizations(std::slice::from_ref(&org))
        .unwrap();
    storage.upsert_accounts(&[account("acc1", &org)]).unwrap();
    storage
        .upsert_transactions("acc1", &[txn("t1", "-5.00", 1_700_000_000)])
        .unwrap();
    storage
        .record_balance("acc1", 1_700_000_000, dec("100.00"))
        .unwrap();

    for file in [
        "organizations.json",
        "accounts.json",
        "balance_verified.json",
        "transactions/acc1.json",
        "balance_history/acc1.json",
    ] {
        assert!(dir.path().join(file).is_file(), "{file} on disk");
    }
}

#[test]
fn orphan_paths_are_relative_to_the_store() {
    let files = MemoryFiles::new();
    let mut storage = JsonStorage::with_files(files.clone()).unwrap();
    storage
        .record_balance("orphan", 1_700_000_000, dec("1.00"))
        .unwrap();
    let orphans = storage.find_orphaned_data().unwrap();
    assert_eq!(orphans.len(), 1);
    assert_eq!(orphans[0].path, "balance_history/orphan.json");

    storage.remove_orphaned_data(&orphans).unwrap();
    assert_eq!(files.read("balance_history/orphan.json").unwrap(), None);
}

// === JsonStorage over MemoryFiles ===

fn dec(s: &str) -> Decimal {
    Decimal::from_str(s).unwrap()
}

fn org() -> Organization {
    Organization {
        sfin_url: "https://example.com/bank".into(),
        domain: None,
        name: Some("Example Bank".into()),
        url: None,
        id: Some("org-1".into()),
    }
}

fn account(id: &str, org: &Organization) -> Account {
    Account {
        org: org.clone(),
        id: id.into(),
        name: format!("Checking ({id})"),
        currency: "USD".into(),
        balance: dec("100.00"),
        available_balance: None,
        balance_date: 1_700_000_000,
        transactions: Vec::new(),
    }
}

fn txn(id: &str, amount: &str, posted: i64) -> Transaction {
    Transaction {
        id: id.into(),
        posted,
        amount: dec(amount),
        description: format!("Purchase {id}"),
        transacted_at: None,
        pending: false,
        payee: None,
        memo: None,
    }
}

fn json<T: serde::Serialize>(value: &T) -> serde_json::Value {
    serde_json::to_value(value).unwrap()
}

/// Populates every kind of stored data.
fn populate(storage: &mut JsonStorage<MemoryFiles>) {
    let org = org();
    storage
        .upsert_organizations(std::slice::from_ref(&org))
        .unwrap();
    storage
        .upsert_accounts(&[account("acc1", &org), account("acc2", &org)])
        .unwrap();
    storage
        .upsert_transactions(
            "acc1",
            &[
                txn("t1", "-12.34", 1_700_000_000),
                txn("t2", "50.00", 1_700_086_400),
            ],
        )
        .unwrap();
    storage
        .upsert_transactions("acc2", &[txn("t3", "-1.00", 1_700_000_500)])
        .unwrap();
    storage.set_last_collected("acc1", 1_700_086_400).unwrap();
    storage
        .record_balance("acc1", 1_700_000_000, dec("100.00"))
        .unwrap();
    storage
        .record_balance("acc1", 1_700_086_400, dec("87.66"))
        .unwrap();

    let mut config = DataConfig::default();
    config
        .excluded_account_patterns
        .push("Duplicate Account".into());
    config
        .display_names
        .insert("acc1".into(), "My Checking".into());
    storage.set_config(&config).unwrap();

    let mut edits = TransactionEdits::default();
    edits.set(
        "acc1",
        "t1",
        TransactionEdit {
            description: Some("Groceries".into()),
            category: Some("groceries".into()),
            hidden: false,
        },
    );
    storage.set_transaction_edits(&edits).unwrap();

    storage
        .save_warnings(&WarningRecord {
            timestamp: 1_700_086_400,
            anomalies: vec![Anomaly::NewAccount {
                account_id: "acc2".into(),
                account_name: "Checking (acc2)".into(),
                balance: dec("100.00"),
            }],
            bridge_messages: vec!["bridge says hi".into()],
        })
        .unwrap();
}

/// Everything readable through `Storage`, as JSON for comparison.
fn snapshot(storage: &JsonStorage<MemoryFiles>) -> serde_json::Value {
    serde_json::json!({
        "orgs": json(&storage.get_organizations(&Default::default()).unwrap()),
        "accounts": json(&storage.get_accounts(&AccountFilter::default()).unwrap()),
        "transactions": json(&storage.get_transactions(&TransactionFilter::default()).unwrap()),
        "last_collected": storage.last_collected("acc1").unwrap(),
        "balances": json(&storage.get_balance_history(&BalanceHistoryFilter::default()).unwrap()),
        "config": json(&storage.get_config().unwrap()),
        "edits": json(&storage.get_transaction_edits().unwrap()),
        "warnings": json(&storage.get_warnings().unwrap()),
    })
}

#[test]
fn memory_storage_round_trips_accounts_and_transactions() {
    let mut storage = JsonStorage::with_files(MemoryFiles::new()).unwrap();
    populate(&mut storage);

    let accounts = storage.get_accounts(&AccountFilter::default()).unwrap();
    assert_eq!(accounts.len(), 2);
    assert_eq!(accounts[0].name, "Checking (acc1)");
    assert!(
        accounts[0].transactions.is_empty(),
        "transactions stored separately"
    );

    let txns = storage
        .get_transactions(&TransactionFilter {
            account_id: Some("acc1".into()),
            ..Default::default()
        })
        .unwrap();
    assert_eq!(txns.len(), 2);
    assert_eq!(txns[0].amount, dec("-12.34"));
    assert_eq!(storage.last_collected("acc1").unwrap(), Some(1_700_086_400));
    assert_eq!(
        storage.max_stored_posted("acc2").unwrap(),
        Some(1_700_000_500)
    );
}

#[test]
fn memory_storage_round_trips_balances() {
    let mut storage = JsonStorage::with_files(MemoryFiles::new()).unwrap();
    populate(&mut storage);
    let history = storage
        .get_balance_history(&BalanceHistoryFilter {
            account_id: Some("acc1".into()),
            ..Default::default()
        })
        .unwrap();
    let balances: Vec<Decimal> = history.iter().map(|s| s.balance).collect();
    assert_eq!(balances, vec![dec("100.00"), dec("87.66")]);
}

#[test]
fn memory_storage_round_trips_config_edits_and_warnings() {
    let mut storage = JsonStorage::with_files(MemoryFiles::new()).unwrap();
    populate(&mut storage);

    let config = storage.get_config().unwrap();
    assert_eq!(config.excluded_account_patterns, vec!["Duplicate Account"]);
    assert_eq!(config.display_names["acc1"], "My Checking");

    let edits = storage.get_transaction_edits().unwrap();
    assert_eq!(
        edits.get("acc1", "t1").unwrap().description.as_deref(),
        Some("Groceries")
    );

    let warnings = storage.get_warnings().unwrap().unwrap();
    assert_eq!(warnings.bridge_messages, vec!["bridge says hi"]);
    assert_eq!(warnings.anomalies.len(), 1);
}

#[test]
fn memory_storage_fresh_is_empty() {
    let storage = JsonStorage::with_files(MemoryFiles::new()).unwrap();
    assert!(
        storage
            .get_accounts(&AccountFilter::default())
            .unwrap()
            .is_empty()
    );
    assert!(storage.get_warnings().unwrap().is_none());
    assert!(storage.find_orphaned_data().unwrap().is_empty());
}

#[test]
fn memory_entries_restore_identical_data() {
    let files = MemoryFiles::new();
    let mut storage = JsonStorage::with_files(files.clone()).unwrap();
    populate(&mut storage);
    let original = snapshot(&storage);

    let restored_files = MemoryFiles::from_entries(files.entries());
    assert_eq!(restored_files.entries(), files.entries());
    let restored = JsonStorage::with_files(restored_files).unwrap();
    assert_eq!(snapshot(&restored), original);
}

#[test]
fn memory_storage_matches_disk_storage_bytes() {
    let files = MemoryFiles::new();
    let mut memory = JsonStorage::with_files(files.clone()).unwrap();
    populate(&mut memory);

    let dir = tempfile::tempdir().unwrap();
    let disk_files = DiskFiles::new(dir.path());
    let mut disk = JsonStorage::with_files(disk_files.clone()).unwrap();
    let org = org();
    disk.upsert_organizations(std::slice::from_ref(&org))
        .unwrap();
    disk.upsert_accounts(&[account("acc1", &org), account("acc2", &org)])
        .unwrap();

    for path in ["organizations.json", "accounts.json"] {
        assert_eq!(files.read(path).unwrap(), disk_files.read(path).unwrap());
    }
}
