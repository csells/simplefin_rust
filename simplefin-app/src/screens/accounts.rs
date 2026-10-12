//! Accounts: every account grouped by type with group totals.

use ely_gpui_component::{
    data_display::{Badge, Tone},
    lists::{List, ListItem},
};
use gpui::{
    AnyElement, Context, IntoElement, ParentElement, Render, Styled, Subscription, Window, div,
};

use super::Ctx;
use crate::nav::Tab;
use crate::snapshot::AccountRow;
use crate::ui::{self, money};

pub struct Accounts {
    ctx: Ctx,
    _store: Subscription,
}

impl Accounts {
    pub fn new(ctx: Ctx, cx: &mut Context<Self>) -> Self {
        let store = cx.observe(&ctx.store, |_, _, cx| cx.notify());
        Self { ctx, _store: store }
    }
}

impl Render for Accounts {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let snapshot = self.ctx.store.read(cx).snapshot();
        let currency = snapshot.currency.clone();
        let body: AnyElement = if !snapshot.has_accounts() {
            ui::empty("No accounts yet.", cx).into_any_element()
        } else {
            let mut column = div().flex().flex_col().gap_4();
            if let Some(summary) = &snapshot.net_worth {
                column = column.child(
                    div()
                        .flex()
                        .flex_col()
                        .gap_1()
                        .child(ui::muted("Net worth", cx))
                        .child(ui::hero_amount(summary.net_worth, &currency)),
                );
            }
            for group in &snapshot.groups {
                let list = group.accounts.iter().fold(List::new().divided(), |list, account| {
                    list.child(account_item(&self.ctx, account))
                });
                column = column.child(ui::section_with_action(
                    group.label.clone(),
                    div().font_weight(gpui::FontWeight::SEMIBOLD).child(money(group.total, &currency)),
                    list,
                ));
            }
            column.into_any_element()
        };
        ui::page("accounts-scroll", "Accounts", None, body, cx)
    }
}

fn account_item(ctx: &Ctx, account: &AccountRow) -> ListItem {
    let ctx = ctx.clone();
    let id = account.id.clone();
    let mut badges = div().flex().flex_row().gap_1().items_center();
    if account.manual {
        badges = badges.child(Badge::new("Manual"));
    }
    if account.stale {
        badges = badges.child(Badge::new("Update").tone(Tone::Warning));
    }
    if account.excluded {
        badges = badges.child(Badge::new("Excluded"));
    }
    ListItem::new(format!("account-{}", account.id), account.name.clone())
        .description(account.org_name.clone())
        .trailing(
            div()
                .flex()
                .flex_row()
                .gap_2()
                .items_center()
                .child(badges)
                .child(money(account.balance, &account.currency)),
        )
        .on_click(move |_, window, cx| ctx.open_account(Tab::Accounts, id.clone(), window, cx))
}
