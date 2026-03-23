use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

use crate::error::{Result, SimplefinError};
use crate::models::{Account, Organization, Transaction};

use rust_decimal::Decimal;

use super::traits::{
    AccountFilter, BalanceHistoryFilter, BalanceSnapshot, DataConfig, ManualAccount, OrgFilter,
    OrphanedData, OrphanedDataType, StaleAccount, Storage, TransactionFilter,
    TransactionWithContext, WarningRecord,
};

/// JSON-file-based storage backend.
///
/// Directory layout:
/// ```text
/// {root}/
///   organizations.json    — Vec<Organization>
///   accounts.json         — Vec<Account> (transactions always empty)
///   transactions/
///     {account_id}.json   — Vec<Transaction>
///   state.json            — HashMap<account_id, last_collected_epoch>
///   manual_accounts.json  — Vec<ManualAccount>
///   balance_history/
///     {account_id}.json   — Vec<BalanceSnapshot>
/// ```
pub struct JsonStorage {
    root: PathBuf,
}

impl JsonStorage {
    /// Opens or creates a JSON storage directory at the given path.
    pub fn open(root: impl AsRef<Path>) -> Result<Self> {
        let root = root.as_ref().to_path_buf();
        fs::create_dir_all(&root).map_err(|e| SimplefinError::Storage {
            message: format!("failed to create storage directory: {}", root.display()),
            source: Some(Box::new(e)),
        })?;
        fs::create_dir_all(root.join("transactions")).map_err(|e| SimplefinError::Storage {
            message: "failed to create transactions directory".into(),
            source: Some(Box::new(e)),
        })?;
        fs::create_dir_all(root.join("balance_history")).map_err(|e| SimplefinError::Storage {
            message: "failed to create balance_history directory".into(),
            source: Some(Box::new(e)),
        })?;
        Ok(JsonStorage { root })
    }

    fn orgs_path(&self) -> PathBuf {
        self.root.join("organizations.json")
    }

    fn accounts_path(&self) -> PathBuf {
        self.root.join("accounts.json")
    }

    fn transactions_path(&self, account_id: &str) -> PathBuf {
        self.root
            .join("transactions")
            .join(format!("{account_id}.json"))
    }

    fn state_path(&self) -> PathBuf {
        self.root.join("state.json")
    }

    fn manual_accounts_path(&self) -> PathBuf {
        self.root.join("manual_accounts.json")
    }

    fn balance_history_path(&self, account_id: &str) -> PathBuf {
        self.root
            .join("balance_history")
            .join(format!("{account_id}.json"))
    }

    fn config_path(&self) -> PathBuf {
        self.root.join("config.json")
    }

    fn warnings_path(&self) -> PathBuf {
        self.root.join("warnings.json")
    }

    fn spending_patterns_path(&self) -> PathBuf {
        self.root.join("spending_patterns.json")
    }

    fn read_json<T: serde::de::DeserializeOwned + Default>(&self, path: &Path) -> Result<T> {
        if !path.exists() {
            return Ok(T::default());
        }
        let data = fs::read_to_string(path).map_err(|e| SimplefinError::Storage {
            message: format!("failed to read {}", path.display()),
            source: Some(Box::new(e)),
        })?;
        serde_json::from_str(&data).map_err(|e| SimplefinError::Storage {
            message: format!("failed to parse {}", path.display()),
            source: Some(Box::new(e)),
        })
    }

    fn write_json<T: serde::Serialize>(&self, path: &Path, value: &T) -> Result<()> {
        let tmp = path.with_extension("json.tmp");
        let data = serde_json::to_string_pretty(value).map_err(|e| SimplefinError::Storage {
            message: "failed to serialize data".into(),
            source: Some(Box::new(e)),
        })?;
        fs::write(&tmp, &data).map_err(|e| SimplefinError::Storage {
            message: format!("failed to write {}", tmp.display()),
            source: Some(Box::new(e)),
        })?;
        fs::rename(&tmp, path).map_err(|e| SimplefinError::Storage {
            message: format!("failed to rename {} to {}", tmp.display(), path.display()),
            source: Some(Box::new(e)),
        })?;
        Ok(())
    }

    fn read_state(&self) -> Result<HashMap<String, i64>> {
        self.read_json(&self.state_path())
    }

