//! Manifest of everything `install` created, so `uninstall` and `status`
//! operate on exactly what we wrote (and nothing else).

use std::path::PathBuf;

use serde::{Deserialize, Serialize};

/// One installed hook. Filesystem paths cover Linux/macOS; registry keys are
/// Windows-only and stored as `HKCU\...` strings.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "kind", rename_all = "lowercase")]
pub enum Hook {
    File { path: PathBuf },
    RegistryKey { key: String },
}

impl Hook {
    pub fn describe(&self) -> String {
        match self {
            Hook::File { path } => path.display().to_string(),
            Hook::RegistryKey { key } => format!("registry: {key}"),
        }
    }
}

fn manifest_path() -> Option<PathBuf> {
    directories::ProjectDirs::from("com", "hideterms", "boinc")
        .map(|dirs| dirs.config_dir().join("integration.json"))
}

pub fn load() -> Vec<Hook> {
    let Some(path) = manifest_path() else {
        return Vec::new();
    };
    std::fs::read(&path)
        .ok()
        .and_then(|bytes| serde_json::from_slice(&bytes).ok())
        .unwrap_or_default()
}

pub fn save(hooks: &[Hook]) -> std::io::Result<()> {
    let Some(path) = manifest_path() else {
        return Err(std::io::Error::other("no config directory available"));
    };
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir)?;
    }
    std::fs::write(path, serde_json::to_vec_pretty(hooks)?)
}

pub fn clear() -> std::io::Result<()> {
    let Some(path) = manifest_path() else {
        return Ok(());
    };
    match std::fs::remove_file(path) {
        Err(err) if err.kind() != std::io::ErrorKind::NotFound => Err(err),
        _ => Ok(()),
    }
}
