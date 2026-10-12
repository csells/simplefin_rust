//! Where the app's data files live: disk on native platforms, `localStorage`
//! in a browser. Everything goes through `simplefin::FileStore`.

use std::sync::Arc;

use simplefin::{FileStore, JsonStorage};

/// A cheaply clonable, thread-safe handle to the app's file store.
#[derive(Clone)]
pub struct SharedFiles(Arc<dyn FileStore + Send + Sync>);

impl SharedFiles {
    pub fn new(files: impl FileStore + Send + Sync + 'static) -> Self {
        Self(Arc::new(files))
    }

    /// The user's financial data over these files.
    pub fn storage(&self) -> anyhow::Result<JsonStorage<SharedFiles>> {
        Ok(JsonStorage::with_files(self.clone())?)
    }
}

impl FileStore for SharedFiles {
    fn read(&self, path: &str) -> simplefin::Result<Option<String>> {
        self.0.read(path)
    }

    fn write(&self, path: &str, contents: &str) -> simplefin::Result<()> {
        self.0.write(path, contents)
    }

    fn list(&self, dir: &str) -> simplefin::Result<Vec<String>> {
        self.0.list(dir)
    }

    fn remove(&self, path: &str) -> simplefin::Result<()> {
        self.0.remove(path)
    }
}

/// `localStorage`-backed files for the browser build. Each file is one key under
/// a fixed prefix; the store holds no handle, so it is `Send + Sync` and
/// re-acquires `window.localStorage` on every call (the web build is
/// single-threaded).
#[cfg(target_arch = "wasm32")]
pub struct LocalStorageFiles {
    prefix: &'static str,
}

#[cfg(target_arch = "wasm32")]
impl LocalStorageFiles {
    pub const fn new(prefix: &'static str) -> Self {
        Self { prefix }
    }

    fn storage() -> simplefin::Result<web_sys::Storage> {
        web_sys::window()
            .and_then(|w| w.local_storage().ok().flatten())
            .ok_or_else(|| storage_error("browser storage is unavailable".into()))
    }

    fn key(&self, path: &str) -> String {
        format!("{}{}", self.prefix, path.trim_start_matches('/'))
    }

    fn fail(action: &str, path: &str, err: wasm_bindgen::JsValue) -> simplefin::SimplefinError {
        storage_error(format!("{action} {path}: {err:?}"))
    }
}

#[cfg(target_arch = "wasm32")]
fn storage_error(message: String) -> simplefin::SimplefinError {
    simplefin::SimplefinError::Storage { message, source: None }
}

#[cfg(target_arch = "wasm32")]
impl FileStore for LocalStorageFiles {
    fn read(&self, path: &str) -> simplefin::Result<Option<String>> {
        Self::storage()?.get_item(&self.key(path)).map_err(|e| Self::fail("reading", path, e))
    }

    fn write(&self, path: &str, contents: &str) -> simplefin::Result<()> {
        // setItem replaces atomically; it fails (QuotaExceededError) rather than truncating.
        Self::storage()?
            .set_item(&self.key(path), contents)
            .map_err(|e| Self::fail("writing", path, e))
    }

    fn list(&self, dir: &str) -> simplefin::Result<Vec<String>> {
        let storage = Self::storage()?;
        let dir_prefix = format!("{}/", self.key(dir).trim_end_matches('/'));
        let len = storage.length().map_err(|e| Self::fail("listing", dir, e))?;
        let mut names = Vec::new();
        for i in 0..len {
            if let Some(key) = storage.key(i).map_err(|e| Self::fail("listing", dir, e))?
                && let Some(rest) = key.strip_prefix(&dir_prefix)
                && !rest.contains('/')
            {
                names.push(rest.to_string());
            }
        }
        names.sort();
        Ok(names)
    }

    fn remove(&self, path: &str) -> simplefin::Result<()> {
        Self::storage()?.remove_item(&self.key(path)).map_err(|e| Self::fail("removing", path, e))
    }
}
