//! Persists the SimpleFIN access URL, which embeds the user's credentials.
//!
//! Natively the file lives in the app's private data directory (on Android,
//! app-internal storage no other app can read) and is created owner-read/write
//! only. In a browser it is one `localStorage` key, readable only by this
//! origin.

#[cfg(not(target_arch = "wasm32"))]
pub use disk::CredentialStore;
#[cfg(target_arch = "wasm32")]
pub use browser::CredentialStore;

#[cfg(target_arch = "wasm32")]
mod browser {
    pub struct CredentialStore {
        key: &'static str,
    }

    impl CredentialStore {
        pub fn browser(key: &'static str) -> Self {
            Self { key }
        }

        fn storage() -> anyhow::Result<web_sys::Storage> {
            web_sys::window()
                .and_then(|w| w.local_storage().ok().flatten())
                .ok_or_else(|| anyhow::anyhow!("browser storage is unavailable"))
        }

        pub fn load(&self) -> anyhow::Result<Option<String>> {
            let value = Self::storage()?
                .get_item(self.key)
                .map_err(|e| anyhow::anyhow!("reading credentials: {e:?}"))?;
            Ok(value.map(|v| v.trim().to_string()).filter(|v| !v.is_empty()))
        }

        pub fn save(&self, access_url: &str) -> anyhow::Result<()> {
            Self::storage()?
                .set_item(self.key, access_url.trim())
                .map_err(|e| anyhow::anyhow!("saving credentials: {e:?}"))
        }

        pub fn clear(&self) -> anyhow::Result<()> {
            Self::storage()?
                .remove_item(self.key)
                .map_err(|e| anyhow::anyhow!("clearing credentials: {e:?}"))
        }
    }
}

#[cfg(not(target_arch = "wasm32"))]
mod disk {
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};

use anyhow::Context as _;

pub struct CredentialStore {
    path: PathBuf,
}

impl CredentialStore {
    pub fn new(path: impl Into<PathBuf>) -> Self {
        Self { path: path.into() }
    }

    /// The stored access URL, or `None` before setup.
    pub fn load(&self) -> anyhow::Result<Option<String>> {
        match fs::read_to_string(&self.path) {
            Ok(raw) => {
                let url = raw.trim();
                Ok((!url.is_empty()).then(|| url.to_string()))
            }
            Err(err) if err.kind() == std::io::ErrorKind::NotFound => Ok(None),
            Err(err) => Err(err).with_context(|| format!("reading {}", self.path.display())),
        }
    }

    pub fn save(&self, access_url: &str) -> anyhow::Result<()> {
        if let Some(dir) = self.path.parent() {
            fs::create_dir_all(dir).with_context(|| format!("creating {}", dir.display()))?;
        }
        let tmp = self.path.with_extension("tmp");
        write_private(&tmp, access_url.trim().as_bytes())?;
        fs::rename(&tmp, &self.path)
            .with_context(|| format!("replacing {}", self.path.display()))
    }

    pub fn clear(&self) -> anyhow::Result<()> {
        match fs::remove_file(&self.path) {
            Err(err) if err.kind() != std::io::ErrorKind::NotFound => {
                Err(err).with_context(|| format!("removing {}", self.path.display()))
            }
            _ => Ok(()),
        }
    }
}

fn write_private(path: &Path, bytes: &[u8]) -> anyhow::Result<()> {
    let mut options = fs::OpenOptions::new();
    options.write(true).create(true).truncate(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o600);
    }
    let mut file = options
        .open(path)
        .with_context(|| format!("opening {}", path.display()))?;
    file.write_all(bytes)
        .and_then(|()| file.sync_all())
        .with_context(|| format!("writing {}", path.display()))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn store() -> (tempfile::TempDir, CredentialStore) {
        let dir = tempfile::tempdir().unwrap();
        let store = CredentialStore::new(dir.path().join("nested").join("access_url"));
        (dir, store)
    }

    #[test]
    fn load_before_setup_is_none() {
        let (_dir, store) = store();
        assert_eq!(store.load().unwrap(), None);
    }

    #[test]
    fn save_then_load_round_trips_trimmed() {
        let (_dir, store) = store();
        store.save("  https://user:pass@bridge.example/simplefin\n").unwrap();
        assert_eq!(
            store.load().unwrap().as_deref(),
            Some("https://user:pass@bridge.example/simplefin")
        );
    }

    #[cfg(unix)]
    #[test]
    fn saved_file_is_owner_only() {
        use std::os::unix::fs::PermissionsExt;
        let (_dir, store) = store();
        store.save("https://user:pass@bridge.example/simplefin").unwrap();
        let mode = fs::metadata(&store.path).unwrap().permissions().mode();
        assert_eq!(mode & 0o777, 0o600);
    }

    #[test]
    fn clear_removes_and_is_idempotent() {
        let (_dir, store) = store();
        store.save("https://user:pass@bridge.example/simplefin").unwrap();
        store.clear().unwrap();
        store.clear().unwrap();
        assert_eq!(store.load().unwrap(), None);
    }
}
}
