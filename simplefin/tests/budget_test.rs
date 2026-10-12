//! Monthly budget status: pace, comparisons, averages, category limits.

use chrono::NaiveDate;
use rust_decimal::Decimal;
use rust_decimal_macros::dec;
use simplefin::storage::{DataConfig, TransactionWithContext};
use simplefin::{
    BudgetConfig, Categorizer, OTHER_CATEGORY, SpendingRule, compute_budget_status,
    default_spending_patterns,
};

fn ts(y: i32, m: u32, d: u32) -> i64 {
    NaiveDate::from_ymd_opt(y, m, d)
        .unwrap()
        .and_hms_opt(12, 0, 0)
        .unwrap()
        .and_utc()
        .timestamp()
}

fn txn(id: &str, posted: i64, amount: Decimal, description: &str) -> TransactionWithContext {
    TransactionWithContext {
        id: id.into(),
        account_id: "chk".into(),
        account_name: "Checking (1234)".into(),
        org_name: "Example Bank".into(),
        currency: "USD".into(),
        posted,
        amount,
        description: description.into(),
        ..Default::default()
    }
}

/// "Now" is 2024-03-15 (March has 31 days).
fn now() -> i64 {
    ts(2024, 3, 15)
}

fn history() -> Vec<TransactionWithContext> {
    vec![
        txn("nov", ts(2023, 11, 10), dec!(-1000.00), "WHOLE FOODS"),
        txn("dec", ts(2023, 12, 10), dec!(-700.00), "WHOLE FOODS"),
        txn("jan", ts(2024, 1, 10), dec!(-500.00), "WHOLE FOODS"),
        txn("feb1", ts(2024, 2, 5), dec!(-400.00), "WHOLE FOODS"),
        txn("feb2", ts(2024, 2, 20), dec!(-200.00), "WHOLE FOODS"),
        txn("mar1", ts(2024, 3, 2), dec!(-300.00), "WHOLE FOODS"),
        txn("mar2", ts(2024, 3, 10), dec!(-100.00), "CHIPOTLE"),
        txn("mar3", ts(2024, 3, 14), dec!(-50.00), "AMAZON"),
        txn(
            "marx",
            ts(2024, 3, 14),
            dec!(-500.00),
            "ONLINE TRANSFER TO SAVINGS",
        ),
        txn("marpay", ts(2024, 3, 1), dec!(3000.00), "PAYROLL DEPOSIT"),
    ]
}

fn config_with_limit(limit: Option<Decimal>) -> DataConfig {
    DataConfig {
        budget: BudgetConfig {
            monthly_limit: limit,
            ..Default::default()
        },
        ..Default::default()
    }
}

fn categorizer() -> Categorizer {
    Categorizer::from_rules(default_spending_patterns())
}

#[test]
fn reports_spent_against_limit_with_linear_pace() {
    let status = compute_budget_status(
        &history(),
        &config_with_limit(Some(dec!(1000))),
        &categorizer(),
        now(),
    );

    assert_eq!(status.month, "2024-03");
    assert_eq!(status.day_of_month, 15);
    assert_eq!(status.days_in_month, 31);
    assert_eq!(status.spent, dec!(450.00));
    assert_eq!(status.monthly_limit, Some(dec!(1000)));
    assert_eq!(status.remaining, Some(dec!(550.00)));
    assert!(!status.over_budget);
    // 1000 * 15 / 31
    assert_eq!(status.expected_spend, Some(dec!(483.87)));
    assert_eq!(status.suggested_monthly_limit, None);
}

#[test]
fn over_budget_when_spent_exceeds_limit() {
    let status = compute_budget_status(
        &history(),
        &config_with_limit(Some(dec!(400))),
        &categorizer(),
        now(),
    );
    assert!(status.over_budget);
    assert_eq!(status.remaining, Some(dec!(-50.00)));
}

#[test]
fn compares_with_last_month_at_the_same_day() {
    let status = compute_budget_status(&history(), &DataConfig::default(), &categorizer(), now());
    // Only the Feb 5 purchase is on or before Feb 15.
    assert_eq!(status.last_month_same_day, dec!(400.00));
}

#[test]
fn same_day_comparison_clamps_to_short_months() {
    let txns = vec![
        txn("feb", ts(2024, 2, 29), dec!(-10.00), "CHIPOTLE"),
        txn("mar", ts(2024, 3, 31), dec!(-5.00), "CHIPOTLE"),
    ];
    let status = compute_budget_status(
        &txns,
        &DataConfig::default(),
        &categorizer(),
        ts(2024, 3, 31),
    );
    assert_eq!(status.last_month_same_day, dec!(10.00));
}

