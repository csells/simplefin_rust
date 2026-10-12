//! Per-transaction user edits layered over collected data.
//!
//! Edits live in their own file in the data directory (never in the
//! transaction files), so re-collecting a transaction never discards them.

use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use crate::categories::normalize_category_name;
use crate::error::{Result, SimplefinError};
use crate::recurring::normalize_merchant;
use crate::spending::{SpendingRule, add_user_spending_rule, classify_transaction};
use crate::storage::{Storage, TransactionWithContext};

/// A user's edit to one transaction. Unset fields leave the original intact.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct TransactionEdit {
    /// Replacement description.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    /// Category override; takes priority over every rule.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub category: Option<String>,
    /// Hidden from all reporting (for example, a duplicate), but still listed
    /// when a query asks for hidden transactions. Reversible.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub hidden: bool,
}

impl TransactionEdit {
    /// True when the edit changes nothing.
    pub fn is_empty(&self) -> bool {
        self.description.is_none() && self.category.is_none() && !self.hidden
    }
}

/// All transaction edits, keyed by account ID and then transaction ID.
///
/// SimpleFIN transaction IDs are only unique within an account, so edits are
/// scoped by account to avoid touching another account's transaction.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct TransactionEdits(HashMap<String, HashMap<String, TransactionEdit>>);

impl TransactionEdits {
    /// The edit for a transaction, if any.
    pub fn get(&self, account_id: &str, transaction_id: &str) -> Option<&TransactionEdit> {
        self.0.get(account_id)?.get(transaction_id)
    }

    /// The edit for a stored transaction, if any.
    pub fn for_transaction(&self, txn: &TransactionWithContext) -> Option<&TransactionEdit> {
        self.get(&txn.account_id, &txn.id)
    }

    /// Set (or, when `edit` is empty, remove) the edit for a transaction.
    pub fn set(&mut self, account_id: &str, transaction_id: &str, edit: TransactionEdit) {
        if edit.is_empty() {
            self.remove(account_id, transaction_id);
            return;
        }
        self.0
            .entry(account_id.to_string())
            .or_default()
            .insert(transaction_id.to_string(), edit);
    }

    /// Remove the edit for a transaction, returning it.
    pub fn remove(&mut self, account_id: &str, transaction_id: &str) -> Option<TransactionEdit> {
        let account = self.0.get_mut(account_id)?;
        let removed = account.remove(transaction_id);
        if account.is_empty() {
            self.0.remove(account_id);
        }
        removed
    }

    /// Number of edited transactions.
    pub fn len(&self) -> usize {
        self.0.values().map(HashMap::len).sum()
    }

    /// True when no transaction is edited.
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

/// The user's description for a transaction, or the original one.
pub fn effective_description<'a>(
    txn: &'a TransactionWithContext,
    edits: &'a TransactionEdits,
) -> &'a str {
    edits
        .for_transaction(txn)
        .and_then(|e| e.description.as_deref())
        .unwrap_or(&txn.description)
}

/// A transaction's category: the per-transaction edit, else the first
/// matching rule (callers order user rules before patterns), else "other".
///
/// Rules match the original provider description, because rules created from
/// transactions are derived from provider text that future transactions share.
pub fn effective_category(
    txn: &TransactionWithContext,
    edits: &TransactionEdits,
    rules: &[SpendingRule],
) -> String {
    edits
        .for_transaction(txn)
        .and_then(|e| e.category.clone())
        .unwrap_or_else(|| classify_transaction(&txn.description, rules))
}

/// Build an "apply to future transactions" rule from a transaction: the
/// pattern is its normalized merchant name, lowercased.
pub fn rule_from_transaction(txn: &TransactionWithContext, category: &str) -> Result<SpendingRule> {
    // `|` separates keywords in a pattern, so it cannot appear in a merchant.
    let merchant = normalize_merchant(&txn.description)
        .replace('|', " ")
        .to_lowercase();
    let pattern = merchant.split_whitespace().collect::<Vec<_>>().join(" ");
    if pattern.is_empty() {
        return Err(SimplefinError::InvalidArgument(format!(
            "cannot derive a merchant pattern from \"{}\"",
            txn.description
        )));
    }
    let category = normalize_category_name(category);
    if category.is_empty() {
        return Err(SimplefinError::InvalidArgument("category is empty".into()));
    }
    Ok(SpendingRule { pattern, category })
}

/// Create a highest-priority user spending rule from a transaction and save it
/// in the user config, replacing any older rule for the same merchant.
pub fn apply_category_to_future(
    storage: &(impl Storage + ?Sized),
    txn: &TransactionWithContext,
    category: &str,
) -> Result<SpendingRule> {
    let rule = rule_from_transaction(txn, category)?;
    let mut config = storage.get_config()?;
    add_user_spending_rule(&mut config, rule.clone());
    storage.set_config(&config)?;
    Ok(rule)
}
