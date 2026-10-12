//! Home: Empower-style dashboard of widgets, each linking into its tool.

use std::sync::Arc;

use ely_gpui_component::{
    buttons::IconButton,
    charts::{LineChart, Series},
    data_display::{Badge, Meter, Tone, TrendIndicator},
    lists::{List, ListItem},
    primitives::IconName,
    theme::ActiveTheme,
};
use gpui::{
    AnyElement, App, Context, IntoElement, ParentElement, Render, Styled, Subscription, Window,
    div, px,
};
use rust_decimal::Decimal;

use super::Ctx;
use crate::nav::Tab;
use crate::snapshot::Snapshot;
use crate::ui::{self, money, signed_money, to_f64};

/// How many recent transactions the Home tab previews.
const RECENT: usize = 5;
/// How many upcoming recurring charges the Home tab previews.
const UPCOMING: usize = 3;

pub struct Home {
    ctx: Ctx,
    _store: Subscription,
}

impl Home {
    pub fn new(ctx: Ctx, cx: &mut Context<Self>) -> Self {
        let store = cx.observe(&ctx.store, |_, _, cx| cx.notify());
        Self { ctx, _store: store }
    }
}

impl Render for Home {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let store = self.ctx.store.read(cx);
        let snapshot = store.snapshot();
        let syncing = store.is_syncing();
        let store_handle = self.ctx.store.clone();
        let refresh = IconButton::new("home-sync", IconName::RefreshCw)
            .disabled(syncing)
            .tooltip("Sync now")
            .on_click(move |_, _, cx| store_handle.update(cx, |s, cx| s.sync(cx)))
            .into_any_element();

        let body = if !snapshot.loaded {
            ui::empty("Loading…", cx).into_any_element()
        } else if !snapshot.has_accounts() {
            ui::empty(
                if syncing { "Syncing your accounts…" } else { "No accounts yet. Tap sync to fetch them." },
                cx,
            )
            .into_any_element()
        } else {
            div()
                .flex()
                .flex_col()
                .gap_4()
                .child(last_sync(&snapshot, syncing, cx))
                .child(net_worth_card(&snapshot, cx))
                .children(attention_card(&snapshot, cx))
                .children(budget_card(&snapshot, cx))
                .children(cash_flow_card(&snapshot, cx))
                .children(recurring_card(&snapshot, cx))
                .child(recent_card(&self.ctx, &snapshot, cx))
                .into_any_element()
        };
        ui::page("home-scroll", "Home", Some(refresh), body, cx)
    }
}

fn last_sync(snapshot: &Snapshot, syncing: bool, cx: &App) -> AnyElement {
    let text = match (syncing, snapshot.last_sync) {
        (true, _) => "Syncing…".to_string(),
        (false, Some(ts)) => format!("Updated {}", ui::ago(ts, snapshot.now)),
        (false, None) => "Never synced".to_string(),
    };
    ui::muted(text, cx).into_any_element()
}

fn change_indicator(change: &simplefin::NetWorthChange) -> Option<TrendIndicator> {
    (change.start_value != Decimal::ZERO)
        .then(|| TrendIndicator::new(to_f64(change.change) / to_f64(change.start_value.abs())))
}

fn net_worth_card(snapshot: &Snapshot, cx: &App) -> AnyElement {
    let Some(summary) = &snapshot.net_worth else { return div().into_any_element() };
    let currency = snapshot.currency.as_str();
    let mut chips = div().flex().flex_row().flex_wrap().gap_3().text_sm();
    // Changes need history; right after the first sync there is only one point.
    let has_history = snapshot.net_worth_history.len() >= 2;
    for (label, change) in [("1 day", &snapshot.change_1d), ("90 days", &snapshot.change_90d)] {
        if let Some(change) = change.as_ref().filter(|_| has_history) {
            chips = chips.child(
                div()
                    .flex()
                    .flex_row()
                    .gap_1()
                    .items_center()
                    .child(ui::muted(label, cx))
                    .child(signed_money(change.change, currency))
                    .children(change_indicator(change)),
            );
        }
    }
    let history = &snapshot.net_worth_history;
    let chart = (history.len() >= 2).then(|| {
        let labels: Vec<String> = history.iter().map(|p| ui::short_date(p.timestamp)).collect();
        let values: Vec<f64> = history.iter().map(|p| to_f64(p.net_worth)).collect();
        let code = snapshot.currency.clone();
        LineChart::new("home-net-worth", labels)
            .series(Series::new("Net worth", values))
            .format(move |v| ely_gpui_component::typography::format::currency(v, &code))
            .h(px(180.))
    });
    ui::section(
        "Net worth",
        div()
            .flex()
            .flex_col()
            .gap_2()
            .child(ui::hero_amount(summary.net_worth, currency))
            .child(
                div()
                    .flex()
                    .flex_row()
                    .gap_4()
                    .text_sm()
                    .child(ui::muted(format!("Assets {}", money(summary.total_assets, currency)), cx))
                    .child(ui::muted(
                        format!("Liabilities {}", money(summary.total_liabilities, currency)),
                        cx,
                    )),
            )
            .child(chips)
            .children(chart),
    )
    .into_any_element()
}

