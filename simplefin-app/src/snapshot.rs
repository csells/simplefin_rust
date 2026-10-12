//! An immutable, fully computed view of the user's data for one render pass.
//!
//! Built off the main thread from local storage by [`Snapshot::load`]; screens
//! only read it. All analysis comes from the `simplefin` library.

use rust_decimal::Decimal;
use simplefin::{
    AccountCategory, BalanceHistoryFilter, BalanceSnapshot, BudgetStatus, CashFlowSummary,
    Categorizer, DataConfig, NetWorthChange, NetWorthSummary, NetWorthTimePoint,
    RecurringSummary, StaleAccount, Storage, TransactionFilter, TransactionQuery,
    TransactionView, TransactionWithContext, UnifiedAccount, WarningRecord, YearMonth,
    AccountFilter, AccountSource, account_category, compute_budget_status, compute_cash_flow,
    compute_net_worth_change, compute_net_worth_detail, compute_net_worth_history,
    detect_recurring, display_name_for, unify_accounts,
};

/// How many recent balance snapshots feed the net worth chart.
const HISTORY_POINTS: usize = 180;
/// Months of cash flow shown on the Plan tab (including the current month).
const CASH_FLOW_MONTHS: u32 = 6;
/// Minimum occurrences before a merchant counts as recurring.
const RECURRING_MIN_OCCURRENCES: usize = 3;

/// Display order of account groups, as on Empower's accounts panel.
pub const GROUP_ORDER: [AccountCategory; 5] = [
    AccountCategory::Cash,
    AccountCategory::Investments,
    AccountCategory::CreditCards,
    AccountCategory::Loans,
    AccountCategory::OtherAssets,
];

#[derive(Debug, Clone)]
pub struct AccountRow {
    pub id: String,
    pub name: String,
    pub org_name: String,
    pub currency: String,
    pub balance: Decimal,
    pub available_balance: Option<Decimal>,
    pub balance_date: Option<i64>,
    pub category: AccountCategory,
    pub manual: bool,
    pub stale: bool,
    pub excluded: bool,
}

#[derive(Debug, Clone)]
pub struct AccountGroup {
    pub label: String,
    pub total: Decimal,
    pub accounts: Vec<AccountRow>,
}

#[derive(Default)]
pub struct Snapshot {
    /// False until the first load completes (distinguishes "loading" from "empty").
    pub loaded: bool,
    pub now: i64,
    pub currency: String,
    pub net_worth: Option<NetWorthSummary>,
    pub net_worth_history: Vec<NetWorthTimePoint>,
    pub change_1d: Option<NetWorthChange>,
    pub change_90d: Option<NetWorthChange>,
    pub groups: Vec<AccountGroup>,
    /// Every transaction (hidden ones flagged), newest first.
    pub transactions: Vec<TransactionView>,
    pub cash_flow: Option<CashFlowSummary>,
    pub budget: Option<BudgetStatus>,
    pub recurring: Option<RecurringSummary>,
    pub stale: Vec<StaleAccount>,
    pub warnings: Option<WarningRecord>,
    pub last_sync: Option<i64>,
    /// Known spending categories (snake_case), for pickers.
    pub categories: Vec<String>,
    pub balance_history: Vec<BalanceSnapshot>,
    pub config: DataConfig,
    raw_transactions: Vec<TransactionWithContext>,
    categorizer: Categorizer,
}

