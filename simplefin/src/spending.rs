use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use crate::storage::TransactionWithContext;

/// Spending category for transaction classification.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SpendingCategory {
    Restaurants,
    Groceries,
    Utilities,
    Transportation,
    Shopping,
    Entertainment,
    Healthcare,
    Income,
    Transfer,
    Other,
}

impl std::fmt::Display for SpendingCategory {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Restaurants => write!(f, "Restaurants"),
            Self::Groceries => write!(f, "Groceries"),
            Self::Utilities => write!(f, "Utilities"),
            Self::Transportation => write!(f, "Transportation"),
            Self::Shopping => write!(f, "Shopping"),
            Self::Entertainment => write!(f, "Entertainment"),
            Self::Healthcare => write!(f, "Healthcare"),
            Self::Income => write!(f, "Income"),
            Self::Transfer => write!(f, "Transfer"),
            Self::Other => write!(f, "Other"),
        }
    }
}

/// A rule for classifying transactions into spending categories.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpendingRule {
    pub pattern: String,
    pub category: SpendingCategory,
}

/// Per-category spending total.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpendingTotal {
    pub category: SpendingCategory,
    pub label: String,
    pub total: Decimal,
    pub transaction_count: usize,
}

/// Spending summary for a time period.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpendingSummary {
    pub categories: Vec<SpendingTotal>,
    pub total_spending: Decimal,
    pub total_income: Decimal,
    pub net: Decimal,
}

/// Built-in keyword patterns for transaction classification.
/// Each entry is (keywords separated by |, category).
const BUILTIN_RULES: &[(&str, SpendingCategory)] = &[
    ("restaurant|dine|dining|cafe|pizza|burger|sushi|taco|chipotle|mcdonald|starbucks|grubhub|doordash|ubereats", SpendingCategory::Restaurants),
    ("grocery|whole foods|trader joe|safeway|kroger|costco|fred meyer|winco|albertson", SpendingCategory::Groceries),
    ("electric|water|sewer|internet|cable|phone|verizon|comcast|xfinity|t-mobile|att|pgande|utility", SpendingCategory::Utilities),
    ("uber|lyft|parking|fuel|gas station|shell|chevron|transit|trimet", SpendingCategory::Transportation),
    ("amazon|target|walmart|best buy|store|shop|purchase|ebay|etsy", SpendingCategory::Shopping),
    ("netflix|spotify|hulu|theater|movie|concert|disney|youtube|gaming|steam", SpendingCategory::Entertainment),
    ("pharmacy|doctor|hospital|dental|medical|health|clinic|urgent care|cvs|walgreen", SpendingCategory::Healthcare),
    ("payroll|direct dep|salary|wage|deposit from employer", SpendingCategory::Income),
    ("transfer|payment|zelle|venmo|paypal|ach", SpendingCategory::Transfer),
];

/// Classify a transaction description into a spending category.
/// Custom rules are checked first, then built-in keyword patterns.
pub fn classify_transaction(description: &str, custom_rules: &[SpendingRule]) -> SpendingCategory {
    let lower = description.to_lowercase();

    // Custom rules first
    for rule in custom_rules {
        if lower.contains(&rule.pattern.to_lowercase()) {
            return rule.category.clone();
        }
    }

    // Built-in rules
    for (keywords, category) in BUILTIN_RULES {
        for keyword in keywords.split('|') {
            if lower.contains(keyword) {
                return category.clone();
            }
        }
    }

    SpendingCategory::Other
}

/// Compute spending summary from transactions.
/// Spending = negative amounts (debits). Income = positive amounts (credits).
pub fn compute_spending(
    transactions: &[TransactionWithContext],
    custom_rules: &[SpendingRule],
) -> SpendingSummary {
    let mut by_category: HashMap<SpendingCategory, (Decimal, usize)> = HashMap::new();
    let mut total_spending = Decimal::ZERO;
    let mut total_income = Decimal::ZERO;

    for txn in transactions {
        if txn.pending {
            continue;
        }

        let category = classify_transaction(&txn.description, custom_rules);

        let entry = by_category.entry(category).or_insert((Decimal::ZERO, 0));
        entry.0 += txn.amount;
        entry.1 += 1;

        if txn.amount < Decimal::ZERO {
            total_spending += txn.amount;
        } else {
            total_income += txn.amount;
        }
    }

    let mut categories: Vec<SpendingTotal> = by_category
        .into_iter()
        .map(|(cat, (total, count))| SpendingTotal {
            label: cat.to_string(),
            category: cat,
            total,
            transaction_count: count,
        })
        .collect();

    // Sort by absolute total descending
    categories.sort_by(|a, b| b.total.abs().cmp(&a.total.abs()));

    SpendingSummary {
        categories,
        total_spending,
        total_income,
        net: total_income + total_spending,
    }
}
