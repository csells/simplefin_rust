//! The file seam beneath [`JsonStorage`](super::JsonStorage): a tiny
//! key/value file API so the same storage layout can live on disk (native)
//! or in memory (browser, tests), where a host persists the entries itself.

use std::collections::BTreeMap;
use std::sync::{Arc, Mutex, MutexGuard};

use crate::error::{Result, SimplefinError};

/// Minimal file operations used by `JsonStorage`. Paths are relative and
/// `/`-separated (e.g. `transactions/acc1.json`).
pub trait FileStore {
    /// Contents of the file at `path` (relative, `/`-separated), or None if absent.
    fn read(&self, path: &str) -> Result<Option<String>>;
    /// Atomically creates or replaces the file, creating parent directories as needed.
    fn write(&self, path: &str, contents: &str) -> Result<()>;
    /// Names (not paths) of files directly inside `dir`, sorted; empty if `dir` is absent.
    fn list(&self, dir: &str) -> Result<Vec<String>>;
    /// Removes the file; Ok if it was already absent.
    fn remove(&self, path: &str) -> Result<()>;
}

/// In-memory [`FileStore`]. Clones share one map, so a host can keep a clone
/// and persist [`entries`](Self::entries) after the storage writes.
#[derive(Debug, Clone, Default)]
pub struct MemoryFiles {
    files: Arc<Mutex<BTreeMap<String, String>>>,
}

impl MemoryFiles {
    /// An empty store.
    pub fn new() -> Self {
        Self::default()
    }

    /// A store holding `entries` (`(path, contents)` pairs), e.g. restored
    /// from a host's persisted copy.
    pub fn from_entries(entries: impl IntoIterator<Item = (String, String)>) -> Self {
        Self {
            files: Arc::new(Mutex::new(entries.into_iter().collect())),
        }
    }

    /// Every `(path, contents)` pair, sorted by path. A poisoned lock still
    /// yields the data (the map is never left half-updated).
    pub fn entries(&self) -> Vec<(String, String)> {
        let files = self
            .files
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        files
            .iter()
            .map(|(path, contents)| (path.clone(), contents.clone()))
            .collect()
    }

    fn lock(&self) -> Result<MutexGuard<'_, BTreeMap<String, String>>> {
        self.files.lock().map_err(|_| SimplefinError::Storage {
            message: "in-memory file store lock poisoned".into(),
            source: None,
        })
    }
}

impl FileStore for MemoryFiles {
    fn read(&self, path: &str) -> Result<Option<String>> {
        Ok(self.lock()?.get(path).cloned())
    }

    fn write(&self, path: &str, contents: &str) -> Result<()> {
        self.lock()?.insert(path.to_string(), contents.to_string());
        Ok(())
    }

    fn list(&self, dir: &str) -> Result<Vec<String>> {
        let prefix = match dir.trim_end_matches('/') {
            "" => String::new(),
            dir => format!("{dir}/"),
        };
        Ok(self
            .lock()?
            .keys()
            .filter_map(|path| path.strip_prefix(&prefix))
            .filter(|name| !name.is_empty() && !name.contains('/'))
            .map(str::to_string)
            .collect())
    }

    fn remove(&self, path: &str) -> Result<()> {
        self.lock()?.remove(path);
        Ok(())
    }
}

#[cfg(not(target_arch = "wasm32"))]
pub use disk::DiskFiles;

#[cfg(not(target_arch = "wasm32"))]
mod disk {
    use std::fs;
    use std::io::ErrorKind;
    use std::path::{Path, PathBuf};

    use super::FileStore;
    use crate::error::{Result, SimplefinError};

    /// [`FileStore`] over a directory on disk. Writes go to `{file}.tmp` and
    /// are renamed into place, so readers never see a partial file.
    #[derive(Debug, Clone)]
    pub struct DiskFiles {
        root: PathBuf,
    }

    impl DiskFiles {
        /// A store rooted at `root`. No I/O happens until the first operation.
        pub fn new(root: impl Into<PathBuf>) -> Self {
            Self { root: root.into() }
        }

        /// The directory this store is rooted at.
        pub fn root(&self) -> &Path {
            &self.root
        }

        /// The on-disk location of the relative `path`.
        pub fn path_of(&self, path: &str) -> PathBuf {
            path.split('/')
                .filter(|part| !part.is_empty())
                .fold(self.root.clone(), |acc, part| acc.join(part))
        }

        /// Creates the relative directory `dir` (and its parents).
        pub(crate) fn create_dir(&self, dir: &str) -> Result<()> {
            let path = self.path_of(dir);
            fs::create_dir_all(&path).map_err(|e| SimplefinError::Storage {
                message: format!("failed to create directory: {}", path.display()),
                source: Some(Box::new(e)),
            })
        }
    }

    impl FileStore for DiskFiles {
        fn read(&self, path: &str) -> Result<Option<String>> {
            let path = self.path_of(path);
            match fs::read_to_string(&path) {
                Ok(data) => Ok(Some(data)),
                Err(e) if e.kind() == ErrorKind::NotFound => Ok(None),
                Err(e) => Err(SimplefinError::Storage {
                    message: format!("failed to read {}", path.display()),
                    source: Some(Box::new(e)),
                }),
            }
        }

        fn write(&self, path: &str, contents: &str) -> Result<()> {
            let path = self.path_of(path);
            if let Some(parent) = path.parent() {
                fs::create_dir_all(parent).map_err(|e| SimplefinError::Storage {
                    message: format!("failed to create directory: {}", parent.display()),
                    source: Some(Box::new(e)),
                })?;
            }
            let mut tmp = path.clone().into_os_string();
            tmp.push(".tmp");
            let tmp = PathBuf::from(tmp);
            fs::write(&tmp, contents).map_err(|e| SimplefinError::Storage {
                message: format!("failed to write {}", tmp.display()),
                source: Some(Box::new(e)),
            })?;
            fs::rename(&tmp, &path).map_err(|e| SimplefinError::Storage {
                message: format!("failed to rename {} to {}", tmp.display(), path.display()),
                source: Some(Box::new(e)),
            })
        }

        fn list(&self, dir: &str) -> Result<Vec<String>> {
            let dir = self.path_of(dir);
            let entries = match fs::read_dir(&dir) {
                Ok(entries) => entries,
                Err(e) if e.kind() == ErrorKind::NotFound => return Ok(Vec::new()),
                Err(e) => {
                    return Err(SimplefinError::Storage {
                        message: format!("failed to read directory {}", dir.display()),
                        source: Some(Box::new(e)),
                    });
                }
            };
            let mut names = Vec::new();
            for entry in entries {
                let entry = entry.map_err(|e| SimplefinError::Storage {
                    message: "failed to read directory entry".into(),
                    source: Some(Box::new(e)),
                })?;
                if entry.path().is_file()
                    && let Some(name) = entry.file_name().to_str()
                {
                    names.push(name.to_string());
                }
            }
            names.sort();
            Ok(names)
        }

        fn remove(&self, path: &str) -> Result<()> {
            let path = self.path_of(path);
            match fs::remove_file(&path) {
                Ok(()) => Ok(()),
                Err(e) if e.kind() == ErrorKind::NotFound => Ok(()),
                Err(e) => Err(SimplefinError::Storage {
                    message: format!("failed to remove file: {}", path.display()),
                    source: Some(Box::new(e)),
                }),
            }
        }
    }
}
