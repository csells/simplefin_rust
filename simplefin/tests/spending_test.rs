use rust_decimal::Decimal;
use rust_decimal_macros::dec;
use simplefin::spending::{
    classify_transaction, compute_spending, default_spending_patterns, OTHER_CATEGORY,
    SpendingRule,
};
use simplefin::storage::{DataConfig, JsonStorage, Storage, TransactionWithContext};
use simplefin::{
    Categorizer, TransactionEdit, TransactionEdits, add_user_spending_rule, compute_trends,
    detect_recurring, load_spending_rules,
};

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
        ..Default::default()
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
        "restaurants"
    );
}

#[test]
fn classifies_grocery_transaction() {
    assert_eq!(
        classify_transaction("WHOLE FOODS MKT #10234", &defaults()),
        "groceries"
    );
}

#[test]
fn classifies_utility_transaction() {
    assert_eq!(
        classify_transaction("VERIZON WIRELESS PAYMENT", &defaults()),
        "utilities"
    );
}

#[test]
fn classifies_income_transaction() {
    assert_eq!(
        classify_transaction("ACME CORP DIRECT DEP PAYROLL", &defaults()),
        "income"
    );
}

#[test]
fn custom_rule_overrides_builtin() {
    // Custom rule placed before defaults
    let mut rules = vec![SpendingRule {
        pattern: "ACME".to_string(),
        category: "transfer".to_string(),
    }];
    rules.extend(defaults());
    assert_eq!(
        classify_transaction("ACME CORP DIRECT DEP PAYROLL", &rules),
        "transfer"
    );
}

#[test]
fn unknown_transaction_classified_as_other() {
    assert_eq!(
        classify_transaction("RANDOM VENDOR XYZ", &defaults()),
        OTHER_CATEGORY
    );
}

#[test]
fn spending_summary_totals() {
    let txns = vec![
        make_txn("CHIPOTLE", dec!(-15.00)),
        make_txn("WHOLE FOODS", dec!(-85.00)),
        make_txn("PAYROLL DEPOSIT", dec!(3000.00)),
    ];
    let summary = compute_spending(&txns, &Categorizer::from_rules(defaults()));
    assert_eq!(summary.total_spending, dec!(-100.00));
    assert_eq!(summary.total_income, dec!(3000.00));
    assert_eq!(summary.net, dec!(2900.00));
}

#[test]
fn spending_excludes_pending() {
    let mut txn = make_txn("CHIPOTLE", dec!(-15.00));
    txn.pending = true;
    let summary = compute_spending(&[txn], &Categorizer::from_rules(defaults()));
    assert_eq!(summary.total_spending, Decimal::ZERO);
    assert!(summary.categories.is_empty());
}

#[test]
fn spending_counts_transactions() {
    let txns = vec![
        make_txn("CHIPOTLE #1", dec!(-15.00)),
        make_txn("STARBUCKS #2", dec!(-5.00)),
    ];
    let summary = compute_spending(&txns, &Categorizer::from_rules(defaults()));
    let restaurants = summary
        .categories
        .iter()
        .find(|c| c.category == "restaurants")
        .unwrap();
    assert_eq!(restaurants.transaction_count, 2);
    assert_eq!(restaurants.total, dec!(-20.00));
}

// --- New category tests ---

#[test]
fn classifies_housing_transaction() {
    assert_eq!(
        classify_transaction("HOA DUES PAYMENT", &defaults()),
        "housing"
    );
}

#[test]
fn classifies_rent_as_housing() {
    assert_eq!(
        classify_transaction("RENT PAYMENT APT 4B", &defaults()),
        "housing"
    );
}

#[test]
fn classifies_insurance_transaction() {
    assert_eq!(
        classify_transaction("GEICO AUTO INSURANCE", &defaults()),
        "insurance"
    );
}

#[test]
fn classifies_generic_insurance() {
    assert_eq!(
        classify_transaction("HOMEOWNERS INSURANCE PREMIUM", &defaults()),
        "insurance"
    );
}

#[test]
fn classifies_subscription_transaction() {
    assert_eq!(
        classify_transaction("ADOBE CREATIVE CLOUD", &defaults()),
        "subscriptions"
    );
}

