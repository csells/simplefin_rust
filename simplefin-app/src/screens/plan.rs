//! Plan: budget, cash flow and recurring expenses.

use ely_gpui_component::{
    buttons::{Button, ButtonVariant, SegmentedControl},
    charts::{BarChart, PieChart, Series},
    data_display::{Badge, Meter, Tone},
    forms::NumberInput,
    lists::{List, ListItem},
    typography::format,
};
use gpui::{
    AnyElement, App, Context, Entity, IntoElement, ParentElement, Render, SharedString, Styled,
    Subscription, Window, div, px,
};
use rust_decimal::Decimal;
use rust_decimal::prelude::FromPrimitive;

use super::Ctx;
use crate::actions;
use crate::store::FinanceStore;
use crate::snapshot::Snapshot;
use crate::ui::{self, money, signed_money, to_f64};

pub struct Plan {
    ctx: Ctx,
    view: SharedString,
    _store: Subscription,
}

impl Plan {
    pub fn new(ctx: Ctx, cx: &mut Context<Self>) -> Self {
        let store = cx.observe(&ctx.store, |_, _, cx| cx.notify());
        Self { ctx, view: "budget".into(), _store: store }
    }

}

fn set_limit(store: &Entity<FinanceStore>, limit: Option<Decimal>, cx: &mut App) {
    store.update(cx, |store, cx| {
        store.mutate("update budget", |storage| actions::set_monthly_budget(storage, limit), cx)
    });
}

impl Render for Plan {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let snapshot = self.ctx.store.read(cx).snapshot();
        let this = cx.entity();
        let picker = SegmentedControl::new("plan-view", self.view.clone())
            .segment("budget", "Budget", None)
            .segment("cashflow", "Cash flow", None)
            .segment("recurring", "Recurring", None)
            .on_change({
                let this = this.clone();
                move |value, _, cx| {
                    let value = value.clone();
                    this.update(cx, |plan, cx| {
                        plan.view = value;
                        cx.notify();
                    })
                }
            });
        let content = match self.view.as_ref() {
            "cashflow" => cash_flow(&snapshot, cx),
            "recurring" => recurring(&snapshot, cx),
            _ => budget(&self.ctx.store, &snapshot, cx),
        };
        let body = div().flex().flex_col().gap_4().child(picker).child(content);
        ui::page("plan-scroll", "Plan", None, body, cx)
    }
}

fn budget(store: &Entity<FinanceStore>, snapshot: &Snapshot, cx: &App) -> AnyElement {
    let Some(budget) = &snapshot.budget else {
        return ui::empty("Sync some transactions to start budgeting.", cx).into_any_element();
    };
    let currency = snapshot.currency.clone();
    let limit_value = budget.monthly_limit.map(to_f64).unwrap_or(0.);
    let limit_input = NumberInput::new("budget-limit", limit_value)
        .currency(&currency)
        .range(0., 10_000_000.)
        .blank("Set a monthly limit")
        .on_commit({
            let store = store.clone();
            move |value, _, cx| {
                let limit = (value > 0.).then(|| Decimal::from_f64(value)).flatten().map(|d| d.round_dp(2));
                set_limit(&store, limit, cx);
            }
        });
    let suggestion = (budget.monthly_limit.is_none())
        .then_some(budget.suggested_monthly_limit)
        .flatten()
        .map(|suggested| {
            let store = store.clone();
            Button::new("budget-suggest", format!("Use suggested {}", money(suggested, &currency)))
                .variant(ButtonVariant::Outline)
                .on_click(move |_, _, cx| set_limit(&store, Some(suggested), cx))
        });

    let limit = budget.monthly_limit.or(budget.suggested_monthly_limit);
    let meter = limit.map(|limit| {
        let share = if limit > Decimal::ZERO { (to_f64(budget.spent) / to_f64(limit)) as f32 } else { 0. };
        Meter::new(
            "budget-meter",
            format!("Spent in {}", ui::month_label(&budget.month, ui::current_year(snapshot.now))),
            share.clamp(0., 1.),
        )
            .detail(format!("{} of {}", money(budget.spent, &currency), money(limit, &currency)))
            .thresholds(0.85, 1.0)
    });
    let status = match (budget.over_budget, budget.expected_spend) {
        (true, _) => Some(Badge::new("Over budget").tone(Tone::Danger)),
        (false, Some(expected)) if budget.spent > expected => Some(Badge::new("Ahead of pace").tone(Tone::Warning)),
        (false, Some(_)) => Some(Badge::new("On track").tone(Tone::Success)),
        _ => None,
    };

    let mut facts = div().flex().flex_col().gap_2();
    if let Some(remaining) = budget.remaining {
        facts = facts.child(ui::row("Remaining", div().child(money(remaining, &currency)), cx));
    }
    if let Some(expected) = budget.expected_spend {
        facts = facts.child(ui::row(
            format!("Expected by day {} of {}", budget.day_of_month, budget.days_in_month),
            div().child(money(expected, &currency)),
            cx,
        ));
    }
    facts = facts
        .child(ui::row(
            format!("Last month by day {}", budget.day_of_month),
            div().child(money(budget.last_month_same_day, &currency)),
            cx,
        ))
        .child(ui::row(
            average_label("3-month average", &budget.average_3_month),
            div().child(money(budget.average_3_month.amount, &currency)),
            cx,
        ))
        .child(ui::row(
            average_label("12-month average", &budget.average_12_month),
            div().child(money(budget.average_12_month.amount, &currency)),
            cx,
        ));

    let donut = (!budget.top_categories.is_empty()).then(|| {
        budget
            .top_categories
            .iter()
            .fold(PieChart::new("budget-categories"), |pie, c| pie.slice(c.label.clone(), to_f64(c.amount).max(0.)))
            .donut(money(budget.spent, &currency), "This month")
            .h(px(240.))
    });
    let categories = budget.top_categories.iter().fold(List::new().divided(), |list, c| {
        list.child(ListItem::new(format!("budget-cat-{}", c.category), c.label.clone()).trailing(money(c.amount, &currency)))
    });

    div()
        .flex()
        .flex_col()
        .gap_4()
        .child(ui::section(
            "Monthly budget",
            div()
                .flex()
                .flex_col()
                .gap_3()
                .child(limit_input)
                .children(suggestion)
                .children(meter)
                .children(status),
        ))
        .child(ui::section("This month", facts))
        .child(ui::section("Top categories", div().flex().flex_col().gap_3().children(donut).child(categories)))
        .into_any_element()
}

