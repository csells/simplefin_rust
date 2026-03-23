use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fmt;

use crate::storage::{BalanceSnapshot, ClassificationField, DataConfig, UnifiedAccount};

/// Standard account categories for net worth reporting.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AccountCategory {
    Cash,
    Investments,
    OtherAssets,
    CreditCards,
    Loans,
}

impl fmt::Display for AccountCategory {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Cash => write!(f, "Cash"),
            Self::Investments => write!(f, "Investments"),
            Self::OtherAssets => write!(f, "Other Assets"),
            Self::CreditCards => write!(f, "Credit Cards"),
            Self::Loans => write!(f, "Loans"),
        }
    }
}

/// Classify an account into an Standard category based on name and org.
pub fn classify_account(name: &str, org_name: &str) -> AccountCategory {
    let lower_name = name.to_lowercase();
    let lower_org = org_name.to_lowercase();

    // Mortgage and loans
    if lower_name.contains("mortgage") || lower_name.contains("loan") {
        return AccountCategory::Loans;
    }

    // Credit cards — check before investments since some card names contain org keywords
    // Chase cards often show as "X. LASTNAME (NNNN)" — no "card" keyword
    if lower_name.contains("visa")
        || lower_name.contains("mastercard")
        || lower_name.contains("mc ")
        || lower_name.ends_with(" mc")
        || lower_name.contains("credit card")
        || lower_name.contains("sapphire")
        || lower_name.contains("freedom")
        || lower_name.contains("skymiles")
        || lower_name.contains("rewards")
        || (lower_org.contains("american express") && !lower_name.contains("savings"))
        || (lower_org.contains("chase")
            && !lower_name.contains("checking")
            && !lower_name.contains("savings"))
    {
        return AccountCategory::CreditCards;
    }

    // Cash — checking and savings (must precede investments check, since
    // account names can contain investment substrings like "roth")
    if lower_name.contains("checking") || lower_name.contains("savings") {
        return AccountCategory::Cash;
    }

    // Investments — brokerage, IRA, 401(k), etc.
    if lower_name.contains("401")
        || lower_name.contains("ira")
        || lower_name.contains("brokerage")
        || lower_name.contains("roth")
        || lower_org.contains("vanguard")
        || lower_org.contains("schwab")
        || lower_org.contains("fidelity")
    {
        return AccountCategory::Investments;
    }

    // Other assets — real estate, vehicles, HSA
    if lower_name.contains("home")
        || lower_name.contains("cottage")
        || lower_name.contains("house")
        || lower_name.contains("property")
        || lower_name.contains("car")
        || lower_name.contains("vehicle")
        || lower_name.contains("hsa")
        || lower_org.contains("healthequity")
    {
        return AccountCategory::OtherAssets;
    }

    // Default: other assets
    AccountCategory::OtherAssets
}

/// Per-account detail within a category.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AccountDetail {
    pub id: String,
    pub name: String,
    pub org_name: String,
    pub balance: Decimal,
}

/// Per-category total in a net worth summary.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CategoryTotal {
    pub category: AccountCategory,
    pub label: String,
    pub total: Decimal,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub accounts: Vec<AccountDetail>,
}

/// Categorized net worth summary.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetWorthSummary {
    pub categories: Vec<CategoryTotal>,
    pub total_assets: Decimal,
    pub total_liabilities: Decimal,
    pub net_worth: Decimal,
}

/// Returns true if this account should be excluded based on config patterns.
fn is_excluded(account: &UnifiedAccount, config: &DataConfig) -> bool {
    let lower_name = account.name.to_lowercase();
    config
        .excluded_account_patterns
        .iter()
        .any(|pattern| lower_name.contains(&pattern.to_lowercase()))
}

/// Returns the display name for an account, falling back to the original name.
pub fn display_name_for(account: &UnifiedAccount, config: &DataConfig) -> String {
    config
        .display_names
        .get(&account.id)
        .cloned()
        .unwrap_or_else(|| account.name.clone())
}

/// Classify an account, respecting config overrides and rules.
/// Priority: ID override > classification rules > heuristic classifier.
fn classify_with_config(account: &UnifiedAccount, config: &DataConfig) -> AccountCategory {
    if let Some(&cat) = config.classification_overrides.get(&account.id) {
        return cat;
    }

    let lower_name = account.name.to_lowercase();
    let lower_org = account.org_name.to_lowercase();
    for rule in &config.classification_rules {
        let lower_pattern = rule.pattern.to_lowercase();
        let matches = match rule.field {
            ClassificationField::Name => lower_name.contains(&lower_pattern),
            ClassificationField::Org => lower_org.contains(&lower_pattern),
        };
        if matches {
            return rule.category;
        }
    }

    classify_account(&account.name, &account.org_name)
}