#[test]
fn classifies_membership_as_subscription() {
    assert_eq!(
        classify_transaction("AAA MEMBERSHIP RENEWAL", &defaults()),
        "subscriptions"
    );
}

// --- Expanded keyword tests ---

#[test]
fn classifies_coffee_as_restaurant() {
    assert_eq!(
        classify_transaction("LOCAL COFFEE HOUSE", &defaults()),
        "restaurants"
    );
}

#[test]
fn classifies_bakery_as_restaurant() {
    assert_eq!(
        classify_transaction("PORTLAND BAKERY #42", &defaults()),
        "restaurants"
    );
}

#[test]
fn classifies_aldi_as_groceries() {
    assert_eq!(
        classify_transaction("ALDI #1234", &defaults()),
        "groceries"
    );
}

#[test]
fn classifies_interest_as_income() {
    assert_eq!(
        classify_transaction("INTEREST EARNED SAVINGS", &defaults()),
        "income"
    );
}

#[test]
fn classifies_dividend_as_income() {
    assert_eq!(
        classify_transaction("DIVIDEND PAYMENT", &defaults()),
        "income"
    );
}

#[test]
fn classifies_gym_as_entertainment() {
    assert_eq!(
        classify_transaction("24 HOUR FITNESS", &defaults()),
        "entertainment"
    );
}

#[test]
fn classifies_dentist_as_healthcare() {
    assert_eq!(
        classify_transaction("DR SMITH DENTIST", &defaults()),
        "healthcare"
    );
}

#[test]
fn classifies_taxi_as_transportation() {
    assert_eq!(
        classify_transaction("YELLOW TAXI NYC", &defaults()),
        "transportation"
    );
}

#[test]
fn classifies_home_depot_as_shopping() {
    assert_eq!(
        classify_transaction("HOME DEPOT #4521", &defaults()),
        "shopping"
    );
}

#[test]
fn classifies_atm_as_transfer() {
    assert_eq!(
        classify_transaction("ATM WITHDRAWAL", &defaults()),
        "transfer"
    );
}

// --- New category tests: Education, Personal Care, Pets ---

#[test]
fn classifies_college_as_education() {
    assert_eq!(
        classify_transaction("PORTLAND COMM COLLEGE", &defaults()),
        "education"
    );
}

#[test]
fn classifies_coursera_as_education() {
    assert_eq!(
        classify_transaction("COURSERA.ORG SUBSCRIPTION", &defaults()),
        "education"
    );
}

#[test]
fn classifies_barber_as_personal_care() {
    assert_eq!(
        classify_transaction("THE BARBERS DOWNTOWN", &defaults()),
        "personal_care"
    );
}

#[test]
fn classifies_beauty_as_personal_care() {
    assert_eq!(
        classify_transaction("BLISS AND BEAUTY LLC", &defaults()),
        "personal_care"
    );
}

#[test]
fn classifies_petco_as_pets() {
    assert_eq!(
        classify_transaction("PETCO 1259", &defaults()),
        "pets"
    );
}

#[test]
fn classifies_veterinary_as_pets() {
    assert_eq!(
        classify_transaction("WILLOWBROOK VETERINARY", &defaults()),
        "pets"
    );
}

// --- Real-world pattern tests ---

#[test]
fn classifies_buffet_as_restaurant() {
    assert_eq!(
        classify_transaction("MIZUMI BUFFET 650000", &defaults()),
        "restaurants"
    );
}

#[test]
fn classifies_donut_as_restaurant() {
    assert_eq!(
        classify_transaction("SESAME DONUTS TIGARD", &defaults()),
        "restaurants"
    );
}

#[test]
fn classifies_pancake_as_restaurant() {
    assert_eq!(
        classify_transaction("PIG 'N PANCAKE-NEWPORT", &defaults()),
        "restaurants"
    );
}

#[test]
fn classifies_food_service_as_restaurant() {
    assert_eq!(
        classify_transaction("SYLVANIA FOOD SERVICE", &defaults()),
        "restaurants"
    );
}

