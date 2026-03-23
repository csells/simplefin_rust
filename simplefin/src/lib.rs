pub mod analysis;
pub mod anomaly;
pub mod clients;
pub mod constants;
pub mod credentials;
pub mod datetime_utils;
pub mod error;
pub mod models;
pub mod spending;
pub mod storage;

pub use analysis::{
    AccountCategory, AccountDetail, BalanceChange, CategoryTotal, NetWorthSummary,
    classify_account, compute_changes, compute_net_worth, compute_net_worth_detail,
    display_name_for,
};
pub use anomaly::{Anomaly, detect_anomalies};
pub use clients::{AccessClient, AccountQueryParams, BridgeClient};
pub use constants::{DEFAULT_BRIDGE_ROOT_URL, DEFAULT_USER_AGENT};
pub use credentials::{AccessCredentials, SetupToken};
pub use error::{Result, SimplefinError};
pub use models::{Account, AccountSet, BridgeInfo, Organization, Transaction};
pub use spending::{
    SpendingCategory, SpendingRule, SpendingSummary, SpendingTotal, classify_transaction,
    compute_spending,
};
pub use storage::{
    AccountFilter, AccountSource, BalanceHistoryFilter, BalanceSnapshot, ClassificationField,
    ClassificationRule, DataConfig, JsonStorage, ManualAccount, OrgFilter, OrphanedData,
    OrphanedDataType, StaleAccount, Storage, TransactionFilter, TransactionWithContext,
    UnifiedAccount, unify_accounts,
};