    fn write_state(&self, state: &HashMap<String, i64>) -> Result<()> {
        self.write_json(&self.state_path(), state)
    }

    fn load_accounts(&self) -> Result<Vec<Account>> {
        self.read_json(&self.accounts_path())
    }

    fn load_orgs(&self) -> Result<Vec<Organization>> {
        self.read_json(&self.orgs_path())
    }
}

impl Storage for JsonStorage {
    fn upsert_organizations(&mut self, orgs: &[Organization]) -> Result<()> {
        let mut existing: Vec<Organization> = self.load_orgs()?;
        for org in orgs {
            let key = org.key().to_string();
            if let Some(pos) = existing.iter().position(|o| o.key() == key) {
                existing[pos] = org.clone();
            } else {
                existing.push(org.clone());
            }
        }
        self.write_json(&self.orgs_path(), &existing)
    }

    fn upsert_accounts(&mut self, accounts: &[Account]) -> Result<()> {
        let mut existing: Vec<Account> = self.load_accounts()?;
        for account in accounts {
            let mut stored = account.clone();
            stored.transactions = Vec::new();
            if let Some(pos) = existing.iter().position(|a| a.id == account.id) {
                existing[pos] = stored;
            } else {
                existing.push(stored);
            }
        }
        self.write_json(&self.accounts_path(), &existing)
    }

    fn upsert_transactions(&mut self, account_id: &str, txns: &[Transaction]) -> Result<usize> {
        let path = self.transactions_path(account_id);
        let mut existing: Vec<Transaction> = self.read_json(&path)?;
        let mut new_count = 0;
        for txn in txns {
            if let Some(pos) = existing.iter().position(|t| t.id == txn.id) {
                existing[pos] = txn.clone();
            } else {
                existing.push(txn.clone());
                new_count += 1;
            }
        }
        self.write_json(&path, &existing)?;
        Ok(new_count)
    }

    fn get_organizations(&self, filter: &OrgFilter) -> Result<Vec<Organization>> {
        let orgs = self.load_orgs()?;
        Ok(orgs
            .into_iter()
            .filter(|o| {
                if let Some(ref id) = filter.org_id
                    && o.id.as_deref() != Some(id.as_str())
                {
                    return false;
                }
                if let Some(ref name) = filter.name
                    && o.name.as_deref() != Some(name.as_str())
                {
                    return false;
                }
                true
            })
            .collect())
    }

    fn get_accounts(&self, filter: &AccountFilter) -> Result<Vec<Account>> {
        let accounts = self.load_accounts()?;
        Ok(accounts
            .into_iter()
            .filter(|a| {
                if let Some(ref id) = filter.account_id
                    && a.id != *id
                {
                    return false;
                }
                if let Some(ref name) = filter.name
                    && a.name != *name
                {
                    return false;
                }
                if let Some(ref org_id) = filter.org_id
                    && a.org.id.as_deref() != Some(org_id.as_str())
                {
                    return false;
                }
                true
            })
            .collect())
    }

    fn get_transactions(&self, filter: &TransactionFilter) -> Result<Vec<TransactionWithContext>> {
        let accounts = self.load_accounts()?;

        let target_accounts: Vec<&Account> = accounts
            .iter()
            .filter(|a| {
                if let Some(ref id) = filter.account_id
                    && a.id != *id
                {
                    return false;
                }
                if let Some(ref org_id) = filter.org_id
                    && a.org.id.as_deref() != Some(org_id.as_str())
                {
                    return false;
                }
                true
            })
            .collect();

        let mut results = Vec::new();
        for account in target_accounts {
            let path = self.transactions_path(&account.id);
            let txns: Vec<Transaction> = self.read_json(&path)?;
            for txn in txns {
                if let Some(start) = filter.start_date
                    && txn.posted < start
                {
                    continue;
                }
                if let Some(end) = filter.end_date
                    && txn.posted >= end
                {
                    continue;
                }
                if let Some(include_pending) = filter.include_pending
                    && !include_pending
                    && txn.pending
                {
                    continue;
                }

                results.push(TransactionWithContext {
                    id: txn.id,
                    account_id: account.id.clone(),
                    account_name: account.name.clone(),
                    org_name: account.org.display_name().to_string(),
                    currency: account.currency.clone(),
                    posted: txn.posted,
                    amount: txn.amount,
                    description: txn.description,
                    transacted_at: txn.transacted_at,
                    pending: txn.pending,
                });
            }
        }

        Ok(results)
    }

