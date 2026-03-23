pub mod analysis;
pub mod anomaly;
pub mod clients;
pub mod constants;
pub mod credentials;
pub mod datetime_utils;
pub mod error;
pub mod models;
pub mod recurring;
pub mod spending;
pub mod storage;
pub mod trends;

pub use analysis::{
    AccountCategory, AccountDetail, BalanceChange, CategoryTotal, ClassificationInfo,
    NetWorthSummary, NetWorthTimePoint, account_is_excluded, classify_account,
    classify_for_display, compute_changes, compute_net_worth, compute_net_worth_detail,
    compute_net_worth_history, display_name_for,
};
pub use anomaly::{Anomaly, detect_anomalies};
pub use clients::{AccessClient, AccountQueryParams, BridgeClient};
pub use constants::{DEFAULT_BRIDGE_ROOT_URL, DEFAULT_USER_AGENT};
pub use credentials::{AccessCredentials, SetupToken};
pub use error::{Result, SimplefinError};
pub use models::{Account, AccountSet, BridgeInfo, Organization, Transaction};
pub use recurring::{RecurringExpense, RecurringSummary, detect_recurring};
pub use spending::{
    OTHER_CATEGORY, SpendingRule, SpendingSummary, SpendingTotal, UnclassifiedTransaction,
    category_label, classify_transaction, compute_spending, default_spending_patterns,
};
pub use trends::{CategoryTrend, MonthlyTotal, TrendDirection, TrendsSummary, compute_trends};
pub use storage::{
    AccountFilter, AccountSource, BalanceHistoryFilter, BalanceSnapshot, ClassificationField,
    ClassificationRule, DataConfig, JsonStorage, ManualAccount, OrgFilter, OrphanedData,
    OrphanedDataType, StaleAccount, Storage, StorageStatus, TransactionFilter,
    TransactionWithContext, UnifiedAccount, WarningRecord, compute_status, unify_accounts,
};
