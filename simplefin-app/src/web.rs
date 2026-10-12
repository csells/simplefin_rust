//! Browser entry point: the wasm module's start function opens the app on a
//! canvas through GPUI's web platform.

use std::rc::Rc;
use std::sync::Arc;

use gpui::Application;
use gpui_web::{CanvasFontFallback, WebBackendPreference, WebPlatform};
use wasm_bindgen::prelude::*;

#[wasm_bindgen(start)]
pub fn start() {
    gpui_platform::web_init();
    // One thread: a statically served page gets no cross-origin isolation.
    // WebGPU where the browser has it, WebGL2 otherwise; the browser lends
    // fonts only for emoji and CJK.
    let platform = Rc::new(WebPlatform::new_with_backend_and_font_fallback(
        false,
        WebBackendPreference::Auto,
        CanvasFontFallback::EmojiAndCjk,
    ));
    let http = Arc::new(platform.fetch_http_client());
    Application::with_platform(platform)
        .with_http_client(http)
        .with_assets(ely_gpui_component::Assets)
        .run(|cx| {
            if let Err(err) = crate::open_main_window(cx, crate::AppEnv::browser()) {
                log::error!("failed to open the app: {err:#}");
            }
        });
}
