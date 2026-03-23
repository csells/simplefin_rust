use chrono::{Datelike, TimeZone, Utc};
use rust_decimal::Decimal;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use crate::spending::{SpendingCategory, SpendingRule, classify_transaction};
use crate::storage::TransactionWithContext;

/// Direction of a spending trend.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum TrendDirection {
    Up,
    Down,
    Stable,
}

impl std::fmt::Display for TrendDirection {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Up => write!(f, "up"),
            Self::Down => write!(f, "down"),
            Self::Stable => write!(f, "stable"),
        }
    }
}

/// Spending total for a single month.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct MonthlyTotal {
    /// Month label (e.g. "2024-03").
    pub period: String,
    /// Total amount for this period (negative for spending).
    pub total: Decimal,
    /// Number of transactions.
    pub transaction_count: usize,
}

/// Spending trend for a single category across multiple months.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct CategoryTrend {
    pub category: SpendingCategory,
    pub label: String,
    /// Monthly totals for this category, ordered chronologically.
    pub months: Vec<MonthlyTotal>,
    /// Average monthly spending in this category.
    pub monthly_average: Decimal,
    /// Trend direction based on recent months vs earlier months.
    pub direction: TrendDirection,
    /// Percent change from first half average to second half average.
    pub change_percent: Option<Decimal>,
}

/// Overall spending trends summary.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct TrendsSummary {
    /// Per-category spending trends.
    pub categories: Vec<CategoryTrend>,
    /// Overall monthly spending totals (all categories combined).
    pub monthly_totals: Vec<MonthlyTotal>,
    /// Average total monthly spending.
    pub overall_monthly_average: Decimal,
    /// Overall trend direction.
    pub overall_direction: TrendDirection,
}

/// Convert an epoch timestamp to a "YYYY-MM" period string.
fn epoch_to_period(ts: i64) -> String {
    let dt = Utc.timestamp_opt(ts, 0).unwrap();
    format!("{:04}-{:02}", dt.year(), dt.month())
}

/// Compute the trend direction from a series of monthly totals.
///
/// Splits the series in half: if the second half average is >10% higher
/// than the first half, it's "up"; >10% lower is "down"; otherwise "stable".
fn compute_direction(months: &[MonthlyTotal]) -> (TrendDirection, Option<Decimal>) {
    if months.len() < 2 {
        return (TrendDirection::Stable, None);
    }

    let mid = months.len() / 2;
    let first_half: Decimal = months[..mid].iter().map(|m| m.total.abs()).sum();
    let second_half: Decimal = months[mid..].iter().map(|m| m.total.abs()).sum();

    let first_avg = first_half / Decimal::from(mid as u64);
    let second_avg = second_half / Decimal::from((months.len() - mid) as u64);

    if first_avg == Decimal::ZERO {
        return (TrendDirection::Stable, None);
    }

    let change = ((second_avg - first_avg) / first_avg) * Decimal::ONE_HUNDRED;

    let direction = if change > Decimal::new(10, 0) {
        TrendDirection::Up
    } else if change < Decimal::new(-10, 0) {
        TrendDirection::Down
    } else {
        TrendDirection::Stable
    };

    Some(change).map(|c| (direction, Some(c))).unwrap_or((TrendDirection::Stable, None))
}

