use rust_decimal::Decimal;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use crate::spending::{SpendingCategory, SpendingRule, classify_transaction};
use crate::storage::TransactionWithContext;

/// A detected recurring expense.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct RecurringExpense {
    /// Normalized merchant name used for grouping.
    pub merchant: String,
    /// Spending category of this recurring expense.
    pub category: SpendingCategory,
    /// Average transaction amount (negative for expenses).
    pub average_amount: Decimal,
    /// Detected frequency in days between occurrences.
    pub frequency_days: u32,
    /// Human-readable frequency label.
    pub frequency_label: String,
    /// Most recent transaction timestamp.
    pub last_seen: i64,
    /// Estimated next occurrence timestamp.
    pub next_expected: Option<i64>,
    /// Total number of matching transactions.
    pub occurrences: usize,
    /// Total amount spent across all occurrences.
    pub total_amount: Decimal,
}

/// Summary of all detected recurring expenses.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct RecurringSummary {
    pub recurring: Vec<RecurringExpense>,
    /// Total monthly cost of all detected recurring expenses.
    pub estimated_monthly_total: Decimal,
}

/// Normalize a transaction description to group similar transactions.
///
/// Strips common POS prefixes, trailing numbers/IDs, and normalizes whitespace.
fn normalize_merchant(description: &str) -> String {
    let mut s = description.to_uppercase();

    // Strip common POS/card prefixes
    let prefixes = [
        "EXT CREDIT CARD DEBIT ",
        "EXT CREDIT CARD ",
        "PURCHASE AUTHORIZED ON ",
        "RECURRING PAYMENT ",
        "AUTOMATIC PAYMENT ",
        "ONLINE PAYMENT ",
        "SQ *",
        "TST* ",
        "DD *",
        "SP ",
        "CKE *",
        "PP*",
    ];
    for prefix in &prefixes {
        if let Some(rest) = s.strip_prefix(prefix) {
            s = rest.to_string();
        }
    }

    // Strip trailing location info (state abbreviations like " CA", " OR", " NY")
    // and trailing numbers/IDs (e.g., " #1234", " 650-253-0000")
    let s = s
        .split_whitespace()
        .take_while(|word| {
            // Stop at: hash-prefixed IDs (#1234), pure numbers/phone numbers,
            // or 2-letter state abbreviations (CA, OR, NY)
            let is_state_abbrev = word.len() == 2 && word.chars().all(|c| c.is_ascii_uppercase());
            let is_number = word.chars().all(|c| c.is_ascii_digit() || c == '-');
            let is_hash_id = word.starts_with('#');
            !is_state_abbrev && !is_number && !is_hash_id
        })
        .collect::<Vec<_>>()
        .join(" ");

    // Trim and collapse whitespace
    s.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// Detect recurring expenses from transaction history.
///
/// Groups transactions by normalized merchant name, then checks if they
/// occur at regular intervals. Returns expenses sorted by estimated monthly cost.
pub fn detect_recurring(
    transactions: &[TransactionWithContext],
    rules: &[SpendingRule],
    min_occurrences: usize,
) -> RecurringSummary {
    // Group by normalized merchant
    let mut by_merchant: HashMap<String, Vec<&TransactionWithContext>> = HashMap::new();

    for txn in transactions {
        if txn.pending {
            continue;
        }
        let merchant = normalize_merchant(&txn.description);
        if !merchant.is_empty() {
            by_merchant.entry(merchant).or_default().push(txn);
        }
    }

    let mut recurring = Vec::new();

    for (merchant, mut txns) in by_merchant {
        if txns.len() < min_occurrences {
            continue;
        }

        // Sort by posted date
        txns.sort_by_key(|t| t.posted);

        // Calculate intervals between consecutive transactions
        let intervals: Vec<i64> = txns
            .windows(2)
            .map(|w| (w[1].posted - w[0].posted) / 86400) // convert to days
            .filter(|&d| d > 0)
            .collect();

        if intervals.is_empty() {
            continue;
        }

        // Calculate median interval
        let mut sorted_intervals = intervals.clone();
        sorted_intervals.sort();
        let median_days = sorted_intervals[sorted_intervals.len() / 2];

        // Only consider it recurring if interval is somewhat consistent
        // Allow 40% variance from median
        let consistent_count = intervals
            .iter()
            .filter(|&&d| {
                let diff = (d - median_days).unsigned_abs();
                diff <= ((median_days as u64) * 2 / 5).max(2)
            })
            .count();

        // More than 2/3 of intervals must be consistent
        if consistent_count * 3 <= intervals.len() * 2 {
            continue;
        }

        let frequency_days = median_days as u32;
        let frequency_label = match frequency_days {
            0..=10 => "weekly".to_string(),
            11..=45 => "monthly".to_string(),
            46..=100 => "quarterly".to_string(),
            101..=200 => "semi-annually".to_string(),
            _ => "annually".to_string(),
        };

        let total_amount: Decimal = txns.iter().map(|t| t.amount).sum();
        let average_amount = total_amount / Decimal::from(txns.len() as u64);

        let last_seen = txns.last().map(|t| t.posted).unwrap_or(0);
        let next_expected = if frequency_days > 0 {
            Some(last_seen + (frequency_days as i64) * 86400)
        } else {
            None
        };

        let category =
            classify_transaction(&txns[0].description, rules);

        recurring.push(RecurringExpense {
            merchant,
            category,
            average_amount,
            frequency_days,
            frequency_label,
            last_seen,
            next_expected,
            occurrences: txns.len(),
            total_amount,
        });
    }

    // Sort by absolute average amount descending (biggest expenses first)
    recurring.sort_by(|a, b| b.average_amount.abs().cmp(&a.average_amount.abs()));

    // Estimate monthly total: sum of (average_amount * 30 / frequency_days)
    let estimated_monthly_total = recurring
        .iter()
        .filter(|r| r.average_amount < Decimal::ZERO && r.frequency_days > 0)
        .map(|r| {
            r.average_amount * Decimal::from(30) / Decimal::from(r.frequency_days as u64)
        })
        .sum();

    RecurringSummary {
        recurring,
        estimated_monthly_total,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_txn(description: &str, amount: Decimal, posted: i64) -> TransactionWithContext {
        TransactionWithContext {
            id: format!("txn-{posted}"),
            account_id: "acc-1".to_string(),
            account_name: "Checking".to_string(),
            org_name: "Bank".to_string(),
            currency: "USD".to_string(),
            posted,
            amount,
            description: description.to_string(),
            transacted_at: None,
            pending: false,
        }
    }

    #[test]
    fn normalize_strips_pos_prefix() {
        assert_eq!(normalize_merchant("SQ *COFFEE HOUSE"), "COFFEE HOUSE");
        assert_eq!(normalize_merchant("TST* PIZZA PLACE"), "PIZZA PLACE");
    }

    #[test]
    fn normalize_strips_trailing_numbers() {
        assert_eq!(normalize_merchant("NETFLIX 1234-5678"), "NETFLIX");
    }

    #[test]
    fn detects_monthly_recurring() {
        let day = 86400i64;
        let base = 1700000000i64;
        let txns: Vec<TransactionWithContext> = (0..4)
            .map(|i| {
                make_txn(
                    "NETFLIX",
                    Decimal::new(-1599, 2), // -15.99
                    base + i * 30 * day,
                )
            })
            .collect();

        let summary = detect_recurring(&txns, &[], 2);
        assert_eq!(summary.recurring.len(), 1);
        assert_eq!(summary.recurring[0].merchant, "NETFLIX");
        assert_eq!(summary.recurring[0].occurrences, 4);
        assert_eq!(summary.recurring[0].frequency_label, "monthly");
    }

    #[test]
    fn ignores_irregular_transactions() {
        let day = 86400i64;
        let base = 1700000000i64;
        // Irregular intervals: 5 days, 90 days, 3 days
        let txns = vec![
            make_txn("RANDOM VENDOR", Decimal::new(-2000, 2), base),
            make_txn("RANDOM VENDOR", Decimal::new(-2000, 2), base + 5 * day),
            make_txn("RANDOM VENDOR", Decimal::new(-2000, 2), base + 95 * day),
            make_txn("RANDOM VENDOR", Decimal::new(-2000, 2), base + 98 * day),
        ];

        let summary = detect_recurring(&txns, &[], 2);
        // Should not detect as recurring due to inconsistent intervals
        assert!(
            summary.recurring.is_empty(),
            "irregular transactions should not be detected as recurring"
        );
    }

    #[test]
    fn respects_min_occurrences() {
        let day = 86400i64;
        let base = 1700000000i64;
        let txns = vec![
            make_txn("HULU", Decimal::new(-1299, 2), base),
            make_txn("HULU", Decimal::new(-1299, 2), base + 30 * day),
        ];

        let summary = detect_recurring(&txns, &[], 3);
        assert!(summary.recurring.is_empty());

        let summary = detect_recurring(&txns, &[], 2);
        assert_eq!(summary.recurring.len(), 1);
    }
}
