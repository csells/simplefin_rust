use rust_decimal::Decimal;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fmt;

use crate::storage::{
    BalanceSnapshot, ClassificationField, DataConfig, TransactionWithContext, UnifiedAccount,
};

/// Standard account categories for net worth reporting.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
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

    // Investment signals that override cash keywords.
    // "401(K) Savings Plan" should be Investments, not Cash.
    if lower_name.contains("401")
        || lower_name.contains("403b")
        || lower_name.contains("457")
        || lower_name.contains("pension")
        || lower_name.contains("annuity")
        || lower_name.contains("thrift savings")
    {
        return AccountCategory::Investments;
    }

    // Cash — checking and savings (must precede remaining investments check,
    // since account names can contain investment substrings like "roth")
    if lower_name.contains("checking") || lower_name.contains("savings") {
        return AccountCategory::Cash;
    }

    // Investments — brokerage, IRA, org-based (401/403b/457/pension already handled above)
    if lower_name.contains("ira")
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
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct AccountDetail {
    pub id: String,
    pub name: String,
    pub org_name: String,
    pub balance: Decimal,
}

/// Per-category total in a net worth summary.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct CategoryTotal {
    pub category: AccountCategory,
    pub label: String,
    pub total: Decimal,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub accounts: Vec<AccountDetail>,
}

/// Categorized net worth summary.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct NetWorthSummary {
    pub categories: Vec<CategoryTotal>,
    pub total_assets: Decimal,
    pub total_liabilities: Decimal,
    pub net_worth: Decimal,
}

/// Returns true if this account should be excluded based on config patterns or IDs.
fn is_excluded(account: &UnifiedAccount, config: &DataConfig) -> bool {
    is_excluded_parts(&account.id, &account.name, config)
}

fn is_excluded_parts(id: &str, name: &str, config: &DataConfig) -> bool {
    if config.excluded_account_ids.iter().any(|x| x == id) {
        return true;
    }
    let lower_name = name.to_lowercase();
    config
        .excluded_account_patterns
        .iter()
        .any(|pattern| lower_name.contains(&pattern.to_lowercase()))
}

/// Whether a transaction's account is excluded via config (IDs or name patterns).
pub fn transaction_account_excluded(txn: &TransactionWithContext, config: &DataConfig) -> bool {
    is_excluded_parts(&txn.account_id, &txn.account_name, config)
}

/// Effective classification of an account (ID override > rules > heuristic).
pub fn account_category(account: &UnifiedAccount, config: &DataConfig) -> AccountCategory {
    classify_with_config(account, config)
}

/// Effective classification of the account a transaction belongs to.
pub fn transaction_account_category(
    txn: &TransactionWithContext,
    config: &DataConfig,
) -> AccountCategory {
    classify_parts(&txn.account_id, &txn.account_name, &txn.org_name, config)
}

/// Returns the display name for an account, falling back to the original name.
pub fn display_name_for(account: &UnifiedAccount, config: &DataConfig) -> String {
    config
        .display_names
        .get(&account.id)
        .cloned()
        .unwrap_or_else(|| account.name.clone())
}

/// Returns whether an account is excluded from net worth calculations.
pub fn account_is_excluded(account: &UnifiedAccount, config: &DataConfig) -> bool {
    is_excluded(account, config)
}

/// Classification details for display purposes.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClassificationInfo {
    /// What the heuristic classifier would choose (ignoring overrides/rules).
    pub heuristic: AccountCategory,
    /// The effective classification after overrides and rules.
    pub effective: AccountCategory,
    /// Whether the effective classification differs from the heuristic.
    pub overridden: bool,
    /// Whether the heuristic classification is high-confidence.
    /// Low confidence when: org-level fallback, default bucket, or conflicting keywords.
    pub confident: bool,
}

/// Classify an account and return both heuristic and effective classifications.
pub fn classify_for_display(account: &UnifiedAccount, config: &DataConfig) -> ClassificationInfo {
    let heuristic = classify_account(&account.name, &account.org_name);
    let effective = classify_with_config(account, config);
    let confident = is_classification_confident(&account.name, &account.org_name, heuristic);
    ClassificationInfo {
        heuristic,
        effective,
        overridden: heuristic != effective,
        confident,
    }
}

/// Determine whether the heuristic classification is confident.
///
/// Low confidence when: org-level fallback was used (e.g. Chase catch-all),
/// name contains conflicting keywords, or no positive keyword match was found
/// (default to OtherAssets).
fn is_classification_confident(name: &str, org_name: &str, category: AccountCategory) -> bool {
    let lower_name = name.to_lowercase();
    let lower_org = org_name.to_lowercase();

    // Default fallback to OtherAssets is always low confidence
    if category == AccountCategory::OtherAssets
        && !lower_name.contains("home")
        && !lower_name.contains("cottage")
        && !lower_name.contains("house")
        && !lower_name.contains("property")
        && !lower_name.contains("car")
        && !lower_name.contains("vehicle")
        && !lower_name.contains("hsa")
        && !lower_org.contains("healthequity")
    {
        return false;
    }

    // Chase org-level fallback: anything at Chase that isn't checking/savings = credit card
    // This is fragile — flag it as low confidence
    if category == AccountCategory::CreditCards
        && lower_org.contains("chase")
        && !lower_name.contains("credit card")
        && !lower_name.contains("sapphire")
        && !lower_name.contains("freedom")
    {
        return false;
    }

    // AmEx org-level fallback
    if category == AccountCategory::CreditCards
        && lower_org.contains("american express")
        && !lower_name.contains("credit card")
        && !lower_name.contains("card")
    {
        return false;
    }

    true
}

