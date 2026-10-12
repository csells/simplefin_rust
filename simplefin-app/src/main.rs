//! Desktop entry point. Android starts in `android_main` and the browser in
//! `web::start` (both in the library).

#[cfg(not(any(target_os = "android", target_os = "ios", target_arch = "wasm32")))]
fn main() {
    env_logger::init();
    let env = match simplefin_app::AppEnv::desktop_default() {
        Ok(env) => env,
        Err(err) => {
            log::error!("cannot find a data directory: {err:#}");
            std::process::exit(1);
        }
    };
    gpui_platform::application()
        .with_assets(ely_gpui_component::Assets)
        .run(move |cx| {
            if let Err(err) = simplefin_app::open_main_window(cx, env) {
                log::error!("failed to open main window: {err:#}");
                cx.quit();
            }
        });
}

/// Mobile apps start in `android_main`; the web build starts in `web::start`.
#[cfg(any(target_os = "android", target_os = "ios", target_arch = "wasm32"))]
fn main() {}
