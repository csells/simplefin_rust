//! Monthly cash flow: income vs expense.
//!
//! A transaction counts toward cash flow unless it is pending, hidden by the
//! user, in a transfer-kind category (transfers, card payments), on an account
//! classified as a loan, or on an account excluded in [`DataConfig`]. Card
//! purchases count because they are expense-kind transactions on a card account.

use std::collections::HashMap;

use rust_decimal::Decimal;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::analysis::{
    AccountCategory, transaction_account_category, transaction_account_excluded,
};
use crate::categories::{Categorizer, CategoryKind};
use crate::month::YearMonth;
use crate::spending::{OTHER_CATEGORY, category_label};
use crate::storage::{DataConfig, TransactionWithContext};

/// How one transaction contributes to cash flow.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CashFlowEntry {
    /// [`CategoryKind::Income`] or [`CategoryKind::Expense`]; never a transfer.
    pub kind: CategoryKind,
    pub category: String,
    /// Contribution in the kind's direction: income adds the signed amount,
    /// expense adds its negation (so purchases are positive and refunds negative).
    pub amount: Decimal,
}

/// The cash-flow contribution of a transaction, or `None` when it is excluded.
/// This is the single definition of income and expense shared by cash flow
/// and budgeting.
pub fn cash_flow_entry(
    txn: &TransactionWithContext,
    config: &DataConfig,
    categorizer: &Categorizer,
) -> Option<CashFlowEntry> {
    if txn.pending
        || categorizer.is_hidden(txn)
        || transaction_account_excluded(txn, config)
        || transaction_account_category(txn, config) == AccountCategory::Loans
    {
        return None;
    }
    let category = categorizer.category(txn);
    // Unclassified transactions have no meaningful kind, so their sign decides:
    // an unknown deposit is income, not negative spending.
    let kind = if category == OTHER_CATEGORY {
        if txn.amount > Decimal::ZERO { CategoryKind::Income } else { CategoryKind::Expense }
    } else {
        categorizer.kind_of(&category)
    };
    let amount = match kind {
        CategoryKind::Income => txn.amount,
        CategoryKind::Expense => -txn.amount,
        CategoryKind::Transfer => return None,
    };
    Some(CashFlowEntry {
        kind,
        category,
        amount,
    })
}

/// One category's cash flow within a month.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct CashFlowCategory {
    pub category: String,
    pub label: String,
    pub kind: CategoryKind,
    /// Positive magnitude in the kind's direction (income received or money spent).
    pub amount: Decimal,
    pub transaction_count: usize,
}

/// Cash flow for one calendar month.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct CashFlowMonth {
    /// UTC month, `YYYY-MM`.
    pub month: String,
    pub income: Decimal,
    /// Positive magnitude.
    pub expense: Decimal,
    /// `income - expense`.
    pub net: Decimal,
    /// Income categories first, then expenses; each by amount descending.
    pub categories: Vec<CashFlowCategory>,
}

/// Cash flow over a range of months.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct CashFlowSummary {
    /// Every month in the range, including months with no activity.
    pub months: Vec<CashFlowMonth>,
    pub total_income: Decimal,
    pub total_expense: Decimal,
    pub net: Decimal,
}

/// Compute cash flow for each UTC month from `start` through `end`, inclusive.
pub fn compute_cash_flow(
    transactions: &[TransactionWithContext],
    config: &DataConfig,
    categorizer: &Categorizer,
    start: YearMonth,
    end: YearMonth,
) -> CashFlowSummary {
    let mut by_month: HashMap<YearMonth, Vec<CashFlowEntry>> = HashMap::new();
    for txn in transactions {
        let month = YearMonth::from_epoch(txn.posted);
        if month < start || month > end {
            continue;
        }
        if let Some(entry) = cash_flow_entry(txn, config, categorizer) {
            by_month.entry(month).or_default().push(entry);
        }
    }

    let months: Vec<CashFlowMonth> = YearMonth::range_inclusive(start, end)
        .into_iter()
        .map(|m| summarize_month(m, by_month.remove(&m).unwrap_or_default()))
        .collect();

    let total_income = months.iter().map(|m| m.income).sum();
    let total_expense = months.iter().map(|m| m.expense).sum();
    CashFlowSummary {
        months,
        total_income,
        total_expense,
        net: total_income - total_expense,
    }
}

fn summarize_month(month: YearMonth, entries: Vec<CashFlowEntry>) -> CashFlowMonth {
    let mut income = Decimal::ZERO;
    let mut expense = Decimal::ZERO;
    let mut by_category: HashMap<(String, CategoryKind), (Decimal, usize)> = HashMap::new();
    for entry in entries {
        match entry.kind {
            CategoryKind::Income => income += entry.amount,
            _ => expense += entry.amount,
        }
        let slot = by_category
            .entry((entry.category, entry.kind))
            .or_insert((Decimal::ZERO, 0));
        slot.0 += entry.amount;
        slot.1 += 1;
    }

    let mut categories: Vec<CashFlowCategory> = by_category
        .into_iter()
        .map(
            |((category, kind), (amount, transaction_count))| CashFlowCategory {
                label: category_label(&category),
                category,
                kind,
                amount,
                transaction_count,
            },
        )
        .collect();
    categories.sort_by(|a, b| {
        let income_first = |k: CategoryKind| k != CategoryKind::Income;
        income_first(a.kind)
            .cmp(&income_first(b.kind))
            .then(b.amount.cmp(&a.amount))
            .then(a.category.cmp(&b.category))
    });

    CashFlowMonth {
        month: month.to_string(),
        income,
        expense,
        net: income - expense,
        categories,
    }
}
