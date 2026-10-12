//! Idempotent collection of SimpleFIN data into a [`Storage`] backend.
//!
//! [`collect_data`] is the single collection pipeline shared by the CLI and
//! the app. Network access is abstracted behind [`AccountFetcher`], so the
//! pipeline is independent of the HTTP transport (asupersync natively,
//! `fetch` in the browser) and can be exercised without a live bridge.

use std::future::Future;

use serde::{Deserialize, Serialize};

use crate::anomaly::{Anomaly, detect_anomalies};
use crate::error::Result;
use crate::models::AccountSet;
use crate::requests::AccountQueryParams;
use crate::storage::{AccountFilter, Storage, WarningRecord};

/// Source of SimpleFIN account data. Natively, `AccessClient::fetcher`
/// adapts an `AccessClient`; other transports build requests with
/// [`AccessCredentials::accounts_request`](crate::AccessCredentials::accounts_request)
/// and parse with [`parse_accounts_response`](crate::parse_accounts_response).
/// Tests supply in-memory fakes.
pub trait AccountFetcher {
    /// Fetch accounts (and, unless `balances_only`, their transactions).
    fn fetch_accounts(
        &self,
        params: &AccountQueryParams,
    ) -> impl Future<Output = Result<AccountSet>>;
}

/// Progress notifications emitted while collecting.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", tag = "type")]
pub enum CollectEvent {
    /// An informational message from the bridge. `account_name` is set when
    /// the message came from a per-account fetch.
    BridgeMessage {
        account_name: Option<String>,
        message: String,
    },
    /// A balance anomaly detected against the previously stored accounts.
    Anomaly(Anomaly),
    /// Missing or zero sync state was rebuilt from stored transactions.
    SyncStateRecovered {
        account_name: String,
        timestamp: i64,
    },
    /// Transactions for one account were stored.
    AccountCollected {
        account_name: String,
        new_transactions: usize,
        existing_transactions: usize,
    },
}

/// Outcome of a collection run.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CollectReport {
    /// Timestamp used for balance snapshots and the warning record.
    pub timestamp: i64,
    /// True when no accounts were stored before this run.
    pub first_run: bool,
    /// Number of accounts the bridge reported in the balances request.
    pub accounts_returned: usize,
    /// Number of accounts whose transactions were collected.
    pub accounts: usize,
    /// Transactions stored for the first time.
    pub new_transactions: usize,
    /// Transactions that were already stored.
    pub duplicates_skipped: usize,
    /// Anomalies detected against the previous accounts.
    pub anomalies: Vec<Anomaly>,
    /// Bridge messages; per-account messages are prefixed with the account name.
    pub bridge_messages: Vec<String>,
}

/// Collect all accounts and transactions into `storage`.
///
/// - Detects anomalies against the previously stored accounts.
/// - Upserts organizations and accounts and records (deduped) balance
///   snapshots at `now`.
/// - Fetches transactions per account incrementally from the stored sync
///   state. Missing or zero state is recovered from the newest stored
///   transaction. Only posted, non-pending transactions advance the state,
///   and the state never moves backwards.
/// - Persists a [`WarningRecord`] with the anomalies and bridge messages.
///
/// When the bridge returns no accounts, nothing is persisted.
pub async fn collect_data<F, S>(
    fetcher: &F,
    storage: &mut S,
    now: i64,
    on_event: &mut dyn FnMut(&CollectEvent),
) -> Result<CollectReport>
where
    F: AccountFetcher + ?Sized,
    S: Storage + ?Sized,
{
    let previous_accounts = storage.get_accounts(&AccountFilter::default())?;
    let first_run = previous_accounts.is_empty();

    let balances_params = AccountQueryParams {
        balances_only: true,
        ..Default::default()
    };
    let account_set = fetcher.fetch_accounts(&balances_params).await?;

    let mut report = CollectReport {
        timestamp: now,
        first_run,
        accounts_returned: account_set.accounts.len(),
        accounts: 0,
        new_transactions: 0,
        duplicates_skipped: 0,
        anomalies: Vec::new(),
        bridge_messages: Vec::new(),
    };

    for msg in &account_set.server_messages {
        on_event(&CollectEvent::BridgeMessage {
            account_name: None,
            message: msg.clone(),
        });
        report.bridge_messages.push(msg.clone());
    }

    if account_set.accounts.is_empty() {
        return Ok(report);
    }

    if !first_run {
        report.anomalies = detect_anomalies(&account_set.accounts, &previous_accounts);
        for anomaly in &report.anomalies {
            on_event(&CollectEvent::Anomaly(anomaly.clone()));
        }
    }

    let orgs: Vec<_> = account_set.accounts.iter().map(|a| a.org.clone()).collect();
    storage.upsert_organizations(&orgs)?;
    storage.upsert_accounts(&account_set.accounts)?;
    for account in &account_set.accounts {
        storage.record_balance(&account.id, now, account.balance)?;
    }

    for account in &account_set.accounts {
        let start_date = incremental_start(storage, &account.id, &account.name, on_event)?;
        let params = AccountQueryParams {
            start_date: Some(start_date),
            include_pending: true,
            account_ids: Some(vec![account.id.clone()]),
            balances_only: false,
            ..Default::default()
        };
        let result = fetcher.fetch_accounts(&params).await?;

        for msg in &result.server_messages {
            on_event(&CollectEvent::BridgeMessage {
                account_name: Some(account.name.clone()),
                message: msg.clone(),
            });
            report
                .bridge_messages
                .push(format!("{}: {msg}", account.name));
        }

        for fetched in &result.accounts {
            let fetched_count = fetched.transactions.len();
            let new_count = storage.upsert_transactions(&fetched.id, &fetched.transactions)?;
            let existing = fetched_count - new_count;

            // Pending transactions may carry posted=0; never let them set state.
            if let Some(max_posted) = fetched
                .transactions
                .iter()
                .filter(|t| !t.pending && t.posted > 0)
                .map(|t| t.posted)
                .max()
            {
                let current = storage.last_collected(&fetched.id)?;
                if current.is_none_or(|ts| max_posted > ts) {
                    storage.set_last_collected(&fetched.id, max_posted)?;
                }
            }

            on_event(&CollectEvent::AccountCollected {
                account_name: fetched.name.clone(),
                new_transactions: new_count,
                existing_transactions: existing,
            });

            report.new_transactions += new_count;
            report.duplicates_skipped += existing;
            report.accounts += 1;
        }
    }

    storage.save_warnings(&WarningRecord {
        timestamp: now,
        anomalies: report.anomalies.clone(),
        bridge_messages: report.bridge_messages.clone(),
    })?;

    Ok(report)
}

/// Start date for an incremental fetch. Missing or zero state (a symptom of
/// earlier corruption) is recovered from the newest stored transaction and
/// written back; with nothing stored, fetch from the epoch.
fn incremental_start<S: Storage + ?Sized>(
    storage: &mut S,
    account_id: &str,
    account_name: &str,
    on_event: &mut dyn FnMut(&CollectEvent),
) -> Result<i64> {
    if let Some(ts) = storage.last_collected(account_id)?
        && ts > 0
    {
        return Ok(ts);
    }
    let Some(recovered) = storage.max_stored_posted(account_id)? else {
        return Ok(0);
    };
    storage.set_last_collected(account_id, recovered)?;
    on_event(&CollectEvent::SyncStateRecovered {
        account_name: account_name.to_string(),
        timestamp: recovered,
    });
    Ok(recovered)
}