impl Snapshot {
    pub fn load(storage: &impl Storage, now: i64) -> anyhow::Result<Self> {
        let config = storage.get_config()?;
        let categorizer = Categorizer::load(storage)?;
        let accounts = storage.get_accounts(&AccountFilter::default())?;
        let manual = storage.get_manual_accounts()?;
        let balance_history = storage.get_balance_history(&BalanceHistoryFilter::default())?;
        let unified = unify_accounts(&accounts, &manual, &balance_history);
        let stale = storage.get_stale_accounts(now)?;
        let transactions = storage.get_transactions(&TransactionFilter::default())?;

        let currency = unified
            .first()
            .map(|a| a.currency.clone())
            .unwrap_or_else(|| "USD".into());
        let current = YearMonth::from_epoch(now);
        let cash_flow_start = current.minus_months(CASH_FLOW_MONTHS - 1);

        Ok(Self {
            loaded: true,
            now,
            currency,
            net_worth: (!unified.is_empty()).then(|| compute_net_worth_detail(&unified, &config, true)),
            net_worth_history: compute_net_worth_history(
                &balance_history,
                &unified,
                &config,
                HISTORY_POINTS,
            ),
            change_1d: compute_net_worth_change(&balance_history, &unified, &config, now, 1, None),
            change_90d: compute_net_worth_change(&balance_history, &unified, &config, now, 90, None),
            groups: group_accounts(&unified, &config, &stale),
            transactions: newest_first(&transactions, &config, &categorizer),
            cash_flow: (!transactions.is_empty()).then(|| {
                compute_cash_flow(&transactions, &config, &categorizer, cash_flow_start, current)
            }),
            budget: (!transactions.is_empty())
                .then(|| compute_budget_status(&transactions, &config, &categorizer, now)),
            recurring: (!transactions.is_empty())
                .then(|| detect_recurring(&transactions, &categorizer, RECURRING_MIN_OCCURRENCES)),
            last_sync: balance_history.iter().map(|s| s.timestamp).max(),
            warnings: storage.get_warnings()?,
            stale,
            categories: known_categories(&categorizer),
            balance_history,
            config,
            raw_transactions: transactions,
            categorizer,
        })
    }

    /// Filters transactions with the library's query (newest first).
    pub fn query(&self, query: &TransactionQuery) -> Vec<TransactionView> {
        query.run(&self.raw_transactions, &self.config, &self.categorizer)
    }

    /// True when never synced or the last sync is older than `max_age_secs`.
    pub fn is_stale(&self, max_age_secs: i64) -> bool {
        self.last_sync.is_none_or(|ts| self.now - ts > max_age_secs)
    }

    pub fn has_accounts(&self) -> bool {
        self.groups.iter().any(|g| !g.accounts.is_empty())
    }

    pub fn account(&self, id: &str) -> Option<&AccountRow> {
        self.groups.iter().flat_map(|g| &g.accounts).find(|a| a.id == id)
    }

    /// Balance snapshots for one account, oldest first.
    pub fn account_history(&self, id: &str) -> Vec<&BalanceSnapshot> {
        let mut points: Vec<_> = self.balance_history.iter().filter(|s| s.account_id == id).collect();
        points.sort_by_key(|s| s.timestamp);
        points
    }

    pub fn transaction(&self, account_id: &str, id: &str) -> Option<&TransactionView> {
        self.transactions
            .iter()
            .find(|t| t.transaction.account_id == account_id && t.transaction.id == id)
    }
}

fn group_accounts(
    accounts: &[UnifiedAccount],
    config: &DataConfig,
    stale: &[StaleAccount],
) -> Vec<AccountGroup> {
    let rows: Vec<AccountRow> = accounts
        .iter()
        .map(|a| AccountRow {
            id: a.id.clone(),
            name: display_name_for(a, config),
            org_name: a.org_name.clone(),
            currency: a.currency.clone(),
            balance: a.balance,
            available_balance: a.available_balance,
            balance_date: a.balance_date,
            category: account_category(a, config),
            manual: matches!(a.source, AccountSource::Manual),
            stale: stale.iter().any(|s| s.id == a.id),
            excluded: simplefin::account_is_excluded(a, config),
        })
        .collect();
    GROUP_ORDER
        .into_iter()
        .filter_map(|category| {
            let mut accounts: Vec<AccountRow> =
                rows.iter().filter(|r| r.category == category).cloned().collect();
            if accounts.is_empty() {
                return None;
            }
            accounts.sort_by_key(|a| std::cmp::Reverse(a.balance.abs()));
            let total = accounts.iter().filter(|a| !a.excluded).map(|a| a.balance).sum();
            Some(AccountGroup { label: category.to_string(), total, accounts })
        })
        .collect()
}

fn newest_first(
    transactions: &[TransactionWithContext],
    config: &DataConfig,
    categorizer: &Categorizer,
) -> Vec<TransactionView> {
    TransactionQuery { include_hidden: true, ..Default::default() }.run(transactions, config, categorizer)
}

fn known_categories(categorizer: &Categorizer) -> Vec<String> {
    let mut categories: Vec<String> = categorizer.rules().iter().map(|r| r.category.clone()).collect();
    categories.push(simplefin::OTHER_CATEGORY.to_string());
    categories.sort();
    categories.dedup();
    categories
}

#[cfg(test)]
mod tests {
    use super::*;
    use simplefin::{AccountSet, JsonStorage, MemoryFiles};