#[test]
fn classifies_cinema_as_entertainment() {
    assert_eq!(
        classify_transaction("CINEMARK PORTLAND OR", &defaults()),
        "entertainment"
    );
}

#[test]
fn classifies_regal_theater_as_entertainment() {
    assert_eq!(
        classify_transaction("REGAL BRIDGEPORT 0652", &defaults()),
        "entertainment"
    );
}

#[test]
fn classifies_casino_as_entertainment() {
    assert_eq!(
        classify_transaction("LUCKY EAGLE CASINO", &defaults()),
        "entertainment"
    );
}

#[test]
fn classifies_apple_bill_as_subscription() {
    assert_eq!(
        classify_transaction("Ext Credit Card Debit APPLE.COM/BILL CUPERTINO CA", &defaults()),
        "subscriptions"
    );
}

#[test]
fn classifies_google_service_as_subscription() {
    assert_eq!(
        classify_transaction(
            "Ext Credit Card Debit GOOGLE *GOOGLE ONE 650-253-0000 CA",
            &defaults()
        ),
        "subscriptions"
    );
}

#[test]
fn classifies_hotel_as_transportation() {
    assert_eq!(
        classify_transaction("WHALER MOTEL NEWPORT OR", &defaults()),
        "transportation"
    );
}

#[test]
fn classifies_truncated_transit_as_transportation() {
    // Bank truncated "TRANSIT" to "TRANSI"
    assert_eq!(
        classify_transaction("SALEM AREA MASS TRANSI", &defaults()),
        "transportation"
    );
}

#[test]
fn classifies_disposal_as_utilities() {
    assert_eq!(
        classify_transaction("PRIDE DISPOSAL 13980", &defaults()),
        "utilities"
    );
}

#[test]
fn classifies_general_electric_as_utilities() {
    assert_eq!(
        classify_transaction("PORTLAND GENERAL ELECT", &defaults()),
        "utilities"
    );
}

#[test]
fn classifies_check_as_transfer() {
    assert_eq!(
        classify_transaction("Check #1575", &defaults()),
        "transfer"
    );
}

#[test]
fn custom_rules_still_override_expanded_builtins() {
    let mut rules = vec![SpendingRule {
        pattern: "STARBUCKS".to_string(),
        category: "other".to_string(),
    }];
    rules.extend(defaults());
    assert_eq!(
        classify_transaction("STARBUCKS #1234", &rules),
        "other"
    );
}

// --- Data-driven tests ---

#[test]
fn empty_rules_classifies_everything_as_other() {
    // With no rules at all, everything should be Other
    assert_eq!(
        classify_transaction("CHIPOTLE", &[]),
        OTHER_CATEGORY
    );
}

#[test]
fn spending_reports_unclassified_descriptions() {
    let txns = vec![
        make_txn("UNKNOWN VENDOR ABC", dec!(-25.00)),
        make_txn("MYSTERY CHARGE XYZ", dec!(-10.00)),
        make_txn("CHIPOTLE #1", dec!(-15.00)),
    ];
    let summary = compute_spending(&txns, &Categorizer::from_rules(defaults()));
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
        category: "entertainment".to_string(),
    }];
    assert_eq!(
        classify_transaction("SOMETHING FOO HERE", &rules),
        "entertainment"
    );
    assert_eq!(
        classify_transaction("THE BAR", &rules),
        "entertainment"
    );
    assert_eq!(
        classify_transaction("NO MATCH", &rules),
        OTHER_CATEGORY
    );
}

// --- User-defined category test ---

#[test]
fn user_defined_category_works() {
    // Any string can be a category — no enum required
    let rules = vec![SpendingRule {
        pattern: "red cross|salvation army".to_string(),
        category: "donations".to_string(),
    }];
    assert_eq!(
        classify_transaction("RED CROSS DONATION", &rules),
        "donations"
    );
}

// --- Edits overlay and shared rule loading ---

fn hide(edits: &mut TransactionEdits, txn: &TransactionWithContext) {
    edits.set(
        &txn.account_id,
        &txn.id,
        TransactionEdit {
            hidden: true,
            ..Default::default()
        },
    );
}

