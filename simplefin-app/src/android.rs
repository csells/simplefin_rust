//! Android entry point: `GpuiActivity` (a NativeActivity) loads this cdylib and
//! `android-activity` calls `android_main` on its own thread.

use gpui::Application;
use gpui_mobile::android::jni;

#[unsafe(no_mangle)]
fn android_main(app: android_activity::AndroidApp) {
    android_logger::init_once(
        android_logger::Config::default()
            .with_max_level(log::LevelFilter::Info)
            .with_tag("simplefin-app"),
    );
    jni::install_panic_hook();

    let Some(data_dir) = app.internal_data_path() else {
        log::error!("android_main: no internal data path; cannot start");
        return;
    };
    let env = crate::AppEnv::on_disk(data_dir.join("simplefin"));
    let _platform = jni::init_platform(&app);
    let Some(shared) = jni::shared_platform() else {
        log::error!("android_main: no shared platform; cannot start");
        return;
    };

    Application::with_platform(shared.into_rc())
        .with_assets(ely_gpui_component::Assets)
        .run(move |cx| {
            if let Err(err) = crate::open_main_window(cx, env) {
                log::error!("failed to open main window: {err:#}");
            }
        });
}