#[test]
fn averages_complete_months_clipped_to_history() {
    let status = compute_budget_status(&history(), &DataConfig::default(), &categorizer(), now());

    // Dec, Jan, Feb
    assert_eq!(status.average_3_month.amount, dec!(600.00));
    assert_eq!(status.average_3_month.months, 3);
    assert!(!status.average_3_month.extrapolated);
    // History starts in Nov: Nov..Feb
    assert_eq!(status.average_12_month.amount, dec!(700.00));
    assert_eq!(status.average_12_month.months, 4);
}

#[test]
fn suggests_three_month_average_when_no_limit() {
    let status = compute_budget_status(&history(), &DataConfig::default(), &categorizer(), now());
    assert_eq!(status.monthly_limit, None);
    assert_eq!(status.remaining, None);
    assert_eq!(status.expected_spend, None);
    assert!(!status.over_budget);
    assert_eq!(status.suggested_monthly_limit, Some(dec!(600.00)));
}

#[test]
fn thin_history_extrapolates_current_month() {
    let txns = vec![txn("mar", ts(2024, 3, 2), dec!(-150.00), "WHOLE FOODS")];
    let status = compute_budget_status(&txns, &DataConfig::default(), &categorizer(), now());
    // 150 / 15 days * 31 days
    assert_eq!(status.average_3_month.amount, dec!(310.00));
    assert_eq!(status.average_3_month.months, 0);
    assert!(status.average_3_month.extrapolated);
    assert_eq!(status.suggested_monthly_limit, Some(dec!(310.00)));
}

#[test]
fn no_history_means_zero_and_no_suggestion() {
    let status = compute_budget_status(&[], &DataConfig::default(), &categorizer(), now());
    assert_eq!(status.spent, Decimal::ZERO);
    assert_eq!(status.average_3_month.amount, Decimal::ZERO);
    assert!(!status.average_3_month.extrapolated);
    assert_eq!(status.suggested_monthly_limit, None);
    assert!(status.top_categories.is_empty());
}

#[test]
fn top_five_categories_plus_other() {
    let names = [
        "alpha", "bravo", "charlie", "delta", "echo", "foxtrot", "golf",
    ];
    let rules: Vec<SpendingRule> = names
        .iter()
        .map(|n| SpendingRule {
            pattern: n.to_string(),
            category: n.to_string(),
        })
        .collect();
    let mut txns: Vec<_> = names
        .iter()
        .enumerate()
        .map(|(i, n)| {
            txn(
                n,
                ts(2024, 3, 2),
                Decimal::from(-(70 - 10 * i as i64)),
                &n.to_uppercase(),
            )
        })
        .collect();
    txns.push(txn("unk", ts(2024, 3, 3), dec!(-100), "UNKNOWN VENDOR"));

    let status = compute_budget_status(
        &txns,
        &DataConfig::default(),
        &Categorizer::from_rules(rules),
        now(),
    );

    let cats: Vec<&str> = status
        .top_categories
        .iter()
        .map(|c| c.category.as_str())
        .collect();
    // Real "other" spending is folded into the remainder bucket.
    assert_eq!(
        cats,
        ["alpha", "bravo", "charlie", "delta", "echo", OTHER_CATEGORY]
    );
    // foxtrot 20 + golf 10 + unknown 100
    assert_eq!(status.top_categories[5].amount, dec!(130));
    assert_eq!(status.top_categories[0].label, "Alpha");
}

#[test]
fn per_category_limits() {
    let mut config = DataConfig::default();
    config
        .budget
        .category_limits
        .insert("groceries".into(), dec!(250));
    config
        .budget
        .category_limits
        .insert("restaurants".into(), dec!(310));

    let status = compute_budget_status(&history(), &config, &categorizer(), now());

    assert_eq!(status.category_limits.len(), 2);
    let groceries = &status.category_limits[0];
    assert_eq!(groceries.category, "groceries");
    assert_eq!(groceries.spent, dec!(300.00));
    assert_eq!(groceries.remaining, dec!(-50.00));
    assert!(groceries.over_limit);
    let restaurants = &status.category_limits[1];
    assert_eq!(restaurants.spent, dec!(100.00));
    assert_eq!(restaurants.expected_spend, dec!(150.00));
    assert!(!restaurants.over_limit);
}

#[test]
fn budget_config_roundtrips_and_is_omitted_when_empty() {
    let config = config_with_limit(Some(dec!(1200)));
    let json = serde_json::to_string(&config).unwrap();
    assert_eq!(json, r#"{"budget":{"monthly_limit":"1200"}}"#);
    let back: DataConfig = serde_json::from_str(&json).unwrap();
    assert_eq!(back.budget.monthly_limit, Some(dec!(1200)));
    assert!(BudgetConfig::default().is_empty());
}
