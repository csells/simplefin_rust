//! The five tab screens plus pushed detail pages.

mod account_detail;
mod accounts;
mod home;
mod more;
mod plan;
mod transaction_detail;
mod transactions;

use std::collections::HashMap;

use gpui::{AnyView, App, AppContext, Context, Entity, Window};

use crate::nav::Tab;
use crate::store::FinanceStore;

/// Per-tab stacks of pushed detail pages (account, transaction).
#[derive(Default)]
pub struct Navigator {
    stacks: HashMap<Tab, Vec<AnyView>>,
}

impl Navigator {
    pub fn push(&mut self, tab: Tab, view: AnyView, cx: &mut Context<Self>) {
        self.stacks.entry(tab).or_default().push(view);
        cx.notify();
    }

    pub fn pop(&mut self, tab: Tab, cx: &mut Context<Self>) {
        if let Some(stack) = self.stacks.get_mut(&tab) {
            stack.pop();
        }
        cx.notify();
    }

    pub fn pop_to_root(&mut self, tab: Tab, cx: &mut Context<Self>) {
        self.stacks.remove(&tab);
        cx.notify();
    }

    fn top(&self, tab: Tab) -> Option<AnyView> {
        self.stacks.get(&tab).and_then(|s| s.last().cloned())
    }
}

/// What a screen needs to open detail pages.
#[derive(Clone)]
pub struct Ctx {
    pub store: Entity<FinanceStore>,
    pub nav: Entity<Navigator>,
}

impl Ctx {
    pub fn open_account(&self, tab: Tab, account_id: String, window: &mut Window, cx: &mut App) {
        let view = account_detail::AccountDetail::open(self.clone(), tab, account_id, window, cx);
        self.nav.update(cx, |nav, cx| nav.push(tab, view.into(), cx));
    }

    pub fn open_transaction(
        &self,
        tab: Tab,
        account_id: String,
        transaction_id: String,
        window: &mut Window,
        cx: &mut App,
    ) {
        let view = transaction_detail::TransactionDetail::open(
            self.clone(),
            tab,
            account_id,
            transaction_id,
            window,
            cx,
        );
        self.nav.update(cx, |nav, cx| nav.push(tab, view.into(), cx));
    }

    pub fn back(&self, tab: Tab, cx: &mut App) {
        self.nav.update(cx, |nav, cx| nav.pop(tab, cx));
    }
}

pub struct Screens {
    ctx: Ctx,
    home: AnyView,
    accounts: AnyView,
    transactions: AnyView,
    plan: AnyView,
    more: AnyView,
}

impl Screens {
    pub fn new<T>(store: &Entity<FinanceStore>, window: &mut Window, cx: &mut Context<T>) -> Self {
        let ctx = Ctx { store: store.clone(), nav: cx.new(|_| Navigator::default()) };
        Self {
            home: cx.new(|cx| home::Home::new(ctx.clone(), cx)).into(),
            accounts: cx.new(|cx| accounts::Accounts::new(ctx.clone(), cx)).into(),
            transactions: cx
                .new(|cx| transactions::Transactions::new(ctx.clone(), window, cx))
                .into(),
            plan: cx.new(|cx| plan::Plan::new(ctx.clone(), cx)).into(),
            more: cx.new(|cx| more::More::new(ctx.clone(), cx)).into(),
            ctx,
        }
    }

    pub fn navigator(&self) -> &Entity<Navigator> {
        &self.ctx.nav
    }

    pub fn view(&self, tab: Tab, cx: &App) -> AnyView {
        self.ctx.nav.read(cx).top(tab).unwrap_or_else(|| match tab {
            Tab::Home => self.home.clone(),
            Tab::Accounts => self.accounts.clone(),
            Tab::Transactions => self.transactions.clone(),
            Tab::Plan => self.plan.clone(),
            Tab::More => self.more.clone(),
        })
    }

    pub fn pop_to_root<T>(&self, tab: Tab, cx: &mut Context<T>) {
        self.ctx.nav.update(cx, |nav, cx| nav.pop_to_root(tab, cx));
    }
}
