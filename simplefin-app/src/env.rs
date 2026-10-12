//! The platform-specific pieces the app runs on: where data lives and where
//! the SimpleFIN access URL is kept.

use crate::credentials::CredentialStore;
use crate::files::SharedFiles;

pub struct AppEnv {
    /// Financial data, configuration and the error log.
    pub files: SharedFiles,
    /// The SimpleFIN access URL (which embeds the user's credentials).
    pub credentials: CredentialStore,
    /// Where the data lives, for the About section.
    pub location: String,
}

impl AppEnv {
    /// Data and credentials in `data_dir` on disk (desktop and mobile).
    #[cfg(not(target_arch = "wasm32"))]
    pub fn on_disk(data_dir: std::path::PathBuf) -> Self {
        Self {
            files: SharedFiles::new(simplefin::DiskFiles::new(data_dir.clone())),
            credentials: CredentialStore::new(data_dir.join("access_url")),
            location: data_dir.display().to_string(),
        }
    }

    /// Desktop default: `$SIMPLEFIN_DATA` if set (shared with the CLI), otherwise
    /// the platform's per-user data directory.
    #[cfg(not(any(target_os = "android", target_os = "ios", target_arch = "wasm32")))]
    pub fn desktop_default() -> anyhow::Result<Self> {
        if let Some(dir) = std::env::var_os("SIMPLEFIN_DATA") {
            return Ok(Self::on_disk(dir.into()));
        }
        let base = dirs::data_dir()
            .ok_or_else(|| anyhow::anyhow!("no per-user data directory on this platform"))?;
        Ok(Self::on_disk(base.join("SimpleFIN")))
    }

    /// Everything in this browser's `localStorage` for the app's origin.
    #[cfg(target_arch = "wasm32")]
    pub fn browser() -> Self {
        Self {
            files: SharedFiles::new(crate::files::LocalStorageFiles::new("simplefin/")),
            credentials: CredentialStore::browser("simplefin.access_url"),
            location: "This browser's local storage".to_string(),
        }
    }
}