    const FIXTURE: &str = r#"{
        "errors": [],
        "accounts": [
            {"org": {"domain": "bank.example", "sfin-url": "https://bridge.example/simplefin", "name": "Example Bank"},
             "id": "chk", "name": "Checking (1234)", "currency": "USD", "balance": "2500.00",
             "available-balance": "2400.00", "balance-date": 1790000000,
             "transactions": [
                {"id": "t1", "posted": 1789900000, "amount": "-42.10", "description": "GROCERY MART #12"},
                {"id": "t2", "posted": 1789950000, "amount": "3000.00", "description": "PAYROLL DIRECT DEP"}
             ]},
            {"org": {"domain": "card.example", "sfin-url": "https://bridge.example/simplefin", "name": "Example Card"},
             "id": "cc", "name": "Rewards Credit Card", "currency": "USD", "balance": "-300.00",
             "balance-date": 1790000000, "transactions": []}
        ]
    }"#;

    fn seeded() -> JsonStorage<MemoryFiles> {
        let mut storage = JsonStorage::with_files(MemoryFiles::new()).unwrap();
        let set: AccountSet = serde_json::from_str(FIXTURE).unwrap();
        let orgs: Vec<_> = set.accounts.iter().map(|a| a.org.clone()).collect();
        storage.upsert_organizations(&orgs).unwrap();
        storage.upsert_accounts(&set.accounts).unwrap();
        for account in &set.accounts {
            storage.record_balance(&account.id, 1790000000, account.balance).unwrap();
            storage.upsert_transactions(&account.id, &account.transactions).unwrap();
        }
        storage
    }

    #[test]
    fn empty_storage_is_an_empty_loaded_snapshot() {
        let empty = JsonStorage::with_files(MemoryFiles::new()).unwrap();
        let snapshot = Snapshot::load(&empty, 1790000000).unwrap();
        assert!(snapshot.loaded);
        assert!(!snapshot.has_accounts());
        assert!(snapshot.transactions.is_empty());
        assert!(snapshot.budget.is_none());
    }

    #[test]
    fn groups_accounts_in_empower_order_with_totals() {
        let storage = seeded();
        let snapshot = Snapshot::load(&storage, 1790000000).unwrap();
        let groups: Vec<_> = snapshot.groups.iter().map(|g| g.label.clone()).collect();
        assert_eq!(groups, vec![AccountCategory::Cash.to_string(), AccountCategory::CreditCards.to_string()]);
        assert_eq!(snapshot.groups[0].total, Decimal::new(250000, 2));
        assert_eq!(snapshot.account("chk").unwrap().available_balance, Some(Decimal::new(240000, 2)));
        assert_eq!(snapshot.net_worth.as_ref().unwrap().net_worth, Decimal::new(220000, 2));
        assert_eq!(snapshot.last_sync, Some(1790000000));
    }

    #[test]
    fn transactions_are_newest_first_and_categorized() {
        let storage = seeded();
        let snapshot = Snapshot::load(&storage, 1790000000).unwrap();
        let ids: Vec<_> = snapshot.transactions.iter().map(|t| t.transaction.id.as_str()).collect();
        assert_eq!(ids, vec!["t2", "t1"]);
        assert_eq!(snapshot.transactions[0].category, "income");
        assert!(snapshot.transaction("chk", "t1").is_some());
        assert!(snapshot.cash_flow.is_some());
        assert!(snapshot.categories.contains(&"income".to_string()));
    }

    #[test]
    fn staleness_follows_last_sync() {
        let storage = seeded();
        let fresh = Snapshot::load(&storage, 1790000000 + 60).unwrap();
        assert!(!fresh.is_stale(3600));
        let old = Snapshot::load(&storage, 1790000000 + 7200).unwrap();
        assert!(old.is_stale(3600));
        let empty = JsonStorage::with_files(MemoryFiles::new()).unwrap();
        let never = Snapshot::load(&empty, 1790000000).unwrap();
        assert!(never.is_stale(3600));
    }

    #[test]
    fn query_filters_by_text() {
        let storage = seeded();
        let snapshot = Snapshot::load(&storage, 1790000000).unwrap();
        let hits = snapshot.query(&TransactionQuery { text: Some("grocery".into()), ..Default::default() });
        assert_eq!(hits.len(), 1);
        assert_eq!(hits[0].transaction.id, "t1");
    }
}
