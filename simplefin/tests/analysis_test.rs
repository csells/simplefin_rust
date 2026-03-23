use rust_decimal::Decimal;
use rust_decimal_macros::dec;
use simplefin::{
    AccountCategory, AccountSource, BalanceSnapshot, DataConfig, UnifiedAccount, classify_account,
    compute_changes, compute_net_worth,
};

fn default_config() -> DataConfig {
    DataConfig::default()
}

fn make_account(id: &str, name: &str, org: &str, balance: Decimal) -> UnifiedAccount {
    UnifiedAccount {
        id: id.to_string(),
        name: name.to_string(),
        org_name: org.to_string(),
        currency: "USD".to_string(),
        balance,
        available_balance: None,
        balance_date: Some(1000),
        source: AccountSource::Simplefin,
    }
}

// --- classify_account tests ---

#[test]
fn classify_checking_as_cash() {
    assert_eq!(
        classify_account("Main Checking (1234)", "My Credit Union"),
        AccountCategory::Cash
    );
}

#[test]
fn classify_savings_as_cash() {
    assert_eq!(
        classify_account("Basic Savings (5678)", "My Credit Union"),
        AccountCategory::Cash
    );
}

#[test]
fn classify_brokerage_as_investments() {
    assert_eq!(
        classify_account("Brokerage Account (1111)", "Vanguard"),
        AccountCategory::Investments
    );
}

#[test]
fn classify_ira_as_investments() {
    assert_eq!(
        classify_account("Traditional IRA Brokerage Account (2222)", "Vanguard"),
        AccountCategory::Investments
    );
}

#[test]
fn classify_checking_with_roth_substring_as_cash() {
    // "Brotherington" contains "roth" — checking/savings must win
    assert_eq!(
        classify_account("Checking Brotherington, Inc. (3333)", "Some Credit Union"),
        AccountCategory::Cash
    );
}

#[test]
fn classify_savings_with_roth_substring_as_cash() {
    assert_eq!(
        classify_account("Savings Brotherington, Inc. (4444)", "Some Credit Union"),
        AccountCategory::Cash
    );
}

#[test]
fn classify_roth_as_investments() {
    assert_eq!(
        classify_account("Roth IRA Brokerage Account (5555)", "Vanguard"),
        AccountCategory::Investments
    );
}

#[test]
fn classify_401k_as_investments() {
    assert_eq!(
        classify_account("My 401k", "My Provider"),
        AccountCategory::Investments
    );
}

#[test]
fn classify_schwab_as_investments() {
    assert_eq!(
        classify_account("Individual ...999 (999)", "Charles Schwab US"),
        AccountCategory::Investments
    );
}

#[test]
fn classify_mortgage_as_loans() {
    assert_eq!(
        classify_account("Mortgage (6666)", "My Credit Union"),
        AccountCategory::Loans
    );
}

#[test]
fn classify_credit_card_as_credit_cards() {
    assert_eq!(
        classify_account("Rewards World MC (7777)", "My Credit Union"),
        AccountCategory::CreditCards
    );
}

#[test]
fn classify_chase_card_as_credit_cards() {
    assert_eq!(
        classify_account("J. DOE (8888)", "Chase Bank"),
        AccountCategory::CreditCards
    );
}

#[test]
fn classify_amex_as_credit_cards() {
    assert_eq!(
        classify_account("Delta SkyMiles Blue Card (9999)", "American Express"),
        AccountCategory::CreditCards
    );
}

#[test]
fn classify_chase_freedom_as_credit_cards() {
    assert_eq!(
        classify_account("Chase Freedom Unlimited (1010)", "Chase Bank"),
        AccountCategory::CreditCards
    );
}

#[test]
fn classify_hsa_as_other_assets() {
    assert_eq!(
        classify_account("HSA", "My HSA Provider"),
        AccountCategory::OtherAssets
    );
}

#[test]
fn classify_home_as_other_assets() {
    assert_eq!(
        classify_account("Home - 123 Main St", "Manual"),
        AccountCategory::OtherAssets
    );
}

#[test]
fn classify_vehicle_as_other_assets() {
    assert_eq!(
        classify_account("My Car", "Manual"),
        AccountCategory::OtherAssets
    );
}

// --- compute_net_worth tests ---