/// Compute net worth grouped by standard categories.
///
/// Accounts matching `config.excluded_account_patterns` are excluded.
/// When `detail` is true, each category includes per-account breakdowns.
pub fn compute_net_worth(accounts: &[UnifiedAccount], config: &DataConfig) -> NetWorthSummary {
    compute_net_worth_detail(accounts, config, false)
}

/// Compute net worth with optional per-account detail.
pub fn compute_net_worth_detail(
    accounts: &[UnifiedAccount],
    config: &DataConfig,
    detail: bool,
) -> NetWorthSummary {
    let mut by_category: HashMap<AccountCategory, Decimal> = HashMap::new();
    let mut accounts_by_category: HashMap<AccountCategory, Vec<AccountDetail>> = HashMap::new();

    for account in accounts {
        if is_excluded(account, config) {
            continue;
        }

        let cat = classify_with_config(account, config);
        *by_category.entry(cat).or_insert(Decimal::ZERO) += account.balance;

        if detail {
            accounts_by_category
                .entry(cat)
                .or_default()
                .push(AccountDetail {
                    id: account.id.clone(),
                    name: display_name_for(account, config),
                    org_name: account.org_name.clone(),
                    balance: account.balance,
                });
        }
    }

    // Sort accounts within each category by absolute balance descending
    if detail {
        for accounts in accounts_by_category.values_mut() {
            accounts.sort_by(|a, b| b.balance.abs().cmp(&a.balance.abs()));
        }
    }

    // Build sorted category list
    let order = [
        AccountCategory::Cash,
        AccountCategory::Investments,
        AccountCategory::OtherAssets,
        AccountCategory::CreditCards,
        AccountCategory::Loans,
    ];
    let categories: Vec<CategoryTotal> = order
        .iter()
        .filter_map(|cat| {
            by_category.get(cat).map(|&total| CategoryTotal {
                category: *cat,
                label: cat.to_string(),
                total,
                accounts: accounts_by_category.remove(cat).unwrap_or_default(),
            })
        })
        .collect();

    let total_assets = categories
        .iter()
        .filter(|c| {
            matches!(
                c.category,
                AccountCategory::Cash | AccountCategory::Investments | AccountCategory::OtherAssets
            )
        })
        .map(|c| c.total)
        .sum();

    let total_liabilities = categories
        .iter()
        .filter(|c| {
            matches!(
                c.category,
                AccountCategory::CreditCards | AccountCategory::Loans
            )
        })
        .map(|c| c.total)
        .sum();

    NetWorthSummary {
        categories,
        total_assets,
        total_liabilities,
        net_worth: total_assets + total_liabilities, // liabilities are negative
    }
}

/// A balance change for a single account between two points in time.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BalanceChange {
    pub account_id: String,
    pub account_name: String,
    pub org_name: String,
    pub previous_balance: Decimal,
    pub current_balance: Decimal,
    pub change: Decimal,
    pub category: AccountCategory,
}

/// Compute balance changes between two sets of snapshots.
///
/// Uses the most recent snapshot per account in each set.
/// Accounts matching `config.excluded_account_patterns` are excluded.
pub fn compute_changes(
    accounts: &[UnifiedAccount],
    current_snapshots: &[BalanceSnapshot],
    previous_snapshots: &[BalanceSnapshot],
    config: &DataConfig,
) -> Vec<BalanceChange> {
    let latest = |snapshots: &[BalanceSnapshot]| -> HashMap<String, Decimal> {
        let mut map: HashMap<String, (i64, Decimal)> = HashMap::new();
        for s in snapshots {
            map.entry(s.account_id.clone())
                .and_modify(|(ts, bal)| {
                    if s.timestamp > *ts {
                        *ts = s.timestamp;
                        *bal = s.balance;
                    }
                })
                .or_insert((s.timestamp, s.balance));
        }
        map.into_iter().map(|(k, (_, v))| (k, v)).collect()
    };

    let current_map = latest(current_snapshots);
    let previous_map = latest(previous_snapshots);

    let mut changes = Vec::new();
    for account in accounts {
        if is_excluded(account, config) {
            continue;
        }

        let current_bal = current_map
            .get(&account.id)
            .copied()
            .unwrap_or(account.balance);
        let previous_bal = previous_map
            .get(&account.id)
            .copied()
            .unwrap_or(Decimal::ZERO);
        let change = current_bal - previous_bal;

        if change != Decimal::ZERO {
            changes.push(BalanceChange {
                account_id: account.id.clone(),
                account_name: account.name.clone(),
                org_name: account.org_name.clone(),
                previous_balance: previous_bal,
                current_balance: current_bal,
                change,
                category: classify_with_config(account, config),
            });
        }
    }

    changes
}
