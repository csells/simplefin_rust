//! Activity: searchable, filterable transaction list.

use std::sync::Arc;

use ely_gpui_component::{
    buttons::{Button, ButtonVariant, SegmentedControl},
    forms::{InputEvent, SearchInput, TextInput},
    lists::{List, ListItem},
};
use gpui::{
    AnyElement, AppContext, Context, Entity, IntoElement, ParentElement, Render, SharedString,
    Styled, Subscription, Window, div,
};
use simplefin::{AccountCategory, TransactionQuery, TransactionView};

use super::Ctx;
use crate::nav::Tab;
use crate::snapshot::Snapshot;
use crate::ui::{self, signed_money};

/// Rows rendered per "Show more" step.
const PAGE: usize = 100;
const DAY: i64 = 86_400;

/// Date-range presets; Empower's default is 90 days.
const RANGES: [(&str, &str, Option<i64>); 4] = [
    ("30d", "30 days", Some(30)),
    ("90d", "90 days", Some(90)),
    ("1y", "1 year", Some(365)),
    ("all", "All", None),
];

const ACCOUNT_FILTERS: [(&str, &str, Option<AccountCategory>); 5] = [
    ("all", "All", None),
    ("cash", "Cash", Some(AccountCategory::Cash)),
    ("credit", "Credit", Some(AccountCategory::CreditCards)),
    ("invest", "Invest", Some(AccountCategory::Investments)),
    ("loans", "Loans", Some(AccountCategory::Loans)),
];

pub struct Transactions {
    ctx: Ctx,
    search: Entity<TextInput>,
    range: SharedString,
    accounts: SharedString,
    shown: usize,
    cache: Option<(usize, TransactionQuery, Arc<Vec<TransactionView>>)>,
    _subscriptions: Vec<Subscription>,
}

impl Transactions {
    pub fn new(ctx: Ctx, window: &mut Window, cx: &mut Context<Self>) -> Self {
        let search = cx.new(|cx| TextInput::new(window, cx).placeholder("Search activity"));
        let subscriptions = vec![
            cx.observe(&ctx.store, |_, _, cx| cx.notify()),
            cx.subscribe(&search, |this, _, event: &InputEvent, cx| {
                if *event == InputEvent::Changed {
                    this.shown = PAGE;
                    cx.notify();
                }
            }),
        ];
        Self {
            ctx,
            search,
            range: "90d".into(),
            accounts: "all".into(),
            shown: PAGE,
            cache: None,
            _subscriptions: subscriptions,
        }
    }

    fn query(&self, now: i64, cx: &gpui::App) -> TransactionQuery {
        let text = self.search.read(cx).text().trim();
        let days = RANGES.iter().find(|r| r.0 == self.range.as_ref()).and_then(|r| r.2);
        TransactionQuery {
            text: (!text.is_empty()).then(|| text.to_string()),
            account_category: ACCOUNT_FILTERS
                .iter()
                .find(|f| f.0 == self.accounts.as_ref())
                .and_then(|f| f.2),
            start_date: days.map(|d| now - d * DAY),
            include_hidden: true,
            ..Default::default()
        }
    }

    fn results(&mut self, snapshot: &Arc<Snapshot>, cx: &gpui::App) -> Arc<Vec<TransactionView>> {
        let key = Arc::as_ptr(snapshot) as usize;
        let query = self.query(snapshot.now, cx);
        if let Some((cached_key, cached_query, results)) = &self.cache
            && *cached_key == key
            && *cached_query == query
        {
            return results.clone();
        }
        let results = Arc::new(snapshot.query(&query));
        self.cache = Some((key, query, results.clone()));
        results
    }
}

impl Render for Transactions {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let snapshot = self.ctx.store.read(cx).snapshot();
        let results = self.results(&snapshot, cx);
        let this = cx.entity();

        let range = RANGES.iter().fold(SegmentedControl::new("txn-range", self.range.clone()), |c, r| {
            c.segment(r.0, r.1, None)
        });
        let range = {
            let this = this.clone();
            range.on_change(move |value, _, cx| {
                let value = value.clone();
                this.update(cx, |t, cx| {
                    t.range = value;
                    t.shown = PAGE;
                    cx.notify();
                })
            })
        };
        let accounts = ACCOUNT_FILTERS
            .iter()
            .fold(SegmentedControl::new("txn-accounts", self.accounts.clone()), |c, f| {
                c.segment(f.0, f.1, None)
            });
        let accounts = {
            let this = this.clone();
            accounts.on_change(move |value, _, cx| {
                let value = value.clone();
                this.update(cx, |t, cx| {
                    t.accounts = value;
                    t.shown = PAGE;
                    cx.notify();
                })
            })
        };

        let currency = snapshot.currency.clone();
        let ctx = self.ctx.clone();
        let list = results.iter().take(self.shown).fold(List::new().divided(), |list, view| {
            let txn = &view.transaction;
            let (account_id, id) = (txn.account_id.clone(), txn.id.clone());
            let ctx = ctx.clone();
            list.child(
                ListItem::new(format!("txn-{}-{}", txn.account_id, txn.id), view.description.clone())
                    .description(format!(
                        "{} · {} · {}{}",
                        ui::short_date(simplefin::transaction_date(txn)),
                        view.category_label,
                        txn.account_name,
                        if txn.pending { " · Pending" } else { "" }
                    ))
                    .struck(view.hidden)
                    .trailing(
                        div()
                            .text_color(ui::amount_color(txn.amount, cx))
                            .child(signed_money(txn.amount, &currency)),
                    )
                    .on_click(move |_, window, cx| {
                        ctx.open_transaction(Tab::Transactions, account_id.clone(), id.clone(), window, cx)
                    }),
            )
        });
        let more: Option<AnyElement> = (results.len() > self.shown).then(|| {
            let this = this.clone();
            Button::new("txn-more", format!("Show more ({} left)", results.len() - self.shown))
                .variant(ButtonVariant::Ghost)
                .full_width()
                .on_click(move |_, _, cx| {
                    this.update(cx, |t, cx| {
                        t.shown += PAGE;
                        cx.notify();
                    })
                })
                .into_any_element()
        });

        let body = div()
            .flex()
            .flex_col()
            .gap_3()
            .child(SearchInput::new("txn-search", &self.search))
            .child(accounts)
            .child(range)
            .child(ui::muted(format!("{} transactions", results.len()), cx))
            .child(if results.is_empty() {
                ui::empty("No matching transactions.", cx).into_any_element()
            } else {
                list.into_any_element()
            })
            .children(more);
        ui::page("txn-scroll", "Activity", None, body, cx)
    }
}
