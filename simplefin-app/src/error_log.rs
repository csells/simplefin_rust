//! The app's centralized error log. Every failure the user sees is also
//! appended here (JSON lines in the app's file store) and to the platform log,
//! so problems can be reviewed and fixed later.

use serde::{Deserialize, Serialize};
use simplefin::FileStore;

use crate::files::SharedFiles;

const LOG_FILE: &str = "app_errors.jsonl";
/// Oldest entries are dropped beyond this, so the log can't grow without bound
/// (browser storage is small).
const MAX_ENTRIES: usize = 200;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ErrorEntry {
    /// Epoch seconds.
    pub timestamp: i64,
    /// What the app was doing, e.g. "sync" or "claim setup token".
    pub context: String,
    /// The full error chain.
    pub message: String,
}

#[derive(Clone)]
pub struct ErrorLog {
    files: SharedFiles,
}

impl ErrorLog {
    pub fn new(files: SharedFiles) -> Self {
        Self { files }
    }

    /// Logs `error` and returns the one-line message to show the user.
    pub fn record(&self, context: &str, error: &anyhow::Error) -> String {
        let message = format!("{error:#}");
        log::error!("{context}: {message}");
        let entry = ErrorEntry {
            timestamp: crate::jobs::now(),
            context: context.to_string(),
            message: message.clone(),
        };
        if let Err(write_err) = self.append(entry) {
            log::error!("could not write the error log: {write_err:#}");
        }
        message
    }

    /// All entries, oldest first. Unreadable lines are skipped.
    pub fn entries(&self) -> Vec<ErrorEntry> {
        self.files
            .read(LOG_FILE)
            .ok()
            .flatten()
            .map(|raw| raw.lines().filter_map(|line| serde_json::from_str(line).ok()).collect())
            .unwrap_or_default()
    }

    fn append(&self, entry: ErrorEntry) -> anyhow::Result<()> {
        let mut entries = self.entries();
        entries.push(entry);
        let skip = entries.len().saturating_sub(MAX_ENTRIES);
        let mut out = String::new();
        for entry in &entries[skip..] {
            out.push_str(&serde_json::to_string(entry)?);
            out.push('\n');
        }
        self.files.write(LOG_FILE, &out)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn log() -> ErrorLog {
        ErrorLog::new(SharedFiles::new(simplefin::MemoryFiles::new()))
    }

    #[test]
    fn records_full_chain_and_reads_back_in_order() {
        let log = log();
        let err = anyhow::anyhow!("connection refused").context("fetching accounts");
        assert_eq!(log.record("sync", &err), "fetching accounts: connection refused");
        log.record("claim", &anyhow::anyhow!("bad token"));
        let entries = log.entries();
        assert_eq!(entries.len(), 2);
        assert_eq!(entries[0].context, "sync");
        assert_eq!(entries[0].message, "fetching accounts: connection refused");
        assert_eq!(entries[1].context, "claim");
    }

    #[test]
    fn empty_log_has_no_entries() {
        assert!(log().entries().is_empty());
    }

    #[test]
    fn keeps_only_the_newest_entries() {
        let log = log();
        for i in 0..MAX_ENTRIES + 5 {
            log.record("sync", &anyhow::anyhow!("failure {i}"));
        }
        let entries = log.entries();
        assert_eq!(entries.len(), MAX_ENTRIES);
        assert_eq!(entries[0].message, "failure 5");
    }
}
