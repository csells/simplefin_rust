//! More: sync, appearance, connection, recent problems, about.

use ely_gpui_component::{
    buttons::{Button, ButtonVariant},
    forms::Switch,
    lists::{List, ListItem},
    theme::{ActiveTheme, Mode, Theme},
};
use gpui::{
    Context, IntoElement, ParentElement, Render, Styled, Subscription, Window, div,
};

use super::Ctx;
use crate::ui;

/// Most recent error-log entries shown.
const RECENT_ERRORS: usize = 10;

pub struct More {
    ctx: Ctx,
    confirm_disconnect: bool,
    _store: Subscription,
}

impl More {
    pub fn new(ctx: Ctx, cx: &mut Context<Self>) -> Self {
        let store = cx.observe(&ctx.store, |_, _, cx| cx.notify());
        Self { ctx, confirm_disconnect: false, _store: store }
    }
}

impl Render for More {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let store = self.ctx.store.read(cx);
        let snapshot = store.snapshot();
        let syncing = store.is_syncing();
        let location = store.location().to_string();
        let mut errors = store.errors().entries();
        errors.reverse();
        let this = cx.entity();

        let sync = {
            let store = self.ctx.store.clone();
            Button::new("more-sync", if syncing { "Syncing…" } else { "Sync now" })
                .primary()
                .full_width()
                .loading(syncing)
                .disabled(syncing)
                .on_click(move |_, _, cx| store.update(cx, |s, cx| s.sync(cx)))
        };
        let last = match snapshot.last_sync {
            Some(ts) => format!("Last synced {}", ui::ago(ts, snapshot.now)),
            None => "Not synced yet".to_string(),
        };

        let dark = cx.theme().is_dark();
        let appearance = Switch::new("more-dark", dark)
            .label("Dark mode")
            .on_change(|on, _, cx| Theme::set_mode(if on { Mode::Dark } else { Mode::Light }, cx));

        let disconnect = {
            let this = this.clone();
            let store = self.ctx.store.clone();
            let confirming = self.confirm_disconnect;
            Button::new(
                "more-disconnect",
                if confirming { "Tap again to disconnect" } else { "Disconnect SimpleFIN" },
            )
            .variant(ButtonVariant::Danger)
            .full_width()
            .on_click(move |_, _, cx| {
                if confirming {
                    store.update(cx, |s, cx| s.disconnect(cx));
                }
                this.update(cx, |more, cx| {
                    more.confirm_disconnect = !confirming;
                    cx.notify();
                });
            })
        };

        let problems = if errors.is_empty() {
            ui::empty("No problems recorded.", cx).into_any_element()
        } else {
            errors
                .iter()
                .take(RECENT_ERRORS)
                .enumerate()
                .fold(List::new().divided(), |list, (ix, entry)| {
                    list.child(
                        ListItem::new(format!("error-{ix}"), entry.context.clone())
                            .description(entry.message.clone())
                            .detail(ui::ago(entry.timestamp, snapshot.now)),
                    )
                })
                .into_any_element()
        };

        let body = div()
            .flex()
            .flex_col()
            .gap_4()
            .child(ui::section(
                "Sync",
                div().flex().flex_col().gap_2().child(sync).child(ui::muted(last, cx)),
            ))
            .child(ui::section("Appearance", appearance))
            .child(ui::section(
                "Connection",
                div()
                    .flex()
                    .flex_col()
                    .gap_2()
                    .child(ui::muted(
                        "Disconnecting forgets the access URL on this device. Your synced data stays.",
                        cx,
                    ))
                    .child(disconnect),
            ))
            .child(ui::section("Recent problems", problems))
            .child(ui::section(
                "About",
                div()
                    .flex()
                    .flex_col()
                    .gap_2()
                    .child(ui::row("Version", div().child(env!("CARGO_PKG_VERSION")), cx))
                    .child(ui::muted(format!("Data: {location}"), cx)),
            ));
        ui::page("more-scroll", "More", None, body, cx)
    }
}
