//! Monthly cash flow: income vs expense with transfer/loan/exclusion rules.

use chrono::NaiveDate;
use rust_decimal::Decimal;
use rust_decimal_macros::dec;
use simplefin::storage::{DataConfig, TransactionWithContext};
use simplefin::{
    Categorizer, CategoryKind, TransactionEdit, TransactionEdits, YearMonth, compute_cash_flow,
    default_spending_patterns,
};

const CHECKING: (&str, &str) = ("chk", "Checking (1234)");
const CARD: (&str, &str) = ("card", "Rewards Visa (5678)");
const LOAN: (&str, &str) = ("loan", "Home Mortgage (9999)");

fn ts(y: i32, m: u32, d: u32) -> i64 {
    NaiveDate::from_ymd_opt(y, m, d)
        .unwrap()
        .and_hms_opt(12, 0, 0)
        .unwrap()
        .and_utc()
        .timestamp()
}

fn txn(
    id: &str,
    account: (&str, &str),
    posted: i64,
    amount: Decimal,
    description: &str,
) -> TransactionWithContext {
    TransactionWithContext {
        id: id.into(),
        account_id: account.0.into(),
        account_name: account.1.into(),
        org_name: "Example Bank".into(),
        currency: "USD".into(),
        posted,
        amount,
        description: description.into(),
        ..Default::default()
    }
}

fn month(s: &str) -> YearMonth {
    s.parse().unwrap()
}

fn categorizer() -> Categorizer {
    Categorizer::from_rules(default_spending_patterns())
}

fn january() -> Vec<TransactionWithContext> {
    vec![
        txn(
            "pay",
            CHECKING,
            ts(2024, 1, 2),
            dec!(3000.00),
            "PAYROLL DEPOSIT",
        ),
        txn(
            "groc",
            CHECKING,
            ts(2024, 1, 3),
            dec!(-100.00),
            "WHOLE FOODS",
        ),
        txn(
            "refund",
            CHECKING,
            ts(2024, 1, 4),
            dec!(20.00),
            "WHOLE FOODS",
        ),
        txn("rest", CHECKING, ts(2024, 1, 5), dec!(-50.00), "CHIPOTLE"),
        // Card purchase counts; the card payment on either side is a transfer.
        txn("buy", CARD, ts(2024, 1, 6), dec!(-80.00), "AMAZON"),
        txn(
            "cardpay",
            CARD,
            ts(2024, 1, 7),
            dec!(500.00),
            "PAYMENT - THANK YOU",
        ),
        txn(
            "xfer",
            CHECKING,
            ts(2024, 1, 7),
            dec!(-500.00),
            "ONLINE TRANSFER TO CARD",
        ),
        // Loan account activity is excluded.
        txn("loanfee", LOAN, ts(2024, 1, 8), dec!(-25.00), "LATE CHARGE"),
    ]
}

#[test]
fn computes_income_expense_and_net_per_month() {
    let summary = compute_cash_flow(
        &january(),
        &DataConfig::default(),
        &categorizer(),
        month("2024-01"),
        month("2024-01"),
    );

    assert_eq!(summary.months.len(), 1);
    let jan = &summary.months[0];
    assert_eq!(jan.month, "2024-01");
    assert_eq!(jan.income, dec!(3000.00));
    // 100 - 20 refund + 50 + 80
    assert_eq!(jan.expense, dec!(210.00));
    assert_eq!(jan.net, dec!(2790.00));
    assert_eq!(summary.total_income, dec!(3000.00));
    assert_eq!(summary.total_expense, dec!(210.00));
    assert_eq!(summary.net, dec!(2790.00));
}

#[test]
fn breaks_down_categories_as_positive_magnitudes() {
    let summary = compute_cash_flow(
        &january(),
        &DataConfig::default(),
        &categorizer(),
        month("2024-01"),
        month("2024-01"),
    );
    let jan = &summary.months[0];
    let find = |c: &str| jan.categories.iter().find(|x| x.category == c).unwrap();

    assert_eq!(find("income").amount, dec!(3000.00));
    assert_eq!(find("income").kind, CategoryKind::Income);
    assert_eq!(find("groceries").amount, dec!(80.00));
    assert_eq!(find("groceries").transaction_count, 2);
    assert_eq!(find("groceries").kind, CategoryKind::Expense);
    assert_eq!(find("shopping").amount, dec!(80.00));
    assert_eq!(find("restaurants").amount, dec!(50.00));
    assert!(jan.categories.iter().all(|c| c.category != "transfer"));
    // Income first, then expenses by amount descending.
    assert_eq!(jan.categories[0].category, "income");
}