#[test]
fn net_worth_basic() {
    let accounts = vec![
        make_account("1", "Checking", "Bank", dec!(10000)),
        make_account("2", "Brokerage Account", "Vanguard", dec!(500000)),
        make_account("3", "Home", "Manual", dec!(400000)),
        make_account("4", "Visa Credit Card", "Chase", dec!(-5000)),
        make_account("5", "Mortgage", "Bank", dec!(-200000)),
    ];

    let summary = compute_net_worth(&accounts, &default_config());

    assert_eq!(summary.net_worth, dec!(705000));
    assert_eq!(summary.total_assets, dec!(910000));
    assert_eq!(summary.total_liabilities, dec!(-205000));

    let cash = summary
        .categories
        .iter()
        .find(|c| c.category == AccountCategory::Cash)
        .unwrap();
    assert_eq!(cash.total, dec!(10000));
}

#[test]
fn net_worth_excludes_configured_patterns() {
    let accounts = vec![
        make_account("1", "J. DOE (1234)", "Chase Bank", dec!(-3000)),
        make_account("2", "A. SMITH (5678)", "Chase Bank", dec!(-3000)),
    ];

    let config = DataConfig {
        excluded_account_patterns: vec!["A. SMITH".to_string()],
        ..Default::default()
    };
    let summary = compute_net_worth(&accounts, &config);
    // Only DOE should be counted
    assert_eq!(summary.net_worth, dec!(-3000));
}

#[test]
fn net_worth_no_exclusions_counts_all() {
    let accounts = vec![
        make_account("1", "J. DOE (1234)", "Chase Bank", dec!(-3000)),
        make_account("2", "A. SMITH (5678)", "Chase Bank", dec!(-3000)),
    ];

    let summary = compute_net_worth(&accounts, &default_config());
    assert_eq!(summary.net_worth, dec!(-6000));
}

#[test]
fn net_worth_empty() {
    let summary = compute_net_worth(&[], &default_config());
    assert_eq!(summary.net_worth, dec!(0));
    assert!(summary.categories.is_empty());
}

// --- compute_changes tests ---

#[test]
fn changes_detects_delta() {
    let accounts = vec![make_account("acc1", "Checking", "Bank", dec!(1500))];
    let previous = vec![BalanceSnapshot {
        account_id: "acc1".to_string(),
        timestamp: 100,
        balance: dec!(1000),
    }];
    let current = vec![BalanceSnapshot {
        account_id: "acc1".to_string(),
        timestamp: 200,
        balance: dec!(1500),
    }];

    let changes = compute_changes(&accounts, &current, &previous, &default_config());
    assert_eq!(changes.len(), 1);
    assert_eq!(changes[0].change, dec!(500));
    assert_eq!(changes[0].previous_balance, dec!(1000));
    assert_eq!(changes[0].current_balance, dec!(1500));
}

#[test]
fn changes_skips_no_change() {
    let accounts = vec![make_account("acc1", "Checking", "Bank", dec!(1000))];
    let snapshots = vec![BalanceSnapshot {
        account_id: "acc1".to_string(),
        timestamp: 100,
        balance: dec!(1000),
    }];

    let changes = compute_changes(&accounts, &snapshots, &snapshots, &default_config());
    assert!(changes.is_empty());
}

#[test]
fn changes_excludes_configured_patterns() {
    let accounts = vec![
        make_account("1", "J. DOE (1234)", "Chase Bank", dec!(-5000)),
        make_account("2", "A. SMITH (5678)", "Chase Bank", dec!(-5000)),
    ];
    let previous = vec![
        BalanceSnapshot {
            account_id: "1".to_string(),
            timestamp: 100,
            balance: dec!(-3000),
        },
        BalanceSnapshot {
            account_id: "2".to_string(),
            timestamp: 100,
            balance: dec!(-3000),
        },
    ];
    let current = vec![
        BalanceSnapshot {
            account_id: "1".to_string(),
            timestamp: 200,
            balance: dec!(-5000),
        },
        BalanceSnapshot {
            account_id: "2".to_string(),
            timestamp: 200,
            balance: dec!(-5000),
        },
    ];

    let config = DataConfig {
        excluded_account_patterns: vec!["A. SMITH".to_string()],
        ..Default::default()
    };
    let changes = compute_changes(&accounts, &current, &previous, &config);
    // Only DOE should appear
    assert_eq!(changes.len(), 1);
    assert_eq!(changes[0].account_name, "J. DOE (1234)");
}