fn budget_card(snapshot: &Snapshot, cx: &App) -> Option<AnyElement> {
    let budget = snapshot.budget.as_ref()?;
    let currency = snapshot.currency.as_str();
    let limit = budget.monthly_limit.or(budget.suggested_monthly_limit)?;
    let share = if limit > Decimal::ZERO { (to_f64(budget.spent) / to_f64(limit)) as f32 } else { 0. };
    let pace = budget.expected_spend.map(|expected| {
        if budget.spent > expected {
            Badge::new("Ahead of pace").tone(Tone::Warning)
        } else {
            Badge::new("On track").tone(Tone::Success)
        }
    });
    let title = if budget.monthly_limit.is_some() { "Budget" } else { "Budget (suggested)" };
    Some(
        ui::section(
            title,
            div()
                .flex()
                .flex_col()
                .gap_2()
                .child(
                    Meter::new("home-budget", "Spent this month", share.clamp(0., 1.))
                        .detail(format!("{} of {}", money(budget.spent, currency), money(limit, currency)))
                        .thresholds(0.85, 1.0),
                )
                .child(
                    div()
                        .flex()
                        .flex_row()
                        .justify_between()
                        .items_center()
                        .child(ui::muted(
                            format!(
                                "Last month by day {}: {}",
                                budget.day_of_month,
                                money(budget.last_month_same_day, currency)
                            ),
                            cx,
                        ))
                        .children(pace),
                ),
        )
        .into_any_element(),
    )
}

fn cash_flow_card(snapshot: &Snapshot, cx: &App) -> Option<AnyElement> {
    let month = snapshot.cash_flow.as_ref()?.months.last()?;
    let currency = snapshot.currency.as_str();
    Some(
        ui::section(
            "Cash flow this month",
            div()
                .flex()
                .flex_col()
                .gap_2()
                .child(ui::row("Income", div().text_color(cx.theme().colors.success).child(money(month.income, currency)), cx))
                .child(ui::row("Spending", div().child(money(month.expense, currency)), cx))
                .child(ui::row(
                    "Net",
                    div()
                        .font_weight(gpui::FontWeight::SEMIBOLD)
                        .text_color(ui::amount_color(month.net, cx))
                        .child(signed_money(month.net, currency)),
                    cx,
                )),
        )
        .into_any_element(),
    )
}

fn recurring_card(snapshot: &Snapshot, cx: &App) -> Option<AnyElement> {
    let recurring = snapshot.recurring.as_ref()?;
    if recurring.recurring.is_empty() {
        return None;
    }
    let currency = snapshot.currency.as_str();
    let mut upcoming: Vec<_> = recurring
        .recurring
        .iter()
        .filter_map(|r| r.next_expected.filter(|ts| *ts >= snapshot.now - 86400).map(|ts| (ts, r)))
        .collect();
    upcoming.sort_by_key(|(ts, _)| *ts);
    let list = upcoming.into_iter().take(UPCOMING).fold(List::new().divided(), |list, (ts, r)| {
        list.child(
            ListItem::new(format!("home-recurring-{}", r.merchant), r.merchant.clone())
                .description(format!("{} · {}", r.frequency_label, ui::short_date(ts)))
                .trailing(money(r.average_amount.abs(), currency)),
        )
    });
    Some(
        ui::section(
            "Recurring",
            div()
                .flex()
                .flex_col()
                .gap_2()
                .child(ui::row(
                    "Estimated monthly",
                    div().font_weight(gpui::FontWeight::SEMIBOLD).child(money(recurring.estimated_monthly_total.abs(), currency)),
                    cx,
                ))
                .child(list),
        )
        .into_any_element(),
    )
}

fn attention_card(snapshot: &Snapshot, cx: &App) -> Option<AnyElement> {
    let anomalies = snapshot.warnings.as_ref().map(|w| w.anomalies.as_slice()).unwrap_or_default();
    if snapshot.stale.is_empty() && anomalies.is_empty() {
        return None;
    }
    let mut list = List::new().divided();
    for stale in &snapshot.stale {
        list = list.child(
            ListItem::new(format!("stale-{}", stale.id), stale.name.clone())
                .description(match stale.days_since_update {
                    Some(days) => format!("Balance not updated in {days} days"),
                    None => "Balance never recorded".to_string(),
                })
                .trailing(Badge::new("Update").tone(Tone::Warning)),
        );
    }
    for (ix, anomaly) in anomalies.iter().enumerate() {
        list = list.child(
            ListItem::new(format!("anomaly-{ix}"), anomaly.to_string())
                .trailing(Badge::new("Check").tone(Tone::Danger)),
        );
    }
    let _ = cx;
    Some(ui::section("Needs attention", list).into_any_element())
}

fn recent_card(ctx: &Ctx, snapshot: &Arc<Snapshot>, cx: &App) -> AnyElement {
    let currency = snapshot.currency.clone();
    let list = snapshot
        .transactions
        .iter()
        .filter(|t| !t.hidden)
        .take(RECENT)
        .fold(List::new().divided(), |list, view| {
            let txn = &view.transaction;
            let (account_id, id) = (txn.account_id.clone(), txn.id.clone());
            let ctx = ctx.clone();
            list.child(
                ListItem::new(format!("recent-{}-{}", txn.account_id, txn.id), view.description.clone())
                    .description(format!("{} · {}", view.category_label, ui::short_date(simplefin::transaction_date(txn))))
                    .trailing(
                        div()
                            .text_color(ui::amount_color(txn.amount, cx))
                            .child(signed_money(txn.amount, &currency)),
                    )
                    .on_click(move |_, window, cx| {
                        ctx.open_transaction(Tab::Home, account_id.clone(), id.clone(), window, cx)
                    }),
            )
        });
    ui::section("Recent activity", list).into_any_element()
}
