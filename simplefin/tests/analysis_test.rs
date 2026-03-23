use rust_decimal::Decimal;
use rust_decimal_macros::dec;
use simplefin::{
    AccountCategory, AccountSource, BalanceSnapshot, ClassificationField, ClassificationRule,
    DataConfig, UnifiedAccount, account_is_excluded, classify_account, classify_for_display,
    compute_changes, compute_net_worth, compute_net_worth_detail, compute_net_worth_history,
    display_name_for,
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

// --- classification rules tests ---

#[test]
fn classification_rule_overrides_heuristic() {
    let accounts = vec![make_account(
        "acc1",
        "Unknown Account",
        "Unknown Org",
        dec!(5000),
    )];
    let config = DataConfig {
        classification_rules: vec![ClassificationRule {
            pattern: "Unknown Account".to_string(),
            field: ClassificationField::Name,
            category: AccountCategory::Cash,
        }],
        ..Default::default()
    };
    let summary = compute_net_worth(&accounts, &config);
    let cash = summary
        .categories
        .iter()
        .find(|c| c.category == AccountCategory::Cash);
    assert!(cash.is_some());
    assert_eq!(cash.unwrap().total, dec!(5000));
}

#[test]
fn classification_rule_matches_org() {
    let accounts = vec![make_account(
        "acc1",
        "Generic Account",
        "My Special Bank",
        dec!(1000),
    )];
    let config = DataConfig {
        classification_rules: vec![ClassificationRule {
            pattern: "Special Bank".to_string(),
            field: ClassificationField::Org,
            category: AccountCategory::Investments,
        }],
        ..Default::default()
    };
    let summary = compute_net_worth(&accounts, &config);
    let inv = summary
        .categories
        .iter()
        .find(|c| c.category == AccountCategory::Investments);
    assert!(inv.is_some());
    assert_eq!(inv.unwrap().total, dec!(1000));
}

#[test]
fn id_override_beats_classification_rule() {
    let accounts = vec![make_account("acc1", "Checking", "Bank", dec!(2000))];
    let mut overrides = std::collections::HashMap::new();
    overrides.insert("acc1".to_string(), AccountCategory::Investments);
    let config = DataConfig {
        classification_overrides: overrides,
        classification_rules: vec![ClassificationRule {
            pattern: "Checking".to_string(),
            field: ClassificationField::Name,
            category: AccountCategory::Loans,
        }],
        ..Default::default()
    };
    let summary = compute_net_worth(&accounts, &config);
    let inv = summary
        .categories
        .iter()
        .find(|c| c.category == AccountCategory::Investments);
    assert!(inv.is_some());
    assert_eq!(inv.unwrap().total, dec!(2000));
}

// --- display names tests ---

#[test]
fn display_name_uses_config_override() {
    let account = make_account("acc1", "Raw Name (1234)", "Bank", dec!(100));
    let mut names = std::collections::HashMap::new();
    names.insert("acc1".to_string(), "Friendly Name".to_string());
    let config = DataConfig {
        display_names: names,
        ..Default::default()
    };
    assert_eq!(display_name_for(&account, &config), "Friendly Name");
}

#[test]
fn display_name_falls_back_to_original() {
    let account = make_account("acc1", "Original Name", "Bank", dec!(100));
    assert_eq!(
        display_name_for(&account, &default_config()),
        "Original Name"
    );
}

// --- detail mode tests ---

#[test]
fn detail_mode_includes_account_breakdown() {
    let accounts = vec![
        make_account("1", "Checking A", "Bank", dec!(5000)),
        make_account("2", "Checking B", "Bank", dec!(3000)),
    ];
    let summary = compute_net_worth_detail(&accounts, &default_config(), true);
    let cash = summary
        .categories
        .iter()
        .find(|c| c.category == AccountCategory::Cash)
        .unwrap();
    assert_eq!(cash.accounts.len(), 2);
    // Sorted by absolute balance descending
    assert_eq!(cash.accounts[0].name, "Checking A");
    assert_eq!(cash.accounts[0].balance, dec!(5000));
    assert_eq!(cash.accounts[1].name, "Checking B");
}

#[test]
fn non_detail_mode_omits_accounts() {
    let accounts = vec![make_account("1", "Checking", "Bank", dec!(5000))];
    let summary = compute_net_worth_detail(&accounts, &default_config(), false);
    let cash = summary
        .categories
        .iter()
        .find(|c| c.category == AccountCategory::Cash)
        .unwrap();
    assert!(cash.accounts.is_empty());
}

#[test]
fn detail_mode_uses_display_names() {
    let accounts = vec![make_account("acc1", "Checking (1234)", "Bank", dec!(5000))];
    let mut names = std::collections::HashMap::new();
    names.insert("acc1".to_string(), "My Checking".to_string());
    let config = DataConfig {
        display_names: names,
        ..Default::default()
    };
    let summary = compute_net_worth_detail(&accounts, &config, true);
    let cash = summary
        .categories
        .iter()
        .find(|c| c.category == AccountCategory::Cash)
        .unwrap();
    assert_eq!(cash.accounts[0].name, "My Checking");
}

// --- excluded_account_ids tests ---

#[test]
fn excluded_by_id() {
    let account = make_account("acc1", "Checking", "Bank", dec!(1000));
    let config = DataConfig {
        excluded_account_ids: vec!["acc1".to_string()],
        ..Default::default()
    };
    assert!(account_is_excluded(&account, &config));
}

#[test]
fn not_excluded_by_id() {
    let account = make_account("acc1", "Checking", "Bank", dec!(1000));
    let config = DataConfig {
        excluded_account_ids: vec!["acc2".to_string()],
        ..Default::default()
    };
    assert!(!account_is_excluded(&account, &config));
}

#[test]
fn excluded_by_id_skips_net_worth() {
    let accounts = vec![
        make_account("acc1", "Checking", "Bank", dec!(5000)),
        make_account("acc2", "Savings", "Bank", dec!(3000)),
    ];
    let config = DataConfig {
        excluded_account_ids: vec!["acc1".to_string()],
        ..Default::default()
    };
    let summary = compute_net_worth(&accounts, &config);
    // Only acc2 should be counted
    assert_eq!(summary.net_worth, dec!(3000));
}

// --- classify_for_display tests ---

#[test]
fn classify_for_display_no_override() {
    let account = make_account("acc1", "Checking", "Bank", dec!(1000));
    let info = classify_for_display(&account, &default_config());
    assert_eq!(info.heuristic, AccountCategory::Cash);
    assert_eq!(info.effective, AccountCategory::Cash);
    assert!(!info.overridden);
}

#[test]
fn classify_for_display_with_override() {
    let account = make_account("acc1", "Checking", "Bank", dec!(1000));
    let mut overrides = std::collections::HashMap::new();
    overrides.insert("acc1".to_string(), AccountCategory::Investments);
    let config = DataConfig {
        classification_overrides: overrides,
        ..Default::default()
    };
    let info = classify_for_display(&account, &config);
    assert_eq!(info.heuristic, AccountCategory::Cash);
    assert_eq!(info.effective, AccountCategory::Investments);
    assert!(info.overridden);
}

#[test]
fn classify_for_display_confident_for_clear_match() {
    let account = make_account("acc1", "Checking", "Bank", dec!(1000));
    let info = classify_for_display(&account, &default_config());
    assert!(info.confident);
}

#[test]
fn classify_for_display_low_confidence_default_bucket() {
    // "Unknown Account" falls to OtherAssets default — low confidence
    let account = make_account("acc1", "Unknown Account", "Unknown Org", dec!(1000));
    let info = classify_for_display(&account, &default_config());
    assert_eq!(info.heuristic, AccountCategory::OtherAssets);
    assert!(!info.confident);
}

#[test]
fn classify_for_display_low_confidence_chase_fallback() {
    // Chase org-level fallback to CreditCards — low confidence
    let account = make_account("acc1", "Some Account (1234)", "Chase Bank", dec!(-500));
    let info = classify_for_display(&account, &default_config());
    assert_eq!(info.heuristic, AccountCategory::CreditCards);
    assert!(!info.confident);
}

#[test]
fn classify_for_display_high_confidence_explicit_card() {
    // "Sapphire" is an explicit credit card keyword — high confidence
    let account = make_account("acc1", "Sapphire Reserve", "Chase Bank", dec!(-500));
    let info = classify_for_display(&account, &default_config());
    assert_eq!(info.heuristic, AccountCategory::CreditCards);
    assert!(info.confident);
}

// --- net worth history tests ---

#[test]
fn history_empty_snapshots() {
    let accounts = vec![make_account("acc1", "Checking", "Bank", dec!(1000))];
    let history = compute_net_worth_history(&[], &accounts, &default_config(), 5);
    assert!(history.is_empty());
}

#[test]
fn history_single_timestamp() {
    let accounts = vec![make_account("acc1", "Checking", "Bank", dec!(1000))];
    let snapshots = vec![BalanceSnapshot {
        account_id: "acc1".to_string(),
        timestamp: 100,
        balance: dec!(1000),
    }];
    let history = compute_net_worth_history(&snapshots, &accounts, &default_config(), 5);
    assert_eq!(history.len(), 1);
    assert_eq!(history[0].timestamp, 100);
    assert_eq!(history[0].net_worth, dec!(1000));
}

#[test]
fn history_multiple_timestamps() {
    let accounts = vec![make_account("acc1", "Checking", "Bank", dec!(3000))];
    let snapshots = vec![
        BalanceSnapshot {
            account_id: "acc1".to_string(),
            timestamp: 100,
            balance: dec!(1000),
        },
        BalanceSnapshot {
            account_id: "acc1".to_string(),
            timestamp: 200,
            balance: dec!(2000),
        },
        BalanceSnapshot {
            account_id: "acc1".to_string(),
            timestamp: 300,
            balance: dec!(3000),
        },
    ];
    let history = compute_net_worth_history(&snapshots, &accounts, &default_config(), 5);
    assert_eq!(history.len(), 3);
    assert_eq!(history[0].net_worth, dec!(1000));
    assert_eq!(history[1].net_worth, dec!(2000));
    assert_eq!(history[2].net_worth, dec!(3000));
}

#[test]
fn history_respects_n_limit() {
    let accounts = vec![make_account("acc1", "Checking", "Bank", dec!(3000))];
    let snapshots = vec![
        BalanceSnapshot {
            account_id: "acc1".to_string(),
            timestamp: 100,
            balance: dec!(1000),
        },
        BalanceSnapshot {
            account_id: "acc1".to_string(),
            timestamp: 200,
            balance: dec!(2000),
        },
        BalanceSnapshot {
            account_id: "acc1".to_string(),
            timestamp: 300,
            balance: dec!(3000),
        },
    ];
    let history = compute_net_worth_history(&snapshots, &accounts, &default_config(), 2);
    assert_eq!(history.len(), 2);
    // Should be the last 2 timestamps
    assert_eq!(history[0].timestamp, 200);
    assert_eq!(history[1].timestamp, 300);
}

#[test]
fn history_excludes_configured_accounts() {
    let accounts = vec![
        make_account("acc1", "Checking", "Bank", dec!(5000)),
        make_account("acc2", "Savings", "Bank", dec!(3000)),
    ];
    let snapshots = vec![
        BalanceSnapshot {
            account_id: "acc1".to_string(),
            timestamp: 100,
            balance: dec!(5000),
        },
        BalanceSnapshot {
            account_id: "acc2".to_string(),
            timestamp: 100,
            balance: dec!(3000),
        },
    ];
    let config = DataConfig {
        excluded_account_ids: vec!["acc1".to_string()],
        ..Default::default()
    };
    let history = compute_net_worth_history(&snapshots, &accounts, &config, 5);
    assert_eq!(history.len(), 1);
    // Only acc2 counted
    assert_eq!(history[0].net_worth, dec!(3000));
}
