//! Top-level destinations and the two navigation chromes: a bottom bar on
//! phones and a sidebar on wide windows.

use ely_gpui_component::{
    layout::Sidebar,
    navigation::NavItem,
    primitives::{Icon, IconName},
    theme::{ActiveTheme, IconSize},
};
use gpui::{
    App, ClickEvent, InteractiveElement, IntoElement, ParentElement, SharedString,
    StatefulInteractiveElement, Styled, Window, div, px,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Tab {
    Home,
    Accounts,
    Transactions,
    Plan,
    More,
}

impl Tab {
    pub const ALL: [Tab; 5] = [Tab::Home, Tab::Accounts, Tab::Transactions, Tab::Plan, Tab::More];

    pub fn label(self) -> &'static str {
        match self {
            Tab::Home => "Home",
            Tab::Accounts => "Accounts",
            Tab::Transactions => "Activity",
            Tab::Plan => "Plan",
            Tab::More => "More",
        }
    }

    pub fn icon(self) -> IconName {
        match self {
            Tab::Home => IconName::House,
            Tab::Accounts => IconName::Wallet,
            Tab::Transactions => IconName::List,
            Tab::Plan => IconName::ChartPie,
            Tab::More => IconName::Ellipsis,
        }
    }

    fn id(self) -> SharedString {
        SharedString::new_static(match self {
            Tab::Home => "tab-home",
            Tab::Accounts => "tab-accounts",
            Tab::Transactions => "tab-transactions",
            Tab::Plan => "tab-plan",
            Tab::More => "tab-more",
        })
    }
}

pub type OnSelect = std::rc::Rc<dyn Fn(Tab, &mut Window, &mut App)>;

/// Phone navigation: five equal-width icon + label targets along the bottom.
pub fn bottom_bar(current: Tab, on_select: OnSelect, cx: &App) -> impl IntoElement {
    let colors = &cx.theme().colors;
    div()
        .flex()
        .flex_row()
        .w_full()
        .border_t_1()
        .border_color(colors.border)
        .bg(colors.surface)
        .children(Tab::ALL.into_iter().map(|tab| {
            let selected = tab == current;
            let color = if selected { colors.accent } else { colors.fg_muted };
            let on_select = on_select.clone();
            div()
                .id(tab.id())
                .flex_1()
                .flex()
                .flex_col()
                .items_center()
                .gap_0p5()
                .py_2()
                .min_h(px(56.))
                .text_xs()
                .text_color(color)
                .child(Icon::new(tab.icon()).size(IconSize::Lg).color(color))
                .child(tab.label())
                .on_click(move |_: &ClickEvent, window, cx| on_select(tab, window, cx))
        }))
}

/// Wide-window navigation.
pub fn side_bar(current: Tab, on_select: OnSelect) -> impl IntoElement {
    Sidebar::new("nav-sidebar", false).child(
        div().flex().flex_col().gap_1().p_2().children(Tab::ALL.into_iter().map(|tab| {
            let on_select = on_select.clone();
            NavItem::new(tab.id(), tab.icon(), tab.label())
                .active(tab == current)
                .on_click(move |window, cx| on_select(tab, window, cx))
        })),
    )
}
