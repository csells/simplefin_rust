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

// --- New category tests ---

#[test]
fn classifies_housing_transaction() {
    assert_eq!(
        classify_transaction("HOA DUES PAYMENT", &[]),
        SpendingCategory::Housing
    );
}

#[test]
fn classifies_rent_as_housing() {
    assert_eq!(
        classify_transaction("RENT PAYMENT APT 4B", &[]),
        SpendingCategory::Housing
    );
}

#[test]
fn classifies_insurance_transaction() {
    assert_eq!(
        classify_transaction("GEICO AUTO INSURANCE", &[]),
        SpendingCategory::Insurance
    );
}

#[test]
fn classifies_generic_insurance() {
    assert_eq!(
        classify_transaction("HOMEOWNERS INSURANCE PREMIUM", &[]),
        SpendingCategory::Insurance
    );
}

#[test]
fn classifies_subscription_transaction() {
    assert_eq!(
        classify_transaction("ADOBE CREATIVE CLOUD", &[]),
        SpendingCategory::Subscriptions
    );
}

#[test]
fn classifies_membership_as_subscription() {
    assert_eq!(
        classify_transaction("AAA MEMBERSHIP RENEWAL", &[]),
        SpendingCategory::Subscriptions
    );
}

// --- Expanded keyword tests ---

#[test]
fn classifies_coffee_as_restaurant() {
    assert_eq!(
        classify_transaction("LOCAL COFFEE HOUSE", &[]),
        SpendingCategory::Restaurants
    );
}

#[test]
fn classifies_bakery_as_restaurant() {
    assert_eq!(
        classify_transaction("PORTLAND BAKERY #42", &[]),
        SpendingCategory::Restaurants
    );
}

#[test]
fn classifies_aldi_as_groceries() {
    assert_eq!(
        classify_transaction("ALDI #1234", &[]),
        SpendingCategory::Groceries
    );
}

#[test]
fn classifies_interest_as_income() {
    assert_eq!(
        classify_transaction("INTEREST EARNED SAVINGS", &[]),
        SpendingCategory::Income
    );
}

#[test]
fn classifies_dividend_as_income() {
    assert_eq!(
        classify_transaction("DIVIDEND PAYMENT", &[]),
        SpendingCategory::Income
    );
}

#[test]
fn classifies_gym_as_entertainment() {
    assert_eq!(
        classify_transaction("24 HOUR FITNESS", &[]),
        SpendingCategory::Entertainment
    );
}

#[test]
fn classifies_dentist_as_healthcare() {
    assert_eq!(
        classify_transaction("DR SMITH DENTIST", &[]),
        SpendingCategory::Healthcare
    );
}

#[test]
fn classifies_taxi_as_transportation() {
    assert_eq!(
        classify_transaction("YELLOW TAXI NYC", &[]),
        SpendingCategory::Transportation
    );
}

#[test]
fn classifies_home_depot_as_shopping() {
    assert_eq!(
        classify_transaction("HOME DEPOT #4521", &[]),
        SpendingCategory::Shopping
    );
}

#[test]
fn classifies_atm_as_transfer() {
    assert_eq!(
        classify_transaction("ATM WITHDRAWAL", &[]),
        SpendingCategory::Transfer
    );
}

// --- New category tests: Education, Personal Care, Pets ---

#[test]
fn classifies_college_as_education() {
    assert_eq!(
        classify_transaction("PORTLAND COMM COLLEGE", &[]),
        SpendingCategory::Education
    );
}

#[test]
fn classifies_coursera_as_education() {
    assert_eq!(
        classify_transaction("COURSERA.ORG SUBSCRIPTION", &[]),
        SpendingCategory::Education
    );
}

#[test]
fn classifies_barber_as_personal_care() {
    assert_eq!(
        classify_transaction("THE BARBERS DOWNTOWN", &[]),
        SpendingCategory::PersonalCare
    );
}

