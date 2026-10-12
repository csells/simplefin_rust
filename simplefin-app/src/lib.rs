//! SimpleFIN personal finance app: one GPUI + Ely UI for desktop, mobile and
//! the web.

mod actions;
#[cfg(not(target_arch = "wasm32"))]
mod background;
mod credentials;
mod env;
mod error_log;
mod files;
mod insets;
mod jobs;
mod nav;
mod root;
mod screens;
mod setup;
mod snapshot;
mod store;
mod ui;

#[cfg(target_os = "android")]
mod android;
#[cfg(target_arch = "wasm32")]
mod web;

pub use env::AppEnv;
pub use root::open_main_window;