/// Compute spending trends over the last N months.
///
/// Groups transactions by month and category, then analyzes the trend
/// direction for each category and overall.
pub fn compute_trends(
    transactions: &[TransactionWithContext],
    rules: &[SpendingRule],
    months: usize,
) -> TrendsSummary {
    // Determine the cutoff timestamp for N months ago
    let now = Utc::now();
    let cutoff = now
        .checked_sub_months(chrono::Months::new(months as u32))
        .unwrap_or(now);
    let cutoff_ts = cutoff.timestamp();

    // Group transactions by (period, category)
    let mut by_period_category: HashMap<(String, SpendingCategory), (Decimal, usize)> =
        HashMap::new();
    let mut by_period: HashMap<String, (Decimal, usize)> = HashMap::new();

    for txn in transactions {
        if txn.pending || txn.posted < cutoff_ts {
            continue;
        }

        // Only count spending (negative amounts), skip income/transfers for trend analysis
        let category = classify_transaction(&txn.description, rules);
        if category == SpendingCategory::Income || category == SpendingCategory::Transfer {
            continue;
        }

        let period = epoch_to_period(txn.posted);

        let entry = by_period_category
            .entry((period.clone(), category))
            .or_insert((Decimal::ZERO, 0));
        entry.0 += txn.amount;
        entry.1 += 1;

        let overall = by_period.entry(period).or_insert((Decimal::ZERO, 0));
        overall.0 += txn.amount;
        overall.1 += 1;
    }

    // Collect all periods, sorted
    let mut periods: Vec<String> = by_period.keys().cloned().collect();
    periods.sort();

    // Build overall monthly totals
    let monthly_totals: Vec<MonthlyTotal> = periods
        .iter()
        .map(|p| {
            let (total, count) = by_period.get(p).copied().unwrap_or((Decimal::ZERO, 0));
            MonthlyTotal {
                period: p.clone(),
                total,
                transaction_count: count,
            }
        })
        .collect();

    // Build per-category trends
    let mut categories_map: HashMap<SpendingCategory, Vec<MonthlyTotal>> = HashMap::new();
    for ((period, category), (total, count)) in &by_period_category {
        categories_map
            .entry(category.clone())
            .or_default()
            .push(MonthlyTotal {
                period: period.clone(),
                total: *total,
                transaction_count: *count,
            });
    }

    let mut categories: Vec<CategoryTrend> = categories_map
        .into_iter()
        .map(|(category, mut month_data)| {
            month_data.sort_by(|a, b| a.period.cmp(&b.period));

            let total: Decimal = month_data.iter().map(|m| m.total).sum();
            let monthly_average = if month_data.is_empty() {
                Decimal::ZERO
            } else {
                total / Decimal::from(month_data.len() as u64)
            };

            let (direction, change_percent) = compute_direction(&month_data);

            CategoryTrend {
                label: category.to_string(),
                category,
                months: month_data,
                monthly_average,
                direction,
                change_percent,
            }
        })
        .collect();

    // Sort by absolute monthly average descending
    categories.sort_by(|a, b| b.monthly_average.abs().cmp(&a.monthly_average.abs()));

    let overall_monthly_average = if monthly_totals.is_empty() {
        Decimal::ZERO
    } else {
        let total: Decimal = monthly_totals.iter().map(|m| m.total).sum();
        total / Decimal::from(monthly_totals.len() as u64)
    };

    let (overall_direction, _) = compute_direction(&monthly_totals);

    TrendsSummary {
        categories,
        monthly_totals,
        overall_monthly_average,
        overall_direction,
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
    fn epoch_to_period_formats_correctly() {
        // 2024-03-15 00:00:00 UTC
        let ts = 1710460800;
        assert_eq!(epoch_to_period(ts), "2024-03");
    }

    #[test]
    fn computes_monthly_totals() {
        let day = 86400i64;
        let now = Utc::now().timestamp();
        let txns = vec![
            make_txn("GROCERY STORE", Decimal::new(-5000, 2), now - 10 * day),
            make_txn("RESTAURANT", Decimal::new(-2500, 2), now - 5 * day),
        ];

        let summary = compute_trends(&txns, &crate::spending::default_spending_patterns(), 6);
        assert!(!summary.monthly_totals.is_empty());
        // Total should be -75.00
        let total: Decimal = summary.monthly_totals.iter().map(|m| m.total).sum();
        assert_eq!(total, Decimal::new(-7500, 2));
    }

    #[test]
    fn excludes_income_and_transfers() {
        let now = Utc::now().timestamp();
        let txns = vec![
            make_txn("PAYROLL DEPOSIT", Decimal::new(300000, 2), now - 86400),
            make_txn("ATM WITHDRAWAL", Decimal::new(-20000, 2), now - 86400),
            make_txn("GROCERY STORE", Decimal::new(-5000, 2), now - 86400),
        ];

        let summary = compute_trends(&txns, &crate::spending::default_spending_patterns(), 6);
        // Only grocery should appear in trends (income and transfer excluded)
        let total: Decimal = summary.monthly_totals.iter().map(|m| m.total).sum();
        assert_eq!(total, Decimal::new(-5000, 2));
    }

    #[test]
    fn stable_direction_for_consistent_spending() {
        let months = vec![
            MonthlyTotal {
                period: "2024-01".into(),
                total: Decimal::new(-10000, 2),
                transaction_count: 5,
            },
            MonthlyTotal {
                period: "2024-02".into(),
                total: Decimal::new(-10500, 2),
                transaction_count: 5,
            },
            MonthlyTotal {
                period: "2024-03".into(),
                total: Decimal::new(-9800, 2),
                transaction_count: 5,
            },
            MonthlyTotal {
                period: "2024-04".into(),
                total: Decimal::new(-10200, 2),
                transaction_count: 5,
            },
        ];
        let (direction, _) = compute_direction(&months);
        assert_eq!(direction, TrendDirection::Stable);
    }
}
