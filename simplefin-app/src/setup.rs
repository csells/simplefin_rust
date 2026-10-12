//! First-run screen: connect a SimpleFIN Bridge account by pasting a setup
//! token (or an existing access URL).

use ely_gpui_component::{
    buttons::Button,
    forms::{Input, InputEvent, TextInput},
    layout::ScrollArea,
    primitives::{Icon, IconName},
    theme::{ActiveTheme, IconSize},
    typography::{Caption, ExternalLink, Paragraph, Title},
};
use gpui::{
    AppContext, Context, Entity, IntoElement, ParentElement, Render, Styled, Subscription, Window, div, px, App, rems,
};

use crate::store::FinanceStore;
use crate::ui;

/// Where users sign in to SimpleFIN Bridge and create setup tokens.
const SIMPLEFIN_BRIDGE_URL: &str = "https://bridge.simplefin.org/";
const SIMPLEFIN_SITE_URL: &str = "https://www.simplefin.org/";

pub struct SetupScreen {
    store: Entity<FinanceStore>,
    input: Entity<TextInput>,
    _subscriptions: Vec<Subscription>,
}

impl SetupScreen {
    pub fn new(store: Entity<FinanceStore>, window: &mut Window, cx: &mut Context<Self>) -> Self {
        let input = cx.new(|cx| {
            TextInput::new(window, cx).placeholder("Setup token or access URL")
        });
        let subscriptions = vec![
            cx.subscribe(&input, |this, _, event: &InputEvent, cx| {
                if *event == InputEvent::Submit {
                    this.connect(cx);
                }
            }),
            cx.observe(&store, |_, _, cx| cx.notify()),
        ];
        Self { store, input, _subscriptions: subscriptions }
    }

    /// Fills the field from the clipboard. Browsers only hand the clipboard over
    /// asynchronously (and may ask the user first), so always read it that way.
    fn paste(&mut self, cx: &mut Context<Self>) {
        let read = cx.read_from_clipboard_async();
        cx.spawn(async move |this, cx| {
            let text = match read.await {
                Ok(item) => item.and_then(|item| item.text()),
                Err(err) => {
                    log::warn!("setup: clipboard unavailable: {err:?}");
                    None
                }
            };
            let Some(text) = text.map(|t| t.trim().to_string()).filter(|t| !t.is_empty()) else {
                return;
            };
            this.update(cx, |this, cx| this.input.update(cx, |input, cx| input.set_text(text, cx)))
                .ok();
        })
        .detach();
    }

    fn connect(&mut self, cx: &mut Context<Self>) {
        let text = self.input.read(cx).text().trim().to_string();
        if text.is_empty() {
            return;
        }
        self.store.update(cx, |store, cx| store.connect(text, cx));
    }
}

impl Render for SetupScreen {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let colors = cx.theme().colors.clone();
        let store = self.store.read(cx);
        let busy = store.is_connecting();
        let error = store.setup_error().map(|e| e.to_string());
        let this = cx.entity();

        // Top-aligned and scrollable so the on-screen keyboard never hides Connect.
        ScrollArea::new("setup-scroll").vertical().size_full().child(
            div().flex().flex_col().items_center().p_6().pt_12().child(
            div()
                .w_full()
                .max_w(px(480.))
                .flex()
                .flex_col()
                .gap_4()
                .child(Icon::new(IconName::Wallet).size(IconSize::Xxl).color(colors.accent))
                .child(Title::new("Connect your accounts"))
                .child(Paragraph::new(
                    "SimpleFIN Bridge securely links your banks, cards and investments to this app.",
                ))
                .child(step(1, "Create a setup token at bridge.simplefin.org and copy it.", cx))
                .child(
                    Button::new("setup-get-token", "Get a setup token")
                        .trailing_icon(IconName::ArrowUpRight)
                        .full_width()
                        .on_click(|_, _, cx| ui::open_external(SIMPLEFIN_BRIDGE_URL, cx)),
                )
                .child(step(2, "Paste it here. An access URL you already have works too.", cx))
                .child(
                    div()
                        .flex()
                        .flex_row()
                        .gap_2()
                        .items_center()
                        .child(div().flex_1().child(Input::new(&self.input).clearable().invalid(error.is_some())))
                        .child({
                            let this = this.clone();
                            Button::new("setup-paste", "Paste")
                                .icon(IconName::Clipboard)
                                .on_click(move |_, _, cx| this.update(cx, |this, cx| this.paste(cx)))
                        }),
                )
                .children(error.map(|e| div().text_sm().text_color(colors.danger).child(e)))
                .child(
                    Button::new("setup-connect", "Connect")
                        .primary()
                        .full_width()
                        .loading(busy)
                        .disabled(busy)
                        .on_click(move |_, _, cx| this.update(cx, |this, cx| this.connect(cx))),
                )
                .child(Caption::new(
                    "Your access URL is stored only on this device, in the app's private storage.",
                ))
                .child(
                    div()
                        .flex()
                        .flex_row()
                        .flex_wrap()
                        .items_center()
                        .gap_1()
                        .text_sm()
                        .text_color(colors.fg_muted)
                        .child("New to SimpleFIN?")
                        .child(ExternalLink::new("setup-about", "Learn about SimpleFIN", SIMPLEFIN_SITE_URL)),
                ),
            ),
        )
    }
}

/// A numbered setup instruction.
fn step(number: u8, text: &'static str, cx: &App) -> impl IntoElement {
    let colors = &cx.theme().colors;
    div()
        .flex()
        .flex_row()
        .items_start()
        .gap_3()
        .child(
            div()
                .flex_none()
                .size(rems(1.5))
                .rounded_full()
                .bg(colors.accent)
                .text_color(colors.on_accent)
                .text_sm()
                .flex()
                .items_center()
                .justify_center()
                .child(number.to_string()),
        )
        // min_w_0 lets the text shrink below its single-line width and wrap.
        .child(div().flex_1().min_w_0().child(text))
}
