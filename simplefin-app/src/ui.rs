//! Small shared building blocks for the app's screens.

use chrono::{DateTime, Datelike, Utc};
use ely_gpui_component::{
    layout::{Card, CardHeader, ScrollArea},
    theme::ActiveTheme,
    typography::format,
};
use gpui::{
    AnyElement, App, Div, Hsla, IntoElement, ParentElement, SharedString, Styled, div,
};
use rust_decimal::Decimal;
use rust_decimal::prelude::ToPrimitive;

/// A full-height page: a title row with optional actions, then a vertically
/// scrolling body.
pub fn page(
    id: impl Into<gpui::ElementId>,
    title: impl Into<SharedString>,
    actions: Option<AnyElement>,
    body: impl IntoElement,
    cx: &App,
) -> Div {
    let colors = &cx.theme().colors;
    div()
        .size_full()
        .flex()
        .flex_col()
        .child(
            div()
                .flex()
                .flex_row()
                .items_center()
                .justify_between()
                .px_4()
                .py_3()
                .border_b_1()
                .border_color(colors.border)
                .child(div().text_xl().font_weight(gpui::FontWeight::SEMIBOLD).child(title.into()))
                .children(actions),
        )
        .child(
            ScrollArea::new(id)
                .vertical()
                .flex_1()
                .min_h_0()
                .child(div().flex().flex_col().gap_4().p_4().pb_8().child(body)),
        )
}

/// A titled card section.
pub fn section(title: impl Into<SharedString>, body: impl IntoElement) -> Card {
    Card::new().w_full().header(CardHeader::new(title.into())).child(body)
}

/// A section whose header carries an action (e.g. "See all").
pub fn section_with_action(
    title: impl Into<SharedString>,
    action: impl IntoElement,
    body: impl IntoElement,
) -> Card {
    Card::new()
        .w_full()
        .header(CardHeader::new(title.into()).action(action))
        .child(body)
}

/// A label/value row used inside cards.
pub fn row(label: impl Into<SharedString>, value: impl IntoElement, cx: &App) -> Div {
    div()
        .flex()
        .flex_row()
        .justify_between()
        .items_center()
        .gap_2()
        .child(div().text_color(cx.theme().colors.fg_muted).child(label.into()))
        .child(value)
}

/// A large tabular money figure.
pub fn hero_amount(amount: Decimal, currency: &str) -> Div {
    div()
        .text_3xl()
        .font_weight(gpui::FontWeight::SEMIBOLD)
        .child(money(amount, currency))
}

pub fn money(amount: Decimal, currency: &str) -> String {
    format::currency(amount.to_f64().unwrap_or_default(), currency)
}

/// Money with an explicit `+` for positive amounts (deposits, gains).
pub fn signed_money(amount: Decimal, currency: &str) -> String {
    let text = money(amount, currency);
    if amount > Decimal::ZERO { format!("+{text}") } else { text }
}

pub fn amount_color(amount: Decimal, cx: &App) -> Hsla {
    let colors = &cx.theme().colors;
    if amount > Decimal::ZERO { colors.success } else { colors.fg }
}

pub fn to_f64(amount: Decimal) -> f64 {
    amount.to_f64().unwrap_or_default()
}

/// "Oct 3" for an epoch-seconds timestamp (UTC).
pub fn short_date(timestamp: i64) -> String {
    DateTime::<Utc>::from_timestamp(timestamp, 0)
        .map(|d| d.format("%b %-d").to_string())
        .unwrap_or_default()
}

/// "Oct 3, 2026".
pub fn long_date(timestamp: i64) -> String {
    DateTime::<Utc>::from_timestamp(timestamp, 0)
        .map(|d| d.format("%b %-d, %Y").to_string())
        .unwrap_or_default()
}

/// "Oct" for "2026-10"; the year is added when it differs from `current_year`.
pub fn month_label(month: &str, current_year: i32) -> String {
    let Some((year, mon)) = month.split_once('-') else { return month.to_string() };
    let (Ok(year), Ok(mon)) = (year.parse::<i32>(), mon.parse::<u32>()) else {
        return month.to_string();
    };
    const NAMES: [&str; 12] =
        ["Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec"];
    let name = NAMES.get(mon.saturating_sub(1) as usize).copied().unwrap_or("?");
    if year == current_year { name.to_string() } else { format!("{name} {}", year % 100) }
}

pub fn current_year(now: i64) -> i32 {
    DateTime::<Utc>::from_timestamp(now, 0).map(|d| d.year()).unwrap_or(1970)
}

/// "just now", "5 min ago", "3 h ago", "2 days ago".
pub fn ago(then: i64, now: i64) -> String {
    let secs = (now - then).max(0);
    match secs {
        0..60 => "just now".into(),
        60..3600 => format!("{} min ago", secs / 60),
        3600..86400 => format!("{} h ago", secs / 3600),
        _ => {
            let days = secs / 86400;
            format!("{days} day{} ago", if days == 1 { "" } else { "s" })
        }
    }
}

/// Opens `url` outside the app: the browser on desktop and the web, the
/// default handler on mobile.
pub fn open_external(url: &str, cx: &mut App) {
    #[cfg(any(target_os = "android", target_os = "ios"))]
    {
        let _ = cx;
        match gpui_mobile::packages::url_launcher::launch_url(url) {
            Ok(true) => {}
            Ok(false) => log::warn!("no app can open {url}"),
            Err(err) => log::error!("opening {url}: {err}"),
        }
    }
    #[cfg(not(any(target_os = "android", target_os = "ios")))]
    cx.open_url(url);
}

/// A muted one-line caption.
pub fn muted(text: impl Into<SharedString>, cx: &App) -> Div {
    div().text_sm().text_color(cx.theme().colors.fg_muted).child(text.into())
}

/// A centered placeholder for empty sections.
pub fn empty(text: impl Into<SharedString>, cx: &App) -> Div {
    div().py_4().flex().justify_center().child(muted(text, cx))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn formats_money() {
        assert_eq!(money(Decimal::new(123456, 2), "USD"), "$1,234.56");
        assert_eq!(signed_money(Decimal::new(500, 2), "USD"), "+$5.00");
        assert!(signed_money(Decimal::new(-500, 2), "USD").contains("5.00"));
    }

    #[test]
    fn formats_dates() {
        // 2026-10-03T00:00:00Z
        assert_eq!(short_date(1_790_985_600), "Oct 3");
        assert_eq!(long_date(1_790_985_600), "Oct 3, 2026");
        assert_eq!(month_label("2026-10", 2026), "Oct");
        assert_eq!(month_label("2025-12", 2026), "Dec 25");
    }

    #[test]
    fn formats_relative_time() {
        assert_eq!(ago(100, 110), "just now");
        assert_eq!(ago(0, 300), "5 min ago");
        assert_eq!(ago(0, 7200), "2 h ago");
        assert_eq!(ago(0, 86400), "1 day ago");
        assert_eq!(ago(0, 3 * 86400), "3 days ago");
    }
}