#[test]
fn includes_empty_months_and_filters_to_range() {
    let mut txns = january();
    txns.push(txn(
        "dec",
        CHECKING,
        ts(2023, 12, 31),
        dec!(-999.00),
        "WHOLE FOODS",
    ));
    txns.push(txn(
        "mar",
        CHECKING,
        ts(2024, 3, 1),
        dec!(-10.00),
        "CHIPOTLE",
    ));
    txns.push(txn(
        "apr",
        CHECKING,
        ts(2024, 4, 1),
        dec!(-999.00),
        "CHIPOTLE",
    ));

    let summary = compute_cash_flow(
        &txns,
        &DataConfig::default(),
        &categorizer(),
        month("2024-01"),
        month("2024-03"),
    );

    let labels: Vec<&str> = summary.months.iter().map(|m| m.month.as_str()).collect();
    assert_eq!(labels, ["2024-01", "2024-02", "2024-03"]);
    assert_eq!(summary.months[1].income, Decimal::ZERO);
    assert_eq!(summary.months[1].expense, Decimal::ZERO);
    assert!(summary.months[1].categories.is_empty());
    assert_eq!(summary.months[2].expense, dec!(10.00));
    assert_eq!(summary.total_expense, dec!(220.00));
}

#[test]
fn excludes_pending_and_hidden() {
    let mut txns = january();
    let mut pending = txn("pend", CHECKING, ts(2024, 1, 9), dec!(-40.00), "CHIPOTLE");
    pending.pending = true;
    txns.push(pending);
    txns.push(txn(
        "dup",
        CHECKING,
        ts(2024, 1, 9),
        dec!(-50.00),
        "CHIPOTLE",
    ));
    let mut edits = TransactionEdits::default();
    edits.set(
        "chk",
        "dup",
        TransactionEdit {
            hidden: true,
            ..Default::default()
        },
    );
    let categorizer = Categorizer::new(default_spending_patterns(), edits, Default::default());

    let summary = compute_cash_flow(
        &txns,
        &DataConfig::default(),
        &categorizer,
        month("2024-01"),
        month("2024-01"),
    );
    assert_eq!(summary.months[0].expense, dec!(210.00));
}

#[test]
fn excludes_configured_accounts_and_transfer_kind_overrides() {
    let mut config = DataConfig::default();
    config.excluded_account_ids.push("card".into());
    let summary = compute_cash_flow(
        &january(),
        &config,
        &categorizer(),
        month("2024-01"),
        month("2024-01"),
    );
    assert_eq!(summary.months[0].expense, dec!(130.00));

    let mut config = DataConfig::default();
    config.excluded_account_patterns.push("rewards visa".into());
    let summary = compute_cash_flow(
        &january(),
        &config,
        &categorizer(),
        month("2024-01"),
        month("2024-01"),
    );
    assert_eq!(summary.months[0].expense, dec!(130.00));

    let mut kinds = std::collections::HashMap::new();
    kinds.insert("restaurants".to_string(), CategoryKind::Transfer);
    let categorizer = Categorizer::new(
        default_spending_patterns(),
        TransactionEdits::default(),
        kinds,
    );
    let summary = compute_cash_flow(
        &january(),
        &DataConfig::default(),
        &categorizer,
        month("2024-01"),
        month("2024-01"),
    );
    assert_eq!(summary.months[0].expense, dec!(160.00));
}

#[test]
fn loan_classification_follows_config_overrides() {
    // Reclassifying the loan account as cash brings its activity back in.
    let mut config = DataConfig::default();
    config
        .classification_overrides
        .insert("loan".into(), simplefin::AccountCategory::Cash);
    let summary = compute_cash_flow(
        &january(),
        &config,
        &categorizer(),
        month("2024-01"),
        month("2024-01"),
    );
    assert_eq!(summary.months[0].expense, dec!(235.00));
}

#[test]
fn category_edit_can_turn_expense_into_income() {
    let mut edits = TransactionEdits::default();
    edits.set(
        "chk",
        "refund",
        TransactionEdit {
            category: Some("income".into()),
            ..Default::default()
        },
    );
    let categorizer = Categorizer::new(default_spending_patterns(), edits, Default::default());
    let summary = compute_cash_flow(
        &january(),
        &DataConfig::default(),
        &categorizer,
        month("2024-01"),
        month("2024-01"),
    );
    assert_eq!(summary.months[0].income, dec!(3020.00));
    assert_eq!(summary.months[0].expense, dec!(230.00));
}

#[test]
fn unclassified_transactions_follow_their_sign() {
    // An unrecognized deposit is far more likely income than a refund, and must
    // not show up as negative spending; an unrecognized debit is spending.
    let txns = vec![
        txn("credit", CHECKING, ts(2024, 1, 5), dec!(500.00), "MYSTERY CREDIT XYZ"),
        txn("debit", CHECKING, ts(2024, 1, 6), dec!(-40.00), "MYSTERY DEBIT QRS"),
    ];
    let summary = compute_cash_flow(
        &txns,
        &DataConfig::default(),
        &categorizer(),
        month("2024-01"),
        month("2024-01"),
    );
    let jan = &summary.months[0];
    assert_eq!(jan.income, dec!(500.00));
    assert_eq!(jan.expense, dec!(40.00));
    let other: Vec<_> = jan.categories.iter().filter(|c| c.category == "other").collect();
    assert_eq!(other.len(), 2, "one income and one expense 'other' bucket");
    assert!(other.iter().all(|c| c.amount > Decimal::ZERO));
    assert!(other.iter().any(|c| c.kind == CategoryKind::Income));
}