#[test]
fn classifies_beauty_as_personal_care() {
    assert_eq!(
        classify_transaction("BLISS AND BEAUTY LLC", &[]),
        SpendingCategory::PersonalCare
    );
}

#[test]
fn classifies_petco_as_pets() {
    assert_eq!(
        classify_transaction("PETCO 1259", &[]),
        SpendingCategory::Pets
    );
}

#[test]
fn classifies_veterinary_as_pets() {
    assert_eq!(
        classify_transaction("WILLOWBROOK VETERINARY", &[]),
        SpendingCategory::Pets
    );
}

// --- Real-world pattern tests ---

#[test]
fn classifies_buffet_as_restaurant() {
    assert_eq!(
        classify_transaction("MIZUMI BUFFET 650000", &[]),
        SpendingCategory::Restaurants
    );
}

#[test]
fn classifies_donut_as_restaurant() {
    assert_eq!(
        classify_transaction("SESAME DONUTS TIGARD", &[]),
        SpendingCategory::Restaurants
    );
}

#[test]
fn classifies_pancake_as_restaurant() {
    assert_eq!(
        classify_transaction("PIG 'N PANCAKE-NEWPORT", &[]),
        SpendingCategory::Restaurants
    );
}

#[test]
fn classifies_food_service_as_restaurant() {
    assert_eq!(
        classify_transaction("SYLVANIA FOOD SERVICE", &[]),
        SpendingCategory::Restaurants
    );
}

#[test]
fn classifies_cinema_as_entertainment() {
    assert_eq!(
        classify_transaction("CINEMARK PORTLAND OR", &[]),
        SpendingCategory::Entertainment
    );
}

#[test]
fn classifies_regal_theater_as_entertainment() {
    assert_eq!(
        classify_transaction("REGAL BRIDGEPORT 0652", &[]),
        SpendingCategory::Entertainment
    );
}

#[test]
fn classifies_casino_as_entertainment() {
    assert_eq!(
        classify_transaction("LUCKY EAGLE CASINO", &[]),
        SpendingCategory::Entertainment
    );
}

#[test]
fn classifies_apple_bill_as_subscription() {
    assert_eq!(
        classify_transaction("Ext Credit Card Debit APPLE.COM/BILL CUPERTINO CA", &[]),
        SpendingCategory::Subscriptions
    );
}

#[test]
fn classifies_google_service_as_subscription() {
    assert_eq!(
        classify_transaction("Ext Credit Card Debit GOOGLE *GOOGLE ONE 650-253-0000 CA", &[]),
        SpendingCategory::Subscriptions
    );
}

#[test]
fn classifies_hotel_as_transportation() {
    assert_eq!(
        classify_transaction("WHALER MOTEL NEWPORT OR", &[]),
        SpendingCategory::Transportation
    );
}

#[test]
fn classifies_truncated_transit_as_transportation() {
    // Bank truncated "TRANSIT" to "TRANSI"
    assert_eq!(
        classify_transaction("SALEM AREA MASS TRANSI", &[]),
        SpendingCategory::Transportation
    );
}

#[test]
fn classifies_disposal_as_utilities() {
    assert_eq!(
        classify_transaction("PRIDE DISPOSAL 13980", &[]),
        SpendingCategory::Utilities
    );
}

#[test]
fn classifies_general_electric_as_utilities() {
    assert_eq!(
        classify_transaction("PORTLAND GENERAL ELECT", &[]),
        SpendingCategory::Utilities
    );
}

#[test]
fn classifies_check_as_transfer() {
    assert_eq!(
        classify_transaction("Check #1575", &[]),
        SpendingCategory::Transfer
    );
}

#[test]
fn custom_rules_still_override_expanded_builtins() {
    let rules = vec![SpendingRule {
        pattern: "STARBUCKS".to_string(),
        category: SpendingCategory::Other,
    }];
    assert_eq!(
        classify_transaction("STARBUCKS #1234", &rules),
        SpendingCategory::Other
    );
}
