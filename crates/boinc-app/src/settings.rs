//! App settings, persisted as JSON in the platform config directory.

use std::path::PathBuf;

use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default)]
pub struct Settings {
    /// Where converted files go; `None` means next to each input.
    pub output_dir: Option<PathBuf>,
    /// Default JPEG quality, 1–100.
    pub jpeg_quality: u8,
    /// Stored preference; actual login-item registration is done by the
    /// integration layer (plan.md Phase 4.5).
    pub launch_at_login: bool,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            output_dir: None,
            jpeg_quality: 90,
            launch_at_login: false,
        }
    }
}

fn config_path() -> Option<PathBuf> {
    directories::ProjectDirs::from("com", "hideterms", "boinc")
        .map(|dirs| dirs.config_dir().join("settings.json"))
}

impl Settings {
    /// Load from disk, falling back to defaults (missing or corrupt file).
    pub fn load() -> Self {
        let Some(path) = config_path() else {
            return Self::default();
        };
        std::fs::read(&path)
            .ok()
            .and_then(|bytes| serde_json::from_slice(&bytes).ok())
            .unwrap_or_default()
    }

    pub fn save(&self) -> std::io::Result<()> {
        let Some(path) = config_path() else {
            return Err(std::io::Error::other("no config directory available"));
        };
        if let Some(dir) = path.parent() {
            std::fs::create_dir_all(dir)?;
        }
        let json = serde_json::to_vec_pretty(self)?;
        std::fs::write(path, json)
    }
}
