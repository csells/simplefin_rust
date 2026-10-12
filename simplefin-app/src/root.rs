//! The window's root view: setup until connected, then the tabbed app.

use std::rc::Rc;

use ely_gpui_component::{
    feedback::{Toast, Toaster, ToastViewport},
    primitives::FocusScope,
    theme::ActiveTheme,
};
use gpui::{
    AnyView, App, AppContext, Bounds, Context, Entity, FocusHandle, IntoElement, ParentElement,
    Render, Styled, Subscription, Window, WindowBounds, WindowOptions, div, px, size,
};

use crate::nav::{OnSelect, Tab, bottom_bar, side_bar};
use crate::env::AppEnv;
use crate::screens::Screens;
use crate::setup::SetupScreen;
use crate::store::{FinanceStore, Notice};

/// Below this width the app uses phone layout (bottom bar, single column).
const NARROW_WIDTH: f32 = 720.;

/// Ely sizes everything in rems at desktop density. Narrow (phone) layouts get a
/// larger rem so text is readable and controls make comfortable touch targets.
const NARROW_REM: f32 = 20.;
const WIDE_REM: f32 = 16.;

/// Initializes Ely, the data store and the app's single window.
pub fn open_main_window(cx: &mut App, env: AppEnv) -> anyhow::Result<()> {
    ely_gpui_component::init(cx)?;
    let store = cx.new(|cx| FinanceStore::new(env, cx));
    let options = WindowOptions {
        window_bounds: desktop_bounds(cx),
        ..Default::default()
    };
    cx.open_window(options, |window, cx| cx.new(|cx| Root::new(store, window, cx)))?;
    cx.activate(true);
    Ok(())
}

fn desktop_bounds(cx: &App) -> Option<WindowBounds> {
    if cfg!(any(target_os = "android", target_os = "ios", target_arch = "wasm32")) {
        return None;
    }
    Some(WindowBounds::Windowed(Bounds::centered(None, size(px(1100.), px(780.)), cx)))
}

struct Root {
    focus: FocusHandle,
    store: Entity<FinanceStore>,
    tab: Tab,
    setup: Entity<SetupScreen>,
    screens: Screens,
    toaster: Entity<Toaster>,
    _subscriptions: Vec<Subscription>,
}

impl Root {
    fn new(store: Entity<FinanceStore>, window: &mut Window, cx: &mut Context<Self>) -> Self {
        let focus = cx.focus_handle();
        window.focus(&focus, cx);
        let toaster = cx.new(|_| Toaster::default());
        let setup = cx.new(|cx| SetupScreen::new(store.clone(), window, cx));
        let screens = Screens::new(&store, window, cx);
        let toasts = toaster.clone();
        let subscriptions = vec![
            cx.observe(&store, |_, _, cx| cx.notify()),
            cx.observe(screens.navigator(), |_, _, cx| cx.notify()),
            cx.subscribe(&store, move |_, _, notice: &Notice, cx| {
                let mut toast = Toast::new(notice.title.clone()).severity(notice.severity);
                if let Some(body) = &notice.body {
                    toast = toast.body(body.clone());
                }
                toasts.update(cx, |toaster, cx| {
                    toaster.push(toast, cx);
                });
            }),
        ];
        Self { focus, store, tab: Tab::Home, setup, screens, toaster, _subscriptions: subscriptions }
    }

    fn select(&mut self, tab: Tab, cx: &mut Context<Self>) {
        if self.tab == tab {
            self.screens.pop_to_root(tab, cx);
        }
        self.tab = tab;
        cx.notify();
    }
}

impl Render for Root {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let colors = cx.theme().colors.clone();
        let viewport = window.viewport_size();
        let (inset_top, inset_bottom) = crate::insets::vertical(window);
        let narrow = viewport.width < px(NARROW_WIDTH);
        log::debug!(
            "layout: viewport {:?} scale {} insets ({:?}, {:?}) narrow {narrow}",
            viewport,
            window.scale_factor(),
            inset_top,
            inset_bottom
        );
        let rem = px(if narrow { NARROW_REM } else { WIDE_REM });
        if window.rem_size() != rem {
            window.set_rem_size(rem);
        }

        let this = cx.entity();
        let on_select: OnSelect =
            Rc::new(move |tab, _, cx| this.update(cx, |root, cx| root.select(tab, cx)));

        let body: AnyView = if self.store.read(cx).is_configured() {
            self.screens.view(self.tab, cx)
        } else {
            self.setup.clone().into()
        };
        let configured = self.store.read(cx).is_configured();

        let content = div().flex_1().min_h_0().min_w_0().size_full().child(body);
        let shell = if !configured {
            div().size_full().child(content)
        } else if narrow {
            div()
                .size_full()
                .flex()
                .flex_col()
                .child(content)
                .child(bottom_bar(self.tab, on_select, cx))
        } else {
            div()
                .size_full()
                .flex()
                .flex_row()
                .child(side_bar(self.tab, on_select))
                .child(content)
        };

        FocusScope::new(&self.focus)
            .root()
            .size_full()
            .flex()
            .flex_col()
            .pt(inset_top)
            .pb(inset_bottom)
            .bg(colors.bg)
            .text_color(colors.fg)
            .child(shell)
            .child(ToastViewport::new("toasts", &self.toaster))
    }
}