fn recategorize(edits: &mut TransactionEdits, txn: &TransactionWithContext, category: &str) {
    edits.set(
        &txn.account_id,
        &txn.id,
        TransactionEdit {
            category: Some(category.to_string()),
            ..Default::default()
        },
    );
}

fn with_id(mut txn: TransactionWithContext, id: &str, posted: i64) -> TransactionWithContext {
    txn.id = id.to_string();
    txn.posted = posted;
    txn
}

#[test]
fn spending_skips_hidden_and_honors_category_edits() {
    let dup = with_id(make_txn("CHIPOTLE", dec!(-15.00)), "dup", 1000);
    let gift = with_id(make_txn("UNKNOWN VENDOR", dec!(-40.00)), "gift", 1000);
    let txns = vec![
        with_id(make_txn("CHIPOTLE", dec!(-15.00)), "t1", 1000),
        dup.clone(),
        gift.clone(),
    ];
    let mut edits = TransactionEdits::default();
    hide(&mut edits, &dup);
    recategorize(&mut edits, &gift, "gifts");
    let categorizer = Categorizer::new(defaults(), edits, Default::default());

    let summary = compute_spending(&txns, &categorizer);

    assert_eq!(summary.total_spending, dec!(-55.00));
    let restaurants = summary.categories.iter().find(|c| c.category == "restaurants").unwrap();
    assert_eq!(restaurants.transaction_count, 1);
    assert!(summary.categories.iter().any(|c| c.category == "gifts"));
    assert!(summary.unclassified.is_empty(), "edited category is not unclassified");
}

#[test]
fn trends_skip_hidden_and_non_expense_kinds() {
    let now = chrono::Utc::now().timestamp();
    let hidden = with_id(make_txn("WHOLE FOODS", dec!(-50.00)), "h", now - 86400);
    let moved = with_id(make_txn("CHIPOTLE", dec!(-20.00)), "m", now - 86400);
    let txns = vec![
        with_id(make_txn("WHOLE FOODS", dec!(-30.00)), "g", now - 86400),
        hidden.clone(),
        moved.clone(),
    ];
    let mut edits = TransactionEdits::default();
    hide(&mut edits, &hidden);
    recategorize(&mut edits, &moved, "transfer");
    let categorizer = Categorizer::new(defaults(), edits, Default::default());

    let summary = compute_trends(&txns, &categorizer, 6);

    let total: Decimal = summary.monthly_totals.iter().map(|m| m.total).sum();
    assert_eq!(total, dec!(-30.00));
}

#[test]
fn trends_honor_category_kind_overrides() {
    let now = chrono::Utc::now().timestamp();
    let txns = vec![
        with_id(make_txn("WHOLE FOODS", dec!(-30.00)), "g", now - 86400),
        with_id(make_txn("CHIPOTLE", dec!(-20.00)), "r", now - 86400),
    ];
    let mut kinds = std::collections::HashMap::new();
    kinds.insert("restaurants".to_string(), simplefin::CategoryKind::Transfer);
    let categorizer = Categorizer::new(defaults(), TransactionEdits::default(), kinds);

    let summary = compute_trends(&txns, &categorizer, 6);

    let total: Decimal = summary.monthly_totals.iter().map(|m| m.total).sum();
    assert_eq!(total, dec!(-30.00));
}

#[test]
fn recurring_skips_hidden_transactions() {
    let day = 86400i64;
    let base = 1_700_000_000i64;
    let txns: Vec<_> = (0..3)
        .map(|i| with_id(make_txn("STREAMING SERVICE", dec!(-9.99)), &format!("s{i}"), base + i * 30 * day))
        .collect();
    let mut edits = TransactionEdits::default();
    hide(&mut edits, &txns[1]);
    hide(&mut edits, &txns[2]);
    let categorizer = Categorizer::new(Vec::new(), edits, Default::default());

    assert!(detect_recurring(&txns, &categorizer, 2).recurring.is_empty());
    assert_eq!(
        detect_recurring(&txns, &Categorizer::default(), 2).recurring.len(),
        1
    );
}

