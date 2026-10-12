//! Monthly budget status. Spending uses the cash-flow expense definition
//! ([`cash_flow_entry`](crate::cashflow::cash_flow_entry)), so budgets and cash
//! flow always agree.

use std::collections::HashMap;

use chrono::{DateTime, Datelike};
use rust_decimal::Decimal;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::cashflow::{CashFlowMonth, cash_flow_entry, compute_cash_flow};
use crate::categories::{Categorizer, CategoryKind};
use crate::month::YearMonth;
use crate::spending::{OTHER_CATEGORY, category_label};
use crate::storage::{DataConfig, TransactionWithContext};

/// Number of individually listed top expense categories.
const TOP_CATEGORY_COUNT: usize = 5;
const SECONDS_PER_DAY: i64 = 86_400;

/// User budget limits, stored in `DataConfig::budget`.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct BudgetConfig {
    /// Overall monthly spending limit.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub monthly_limit: Option<Decimal>,
    /// Monthly limits per spending category.
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub category_limits: HashMap<String, Decimal>,
}

impl BudgetConfig {
    /// True when no limit is set.
    pub fn is_empty(&self) -> bool {
        self.monthly_limit.is_none() && self.category_limits.is_empty()
    }
}

/// Spending in one category (or the remainder bucket).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct CategorySpend {
    pub category: String,
    pub label: String,
    pub amount: Decimal,
}

/// A category's spending this month against its limit.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct CategoryLimitStatus {
    pub category: String,
    pub label: String,
    pub limit: Decimal,
    pub spent: Decimal,
    pub remaining: Decimal,
    pub over_limit: bool,
    /// Spending expected by today at a linear pace.
    pub expected_spend: Decimal,
}

/// Average monthly spending over a trailing window of complete months.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct SpendingAverage {
    pub amount: Decimal,
    /// Complete months averaged (the window is clipped to the start of history).
    pub months: usize,
    /// True when no complete month exists and the amount projects this
    /// month's spending to a full month.
    pub extrapolated: bool,
}

/// Budget status for the current month.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct BudgetStatus {
    /// Current UTC month, `YYYY-MM`.
    pub month: String,
    pub day_of_month: u32,
    pub days_in_month: u32,
    /// Spending so far this month (positive magnitude).
    pub spent: Decimal,
    pub monthly_limit: Option<Decimal>,
    pub remaining: Option<Decimal>,
    pub over_budget: bool,
    /// Spending expected by today at a linear pace through the month.
    pub expected_spend: Option<Decimal>,
    /// Last month's spending through the same day of the month (clamped to
    /// last month's length).
    pub last_month_same_day: Decimal,
    /// Top expense categories this month, then the remainder as "other".
    pub top_categories: Vec<CategorySpend>,
    pub average_3_month: SpendingAverage,
    pub average_12_month: SpendingAverage,
    /// Per-category limits, sorted by category.
    pub category_limits: Vec<CategoryLimitStatus>,
    /// The 3-month average, offered when no monthly limit is set.
    pub suggested_monthly_limit: Option<Decimal>,
}

/// Compute the budget status for the UTC month containing `now`.
pub fn compute_budget_status(
    transactions: &[TransactionWithContext],
    config: &DataConfig,
    categorizer: &Categorizer,
    now: i64,
) -> BudgetStatus {
    let current = YearMonth::from_epoch(now);
    let day = DateTime::from_timestamp(now, 0).unwrap_or_default().day();
    let days = current.days();
    let pace = Decimal::from(day) / Decimal::from(days);
    let paced = |limit: Decimal| (limit * pace).round_dp(2);

    let flow = compute_cash_flow(
        transactions,
        config,
        categorizer,
        current.minus_months(12),
        current,
    );
    let this_month = flow
        .months
        .last()
        .cloned()
        .unwrap_or_else(|| empty_month(current));
    let complete: &[CashFlowMonth] = &flow.months[..flow.months.len() - 1];
    let spent = this_month.expense;

    let history_start = transactions
        .iter()
        .filter(|t| !t.pending && t.posted > 0)
        .map(|t| YearMonth::from_epoch(t.posted))
        .min();
    let average = |n: usize| average_spending(complete, n, history_start, spent, day, days);
    let average_3_month = average(3);
    let average_12_month = average(12);

    let budget = &config.budget;
    let monthly_limit = budget.monthly_limit;
    let suggested_monthly_limit = match monthly_limit {
        Some(_) => None,
        None => Some(average_3_month.amount).filter(|a| *a > Decimal::ZERO),
    };

    let mut category_limits: Vec<CategoryLimitStatus> = budget
        .category_limits
        .iter()
        .map(|(category, &limit)| {
            let spent = expense_in(&this_month, category);
            CategoryLimitStatus {
                category: category.clone(),
                label: category_label(category),
                limit,
                spent,
                remaining: limit - spent,
                over_limit: spent > limit,
                expected_spend: paced(limit),
            }
        })
        .collect();
    category_limits.sort_by(|a, b| a.category.cmp(&b.category));

    BudgetStatus {
        month: current.to_string(),
        day_of_month: day,
        days_in_month: days,
        spent,
        monthly_limit,
        remaining: monthly_limit.map(|l| l - spent),
        over_budget: monthly_limit.is_some_and(|l| spent > l),
        expected_spend: monthly_limit.map(paced),
        last_month_same_day: last_month_through_day(
            transactions,
            config,
            categorizer,
            current,
            day,
        ),
        top_categories: top_categories(&this_month),
        average_3_month,
        average_12_month,
        category_limits,
        suggested_monthly_limit,
    }
}

