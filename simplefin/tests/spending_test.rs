use rust_decimal::Decimal;
use rust_decimal_macros::dec;
use simplefin::spending::{
    classify_transaction, compute_spending, default_spending_patterns, SpendingCategory,
    SpendingRule,
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

/// Helper: default patterns for tests that don't customize rules.
fn defaults() -> Vec<SpendingRule> {
    default_spending_patterns()
}

#[test]
fn classifies_restaurant_transaction() {
    assert_eq!(
        classify_transaction("CHIPOTLE MEXICAN GRILL #1234", &defaults()),
        SpendingCategory::Restaurants
    );
}

#[test]
fn classifies_grocery_transaction() {
    assert_eq!(
        classify_transaction("WHOLE FOODS MKT #10234", &defaults()),
        SpendingCategory::Groceries
    );
}

#[test]
fn classifies_utility_transaction() {
    assert_eq!(
        classify_transaction("VERIZON WIRELESS PAYMENT", &defaults()),
        SpendingCategory::Utilities
    );
}

#[test]
fn classifies_income_transaction() {
    assert_eq!(
        classify_transaction("ACME CORP DIRECT DEP PAYROLL", &defaults()),
        SpendingCategory::Income
    );
}

#[test]
fn custom_rule_overrides_builtin() {
    // Custom rule placed before defaults
    let mut rules = vec![SpendingRule {
        pattern: "ACME".to_string(),
        category: SpendingCategory::Transfer,
    }];
    rules.extend(defaults());
    assert_eq!(
        classify_transaction("ACME CORP DIRECT DEP PAYROLL", &rules),
        SpendingCategory::Transfer
    );
}

#[test]
fn unknown_transaction_classified_as_other() {
    assert_eq!(
        classify_transaction("RANDOM VENDOR XYZ", &defaults()),
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
    let summary = compute_spending(&txns, &defaults());
    assert_eq!(summary.total_spending, dec!(-100.00));
    assert_eq!(summary.total_income, dec!(3000.00));
    assert_eq!(summary.net, dec!(2900.00));
}

#[test]
fn spending_excludes_pending() {
    let mut txn = make_txn("CHIPOTLE", dec!(-15.00));
    txn.pending = true;
    let summary = compute_spending(&[txn], &defaults());
    assert_eq!(summary.total_spending, Decimal::ZERO);
    assert!(summary.categories.is_empty());
}

#[test]
fn spending_counts_transactions() {
    let txns = vec![
        make_txn("CHIPOTLE #1", dec!(-15.00)),
        make_txn("STARBUCKS #2", dec!(-5.00)),
    ];
    let summary = compute_spending(&txns, &defaults());
    let restaurants = summary
        .categories
        .iter()
        .find(|c| c.category == SpendingCategory::Restaurants)
        .unwrap();
    assert_eq!(restaurants.transaction_count, 2);
    assert_eq!(restaurants.total, dec!(-20.00));
}

// --- New category tests ---

#[test]
fn classifies_housing_transaction() {
    assert_eq!(
        classify_transaction("HOA DUES PAYMENT", &defaults()),
        SpendingCategory::Housing
    );
}

#[test]
fn classifies_rent_as_housing() {
    assert_eq!(
        classify_transaction("RENT PAYMENT APT 4B", &defaults()),
        SpendingCategory::Housing
    );
}

#[test]
fn classifies_insurance_transaction() {
    assert_eq!(
        classify_transaction("GEICO AUTO INSURANCE", &defaults()),
        SpendingCategory::Insurance
    );
}

#[test]
fn classifies_generic_insurance() {
    assert_eq!(
        classify_transaction("HOMEOWNERS INSURANCE PREMIUM", &defaults()),
        SpendingCategory::Insurance
    );
}

#[test]
fn classifies_subscription_transaction() {
    assert_eq!(
        classify_transaction("ADOBE CREATIVE CLOUD", &defaults()),
        SpendingCategory::Subscriptions
    );
}

#[test]
fn classifies_membership_as_subscription() {
    assert_eq!(
        classify_transaction("AAA MEMBERSHIP RENEWAL", &defaults()),
        SpendingCategory::Subscriptions
    );
}

// --- Expanded keyword tests ---

#[test]
fn classifies_coffee_as_restaurant() {
    assert_eq!(
        classify_transaction("LOCAL COFFEE HOUSE", &defaults()),
        SpendingCategory::Restaurants
    );
}

#[test]
fn classifies_bakery_as_restaurant() {
    assert_eq!(
        classify_transaction("PORTLAND BAKERY #42", &defaults()),
        SpendingCategory::Restaurants
    );
}

#[test]
fn classifies_aldi_as_groceries() {
    assert_eq!(
        classify_transaction("ALDI #1234", &defaults()),
        SpendingCategory::Groceries
    );
}

#[test]
fn classifies_interest_as_income() {
    assert_eq!(
        classify_transaction("INTEREST EARNED SAVINGS", &defaults()),
        SpendingCategory::Income
    );
}

#[test]
fn classifies_dividend_as_income() {
    assert_eq!(
        classify_transaction("DIVIDEND PAYMENT", &defaults()),
        SpendingCategory::Income
    );
}

#[test]
fn classifies_gym_as_entertainment() {
    assert_eq!(
        classify_transaction("24 HOUR FITNESS", &defaults()),
        SpendingCategory::Entertainment
    );
}

#[test]
fn classifies_dentist_as_healthcare() {
    assert_eq!(
        classify_transaction("DR SMITH DENTIST", &defaults()),
        SpendingCategory::Healthcare
    );
}

#[test]
fn classifies_taxi_as_transportation() {
    assert_eq!(
        classify_transaction("YELLOW TAXI NYC", &defaults()),
        SpendingCategory::Transportation
    );
}

#[test]
fn classifies_home_depot_as_shopping() {
    assert_eq!(
        classify_transaction("HOME DEPOT #4521", &defaults()),
        SpendingCategory::Shopping
    );
}

#[test]
fn classifies_atm_as_transfer() {
    assert_eq!(
        classify_transaction("ATM WITHDRAWAL", &defaults()),
        SpendingCategory::Transfer
    );
}

// --- New category tests: Education, Personal Care, Pets ---

#[test]
fn classifies_college_as_education() {
    assert_eq!(
        classify_transaction("PORTLAND COMM COLLEGE", &defaults()),
        SpendingCategory::Education
    );
}

#[test]
fn classifies_coursera_as_education() {
    assert_eq!(
        classify_transaction("COURSERA.ORG SUBSCRIPTION", &defaults()),
        SpendingCategory::Education
    );
}

#[test]
fn classifies_barber_as_personal_care() {
    assert_eq!(
        classify_transaction("THE BARBERS DOWNTOWN", &defaults()),
        SpendingCategory::PersonalCare
    );
}

#[test]
fn classifies_beauty_as_personal_care() {
    assert_eq!(
        classify_transaction("BLISS AND BEAUTY LLC", &defaults()),
        SpendingCategory::PersonalCare
    );
}

#[test]
fn classifies_petco_as_pets() {
    assert_eq!(
        classify_transaction("PETCO 1259", &defaults()),
        SpendingCategory::Pets
    );
}

#[test]
fn classifies_veterinary_as_pets() {
    assert_eq!(
        classify_transaction("WILLOWBROOK VETERINARY", &defaults()),
        SpendingCategory::Pets
    );
}

// --- Real-world pattern tests ---

#[test]
fn classifies_buffet_as_restaurant() {
    assert_eq!(
        classify_transaction("MIZUMI BUFFET 650000", &defaults()),
        SpendingCategory::Restaurants
    );
}

#[test]
fn classifies_donut_as_restaurant() {
    assert_eq!(
        classify_transaction("SESAME DONUTS TIGARD", &defaults()),
        SpendingCategory::Restaurants
    );
}

#[test]
fn classifies_pancake_as_restaurant() {
    assert_eq!(
        classify_transaction("PIG 'N PANCAKE-NEWPORT", &defaults()),
        SpendingCategory::Restaurants
    );
}

#[test]
fn classifies_food_service_as_restaurant() {
    assert_eq!(
        classify_transaction("SYLVANIA FOOD SERVICE", &defaults()),
        SpendingCategory::Restaurants
    );
}

#[test]
fn classifies_cinema_as_entertainment() {
    assert_eq!(
        classify_transaction("CINEMARK PORTLAND OR", &defaults()),
        SpendingCategory::Entertainment
    );
}

#[test]
fn classifies_regal_theater_as_entertainment() {
    assert_eq!(
        classify_transaction("REGAL BRIDGEPORT 0652", &defaults()),
        SpendingCategory::Entertainment
    );
}

#[test]
fn classifies_casino_as_entertainment() {
    assert_eq!(
        classify_transaction("LUCKY EAGLE CASINO", &defaults()),
        SpendingCategory::Entertainment
    );
}

#[test]
fn classifies_apple_bill_as_subscription() {
    assert_eq!(
        classify_transaction("Ext Credit Card Debit APPLE.COM/BILL CUPERTINO CA", &defaults()),
        SpendingCategory::Subscriptions
    );
}

#[test]
fn classifies_google_service_as_subscription() {
    assert_eq!(
        classify_transaction(
            "Ext Credit Card Debit GOOGLE *GOOGLE ONE 650-253-0000 CA",
            &defaults()
        ),
        SpendingCategory::Subscriptions
    );
}

#[test]
fn classifies_hotel_as_transportation() {
    assert_eq!(
        classify_transaction("WHALER MOTEL NEWPORT OR", &defaults()),
        SpendingCategory::Transportation
    );
}

#[test]
fn classifies_truncated_transit_as_transportation() {
    // Bank truncated "TRANSIT" to "TRANSI"
    assert_eq!(
        classify_transaction("SALEM AREA MASS TRANSI", &defaults()),
        SpendingCategory::Transportation
    );
}

#[test]
fn classifies_disposal_as_utilities() {
    assert_eq!(
        classify_transaction("PRIDE DISPOSAL 13980", &defaults()),
        SpendingCategory::Utilities
    );
}

#[test]
fn classifies_general_electric_as_utilities() {
    assert_eq!(
        classify_transaction("PORTLAND GENERAL ELECT", &defaults()),
        SpendingCategory::Utilities
    );
}

#[test]
fn classifies_check_as_transfer() {
    assert_eq!(
        classify_transaction("Check #1575", &defaults()),
        SpendingCategory::Transfer
    );
}

#[test]
fn custom_rules_still_override_expanded_builtins() {
    let mut rules = vec![SpendingRule {
        pattern: "STARBUCKS".to_string(),
        category: SpendingCategory::Other,
    }];
    rules.extend(defaults());
    assert_eq!(
        classify_transaction("STARBUCKS #1234", &rules),
        SpendingCategory::Other
    );
}

// --- Data-driven tests ---

#[test]
fn empty_rules_classifies_everything_as_other() {
    // With no rules at all, everything should be Other
    assert_eq!(
        classify_transaction("CHIPOTLE", &[]),
        SpendingCategory::Other
    );
}

#[test]
fn spending_reports_unclassified_descriptions() {
    let txns = vec![
        make_txn("UNKNOWN VENDOR ABC", dec!(-25.00)),
        make_txn("MYSTERY CHARGE XYZ", dec!(-10.00)),
        make_txn("CHIPOTLE #1", dec!(-15.00)),
    ];
    let summary = compute_spending(&txns, &defaults());
    assert_eq!(summary.unclassified.len(), 2);
    assert!(summary.unclassified.iter().any(|u| u.description == "UNKNOWN VENDOR ABC"));
    assert!(summary.unclassified.iter().any(|u| u.description == "MYSTERY CHARGE XYZ"));
    // Verify amounts are captured
    let unknown = summary.unclassified.iter().find(|u| u.description == "UNKNOWN VENDOR ABC").unwrap();
    assert_eq!(unknown.amount, dec!(-25.00));
}

#[test]
fn pipe_separated_patterns_work() {
    let rules = vec![SpendingRule {
        pattern: "foo|bar|baz".to_string(),
        category: SpendingCategory::Entertainment,
    }];
    assert_eq!(
        classify_transaction("SOMETHING FOO HERE", &rules),
        SpendingCategory::Entertainment
    );
    assert_eq!(
        classify_transaction("THE BAR", &rules),
        SpendingCategory::Entertainment
    );
    assert_eq!(
        classify_transaction("NO MATCH", &rules),
        SpendingCategory::Other
    );
}
