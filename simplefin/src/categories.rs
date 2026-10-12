//! Category kinds and the shared transaction categorizer.
//!
//! Every analysis categorizes transactions through [`Categorizer`] so the
//! precedence is identical everywhere: per-transaction edit > user spending
//! rules > spending patterns > [`OTHER_CATEGORY`](crate::spending::OTHER_CATEGORY).

use std::collections::HashMap;

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::edits::{TransactionEdits, effective_category, effective_description};
use crate::error::Result;
use crate::spending::{SpendingRule, assemble_spending_rules};
use crate::storage::{Storage, TransactionWithContext};

/// Category name whose default kind is [`CategoryKind::Income`].
pub const INCOME_CATEGORY: &str = "income";

/// Category name whose default kind is [`CategoryKind::Transfer`].
pub const TRANSFER_CATEGORY: &str = "transfer";

/// How a spending category participates in cash flow and budgeting.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum CategoryKind {
    Income,
    Expense,
    /// Money moving between the user's own accounts (including card payments).
    /// Excluded from cash flow and budgets.
    Transfer,
}

/// Default kind for a category: "income" is income, "transfer" is a transfer,
/// and every other category is an expense.
pub fn default_category_kind(category: &str) -> CategoryKind {
    match category {
        INCOME_CATEGORY => CategoryKind::Income,
        TRANSFER_CATEGORY => CategoryKind::Transfer,
        _ => CategoryKind::Expense,
    }
}

/// Kind for a category, honoring user overrides (`DataConfig::category_kinds`).
pub fn category_kind(category: &str, overrides: &HashMap<String, CategoryKind>) -> CategoryKind {
    overrides
        .get(category)
        .copied()
        .unwrap_or_else(|| default_category_kind(category))
}

/// Normalize a user-entered category name to its stored form
/// (trimmed, lowercase, spaces as underscores), e.g. "Personal Care" → "personal_care".
pub fn normalize_category_name(name: &str) -> String {
    name.trim().to_lowercase().replace(' ', "_")
}

/// Categorizes transactions with a single, consistent precedence.
#[derive(Debug, Clone, Default)]
pub struct Categorizer {
    rules: Vec<SpendingRule>,
    edits: TransactionEdits,
    kinds: HashMap<String, CategoryKind>,
}

impl Categorizer {
    /// Build from rules already in priority order (user rules, then patterns),
    /// the edit overlay, and category-kind overrides.
    pub fn new(
        rules: Vec<SpendingRule>,
        edits: TransactionEdits,
        kinds: HashMap<String, CategoryKind>,
    ) -> Self {
        Self {
            rules,
            edits,
            kinds,
        }
    }

    /// Build from rules only: no edits and default category kinds.
    pub fn from_rules(rules: Vec<SpendingRule>) -> Self {
        Self::new(rules, TransactionEdits::default(), HashMap::new())
    }

    /// Load user rules, spending patterns, edits and kind overrides from storage.
    pub fn load(storage: &(impl Storage + ?Sized)) -> Result<Self> {
        let config = storage.get_config()?;
        let patterns = storage.get_spending_patterns()?;
        Ok(Self::new(
            assemble_spending_rules(&config, patterns),
            storage.get_transaction_edits()?,
            config.category_kinds,
        ))
    }

    /// Rules in priority order.
    pub fn rules(&self) -> &[SpendingRule] {
        &self.rules
    }

    /// The per-transaction edit overlay.
    pub fn edits(&self) -> &TransactionEdits {
        &self.edits
    }

    /// Effective (possibly user-renamed) description.
    pub fn description<'a>(&'a self, txn: &'a TransactionWithContext) -> &'a str {
        effective_description(txn, &self.edits)
    }

    /// Effective category.
    pub fn category(&self, txn: &TransactionWithContext) -> String {
        effective_category(txn, &self.edits, &self.rules)
    }

    /// Kind of a category name, honoring overrides.
    pub fn kind_of(&self, category: &str) -> CategoryKind {
        category_kind(category, &self.kinds)
    }

    /// Kind of a transaction's effective category.
    pub fn kind(&self, txn: &TransactionWithContext) -> CategoryKind {
        self.kind_of(&self.category(txn))
    }

    /// Whether the user hid this transaction from reporting.
    pub fn is_hidden(&self, txn: &TransactionWithContext) -> bool {
        self.edits.for_transaction(txn).is_some_and(|e| e.hidden)
    }
}
