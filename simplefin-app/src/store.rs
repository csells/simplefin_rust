//! `FinanceStore`: the single owner of the user's financial data in the app.
//!
//! Views observe the store and re-render when it notifies (push, not poll).
//! Network work runs on a background thread natively ([`crate::background`])
//! and on GPUI's single thread through `fetch` in a browser; storage reads that
//! build the [`Snapshot`] run on GPUI's background executor.

use std::sync::Arc;

use ely_gpui_component::primitives::Severity;
use gpui::{AppContext, Context, EventEmitter, Task};

use crate::env::AppEnv;
use crate::error_log::ErrorLog;
use crate::files::SharedFiles;
use crate::jobs;
use crate::snapshot::Snapshot;

/// Sync automatically at launch when the last sync is older than this
/// (Empower refreshes on login at most every 8 hours).
const AUTO_SYNC_AFTER_SECS: i64 = 8 * 60 * 60;

/// A message the UI should surface (as a toast).
#[derive(Debug, Clone)]
pub struct Notice {
    pub severity: Severity,
    pub title: String,
    pub body: Option<String>,
}

pub struct FinanceStore {
    env: AppEnv,
    errors: ErrorLog,
    configured: bool,
    connecting: bool,
    setup_error: Option<String>,
    syncing: bool,
    snapshot: Arc<Snapshot>,
    _task: Option<Task<()>>,
    _load: Option<Task<()>>,
}

impl EventEmitter<Notice> for FinanceStore {}

impl FinanceStore {
    pub fn new(env: AppEnv, cx: &mut Context<Self>) -> Self {
        let errors = ErrorLog::new(env.files.clone());
        let configured = match env.credentials.load() {
            Ok(url) => url.is_some(),
            Err(err) => {
                errors.record("load credentials", &err);
                false
            }
        };
        let mut store = Self {
            env,
            errors,
            configured,
            connecting: false,
            setup_error: None,
            syncing: false,
            snapshot: Arc::new(Snapshot::default()),
            _task: None,
            _load: None,
        };
        store.reload(cx);
        store
    }

    pub fn is_configured(&self) -> bool {
        self.configured
    }

    pub fn is_connecting(&self) -> bool {
        self.connecting
    }

    pub fn is_syncing(&self) -> bool {
        self.syncing
    }

    pub fn setup_error(&self) -> Option<&str> {
        self.setup_error.as_deref()
    }

    pub fn snapshot(&self) -> Arc<Snapshot> {
        self.snapshot.clone()
    }

    pub fn errors(&self) -> &ErrorLog {
        &self.errors
    }

    /// Where the data lives, for display.
    pub fn location(&self) -> &str {
        &self.env.location
    }

    /// Connects with a setup token (claimed once at the bridge) or an access URL,
    /// stores the access URL, then runs the first sync.
    pub fn connect(&mut self, token_or_url: String, cx: &mut Context<Self>) {
        if self.connecting {
            return;
        }
        self.connecting = true;
        self.setup_error = None;
        cx.notify();
        let claim = Self::claim(token_or_url, cx);
        self._task = Some(cx.spawn(async move |this, cx| {
            let result = claim.await;
            this.update(cx, |store, cx| {
                store.connecting = false;
                match result.and_then(|url| store.env.credentials.save(&url)) {
                    Ok(()) => {
                        store.configured = true;
                        store.sync(cx);
                    }
                    Err(err) => {
                        store.setup_error = Some(store.errors.record("connect", &err));
                    }
                }
                cx.notify();
            })
            .ok();
        }));
    }

    /// Fetches new data from the bridge into local storage, then reloads.
    pub fn sync(&mut self, cx: &mut Context<Self>) {
        if self.syncing {
            return;
        }
        let access_url = match self.env.credentials.load() {
            Ok(Some(url)) => url,
            Ok(None) => return,
            Err(err) => return self.report("load credentials", err, cx),
        };
        self.syncing = true;
        cx.notify();
        let sync = Self::collect(access_url, self.env.files.clone(), cx);
        self._task = Some(cx.spawn(async move |this, cx| {
            let result = sync.await;
            this.update(cx, |store, cx| {
                store.syncing = false;
                match result {
                    Ok(summary) => {
                        cx.emit(Notice {
                            severity: Severity::Success,
                            title: "Up to date".into(),
                            body: Some(summary),
                        });
                        store.reload(cx);
                    }
                    Err(err) => store.report("sync", err, cx),
                }
                cx.notify();
            })
            .ok();
        }));
    }

