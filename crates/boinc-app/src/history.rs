//! Conversion history, persisted as JSON in the platform data directory.

use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};

const MAX_ENTRIES: usize = 100;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct HistoryEntry {
    pub input: PathBuf,
    pub output: Option<PathBuf>,
    pub to: String,
    pub success: bool,
    pub error: Option<String>,
    /// Unix timestamp (seconds).
    pub timestamp: u64,
}

impl HistoryEntry {
    pub fn now(input: PathBuf, output: Option<PathBuf>, to: String, error: Option<String>) -> Self {
        Self {
            input,
            output,
            to,
            success: error.is_none(),
            error,
            timestamp: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .map(|d| d.as_secs())
                .unwrap_or(0),
        }
    }
}

fn history_path() -> Option<PathBuf> {
    directories::ProjectDirs::from("com", "hideterms", "boinc")
        .map(|dirs| dirs.data_dir().join("history.json"))
}

pub fn load() -> Vec<HistoryEntry> {
    let Some(path) = history_path() else {
        return Vec::new();
    };
    std::fs::read(&path)
        .ok()
        .and_then(|bytes| serde_json::from_slice(&bytes).ok())
        .unwrap_or_default()
}

/// Append an entry, keeping only the newest `MAX_ENTRIES`. Persistence is
/// best-effort: failures are logged, never fatal.
pub fn record(entry: HistoryEntry) {
    let Some(path) = history_path() else {
        return;
    };
    let mut entries = load();
    entries.push(entry);
    if entries.len() > MAX_ENTRIES {
        let excess = entries.len() - MAX_ENTRIES;
        entries.drain(..excess);
    }
    let write = || -> std::io::Result<()> {
        if let Some(dir) = path.parent() {
            std::fs::create_dir_all(dir)?;
        }
        std::fs::write(&path, serde_json::to_vec_pretty(&entries)?)
    };
    if let Err(err) = write() {
        eprintln!("boinc: could not save history: {err}");
    }
}
