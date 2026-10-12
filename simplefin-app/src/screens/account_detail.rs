//! One account: balances, balance history and its transactions.

use ely_gpui_component::{
    buttons::back_button,
    charts::{AreaChart, Series},
    lists::{List, ListItem},
    typography::format,
};
use gpui::{
    AnyElement, App, AppContext, Context, Entity, IntoElement, ParentElement, Render, Styled,
    Subscription, Window, div, px,
};

use super::Ctx;
use crate::nav::Tab;
use crate::ui::{self, money, signed_money, to_f64};

/// Most recent transactions listed on the detail page.
const MAX_TRANSACTIONS: usize = 100;

pub struct AccountDetail {
    ctx: Ctx,
    tab: Tab,
    account_id: String,
    _store: Subscription,
}

impl AccountDetail {
    pub fn open(ctx: Ctx, tab: Tab, account_id: String, _: &mut Window, cx: &mut App) -> Entity<Self> {
        cx.new(|cx| {
            let store = cx.observe(&ctx.store, |_, _, cx| cx.notify());
            Self { ctx, tab, account_id, _store: store }
        })
    }
}

impl Render for AccountDetail {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let snapshot = self.ctx.store.read(cx).snapshot();
        let (ctx, tab) = (self.ctx.clone(), self.tab);
        let back = back_button("account-back")
            .on_click(move |_, _, cx| ctx.back(tab, cx))
            .into_any_element();
        let Some(account) = snapshot.account(&self.account_id) else {
            return ui::page("account-scroll", "Account", Some(back), ui::empty("This account no longer exists.", cx), cx);
        };
        let currency = account.currency.clone();

        let mut balances = div()
            .flex()
            .flex_col()
            .gap_2()
            .child(ui::hero_amount(account.balance, &currency))
            .child(ui::muted(account.org_name.clone(), cx));
        if let Some(available) = account.available_balance {
            balances = balances.child(ui::row("Available", div().child(money(available, &currency)), cx));
        }
        if let Some(date) = account.balance_date {
            balances = balances.child(ui::row("As of", div().child(ui::long_date(date)), cx));
        }
        balances = balances.child(ui::row("Type", div().child(account.category.to_string()), cx));

        let history = snapshot.account_history(&self.account_id);
        let chart: Option<AnyElement> = (history.len() >= 2).then(|| {
            let labels: Vec<String> = history.iter().map(|s| ui::short_date(s.timestamp)).collect();
            let values: Vec<f64> = history.iter().map(|s| to_f64(s.balance)).collect();
            let code = currency.clone();
            ui::section(
                "Balance history",
                AreaChart::new("account-history", labels)
                    .series(Series::new("Balance", values))
                    .format(move |v| format::currency(v, &code))
                    .h(px(200.)),
            )
            .into_any_element()
        });

        let ctx = self.ctx.clone();
        let list = snapshot
            .transactions
            .iter()
            .filter(|t| t.transaction.account_id == self.account_id)
            .take(MAX_TRANSACTIONS)
            .fold(List::new().divided(), |list, view| {
                let txn = &view.transaction;
                let (account_id, id) = (txn.account_id.clone(), txn.id.clone());
                let ctx = ctx.clone();
                list.child(
                    ListItem::new(format!("acct-txn-{}", txn.id), view.description.clone())
                        .description(format!(
                            "{} · {}{}",
                            view.category_label,
                            ui::short_date(simplefin::transaction_date(txn)),
                            if txn.pending { " · Pending" } else { "" }
                        ))
                        .struck(view.hidden)
                        .trailing(
                            div()
                                .text_color(ui::amount_color(txn.amount, cx))
                                .child(signed_money(txn.amount, &currency)),
                        )
                        .on_click(move |_, window, cx| {
                            ctx.open_transaction(tab, account_id.clone(), id.clone(), window, cx)
                        }),
                )
            });

        let body = div()
            .flex()
            .flex_col()
            .gap_4()
            .child(ui::section("Balance", balances))
            .children(chart)
            .child(ui::section("Transactions", list));
        ui::page("account-scroll", account.name.clone(), Some(back), body, cx)
    }
}
