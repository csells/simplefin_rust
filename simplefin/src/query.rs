//! Filtering, search and sorting over stored transactions with user edits applied.

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::analysis::{AccountCategory, transaction_account_category};
use crate::categories::{Categorizer, CategoryKind};
use crate::spending::category_label;
use crate::storage::{DataConfig, TransactionWithContext};

/// A transaction with its effective (edited) values resolved.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct TransactionView {
    /// The stored transaction, unmodified.
    pub transaction: TransactionWithContext,
    /// Effective description (the user's rename, or the original).
    pub description: String,
    /// Effective category.
    pub category: String,
    pub category_label: String,
    pub kind: CategoryKind,
    /// Effective classification of the transaction's account.
    pub account_category: AccountCategory,
    /// Hidden from reporting by the user.
    pub hidden: bool,
}

/// A pure transaction query. All set criteria must match.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct TransactionQuery {
    /// Case-insensitive substring over effective description, payee and memo.
    /// Blank text matches everything.
    pub text: Option<String>,
    /// Effective category name.
    pub category: Option<String>,
    /// Effective account classification.
    pub account_category: Option<AccountCategory>,
    /// Inclusive start (epoch seconds) of the transaction date.
    pub start_date: Option<i64>,
    /// Exclusive end (epoch seconds) of the transaction date.
    pub end_date: Option<i64>,
    /// Include transactions the user hid.
    pub include_hidden: bool,
}

/// The date a transaction is listed under: `posted`, or `transacted_at` for
/// pending transactions that have no posted date yet.
pub fn transaction_date(txn: &TransactionWithContext) -> i64 {
    if txn.posted > 0 {
        txn.posted
    } else {
        txn.transacted_at.unwrap_or(txn.posted)
    }
}

impl TransactionQuery {
    /// Run the query, returning matches newest first (ties by ID).
    pub fn run(
        &self,
        transactions: &[TransactionWithContext],
        config: &DataConfig,
        categorizer: &Categorizer,
    ) -> Vec<TransactionView> {
        let needle = self
            .text
            .as_deref()
            .map(|t| t.trim().to_lowercase())
            .filter(|t| !t.is_empty());

        let mut rows: Vec<TransactionView> = transactions
            .iter()
            .filter(|t| self.in_date_range(transaction_date(t)))
            .map(|t| view(t, config, categorizer))
            .filter(|v| self.include_hidden || !v.hidden)
            .filter(|v| self.category.as_ref().is_none_or(|c| *c == v.category))
            .filter(|v| {
                self.account_category
                    .is_none_or(|c| c == v.account_category)
            })
            .filter(|v| needle.as_ref().is_none_or(|n| matches_text(v, n)))
            .collect();

        rows.sort_by(|a, b| {
            transaction_date(&b.transaction)
                .cmp(&transaction_date(&a.transaction))
                .then_with(|| a.transaction.id.cmp(&b.transaction.id))
        });
        rows
    }

    fn in_date_range(&self, date: i64) -> bool {
        self.start_date.is_none_or(|s| date >= s) && self.end_date.is_none_or(|e| date < e)
    }
}

fn view(
    txn: &TransactionWithContext,
    config: &DataConfig,
    categorizer: &Categorizer,
) -> TransactionView {
    let category = categorizer.category(txn);
    TransactionView {
        description: categorizer.description(txn).to_string(),
        category_label: category_label(&category),
        kind: categorizer.kind_of(&category),
        category,
        account_category: transaction_account_category(txn, config),
        hidden: categorizer.is_hidden(txn),
        transaction: txn.clone(),
    }
}

fn matches_text(view: &TransactionView, needle: &str) -> bool {
    let contains = |s: &str| s.to_lowercase().contains(needle);
    contains(&view.description)
        || view.transaction.payee.as_deref().is_some_and(contains)
        || view.transaction.memo.as_deref().is_some_and(contains)
}