fn empty_month(month: YearMonth) -> CashFlowMonth {
    CashFlowMonth {
        month: month.to_string(),
        income: Decimal::ZERO,
        expense: Decimal::ZERO,
        net: Decimal::ZERO,
        categories: Vec::new(),
    }
}

fn expense_in(month: &CashFlowMonth, category: &str) -> Decimal {
    month
        .categories
        .iter()
        .find(|c| c.kind == CategoryKind::Expense && c.category == category)
        .map(|c| c.amount)
        .unwrap_or(Decimal::ZERO)
}

/// Average of the last `n` complete months, clipped to the first month of
/// history; with no complete month, project this month's pace.
fn average_spending(
    complete: &[CashFlowMonth],
    n: usize,
    history_start: Option<YearMonth>,
    spent: Decimal,
    day: u32,
    days: u32,
) -> SpendingAverage {
    let Some(history_start) = history_start else {
        return SpendingAverage {
            amount: Decimal::ZERO,
            months: 0,
            extrapolated: false,
        };
    };
    let window: Vec<&CashFlowMonth> = complete
        .iter()
        .rev()
        .take(n)
        .filter(|m| {
            m.month
                .parse::<YearMonth>()
                .is_ok_and(|ym| ym >= history_start)
        })
        .collect();
    if window.is_empty() {
        return SpendingAverage {
            amount: (spent / Decimal::from(day) * Decimal::from(days)).round_dp(2),
            months: 0,
            extrapolated: true,
        };
    }
    let total: Decimal = window.iter().map(|m| m.expense).sum();
    SpendingAverage {
        amount: (total / Decimal::from(window.len())).round_dp(2),
        months: window.len(),
        extrapolated: false,
    }
}

/// Last month's spending from its first day through `day` (clamped).
fn last_month_through_day(
    transactions: &[TransactionWithContext],
    config: &DataConfig,
    categorizer: &Categorizer,
    current: YearMonth,
    day: u32,
) -> Decimal {
    let last = current.prev();
    let cutoff = last.start_epoch() + i64::from(day.min(last.days())) * SECONDS_PER_DAY;
    transactions
        .iter()
        .filter(|t| last.contains(t.posted) && t.posted < cutoff)
        .filter_map(|t| cash_flow_entry(t, config, categorizer))
        .filter(|e| e.kind == CategoryKind::Expense)
        .map(|e| e.amount)
        .sum()
}

/// The top expense categories, then everything else (including spending
/// already categorized as "other") folded into one "other" bucket.
fn top_categories(month: &CashFlowMonth) -> Vec<CategorySpend> {
    let spend = |category: &str, amount: Decimal| CategorySpend {
        category: category.to_string(),
        label: category_label(category),
        amount,
    };
    let (other, named): (Vec<_>, Vec<_>) = month
        .categories
        .iter()
        .filter(|c| c.kind == CategoryKind::Expense)
        .partition(|c| c.category == OTHER_CATEGORY);

    let mut top: Vec<CategorySpend> = named
        .iter()
        .take(TOP_CATEGORY_COUNT)
        .map(|c| spend(&c.category, c.amount))
        .collect();
    let remainder: Vec<_> = named
        .iter()
        .skip(TOP_CATEGORY_COUNT)
        .chain(other.iter())
        .collect();
    if !remainder.is_empty() {
        top.push(spend(
            OTHER_CATEGORY,
            remainder.iter().map(|c| c.amount).sum(),
        ));
    }
    top
}
