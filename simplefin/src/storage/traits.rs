use std::collections::HashMap;

use rust_decimal::Decimal;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::analysis::AccountCategory;
use crate::error::Result;
use crate::models::{Account, Organization, Transaction};

/// Filter for querying organizations.
#[derive(Debug, Default)]
pub struct OrgFilter {
    pub org_id: Option<String>,
    pub name: Option<String>,
}

/// Filter for querying accounts.
#[derive(Debug, Default)]
pub struct AccountFilter {
    pub account_id: Option<String>,
    pub name: Option<String>,
    pub org_id: Option<String>,
}

/// Filter for querying transactions.
#[derive(Debug, Default)]
pub struct TransactionFilter {
    pub account_id: Option<String>,
    pub org_id: Option<String>,
    pub start_date: Option<i64>,
    pub end_date: Option<i64>,
    pub include_pending: Option<bool>,
}

/// A transaction paired with context from its parent account and organization.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct TransactionWithContext {
    pub id: String,
    pub account_id: String,
    pub account_name: String,
    pub org_name: String,
    pub currency: String,
    pub posted: i64,
    pub amount: Decimal,
    pub description: String,
    pub transacted_at: Option<i64>,
    pub pending: bool,
}

/// A manually-tracked account not connected to SimpleFIN.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ManualAccount {
    pub id: String,
    pub name: String,
    pub org_name: String,
    pub currency: String,
    /// How often this account's balance should be refreshed, in days.
    /// Defaults to 1 (daily) if not set.
    #[serde(default = "default_refresh_days", skip_serializing_if = "is_default_refresh_days")]
    pub refresh_days: u32,
}

fn default_refresh_days() -> u32 {
    1
}

fn is_default_refresh_days(v: &u32) -> bool {
    *v == 1
}

/// A balance snapshot for a single account at a point in time.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct BalanceSnapshot {
    pub account_id: String,
    pub timestamp: i64,
    pub balance: Decimal,
}

/// Filter for querying balance history.
#[derive(Debug, Default)]
pub struct BalanceHistoryFilter {
    pub account_id: Option<String>,
    pub start_date: Option<i64>,
    pub end_date: Option<i64>,
}

/// Source of a unified account.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "lowercase")]
pub enum AccountSource {
    Simplefin,
    Manual,
}

/// A unified view of an account from any source (SimpleFIN or manual).
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct UnifiedAccount {
    pub id: String,
    pub name: String,
    pub org_name: String,
    pub currency: String,
    pub balance: Decimal,
    pub available_balance: Option<Decimal>,
    pub balance_date: Option<i64>,
    pub source: AccountSource,
}

/// Merge SimpleFIN accounts and manual accounts into a unified list.
/// For manual accounts, the latest balance snapshot is used.
pub fn unify_accounts(
    accounts: &[Account],
    manual_accounts: &[ManualAccount],
    balance_history: &[BalanceSnapshot],
) -> Vec<UnifiedAccount> {
    let mut unified = Vec::new();

    for a in accounts {
        unified.push(UnifiedAccount {
            id: a.id.clone(),
            name: a.name.clone(),
            org_name: a.org.display_name().to_string(),
            currency: a.currency.clone(),
            balance: a.balance,
            available_balance: a.available_balance,
            balance_date: Some(a.balance_date),
            source: AccountSource::Simplefin,
        });
    }

    for ma in manual_accounts {
        let latest = balance_history
            .iter()
            .filter(|s| s.account_id == ma.id)
            .max_by_key(|s| s.timestamp);

        unified.push(UnifiedAccount {
            id: ma.id.clone(),
            name: ma.name.clone(),
            org_name: ma.org_name.clone(),
            currency: ma.currency.clone(),
            balance: latest.map(|s| s.balance).unwrap_or(Decimal::ZERO),
            available_balance: None,
            balance_date: latest.map(|s| s.timestamp),
            source: AccountSource::Manual,
        });
    }

    unified
}