fn average_label(label: &str, average: &simplefin::SpendingAverage) -> String {
    if average.extrapolated { format!("{label} (projected)") } else { label.to_string() }
}

fn cash_flow(snapshot: &Snapshot, cx: &App) -> AnyElement {
    let Some(flow) = &snapshot.cash_flow else {
        return ui::empty("Sync some transactions to see cash flow.", cx).into_any_element();
    };
    let currency = snapshot.currency.clone();
    let year = ui::current_year(snapshot.now);
    let labels: Vec<String> = flow.months.iter().map(|m| ui::month_label(&m.month.to_string(), year)).collect();
    let code = currency.clone();
    let chart = BarChart::new("cashflow-chart", labels)
        .series(Series::new("Income", flow.months.iter().map(|m| to_f64(m.income))))
        .series(Series::new("Spending", flow.months.iter().map(|m| to_f64(m.expense))))
        .format(move |v| format::currency(v, &code))
        .h(px(240.));
    let months = flow.months.iter().rev().fold(List::new().divided(), |list, m| {
        list.child(
            ListItem::new(format!("cashflow-{}", m.month), ui::month_label(&m.month.to_string(), year))
                .description(format!("In {} · Out {}", money(m.income, &currency), money(m.expense, &currency)))
                .trailing(div().text_color(ui::amount_color(m.net, cx)).child(signed_money(m.net, &currency))),
        )
    });
    let current = flow.months.last();
    let breakdown = current.map(|m| {
        let mut categories: Vec<_> = m.categories.iter().collect();
        categories.sort_by_key(|c| std::cmp::Reverse(c.amount.abs()));
        categories.into_iter().fold(List::new().divided(), |list, c| {
            list.child(
                ListItem::new(format!("cashflow-cat-{}", c.category), c.label.clone())
                    .description(format!("{} transactions", c.transaction_count))
                    .trailing(money(c.amount, &currency)),
            )
        })
    });
    div()
        .flex()
        .flex_col()
        .gap_4()
        .child(ui::section(
            "Income vs spending",
            div()
                .flex()
                .flex_col()
                .gap_2()
                .child(ui::row("Total income", div().child(money(flow.total_income, &currency)), cx))
                .child(ui::row("Total spending", div().child(money(flow.total_expense, &currency)), cx))
                .child(ui::row("Net", div().text_color(ui::amount_color(flow.net, cx)).child(signed_money(flow.net, &currency)), cx))
                .child(chart),
        ))
        .child(ui::section("By month", months))
        .children(breakdown.map(|b| ui::section("This month by category", b)))
        .into_any_element()
}

fn recurring(snapshot: &Snapshot, cx: &App) -> AnyElement {
    let Some(summary) = snapshot.recurring.as_ref().filter(|r| !r.recurring.is_empty()) else {
        return ui::empty("No recurring charges detected yet.", cx).into_any_element();
    };
    let currency = snapshot.currency.clone();
    let list = summary.recurring.iter().fold(List::new().divided(), |list, r| {
        let next = r.next_expected.map(|ts| format!(" · next {}", ui::short_date(ts))).unwrap_or_default();
        list.child(
            ListItem::new(format!("recurring-{}", r.merchant), r.merchant.clone())
                .description(format!("{} · {}{}", r.frequency_label, simplefin::category_label(&r.category), next))
                .trailing(money(r.average_amount.abs(), &currency)),
        )
    });
    div()
        .flex()
        .flex_col()
        .gap_4()
        .child(ui::section(
            "Subscriptions and bills",
            div()
                .flex()
                .flex_col()
                .gap_2()
                .child(ui::row(
                    "Estimated monthly",
                    div().font_weight(gpui::FontWeight::SEMIBOLD).child(money(summary.estimated_monthly_total.abs(), &currency)),
                    cx,
                ))
                .child(list),
        ))
        .into_any_element()
}