/// Classify an account, respecting config overrides and rules.
/// Priority: ID override > classification rules > heuristic classifier.
fn classify_with_config(account: &UnifiedAccount, config: &DataConfig) -> AccountCategory {
    classify_parts(&account.id, &account.name, &account.org_name, config)
}

fn classify_parts(id: &str, name: &str, org_name: &str, config: &DataConfig) -> AccountCategory {
    if let Some(&cat) = config.classification_overrides.get(id) {
        return cat;
    }

    let lower_name = name.to_lowercase();
    let lower_org = org_name.to_lowercase();
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

    classify_account(name, org_name)
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
            accounts.sort_by_key(|a| std::cmp::Reverse(a.balance.abs()));
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

/// A single point in a net worth time series.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct NetWorthTimePoint {
    pub timestamp: i64,
    pub net_worth: Decimal,
    pub total_assets: Decimal,
    pub total_liabilities: Decimal,
}

/// Distinct snapshot timestamps, ascending, keeping only the last `n`.
fn last_timestamps(snapshots: &[BalanceSnapshot], n: usize) -> Vec<i64> {
    let mut timestamps: Vec<i64> = snapshots.iter().map(|s| s.timestamp).collect();
    timestamps.sort();
    timestamps.dedup();
    let start = timestamps.len().saturating_sub(n);
    timestamps.split_off(start)
}

/// Accounts with balances reconstructed at `timestamp`: each account takes its
/// most recent snapshot at or before that time, or zero if none exists yet.
pub fn reconstruct_accounts_at(
    snapshots: &[BalanceSnapshot],
    accounts: &[UnifiedAccount],
    timestamp: i64,
) -> Vec<UnifiedAccount> {
    accounts
        .iter()
        .map(|account| {
            let balance = snapshots
                .iter()
                .filter(|s| s.account_id == account.id && s.timestamp <= timestamp)
                .max_by_key(|s| s.timestamp)
                .map(|s| s.balance)
                .unwrap_or(Decimal::ZERO);
            UnifiedAccount {
                balance,
                ..account.clone()
            }
        })
        .collect()
}

/// Net worth summary as of `timestamp`, reconstructed from snapshots.
pub fn net_worth_at(
    snapshots: &[BalanceSnapshot],
    accounts: &[UnifiedAccount],
    config: &DataConfig,
    timestamp: i64,
) -> NetWorthSummary {
    compute_net_worth(&reconstruct_accounts_at(snapshots, accounts, timestamp), config)
}

/// Compute net worth at each of the last N distinct snapshot timestamps.
///
/// For each timestamp, reconstructs account balances using the most recent
/// snapshot at or before that time, then classifies and sums them.
pub fn compute_net_worth_history(
    snapshots: &[BalanceSnapshot],
    accounts: &[UnifiedAccount],
    config: &DataConfig,
    n: usize,
) -> Vec<NetWorthTimePoint> {
    last_timestamps(snapshots, n)
        .into_iter()
        .map(|ts| {
            let summary = net_worth_at(snapshots, accounts, config, ts);
            NetWorthTimePoint {
                timestamp: ts,
                net_worth: summary.net_worth,
                total_assets: summary.total_assets,
                total_liabilities: summary.total_liabilities,
            }
        })
        .collect()
}

/// Total of one account category at a point in time.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct CategoryTimePoint {
    pub timestamp: i64,
    pub total: Decimal,
}

fn category_total(summary: &NetWorthSummary, category: AccountCategory) -> Decimal {
    summary
        .categories
        .iter()
        .find(|c| c.category == category)
        .map(|c| c.total)
        .unwrap_or(Decimal::ZERO)
}

/// Per-category balance history at each of the last N distinct snapshot timestamps.
pub fn compute_category_history(
    snapshots: &[BalanceSnapshot],
    accounts: &[UnifiedAccount],
    config: &DataConfig,
    category: AccountCategory,
    n: usize,
) -> Vec<CategoryTimePoint> {
    last_timestamps(snapshots, n)
        .into_iter()
        .map(|ts| CategoryTimePoint {
            timestamp: ts,
            total: category_total(&net_worth_at(snapshots, accounts, config, ts), category),
        })
        .collect()
}

/// Change in net worth (or one category's total) over a trailing window.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct NetWorthChange {
    /// Requested window length in days.
    pub days: u32,
    /// Start of the window, clamped to the earliest snapshot.
    pub start_timestamp: i64,
    pub end_timestamp: i64,
    pub start_value: Decimal,
    pub end_value: Decimal,
    pub change: Decimal,
}

/// Change over the `days` before `now` (for example, 1-day and 90-day chips).
///
/// Values are net worth, or the total of `category` when given. When history
/// is shorter than the window, the window starts at the earliest snapshot.
/// Returns `None` when there are no snapshots.
pub fn compute_net_worth_change(
    snapshots: &[BalanceSnapshot],
    accounts: &[UnifiedAccount],
    config: &DataConfig,
    now: i64,
    days: u32,
    category: Option<AccountCategory>,
) -> Option<NetWorthChange> {
    let earliest = snapshots.iter().map(|s| s.timestamp).min()?;
    let start_timestamp = (now - i64::from(days) * 86_400).max(earliest);
    let value_at = |ts: i64| {
        let summary = net_worth_at(snapshots, accounts, config, ts);
        match category {
            Some(cat) => category_total(&summary, cat),
            None => summary.net_worth,
        }
    };
    let start_value = value_at(start_timestamp);
    let end_value = value_at(now);
    Some(NetWorthChange {
        days,
        start_timestamp,
        end_timestamp: now,
        start_value,
        end_value,
        change: end_value - start_value,
    })
}

/// A balance change for a single account between two points in time.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
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
