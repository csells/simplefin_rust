//! Transaction query: search, filters, hidden handling and sort order.

use rust_decimal_macros::dec;
use simplefin::storage::{DataConfig, TransactionWithContext};
use simplefin::{
    AccountCategory, Categorizer, CategoryKind, TransactionEdit, TransactionEdits,
    TransactionQuery, default_spending_patterns,
};

fn txn(id: &str, account: (&str, &str), posted: i64, description: &str) -> TransactionWithContext {
    TransactionWithContext {
        id: id.into(),
        account_id: account.0.into(),
        account_name: account.1.into(),
        org_name: "Example Bank".into(),
        currency: "USD".into(),
        posted,
        amount: dec!(-10.00),
        description: description.into(),
        ..Default::default()
    }
}

const CHECKING: (&str, &str) = ("chk", "Checking (1234)");
const CARD: (&str, &str) = ("card", "Rewards Visa (5678)");

fn sample() -> Vec<TransactionWithContext> {
    let mut with_payee = txn("t3", CARD, 3000, "POS 0042");
    with_payee.payee = Some("Corner Bookshop".into());
    let mut with_memo = txn("t4", CHECKING, 4000, "CHECK 101");
    with_memo.memo = Some("Piano lessons".into());
    vec![
        txn("t1", CHECKING, 1000, "WHOLE FOODS"),
        txn("t2", CARD, 2000, "CHIPOTLE"),
        with_payee,
        with_memo,
    ]
}

fn edits() -> TransactionEdits {
    let mut edits = TransactionEdits::default();
    edits.set(
        "chk",
        "t1",
        TransactionEdit {
            description: Some("Weekly groceries".into()),
            ..Default::default()
        },
    );
    edits.set(
        "card",
        "t2",
        TransactionEdit {
            category: Some("business".into()),
            hidden: true,
            ..Default::default()
        },
    );
    edits
}

fn categorizer() -> Categorizer {
    Categorizer::new(default_spending_patterns(), edits(), Default::default())
}

fn ids(rows: &[simplefin::TransactionView]) -> Vec<&str> {
    rows.iter().map(|r| r.transaction.id.as_str()).collect()
}

#[test]
fn default_query_excludes_hidden_and_sorts_newest_first() {
    let rows = TransactionQuery::default().run(&sample(), &DataConfig::default(), &categorizer());
    assert_eq!(ids(&rows), ["t4", "t3", "t1"]);
}

#[test]
fn include_hidden_shows_hidden_rows_flagged() {
    let query = TransactionQuery {
        include_hidden: true,
        ..Default::default()
    };
    let rows = query.run(&sample(), &DataConfig::default(), &categorizer());
    assert_eq!(ids(&rows), ["t4", "t3", "t2", "t1"]);
    let hidden = rows.iter().find(|r| r.transaction.id == "t2").unwrap();
    assert!(hidden.hidden);
    assert_eq!(hidden.category, "business");
}

#[test]
fn rows_carry_effective_values() {
    let rows = TransactionQuery::default().run(&sample(), &DataConfig::default(), &categorizer());
    let t1 = rows.iter().find(|r| r.transaction.id == "t1").unwrap();
    assert_eq!(t1.description, "Weekly groceries");
    assert_eq!(t1.transaction.description, "WHOLE FOODS");
    assert_eq!(t1.category, "groceries");
    assert_eq!(t1.category_label, "Groceries");
    assert_eq!(t1.kind, CategoryKind::Expense);
    assert_eq!(t1.account_category, AccountCategory::Cash);
    assert!(!t1.hidden);
}

#[test]
fn text_search_covers_effective_description_payee_and_memo() {
    let search = |text: &str| {
        let query = TransactionQuery {
            text: Some(text.into()),
            ..Default::default()
        };
        let rows = query.run(&sample(), &DataConfig::default(), &categorizer());
        ids(&rows).into_iter().map(String::from).collect::<Vec<_>>()
    };
    assert_eq!(search("weekly"), ["t1"]);
    assert_eq!(search("bookshop"), ["t3"]);
    assert_eq!(search("PIANO"), ["t4"]);
    assert!(
        search("whole foods").is_empty(),
        "renamed rows match their new name"
    );
    assert_eq!(search("  ").len(), 3, "blank text does not filter");
}

#[test]
fn filters_by_effective_category() {
    let query = TransactionQuery {
        category: Some("business".into()),
        include_hidden: true,
        ..Default::default()
    };
    let rows = query.run(&sample(), &DataConfig::default(), &categorizer());
    assert_eq!(ids(&rows), ["t2"]);
}

#[test]
fn filters_by_account_category_with_config_overrides() {
    let query = TransactionQuery {
        account_category: Some(AccountCategory::CreditCards),
        ..Default::default()
    };
    let rows = query.run(&sample(), &DataConfig::default(), &categorizer());
    assert_eq!(ids(&rows), ["t3"]);

    let mut config = DataConfig::default();
    config
        .classification_overrides
        .insert("card".into(), AccountCategory::Cash);
    let rows = query.run(&sample(), &config, &categorizer());
    assert!(rows.is_empty());
}

#[test]
fn filters_by_date_range_start_inclusive_end_exclusive() {
    let query = TransactionQuery {
        start_date: Some(1000),
        end_date: Some(4000),
        ..Default::default()
    };
    let rows = query.run(&sample(), &DataConfig::default(), &categorizer());
    assert_eq!(ids(&rows), ["t3", "t1"]);
}

#[test]
fn pending_rows_without_posted_date_use_transacted_at() {
    let mut pending = txn("p1", CHECKING, 0, "CHIPOTLE");
    pending.pending = true;
    pending.transacted_at = Some(5000);
    let mut txns = sample();
    txns.push(pending);

    let rows = TransactionQuery {
        start_date: Some(4500),
        ..Default::default()
    }
    .run(&txns, &DataConfig::default(), &categorizer());
    assert_eq!(ids(&rows), ["p1"]);
}