/// Which field a classification rule matches against.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ClassificationField {
    Name,
    Org,
}

/// A user-defined classification rule checked before the heuristic classifier.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClassificationRule {
    /// Substring to match (case-insensitive).
    pub pattern: String,
    /// Which field to match against.
    pub field: ClassificationField,
    /// Target category when the pattern matches.
    pub category: AccountCategory,
}

/// Per-user configuration stored alongside financial data.
///
/// Lives in the data directory (not the repo) so user-specific settings
/// like account exclusions don't pollute the codebase.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct DataConfig {
    /// Substrings matched against account names to exclude from net worth.
    /// Example: `["A. SMITH"]` to skip an authorized-user duplicate card.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub excluded_account_patterns: Vec<String>,

    /// Override the heuristic classification for specific account IDs.
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub classification_overrides: HashMap<String, AccountCategory>,

    /// User-defined classification rules checked before the heuristic classifier.
    /// Rules are checked in order; first match wins.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub classification_rules: Vec<ClassificationRule>,

    /// Friendly display names for accounts, keyed by account ID.
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub display_names: HashMap<String, String>,

    /// User-defined rules for classifying transactions into spending categories.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub spending_rules: Vec<crate::spending::SpendingRule>,

    /// Account IDs to exclude from net worth calculations.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub excluded_account_ids: Vec<String>,
}

/// Orphaned data found during cleanup.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrphanedData {
    pub account_id: String,
    pub data_type: OrphanedDataType,
    pub path: String,
}

/// Type of orphaned data.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum OrphanedDataType {
    BalanceHistory,
    Transactions,
}

/// A manual account whose balance is stale and needs updating.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct StaleAccount {
    pub id: String,
    pub name: String,
    pub org_name: String,
    pub last_updated: Option<i64>,
    pub refresh_days: u32,
    pub days_since_update: Option<u64>,
}

/// Warnings and anomalies recorded during the most recent collection.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct WarningRecord {
    /// Epoch timestamp when the collection occurred.
    pub timestamp: i64,
    /// Anomalies detected by comparing current vs previous account balances.
    pub anomalies: Vec<crate::anomaly::Anomaly>,
    /// Informational messages returned by the SimpleFIN bridge.
    pub bridge_messages: Vec<String>,
}

/// A snapshot of the storage state, useful for quick AI-agent assessment.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct StorageStatus {
    /// Epoch timestamp of the most recent collection, if any.
    pub last_collection_time: Option<i64>,
    /// Human-readable description of how long ago the last collection was.
    pub last_collection_ago: Option<String>,
    /// Number of SimpleFIN accounts.
    pub account_count: usize,
    /// Number of manually-tracked accounts.
    pub manual_account_count: usize,
    /// Names of manual accounts whose balances are stale.
    pub stale_manual_accounts: Vec<String>,
    /// Warnings from the most recent collection.
    pub warnings: Option<WarningRecord>,
}

/// Compute a status snapshot from storage, composing existing trait methods.
pub fn compute_status(storage: &dyn Storage, now: i64) -> Result<StorageStatus> {
    let accounts = storage.get_accounts(&AccountFilter::default())?;
    let manual_accounts = storage.get_manual_accounts()?;
    let stale = storage.get_stale_accounts(now)?;
    let warnings = storage.get_warnings()?;

    // Find last collection time from balance history timestamps
    let all_history = storage.get_balance_history(&BalanceHistoryFilter::default())?;
    let last_collection_time = all_history.iter().map(|s| s.timestamp).max();

    let last_collection_ago = last_collection_time.map(|ts| {
        let elapsed_secs = now - ts;
        if elapsed_secs < 60 {
            "just now".to_string()
        } else if elapsed_secs < 3600 {
            format!("{} minutes ago", elapsed_secs / 60)
        } else if elapsed_secs < 86400 {
            format!("{} hours ago", elapsed_secs / 3600)
        } else {
            format!("{} days ago", elapsed_secs / 86400)
        }
    });

    Ok(StorageStatus {
        last_collection_time,
        last_collection_ago,
        account_count: accounts.len(),
        manual_account_count: manual_accounts.len(),
        stale_manual_accounts: stale.into_iter().map(|s| s.name).collect(),
        warnings,
    })
}