#[test]
fn recurring_category_honors_edits() {
    let day = 86400i64;
    let base = 1_700_000_000i64;
    let txns: Vec<_> = (0..3)
        .map(|i| with_id(make_txn("STREAMING SERVICE", dec!(-9.99)), &format!("s{i}"), base + i * 30 * day))
        .collect();
    let mut edits = TransactionEdits::default();
    recategorize(&mut edits, &txns[0], "subscriptions");
    let categorizer = Categorizer::new(Vec::new(), edits, Default::default());

    let summary = detect_recurring(&txns, &categorizer, 2);
    assert_eq!(summary.recurring[0].category, "subscriptions");
}

#[test]
fn load_spending_rules_puts_user_rules_before_patterns() {
    let dir = tempfile::tempdir().unwrap();
    let storage = JsonStorage::open(dir.path()).unwrap();
    storage
        .set_spending_patterns(&[SpendingRule {
            pattern: "cafe".into(),
            category: "restaurants".into(),
        }])
        .unwrap();
    let mut config = DataConfig::default();
    config.spending_rules.push(SpendingRule {
        pattern: "corner".into(),
        category: "coffee".into(),
    });
    storage.set_config(&config).unwrap();

    let rules = load_spending_rules(&storage).unwrap();
    assert_eq!(rules.len(), 2);
    assert_eq!(rules[0].category, "coffee");
    assert_eq!(rules[1].category, "restaurants");
}

#[test]
fn add_user_spending_rule_replaces_same_pattern_case_insensitively() {
    let mut config = DataConfig::default();
    add_user_spending_rule(
        &mut config,
        SpendingRule {
            pattern: "Corner Cafe".into(),
            category: "restaurants".into(),
        },
    );
    add_user_spending_rule(
        &mut config,
        SpendingRule {
            pattern: "market".into(),
            category: "groceries".into(),
        },
    );
    add_user_spending_rule(
        &mut config,
        SpendingRule {
            pattern: "corner cafe".into(),
            category: "coffee".into(),
        },
    );
    assert_eq!(config.spending_rules.len(), 2);
    assert_eq!(config.spending_rules[0].pattern, "corner cafe");
    assert_eq!(config.spending_rules[0].category, "coffee");
    assert_eq!(config.spending_rules[1].pattern, "market");
}

#[test]
fn recurring_expenses_exclude_income_and_credits() {
    let day = 86400i64;
    let base = 1_700_000_000i64;
    let mut txns = Vec::new();
    for i in 0..3 {
        let at = base + i * 30 * day;
        // Categorized income, an unclassified recurring deposit, and a real bill.
        txns.push(with_id(make_txn("PAYROLL DIRECT DEP", dec!(2500.00)), &format!("p{i}"), at));
        txns.push(with_id(make_txn("PAY DAY!", dec!(2262.24)), &format!("d{i}"), at));
        txns.push(with_id(make_txn("STREAMING SERVICE", dec!(-9.99)), &format!("s{i}"), at));
    }
    let summary = detect_recurring(&txns, &Categorizer::from_rules(defaults()), 2);
    let merchants: Vec<_> = summary.recurring.iter().map(|r| r.merchant.as_str()).collect();
    assert_eq!(merchants.len(), 1, "only the bill recurs as an expense: {merchants:?}");
    assert!(summary.estimated_monthly_total.abs() < dec!(20));
}

#[test]
fn near_daily_purchases_are_labeled_daily_not_weekly() {
    let day = 86400i64;
    let base = 1_700_000_000i64;
    let txns: Vec<_> = (0..10)
        .map(|i| with_id(make_txn("CORNER COFFEE", dec!(-4.50)), &format!("c{i}"), base + i * day))
        .collect();
    let summary = detect_recurring(&txns, &Categorizer::default(), 3);
    assert_eq!(summary.recurring[0].frequency_label, "daily");
    let weekly: Vec<_> = (0..5)
        .map(|i| with_id(make_txn("YOGA STUDIO", dec!(-20.00)), &format!("y{i}"), base + i * 7 * day))
        .collect();
    let summary = detect_recurring(&weekly, &Categorizer::default(), 3);
    assert_eq!(summary.recurring[0].frequency_label, "weekly");
}
