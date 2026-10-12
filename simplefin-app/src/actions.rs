//! Writes the user makes from the UI, applied to local storage.

use rust_decimal::Decimal;
use simplefin::{Storage, TransactionEdit, TransactionWithContext, apply_category_to_future};

/// Changes the user's edit overlay for one transaction.
pub fn edit_transaction(
    storage: &impl Storage,
    account_id: &str,
    transaction_id: &str,
    change: impl FnOnce(&mut TransactionEdit),
) -> anyhow::Result<()> {
    let mut edits = storage.get_transaction_edits()?;
    let mut edit = edits.get(account_id, transaction_id).cloned().unwrap_or_default();
    change(&mut edit);
    edits.set(account_id, transaction_id, edit);
    storage.set_transaction_edits(&edits)?;
    Ok(())
}

/// Recategorizes one transaction and, optionally, adds a rule so future
/// transactions from the same merchant get the same category.
pub fn recategorize(
    storage: &impl Storage,
    txn: &TransactionWithContext,
    category: &str,
    apply_to_future: bool,
) -> anyhow::Result<()> {
    edit_transaction(storage, &txn.account_id, &txn.id, |edit| {
        edit.category = Some(category.to_string());
    })?;
    if apply_to_future {
        apply_category_to_future(storage, txn, category)?;
    }
    Ok(())
}

/// Sets (or clears, with `None`) the overall monthly spending limit.
pub fn set_monthly_budget(storage: &impl Storage, limit: Option<Decimal>) -> anyhow::Result<()> {
    let mut config = storage.get_config()?;
    config.budget.monthly_limit = limit;
    storage.set_config(&config)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use simplefin::{JsonStorage, MemoryFiles};

    fn storage() -> JsonStorage<MemoryFiles> {
        JsonStorage::with_files(MemoryFiles::new()).unwrap()
    }

    fn txn() -> TransactionWithContext {
        TransactionWithContext {
            id: "t1".into(),
            account_id: "chk".into(),
            account_name: "Checking (1234)".into(),
            org_name: "Example Bank".into(),
            currency: "USD".into(),
            posted: 1_790_000_000,
            amount: Decimal::new(-1299, 2),
            description: "STREAMCO*MONTHLY 8001234".into(),
            ..Default::default()
        }
    }

    #[test]
    fn edits_accumulate_per_transaction() {
        let storage = storage();
        edit_transaction(&storage, "chk", "t1", |e| e.description = Some("Streaming".into())).unwrap();
        edit_transaction(&storage, "chk", "t1", |e| e.hidden = true).unwrap();
        let edits = storage.get_transaction_edits().unwrap();
        let edit = edits.get("chk", "t1").unwrap();
        assert_eq!(edit.description.as_deref(), Some("Streaming"));
        assert!(edit.hidden);
        assert!(edits.get("other", "t1").is_none());
    }

    #[test]
    fn recategorize_with_rule_applies_to_future_transactions() {
        let storage = storage();
        recategorize(&storage, &txn(), "entertainment", true).unwrap();
        let categorizer = simplefin::Categorizer::load(&storage).unwrap();
        let future = TransactionWithContext { id: "t2".into(), ..txn() };
        assert_eq!(categorizer.category(&future), "entertainment");
    }

    #[test]
    fn recategorize_once_leaves_other_transactions_alone() {
        let storage = storage();
        recategorize(&storage, &txn(), "entertainment", false).unwrap();
        let categorizer = simplefin::Categorizer::load(&storage).unwrap();
        assert_eq!(categorizer.category(&txn()), "entertainment");
        let future = TransactionWithContext { id: "t2".into(), ..txn() };
        assert_ne!(categorizer.category(&future), "entertainment");
    }

    #[test]
    fn monthly_budget_round_trips() {
        let storage = storage();
        set_monthly_budget(&storage, Some(Decimal::new(250000, 2))).unwrap();
        let config = storage.get_config().unwrap();
        assert_eq!(config.budget.monthly_limit, Some(Decimal::new(250000, 2)));
        set_monthly_budget(&storage, None).unwrap();
        let config = storage.get_config().unwrap();
        assert_eq!(config.budget.monthly_limit, None);
    }
}
