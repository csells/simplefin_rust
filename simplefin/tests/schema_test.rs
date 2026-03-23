use schemars::schema_for;

/// Verify that JSON Schema generation produces valid output for all key types.

#[test]
fn schema_net_worth_summary() {
    let schema = schema_for!(simplefin::NetWorthSummary);
    let json = serde_json::to_string(&schema).unwrap();
    let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();
    assert_eq!(parsed["title"], "NetWorthSummary");
    assert!(parsed["properties"]["net_worth"].is_object());
    assert!(parsed["properties"]["categories"].is_object());
}

#[test]
fn schema_unified_account() {
    let schema = schema_for!(simplefin::UnifiedAccount);
    let json = serde_json::to_string(&schema).unwrap();
    let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();
    assert_eq!(parsed["title"], "UnifiedAccount");
    assert!(parsed["properties"]["balance"].is_object());
    assert!(parsed["properties"]["source"].is_object());
}

#[test]
fn schema_transaction_with_context() {
    let schema = schema_for!(simplefin::TransactionWithContext);
    let json = serde_json::to_string(&schema).unwrap();
    let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();
    assert_eq!(parsed["title"], "TransactionWithContext");
    assert!(parsed["properties"]["amount"].is_object());
    assert!(parsed["properties"]["description"].is_object());
}

#[test]
fn schema_spending_summary() {
    let schema = schema_for!(simplefin::SpendingSummary);
    let json = serde_json::to_string(&schema).unwrap();
    let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();
    assert_eq!(parsed["title"], "SpendingSummary");
    assert!(parsed["properties"]["total_spending"].is_object());
}

#[test]
fn schema_storage_status() {
    let schema = schema_for!(simplefin::StorageStatus);
    let json = serde_json::to_string(&schema).unwrap();
    let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();
    assert_eq!(parsed["title"], "StorageStatus");
    assert!(parsed["properties"]["account_count"].is_object());
}

#[test]
fn schema_warning_record() {
    let schema = schema_for!(simplefin::WarningRecord);
    let json = serde_json::to_string(&schema).unwrap();
    let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();
    assert_eq!(parsed["title"], "WarningRecord");
    assert!(parsed["properties"]["anomalies"].is_object());
}

#[test]
fn schema_stale_account() {
    let schema = schema_for!(simplefin::StaleAccount);
    let json = serde_json::to_string(&schema).unwrap();
    let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();
    assert_eq!(parsed["title"], "StaleAccount");
    assert!(parsed["properties"]["refresh_days"].is_object());
}

#[test]
fn schema_balance_change() {
    let schema = schema_for!(simplefin::BalanceChange);
    let json = serde_json::to_string(&schema).unwrap();
    let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();
    assert_eq!(parsed["title"], "BalanceChange");
    assert!(parsed["properties"]["change"].is_object());
}

#[test]
fn schema_net_worth_time_point() {
    let schema = schema_for!(simplefin::NetWorthTimePoint);
    let json = serde_json::to_string(&schema).unwrap();
    let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();
    assert_eq!(parsed["title"], "NetWorthTimePoint");
    assert!(parsed["properties"]["timestamp"].is_object());
}

#[test]
fn schema_balance_snapshot() {
    let schema = schema_for!(simplefin::BalanceSnapshot);
    let json = serde_json::to_string(&schema).unwrap();
    let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();
    assert_eq!(parsed["title"], "BalanceSnapshot");
    assert!(parsed["properties"]["account_id"].is_object());
}
