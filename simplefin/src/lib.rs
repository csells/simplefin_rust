pub mod analysis;
pub mod anomaly;
pub mod budget;
pub mod cashflow;
pub mod categories;
#[cfg(not(target_arch = "wasm32"))]
pub mod clients;
pub mod constants;
pub mod credentials;
pub mod datetime_utils;
pub mod edits;
pub mod error;
pub mod models;
pub mod month;
pub mod query;
pub mod recurring;
pub mod requests;
pub mod spending;
pub mod storage;
pub mod sync;
pub mod trends;

pub use analysis::{
    AccountCategory, AccountDetail, BalanceChange, CategoryTimePoint, CategoryTotal,
    ClassificationInfo, NetWorthChange, NetWorthSummary, NetWorthTimePoint, account_category,
    account_is_excluded, classify_account, classify_for_display, compute_category_history,
    compute_changes, compute_net_worth, compute_net_worth_change, compute_net_worth_detail,
    compute_net_worth_history, display_name_for, net_worth_at, reconstruct_accounts_at,
    transaction_account_category, transaction_account_excluded,
};
pub use anomaly::{Anomaly, detect_anomalies};
pub use budget::{
    BudgetConfig, BudgetStatus, CategoryLimitStatus, CategorySpend, SpendingAverage,
    compute_budget_status,
};
pub use cashflow::{
    CashFlowCategory, CashFlowEntry, CashFlowMonth, CashFlowSummary, cash_flow_entry,
    compute_cash_flow,
};
pub use categories::{
    CategoryKind, Categorizer, INCOME_CATEGORY, TRANSFER_CATEGORY, category_kind,
    default_category_kind, normalize_category_name,
};
#[cfg(not(target_arch = "wasm32"))]
pub use clients::{AccessClient, AccessFetcher, BridgeClient};
pub use constants::{DEFAULT_BRIDGE_ROOT_URL, DEFAULT_USER_AGENT};
pub use credentials::{AccessCredentials, SetupToken};
pub use edits::{
    TransactionEdit, TransactionEdits, apply_category_to_future, effective_category,
    effective_description, rule_from_transaction,
};
pub use error::{Result, SimplefinError};
pub use models::{Account, AccountSet, BridgeInfo, Organization, Transaction};
pub use month::YearMonth;
pub use query::{TransactionQuery, TransactionView, transaction_date};
pub use recurring::{RecurringExpense, RecurringSummary, detect_recurring, normalize_merchant};
pub use requests::{
    AccountQueryParams, HttpRequestSpec, parse_accounts_response, parse_claim_response,
};
pub use spending::{
    OTHER_CATEGORY, SpendingRule, SpendingSummary, SpendingTotal, UnclassifiedTransaction,
    add_user_spending_rule, assemble_spending_rules, category_label, classify_transaction,
    compute_spending, default_spending_patterns, load_spending_rules,
};
pub use storage::{
    AccountFilter, AccountSource, BalanceHistoryFilter, BalanceSnapshot, ClassificationField,
    ClassificationRule, DataConfig, DefaultFileStore, FileStore, JsonStorage, ManualAccount,
    MemoryFiles, OrgFilter, OrphanedData,
    OrphanedDataType, StaleAccount, Storage, StorageStatus, TransactionFilter,
    TransactionWithContext, UnifiedAccount, WarningRecord, compute_status, unify_accounts,
};
#[cfg(not(target_arch = "wasm32"))]
pub use storage::DiskFiles;
pub use sync::{AccountFetcher, CollectEvent, CollectReport, collect_data};
pub use trends::{CategoryTrend, MonthlyTotal, TrendDirection, TrendsSummary, compute_trends};