/// Abstraction for persisting and querying collected SimpleFIN data.
pub trait Storage {
    /// Insert or update organizations. Existing orgs (matched by key) are updated.
    fn upsert_organizations(&mut self, orgs: &[Organization]) -> Result<()>;

    /// Insert or update accounts. Balances are updated to latest values.
    fn upsert_accounts(&mut self, accounts: &[Account]) -> Result<()>;

    /// Insert or update transactions for a specific account. Deduped by transaction ID.
    /// Returns the number of newly inserted transactions.
    fn upsert_transactions(&mut self, account_id: &str, txns: &[Transaction]) -> Result<usize>;

    /// Retrieve organizations matching the filter.
    fn get_organizations(&self, filter: &OrgFilter) -> Result<Vec<Organization>>;

    /// Retrieve accounts matching the filter.
    fn get_accounts(&self, filter: &AccountFilter) -> Result<Vec<Account>>;

    /// Retrieve transactions with context, matching the filter.
    fn get_transactions(&self, filter: &TransactionFilter) -> Result<Vec<TransactionWithContext>>;

    /// Returns the most recent transaction timestamp for an account, used for incremental fetching.
    fn last_collected(&self, account_id: &str) -> Result<Option<i64>>;

    /// Records the last collection timestamp for an account.
    fn set_last_collected(&mut self, account_id: &str, timestamp: i64) -> Result<()>;

    /// Returns the maximum `posted` timestamp from stored transactions for an account,
    /// excluding pending (posted <= 0) entries. Used to recover from corrupted state.
    fn max_stored_posted(&self, account_id: &str) -> Result<Option<i64>>;

    /// Insert or update manual accounts (not connected to SimpleFIN).
    fn upsert_manual_accounts(&mut self, accounts: &[ManualAccount]) -> Result<()>;

    /// Retrieve all manual accounts.
    fn get_manual_accounts(&self) -> Result<Vec<ManualAccount>>;

    /// Record a balance snapshot for an account at a point in time.
    fn record_balance(&mut self, account_id: &str, timestamp: i64, balance: Decimal) -> Result<()>;

    /// Retrieve balance history matching the filter, sorted chronologically.
    fn get_balance_history(&self, filter: &BalanceHistoryFilter) -> Result<Vec<BalanceSnapshot>>;

    /// Read the per-user data config. Returns default if no config file exists.
    fn get_config(&self) -> Result<DataConfig>;

    /// Write the per-user data config.
    fn set_config(&self, config: &DataConfig) -> Result<()>;

    /// Return manual accounts whose balance is stale based on their refresh_days.
    fn get_stale_accounts(&self, now: i64) -> Result<Vec<StaleAccount>>;

    /// Find orphaned data (balance history/transactions for accounts that no longer exist).
    fn find_orphaned_data(&self) -> Result<Vec<OrphanedData>>;

    /// Remove orphaned data files.
    fn remove_orphaned_data(&self, orphans: &[OrphanedData]) -> Result<()>;

    /// Save warnings from the most recent collection, replacing any previous warnings.
    fn save_warnings(&self, record: &WarningRecord) -> Result<()>;

    /// Load warnings from the most recent collection, if any.
    fn get_warnings(&self) -> Result<Option<WarningRecord>>;

    /// Load spending patterns from storage. If no patterns file exists, seeds it
    /// with `default_spending_patterns()` and returns those defaults.
    fn get_spending_patterns(&self) -> Result<Vec<crate::spending::SpendingRule>>;

    /// Save spending patterns to storage, replacing any existing patterns.
    fn set_spending_patterns(&self, patterns: &[crate::spending::SpendingRule]) -> Result<()>;
}
