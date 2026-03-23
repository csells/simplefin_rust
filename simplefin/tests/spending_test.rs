use rust_decimal::Decimal;
use rust_decimal_macros::dec;
use simplefin::spending::{
    classify_transaction, compute_spending, SpendingCategory, SpendingRule,
};
use simplefin::storage::TransactionWithContext;

fn make_txn(description: &str, amount: Decimal) -> TransactionWithContext {
    TransactionWithContext {
        id: "txn-1".to_string(),
        account_id: "acc-1".to_string(),
        account_name: "Checking".to_string(),
        org_name: "Bank".to_string(),
        currency: "USD".to_string(),
        posted: 1000,
        amount,
        description: description.to_string(),
        transacted_at: None,
        pending: false,
    }
}

#[test]
fn classifies_restaurant_transaction() {
    assert_eq!(
        classify_transaction("CHIPOTLE MEXICAN GRILL #1234", &[]),
        SpendingCategory::Restaurants
    );
}

#[test]
fn classifies_grocery_transaction() {
    assert_eq!(
        classify_transaction("WHOLE FOODS MKT #10234", &[]),
        SpendingCategory::Groceries
    );
}

#[test]
fn classifies_utility_transaction() {
    assert_eq!(
        classify_transaction("VERIZON WIRELESS PAYMENT", &[]),
        SpendingCategory::Utilities
    );
}

#[test]
fn classifies_income_transaction() {
    assert_eq!(
        classify_transaction("ACME CORP DIRECT DEP PAYROLL", &[]),
        SpendingCategory::Income
    );
}

#[test]
fn custom_rule_overrides_builtin() {
    let rules = vec![SpendingRule {
        pattern: "ACME".to_string(),
        category: SpendingCategory::Transfer,
    }];
    assert_eq!(
        classify_transaction("ACME CORP DIRECT DEP PAYROLL", &rules),
        SpendingCategory::Transfer
    );
}

#[test]
fn unknown_transaction_classified_as_other() {
    assert_eq!(
        classify_transaction("RANDOM VENDOR XYZ", &[]),
        SpendingCategory::Other
    );
}

#[test]
fn spending_summary_totals() {
    let txns = vec![
        make_txn("CHIPOTLE", dec!(-15.00)),
        make_txn("WHOLE FOODS", dec!(-85.00)),
        make_txn("PAYROLL DEPOSIT", dec!(3000.00)),
    ];
    let summary = compute_spending(&txns, &[]);
    assert_eq!(summary.total_spending, dec!(-100.00));
    assert_eq!(summary.total_income, dec!(3000.00));
    assert_eq!(summary.net, dec!(2900.00));
}

#[test]
fn spending_excludes_pending() {
    let mut txn = make_txn("CHIPOTLE", dec!(-15.00));
    txn.pending = true;
    let summary = compute_spending(&[txn], &[]);
    assert_eq!(summary.total_spending, Decimal::ZERO);
    assert!(summary.categories.is_empty());
}

#[test]
fn spending_counts_transactions() {
    let txns = vec![
        make_txn("CHIPOTLE #1", dec!(-15.00)),
        make_txn("STARBUCKS #2", dec!(-5.00)),
    ];
    let summary = compute_spending(&txns, &[]);
    let restaurants = summary
        .categories
        .iter()
        .find(|c| c.category == SpendingCategory::Restaurants)
        .unwrap();
    assert_eq!(restaurants.transaction_count, 2);
    assert_eq!(restaurants.total, dec!(-20.00));
}
