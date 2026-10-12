//! Category kinds, the transaction-edit overlay, and the shared categorizer.

use rust_decimal_macros::dec;
use simplefin::models::Transaction;
use simplefin::storage::{DataConfig, JsonStorage, Storage, TransactionWithContext};
use simplefin::{
    Categorizer, CategoryKind, SpendingRule, TransactionEdit, TransactionEdits,
    apply_category_to_future, category_kind, default_category_kind, effective_category,
    effective_description, normalize_category_name, rule_from_transaction,
};

fn txn(account_id: &str, id: &str, description: &str) -> TransactionWithContext {
    TransactionWithContext {
        id: id.into(),
        account_id: account_id.into(),
        account_name: "Checking (1234)".into(),
        org_name: "Example Bank".into(),
        currency: "USD".into(),
        posted: 1_700_000_000,
        amount: dec!(-12.34),
        description: description.into(),
        ..Default::default()
    }
}

fn rule(pattern: &str, category: &str) -> SpendingRule {
    SpendingRule {
        pattern: pattern.into(),
        category: category.into(),
    }
}

fn temp_storage() -> (tempfile::TempDir, JsonStorage) {
    let dir = tempfile::tempdir().unwrap();
    let storage = JsonStorage::open(dir.path()).unwrap();
    (dir, storage)
}

// === Category kinds ===

#[test]
fn default_kinds_are_data_driven_by_category_name() {
    assert_eq!(default_category_kind("income"), CategoryKind::Income);
    assert_eq!(default_category_kind("transfer"), CategoryKind::Transfer);
    assert_eq!(default_category_kind("groceries"), CategoryKind::Expense);
    assert_eq!(default_category_kind("other"), CategoryKind::Expense);
    assert_eq!(default_category_kind("donations"), CategoryKind::Expense);
}

#[test]
fn config_overrides_category_kind() {
    let mut config = DataConfig::default();
    config
        .category_kinds
        .insert("side_gig".into(), CategoryKind::Income);
    config
        .category_kinds
        .insert("income".into(), CategoryKind::Transfer);

    assert_eq!(
        category_kind("side_gig", &config.category_kinds),
        CategoryKind::Income
    );
    assert_eq!(
        category_kind("income", &config.category_kinds),
        CategoryKind::Transfer
    );
    assert_eq!(
        category_kind("groceries", &config.category_kinds),
        CategoryKind::Expense
    );
}

#[test]
fn normalizes_category_names() {
    assert_eq!(normalize_category_name("Personal Care"), "personal_care");
    assert_eq!(normalize_category_name("  Donations "), "donations");
}

#[test]
fn old_config_files_still_load_and_new_fields_are_omitted_when_empty() {
    let old = r#"{"display_names":{"a1":"My Checking"}}"#;
    let config: DataConfig = serde_json::from_str(old).unwrap();
    assert!(config.category_kinds.is_empty());
    assert!(config.budget.monthly_limit.is_none());
    assert!(config.budget.category_limits.is_empty());

    let json = serde_json::to_string(&DataConfig::default()).unwrap();
    assert_eq!(json, "{}");
}

