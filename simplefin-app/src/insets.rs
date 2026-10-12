//! Space at the top and bottom of the window covered by system UI (status
//! bar, navigation bar, keyboard), so content can avoid it.

use gpui::{Pixels, Window, px};

/// `(top, bottom)` insets in logical pixels.
pub fn vertical(window: &Window) -> (Pixels, Pixels) {
    let viewport = window.viewport_size();
    let visible = window.fully_visible_bounds();
    let top = visible.origin.y;
    let bottom = (viewport.height - visible.origin.y - visible.size.height).max(px(0.));
    let (platform_top, platform_bottom) = platform_safe_area();
    (top.max(platform_top), bottom.max(platform_bottom))
}

/// gpui-mobile tracks Android's system-bar insets on its own window type
/// rather than through GPUI's `WindowInsets`.
#[cfg(target_os = "android")]
fn platform_safe_area() -> (Pixels, Pixels) {
    gpui_mobile::android::jni::platform()
        .and_then(|platform| platform.primary_window())
        .map(|window| {
            let insets = window.safe_area_insets_logical();
            (px(insets.top), px(insets.bottom))
        })
        .unwrap_or((px(0.), px(0.)))
}

#[cfg(not(target_os = "android"))]
fn platform_safe_area() -> (Pixels, Pixels) {
    (px(0.), px(0.))
}