    fn last_collected(&self, account_id: &str) -> Result<Option<i64>> {
        let state = self.read_state()?;
        Ok(state.get(account_id).copied())
    }

    fn set_last_collected(&mut self, account_id: &str, timestamp: i64) -> Result<()> {
        let mut state = self.read_state()?;
        state.insert(account_id.to_string(), timestamp);
        self.write_state(&state)
    }

    fn upsert_manual_accounts(&mut self, accounts: &[ManualAccount]) -> Result<()> {
        let path = self.manual_accounts_path();
        let mut existing: Vec<ManualAccount> = self.read_json(&path)?;
        for account in accounts {
            if let Some(pos) = existing.iter().position(|a| a.id == account.id) {
                existing[pos] = account.clone();
            } else {
                existing.push(account.clone());
            }
        }
        self.write_json(&path, &existing)
    }

    fn get_manual_accounts(&self) -> Result<Vec<ManualAccount>> {
        self.read_json(&self.manual_accounts_path())
    }

    fn record_balance(&mut self, account_id: &str, timestamp: i64, balance: Decimal) -> Result<()> {
        let path = self.balance_history_path(account_id);
        let mut history: Vec<BalanceSnapshot> = self.read_json(&path)?;

        // Skip if the most recent snapshot has the same balance
        if let Some(last) = history.last()
            && last.balance == balance
        {
            return Ok(());
        }

        history.push(BalanceSnapshot {
            account_id: account_id.to_string(),
            timestamp,
            balance,
        });
        history.sort_by_key(|s| s.timestamp);
        self.write_json(&path, &history)
    }

    fn get_config(&self) -> Result<DataConfig> {
        self.read_json(&self.config_path())
    }

    fn set_config(&self, config: &DataConfig) -> Result<()> {
        self.write_json(&self.config_path(), config)
    }

    fn get_stale_accounts(&self, now: i64) -> Result<Vec<StaleAccount>> {
        let manual_accounts = self.get_manual_accounts()?;
        let mut stale = Vec::new();

        for ma in &manual_accounts {
            let latest_ts = self
                .get_balance_history(&BalanceHistoryFilter {
                    account_id: Some(ma.id.clone()),
                    ..Default::default()
                })?
                .last()
                .map(|s| s.timestamp);

            let days_since = latest_ts.map(|ts| ((now - ts) as u64) / 86400);
            let is_stale = match days_since {
                None => true, // never recorded
                Some(d) => d >= ma.refresh_days as u64,
            };

            if is_stale {
                stale.push(StaleAccount {
                    id: ma.id.clone(),
                    name: ma.name.clone(),
                    org_name: ma.org_name.clone(),
                    last_updated: latest_ts,
                    refresh_days: ma.refresh_days,
                    days_since_update: days_since,
                });
            }
        }

        Ok(stale)
    }

    fn find_orphaned_data(&self) -> Result<Vec<OrphanedData>> {
        let accounts = self.load_accounts()?;
        let manual_accounts = self.get_manual_accounts()?;

        let known_ids: std::collections::HashSet<&str> = accounts
            .iter()
            .map(|a| a.id.as_str())
            .chain(manual_accounts.iter().map(|a| a.id.as_str()))
            .collect();

        let mut orphans = Vec::new();

        // Check balance_history/ directory
        let bh_dir = self.root.join("balance_history");
        if bh_dir.exists() {
            for entry in fs::read_dir(&bh_dir).map_err(|e| SimplefinError::Storage {
                message: "failed to read balance_history directory".into(),
                source: Some(Box::new(e)),
            })? {
                let entry = entry.map_err(|e| SimplefinError::Storage {
                    message: "failed to read directory entry".into(),
                    source: Some(Box::new(e)),
                })?;
                let path = entry.path();
                if let Some(stem) = path.file_stem().and_then(|s| s.to_str())
                    && path.extension().is_some_and(|ext| ext == "json")
                    && !known_ids.contains(stem)
                {
                    orphans.push(OrphanedData {
                        account_id: stem.to_string(),
                        data_type: OrphanedDataType::BalanceHistory,
                        path: path.to_string_lossy().to_string(),
                    });
                }
            }
        }

        // Check transactions/ directory
        let txn_dir = self.root.join("transactions");
        if txn_dir.exists() {
            for entry in fs::read_dir(&txn_dir).map_err(|e| SimplefinError::Storage {
                message: "failed to read transactions directory".into(),
                source: Some(Box::new(e)),
            })? {
                let entry = entry.map_err(|e| SimplefinError::Storage {
                    message: "failed to read directory entry".into(),
                    source: Some(Box::new(e)),
                })?;
                let path = entry.path();
                if let Some(stem) = path.file_stem().and_then(|s| s.to_str())
                    && path.extension().is_some_and(|ext| ext == "json")
                    && !known_ids.contains(stem)
                {
                    orphans.push(OrphanedData {
                        account_id: stem.to_string(),
                        data_type: OrphanedDataType::Transactions,
                        path: path.to_string_lossy().to_string(),
                    });
                }
            }
        }

        Ok(orphans)
    }