    /// Forgets the access URL. Collected data stays on the device.
    pub fn disconnect(&mut self, cx: &mut Context<Self>) {
        match self.env.credentials.clear() {
            Ok(()) => {
                self.configured = false;
                cx.notify();
            }
            Err(err) => self.report("disconnect", err, cx),
        }
    }

    /// Rebuilds the snapshot from local storage off the main thread.
    pub fn reload(&mut self, cx: &mut Context<Self>) {
        let files = self.env.files.clone();
        let load = cx.background_spawn(async move { Snapshot::load(&files.storage()?, jobs::now()) });
        self._load = Some(cx.spawn(async move |this, cx| {
            let result = load.await;
            this.update(cx, |store, cx| {
                let first_load = !store.snapshot.loaded;
                match result {
                    Ok(snapshot) => store.snapshot = Arc::new(snapshot),
                    Err(err) => store.report("load data", err, cx),
                }
                if first_load && store.configured && store.snapshot.is_stale(AUTO_SYNC_AFTER_SECS) {
                    store.sync(cx);
                }
                cx.notify();
            })
            .ok();
        }));
    }

    /// Applies a change to stored data (edits, config) and reloads.
    pub fn mutate(
        &mut self,
        context: &str,
        change: impl FnOnce(&simplefin::JsonStorage<SharedFiles>) -> anyhow::Result<()>,
        cx: &mut Context<Self>,
    ) {
        match self.env.files.storage().and_then(|storage| change(&storage)) {
            Ok(()) => self.reload(cx),
            Err(err) => self.report(context, err, cx),
        }
    }

    /// Claims a setup token (or validates an access URL), off the UI thread.
    #[cfg(not(target_arch = "wasm32"))]
    fn claim(token_or_url: String, _: &mut Context<Self>) -> impl std::future::Future<Output = anyhow::Result<String>> + use<> {
        let rx = crate::background::spawn_io("simplefin-claim", move |io| async move {
            jobs::native::resolve_access_url(&io, &token_or_url).await
        });
        async move { rx.await.unwrap_or_else(|_| Err(anyhow::anyhow!("claim was interrupted"))) }
    }

    #[cfg(target_arch = "wasm32")]
    fn claim(token_or_url: String, cx: &mut Context<Self>) -> impl std::future::Future<Output = anyhow::Result<String>> + use<> {
        let http = cx.http_client();
        async move { jobs::web::resolve_access_url(http, &token_or_url).await }
    }

    /// Collects from the bridge into `files`, off the UI thread where possible.
    #[cfg(not(target_arch = "wasm32"))]
    fn collect(access_url: String, files: SharedFiles, _: &mut Context<Self>) -> impl std::future::Future<Output = anyhow::Result<String>> + use<> {
        let rx = crate::background::spawn_io("simplefin-sync", move |io| async move {
            jobs::native::sync(&io, &access_url, files).await
        });
        async move { rx.await.unwrap_or_else(|_| Err(anyhow::anyhow!("sync was interrupted"))) }
    }

    #[cfg(target_arch = "wasm32")]
    fn collect(access_url: String, files: SharedFiles, cx: &mut Context<Self>) -> impl std::future::Future<Output = anyhow::Result<String>> + use<> {
        let http = cx.http_client();
        async move { jobs::web::sync(http, &access_url, files).await }
    }

    fn report(&mut self, context: &str, err: anyhow::Error, cx: &mut Context<Self>) {
        let message = self.errors.record(context, &err);
        cx.emit(Notice {
            severity: Severity::Danger,
            title: format!("Couldn't {context}"),
            body: Some(message),
        });
        cx.notify();
    }
}
