//! One transaction: details plus Empower's edit flow (rename, recategorize
//! with an optional "apply to future" rule, hide duplicate).

use ely_gpui_component::{
    buttons::{Button, back_button},
    forms::{Choice, Input, InputEvent, Select, Switch, TextInput},
};
use gpui::{
    App, AppContext, Context, Entity, IntoElement, ParentElement, Render, Styled, Subscription,
    Window, div,
};
use simplefin::category_label;

use super::Ctx;
use crate::actions;
use crate::nav::Tab;
use crate::ui::{self, signed_money};

pub struct TransactionDetail {
    ctx: Ctx,
    tab: Tab,
    account_id: String,
    transaction_id: String,
    rename: Entity<TextInput>,
    apply_to_future: bool,
    _subscriptions: Vec<Subscription>,
}

impl TransactionDetail {
    pub fn open(
        ctx: Ctx,
        tab: Tab,
        account_id: String,
        transaction_id: String,
        window: &mut Window,
        cx: &mut App,
    ) -> Entity<Self> {
        let current = ctx
            .store
            .read(cx)
            .snapshot()
            .transaction(&account_id, &transaction_id)
            .map(|t| t.description.clone())
            .unwrap_or_default();
        cx.new(|cx: &mut Context<Self>| {
            let rename = cx.new(|cx| {
                let mut input = TextInput::new(window, cx).placeholder("Description");
                input.set_text(current, cx);
                input
            });
            let subscriptions = vec![
                cx.observe(&ctx.store, |_, _, cx| cx.notify()),
                cx.subscribe(&rename, |this, _, event: &InputEvent, cx| {
                    if *event == InputEvent::Submit {
                        this.save_description(cx);
                    }
                }),
            ];
            Self {
                ctx,
                tab,
                account_id,
                transaction_id,
                rename,
                apply_to_future: true,
                _subscriptions: subscriptions,
            }
        })
    }

    fn save_description(&mut self, cx: &mut Context<Self>) {
        let text = self.rename.read(cx).text().trim().to_string();
        let (account_id, id) = (self.account_id.clone(), self.transaction_id.clone());
        self.ctx.store.update(cx, |store, cx| {
            store.mutate(
                "rename transaction",
                |storage| {
                    actions::edit_transaction(storage, &account_id, &id, |edit| {
                        edit.description = (!text.is_empty()).then_some(text);
                    })
                },
                cx,
            )
        });
    }

    fn set_category(&mut self, category: String, cx: &mut Context<Self>) {
        let snapshot = self.ctx.store.read(cx).snapshot();
        let Some(view) = snapshot.transaction(&self.account_id, &self.transaction_id) else { return };
        let txn = view.transaction.clone();
        let apply = self.apply_to_future;
        self.ctx.store.update(cx, |store, cx| {
            store.mutate("recategorize", |storage| actions::recategorize(storage, &txn, &category, apply), cx)
        });
    }

    fn set_hidden(&mut self, hidden: bool, cx: &mut Context<Self>) {
        let (account_id, id) = (self.account_id.clone(), self.transaction_id.clone());
        self.ctx.store.update(cx, |store, cx| {
            store.mutate(
                "hide transaction",
                |storage| actions::edit_transaction(storage, &account_id, &id, |edit| edit.hidden = hidden),
                cx,
            )
        });
    }
}

impl Render for TransactionDetail {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let snapshot = self.ctx.store.read(cx).snapshot();
        let (ctx, tab) = (self.ctx.clone(), self.tab);
        let back = back_button("txn-back").on_click(move |_, _, cx| ctx.back(tab, cx)).into_any_element();
        let Some(view) = snapshot.transaction(&self.account_id, &self.transaction_id) else {
            return ui::page("txn-detail", "Transaction", Some(back), ui::empty("This transaction no longer exists.", cx), cx);
        };
        let txn = &view.transaction;
        let this = cx.entity();

        let facts = div()
            .flex()
            .flex_col()
            .gap_2()
            .child(
                div()
                    .text_3xl()
                    .font_weight(gpui::FontWeight::SEMIBOLD)
                    .text_color(ui::amount_color(txn.amount, cx))
                    .child(signed_money(txn.amount, &txn.currency)),
            )
            .child(ui::row("Date", div().child(ui::long_date(simplefin::transaction_date(txn))), cx))
            .child(ui::row("Account", div().child(txn.account_name.clone()), cx))
            .child(ui::row("Institution", div().child(txn.org_name.clone()), cx))
            .child(ui::row("Status", div().child(if txn.pending { "Pending" } else { "Posted" }), cx))
            .children(txn.payee.clone().map(|p| ui::row("Payee", div().child(p), cx)))
            .children(txn.memo.clone().map(|m| ui::row("Memo", div().child(m), cx)))
            .child(ui::row("Bank description", div().child(txn.description.clone()), cx));

        let mut categories = snapshot.categories.clone();
        if !categories.contains(&view.category) {
            categories.push(view.category.clone());
        }
        let choices: Vec<Choice> = categories.iter().map(|c| Choice::new(c.clone(), category_label(c))).collect();
        let category = {
            let this = this.clone();
            Select::new("txn-category", choices)
                .selected(view.category.clone())
                .on_change(move |value, _, cx| {
                    let value = value.to_string();
                    this.update(cx, |t, cx| t.set_category(value, cx))
                })
        };
        let future = {
            let this = this.clone();
            Switch::new("txn-future", self.apply_to_future)
                .label("Apply to all transactions from this merchant")
                .on_change(move |on, _, cx| {
                    this.update(cx, |t, cx| {
                        t.apply_to_future = on;
                        cx.notify();
                    })
                })
        };
        let rename_save = {
            let this = this.clone();
            Button::new("txn-rename", "Save name").on_click(move |_, _, cx| {
                this.update(cx, |t, cx| t.save_description(cx))
            })
        };
        let hidden = {
            let this = this.clone();
            Switch::new("txn-hidden", view.hidden)
                .label("Hide this duplicate from reports")
                .on_change(move |on, _, cx| this.update(cx, |t, cx| t.set_hidden(on, cx)))
        };

        let edit = div()
            .flex()
            .flex_col()
            .gap_3()
            .child(ui::muted("Name", cx))
            .child(
                div()
                    .flex()
                    .flex_row()
                    .gap_2()
                    .items_center()
                    .child(div().flex_1().child(Input::new(&self.rename).clearable()))
                    .child(rename_save),
            )
            .child(ui::muted("Category", cx))
            .child(category)
            .child(future)
            .child(hidden);

        let body = div()
            .flex()
            .flex_col()
            .gap_4()
            .child(ui::section("Details", facts))
            .child(ui::section("Edit", edit));
        ui::page("txn-detail", view.description.clone(), Some(back), body, cx)
    }
}