#[test]
fn category_kinds_serialize_snake_case() {
    let mut config = DataConfig::default();
    config
        .category_kinds
        .insert("side_gig".into(), CategoryKind::Income);
    let json = serde_json::to_string(&config).unwrap();
    assert_eq!(json, r#"{"category_kinds":{"side_gig":"income"}}"#);
}

// === Transaction edits overlay ===

#[test]
fn edits_are_scoped_to_account_and_transaction() {
    let mut edits = TransactionEdits::default();
    edits.set(
        "a1",
        "t1",
        TransactionEdit {
            hidden: true,
            ..Default::default()
        },
    );

    assert!(edits.get("a1", "t1").unwrap().hidden);
    assert!(
        edits.get("a2", "t1").is_none(),
        "same id on another account is untouched"
    );
    assert!(edits.for_transaction(&txn("a1", "t1", "X")).is_some());
    assert_eq!(edits.len(), 1);
}

#[test]
fn setting_an_empty_edit_removes_it() {
    let mut edits = TransactionEdits::default();
    edits.set(
        "a1",
        "t1",
        TransactionEdit {
            description: Some("Coffee".into()),
            ..Default::default()
        },
    );
    edits.set("a1", "t1", TransactionEdit::default());
    assert!(edits.is_empty());
    assert_eq!(serde_json::to_string(&edits).unwrap(), "{}");
}

#[test]
fn edits_roundtrip_through_json() {
    let mut edits = TransactionEdits::default();
    edits.set(
        "a1",
        "t1",
        TransactionEdit {
            description: Some("Corner Cafe".into()),
            category: Some("restaurants".into()),
            hidden: false,
        },
    );
    let json = serde_json::to_string(&edits).unwrap();
    assert_eq!(
        json,
        r#"{"a1":{"t1":{"description":"Corner Cafe","category":"restaurants"}}}"#
    );
    let back: TransactionEdits = serde_json::from_str(&json).unwrap();
    assert_eq!(back, edits);
}

#[test]
fn effective_description_prefers_edit() {
    let t = txn("a1", "t1", "SQ *CORNER CAFE 1234");
    let mut edits = TransactionEdits::default();
    assert_eq!(effective_description(&t, &edits), "SQ *CORNER CAFE 1234");

    edits.set(
        "a1",
        "t1",
        TransactionEdit {
            description: Some("Corner Cafe".into()),
            ..Default::default()
        },
    );
    assert_eq!(effective_description(&t, &edits), "Corner Cafe");
}

#[test]
fn effective_category_precedence_is_edit_then_rules_then_other() {
    let t = txn("a1", "t1", "CORNER CAFE");
    // User rule first, then pattern, as assembled by callers.
    let rules = vec![rule("corner", "coffee"), rule("cafe", "restaurants")];
    let mut edits = TransactionEdits::default();

    assert_eq!(effective_category(&t, &edits, &rules), "coffee");
    assert_eq!(effective_category(&t, &edits, &rules[1..]), "restaurants");
    assert_eq!(effective_category(&t, &edits, &[]), "other");

    edits.set(
        "a1",
        "t1",
        TransactionEdit {
            category: Some("business".into()),
            ..Default::default()
        },
    );
    assert_eq!(effective_category(&t, &edits, &rules), "business");
}

#[test]
fn categorizer_load_assembles_user_rules_before_patterns() {
    let (_dir, storage) = temp_storage();
    storage
        .set_spending_patterns(&[rule("cafe", "restaurants")])
        .unwrap();
    let mut config = DataConfig::default();
    config.spending_rules.push(rule("corner", "coffee"));
    config
        .category_kinds
        .insert("coffee".into(), CategoryKind::Transfer);
    storage.set_config(&config).unwrap();
    let mut edits = TransactionEdits::default();
    edits.set(
        "a1",
        "hidden",
        TransactionEdit {
            hidden: true,
            ..Default::default()
        },
    );
    storage.set_transaction_edits(&edits).unwrap();

    let categorizer = Categorizer::load(&storage).unwrap();

    let t = txn("a1", "t1", "CORNER CAFE");
    assert_eq!(categorizer.category(&t), "coffee");
    assert_eq!(categorizer.kind(&t), CategoryKind::Transfer);
    assert_eq!(
        categorizer.category(&txn("a1", "t2", "OTHER CAFE")),
        "restaurants"
    );
    assert!(categorizer.is_hidden(&txn("a1", "hidden", "X")));
    assert!(!categorizer.is_hidden(&t));
}

// === Storage ===

#[test]
fn storage_edits_default_to_empty_and_roundtrip() {
    let (_dir, storage) = temp_storage();
    assert!(storage.get_transaction_edits().unwrap().is_empty());

    let mut edits = TransactionEdits::default();
    edits.set(
        "a1",
        "t1",
        TransactionEdit {
            category: Some("groceries".into()),
            ..Default::default()
        },
    );
    storage.set_transaction_edits(&edits).unwrap();
    assert_eq!(storage.get_transaction_edits().unwrap(), edits);
}

#[test]
fn edits_survive_recollection_of_the_same_transaction() {
    let (_dir, mut storage) = temp_storage();
    let raw = Transaction {
        id: "t1".into(),
        posted: 1_700_000_000,
        amount: dec!(-5.00),
        description: "BANK TEXT".into(),
        ..Default::default()
    };
    storage
        .upsert_transactions("a1", std::slice::from_ref(&raw))
        .unwrap();
    let mut edits = TransactionEdits::default();
    edits.set(
        "a1",
        "t1",
        TransactionEdit {
            description: Some("Renamed".into()),
            ..Default::default()
        },
    );
    storage.set_transaction_edits(&edits).unwrap();

    storage.upsert_transactions("a1", &[raw]).unwrap();

    assert_eq!(
        storage
            .get_transaction_edits()
            .unwrap()
            .get("a1", "t1")
            .unwrap()
            .description
            .as_deref(),
        Some("Renamed")
    );
}

// === Apply to future ===

#[test]
fn rule_from_transaction_uses_normalized_merchant() {
    let t = txn("a1", "t1", "SQ *CORNER CAFE 555-1234");
    let r = rule_from_transaction(&t, "Coffee Shops").unwrap();
    assert_eq!(r.pattern, "corner cafe");
    assert_eq!(r.category, "coffee_shops");
}

#[test]
fn rule_from_transaction_rejects_unusable_merchant() {
    let t = txn("a1", "t1", "12345");
    assert!(rule_from_transaction(&t, "coffee").is_err());
}

#[test]
fn apply_to_future_adds_highest_priority_user_rule() {
    let (_dir, storage) = temp_storage();
    let mut config = DataConfig::default();
    config
        .spending_rules
        .push(rule("corner cafe", "restaurants"));
    config.spending_rules.push(rule("market", "groceries"));
    storage.set_config(&config).unwrap();

    let t = txn("a1", "t1", "SQ *CORNER CAFE #42");
    let added = apply_category_to_future(&storage, &t, "coffee").unwrap();
    assert_eq!(added.pattern, "corner cafe");

    let config = storage.get_config().unwrap();
    // Newer rule replaces the older one for the same merchant, at the front.
    assert_eq!(config.spending_rules.len(), 2);
    assert_eq!(config.spending_rules[0].category, "coffee");
    assert_eq!(config.spending_rules[1].pattern, "market");

    let categorizer = Categorizer::load(&storage).unwrap();
    assert_eq!(
        categorizer.category(&txn("a1", "t9", "CORNER CAFE PORTLAND")),
        "coffee"
    );
}