    fn remove_orphaned_data(&self, orphans: &[OrphanedData]) -> Result<()> {
        for orphan in orphans {
            let path = Path::new(&orphan.path);
            if path.exists() {
                fs::remove_file(path).map_err(|e| SimplefinError::Storage {
                    message: format!("failed to remove orphaned file: {}", orphan.path),
                    source: Some(Box::new(e)),
                })?;
            }
        }
        Ok(())
    }

    fn save_warnings(&self, record: &WarningRecord) -> Result<()> {
        self.write_json(&self.warnings_path(), record)
    }

    fn get_warnings(&self) -> Result<Option<WarningRecord>> {
        let path = self.warnings_path();
        if !path.exists() {
            return Ok(None);
        }
        let data = fs::read_to_string(&path).map_err(|e| SimplefinError::Storage {
            message: format!("failed to read {}", path.display()),
            source: Some(Box::new(e)),
        })?;
        let record: WarningRecord =
            serde_json::from_str(&data).map_err(|e| SimplefinError::Storage {
                message: format!("failed to parse {}", path.display()),
                source: Some(Box::new(e)),
            })?;
        Ok(Some(record))
    }

    fn get_spending_patterns(&self) -> Result<Vec<crate::spending::SpendingRule>> {
        let path = self.spending_patterns_path();
        if !path.exists() {
            // Seed with defaults on first use
            let defaults = crate::spending::default_spending_patterns();
            self.write_json(&path, &defaults)?;
            return Ok(defaults);
        }
        self.read_json(&path)
    }

    fn set_spending_patterns(&self, patterns: &[crate::spending::SpendingRule]) -> Result<()> {
        self.write_json(&self.spending_patterns_path(), &patterns)
    }

    fn get_balance_history(&self, filter: &BalanceHistoryFilter) -> Result<Vec<BalanceSnapshot>> {
        if let Some(ref account_id) = filter.account_id {
            // Single account
            let path = self.balance_history_path(account_id);
            let history: Vec<BalanceSnapshot> = self.read_json(&path)?;
            Ok(Self::filter_snapshots(history, filter))
        } else {
            // All accounts — read all files in balance_history/
            let dir = self.root.join("balance_history");
            let mut all = Vec::new();
            if dir.exists() {
                for entry in fs::read_dir(&dir).map_err(|e| SimplefinError::Storage {
                    message: "failed to read balance_history directory".into(),
                    source: Some(Box::new(e)),
                })? {
                    let entry = entry.map_err(|e| SimplefinError::Storage {
                        message: "failed to read directory entry".into(),
                        source: Some(Box::new(e)),
                    })?;
                    let path = entry.path();
                    if path.extension().is_some_and(|ext| ext == "json") {
                        let snapshots: Vec<BalanceSnapshot> = self.read_json(&path)?;
                        all.extend(snapshots);
                    }
                }
            }
            all.sort_by_key(|s| s.timestamp);
            Ok(Self::filter_snapshots(all, filter))
        }
    }
}

impl JsonStorage {
    fn filter_snapshots(
        snapshots: Vec<BalanceSnapshot>,
        filter: &BalanceHistoryFilter,
    ) -> Vec<BalanceSnapshot> {
        snapshots
            .into_iter()
            .filter(|s| {
                if let Some(start) = filter.start_date
                    && s.timestamp < start
                {
                    return false;
                }
                if let Some(end) = filter.end_date
                    && s.timestamp >= end
                {
                    return false;
                }
                true
            })
            .collect()
    }
}
