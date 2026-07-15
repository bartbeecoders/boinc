//! Per-platform installation of file-browser context menus and
//! launch-at-login hooks (plan.md Phase 4).
//!
//! Menu entries are generated from the converter registry, so a new
//! converter shows up in context menus after re-running
//! `boinc integrate install`. Everything created is recorded in a manifest
//! (`integration.json` in the config dir); `uninstall` and `status` operate
//! only on manifest entries.

use std::collections::BTreeMap;
use std::io;
use std::path::Path;

use boinc_core::{ConverterRegistry, Format};

pub mod entries;
pub mod ipc;
mod manifest;

#[cfg(target_os = "linux")]
mod linux;
#[cfg(target_os = "macos")]
mod macos;
#[cfg(windows)]
mod windows;

pub use entries::{MenuEntry, menu_entries};
pub use manifest::Hook;

/// What an install/uninstall run did.
#[derive(Debug, Default)]
pub struct Report {
    pub created: Vec<String>,
    pub removed: Vec<String>,
}

/// Install context menus for every currently-available conversion. `cli` is
/// the absolute path to the `boinc` binary the menus will invoke.
/// Idempotent: existing hooks are overwritten and the manifest replaced.
pub fn install(registry: &ConverterRegistry, cli: &Path) -> io::Result<Report> {
    let entries = menu_entries(registry);
    let mut by_from: BTreeMap<Format, Vec<MenuEntry>> = BTreeMap::new();
    for entry in &entries {
        by_from.entry(entry.from).or_default().push(*entry);
    }

    let hooks = install_platform(&by_from, cli)?;
    manifest::save(&hooks)?;

    Ok(Report {
        created: hooks.iter().map(Hook::describe).collect(),
        removed: Vec::new(),
    })
}

/// Remove everything the manifest says we created.
pub fn uninstall() -> io::Result<Report> {
    let mut report = Report::default();
    for hook in manifest::load() {
        let result = match &hook {
            Hook::File { path } => {
                // Quick Action bundles on macOS are directories.
                let removal = if path.is_dir() {
                    std::fs::remove_dir_all(path)
                } else {
                    std::fs::remove_file(path)
                };
                match removal {
                    Err(err) if err.kind() != io::ErrorKind::NotFound => Err(err),
                    _ => Ok(()),
                }
            }
            Hook::RegistryKey { key } => remove_registry_key(key),
        };
        match result {
            Ok(()) => report.removed.push(hook.describe()),
            Err(err) => eprintln!("boinc: could not remove {}: {err}", hook.describe()),
        }
    }
    // Nemo layout is not a simple "we own this file" hook — merge carefully.
    #[cfg(target_os = "linux")]
    if let Err(err) = linux::clear_nemo_submenu() {
        eprintln!("boinc: could not clear Nemo Boinc submenu: {err}");
    }
    manifest::clear()?;
    Ok(report)
}

/// Each manifest entry and whether it is still present.
pub fn status() -> Vec<(String, bool)> {
    manifest::load()
        .into_iter()
        .map(|hook| {
            let present = match &hook {
                Hook::File { path } => path.exists(),
                Hook::RegistryKey { key } => registry_key_exists(key),
            };
            (hook.describe(), present)
        })
        .collect()
}

/// Register (or unregister) the tray app to start at login.
pub fn set_autostart(enabled: bool, app: &Path) -> io::Result<()> {
    #[cfg(target_os = "linux")]
    {
        let dir = linux::autostart_dir()
            .ok_or_else(|| io::Error::other("no home directory available"))?;
        if enabled {
            linux::autostart_file(&dir, app).map(|_| ())
        } else {
            match std::fs::remove_file(dir.join("boinc.desktop")) {
                Err(err) if err.kind() != io::ErrorKind::NotFound => Err(err),
                _ => Ok(()),
            }
        }
    }
    #[cfg(windows)]
    {
        windows::set_autostart(enabled, app)
    }
    #[cfg(target_os = "macos")]
    {
        let dir = macos::launch_agents_dir()
            .ok_or_else(|| io::Error::other("no home directory available"))?;
        if enabled {
            macos::launch_agent(&dir, app).map(|_| ())
        } else {
            match std::fs::remove_file(dir.join("com.hideterms.boinc.plist")) {
                Err(err) if err.kind() != io::ErrorKind::NotFound => Err(err),
                _ => Ok(()),
            }
        }
    }
    #[cfg(not(any(target_os = "linux", windows, target_os = "macos")))]
    {
        let _ = (enabled, app);
        Err(io::Error::other(
            "autostart is not supported on this platform",
        ))
    }
}

#[cfg(target_os = "linux")]
fn install_platform(
    by_from: &BTreeMap<Format, Vec<MenuEntry>>,
    cli: &Path,
) -> io::Result<Vec<Hook>> {
    let mut hooks = Vec::new();
    let kde = linux::kde_dir().ok_or_else(|| io::Error::other("no home directory available"))?;
    let nautilus =
        linux::nautilus_dir().ok_or_else(|| io::Error::other("no home directory available"))?;
    let nemo = linux::nemo_dir().ok_or_else(|| io::Error::other("no home directory available"))?;

    let mut all_entries: Vec<MenuEntry> = Vec::new();
    for (from, entries) in by_from {
        let path = linux::kde_service_menu(&kde, cli, *from, entries)?;
        hooks.push(Hook::File { path });
        for entry in entries {
            let path = linux::nautilus_script(&nautilus, cli, entry)?;
            hooks.push(Hook::File { path });
            let path = linux::nemo_action(&nemo, cli, entry)?;
            hooks.push(Hook::File { path });
            all_entries.push(*entry);
        }
    }
    // Nemo 6+: nest every boinc-*.nemo_action under a single "Boinc" submenu
    // via actions-tree.json (merges with any existing user layout).
    linux::sync_nemo_submenu(&all_entries)?;
    Ok(hooks)
}

#[cfg(windows)]
fn install_platform(
    by_from: &BTreeMap<Format, Vec<MenuEntry>>,
    cli: &Path,
) -> io::Result<Vec<Hook>> {
    let mut hooks = Vec::new();
    for (from, entries) in by_from {
        hooks.extend(windows::install_menus(cli, *from, entries)?);
    }
    Ok(hooks)
}

#[cfg(target_os = "macos")]
fn install_platform(
    by_from: &BTreeMap<Format, Vec<MenuEntry>>,
    cli: &Path,
) -> io::Result<Vec<Hook>> {
    let mut hooks = Vec::new();
    let services =
        macos::services_dir().ok_or_else(|| io::Error::other("no home directory available"))?;
    for entries in by_from.values() {
        for entry in entries {
            let path = macos::quick_action(&services, cli, entry)?;
            hooks.push(Hook::File { path });
        }
    }
    Ok(hooks)
}

#[cfg(not(any(target_os = "linux", windows, target_os = "macos")))]
fn install_platform(
    _by_from: &BTreeMap<Format, Vec<MenuEntry>>,
    _cli: &Path,
) -> io::Result<Vec<Hook>> {
    Err(io::Error::other(
        "context-menu integration is not supported on this platform",
    ))
}

fn remove_registry_key(key: &str) -> io::Result<()> {
    #[cfg(windows)]
    {
        windows::remove_key(key)
    }
    #[cfg(not(windows))]
    {
        let _ = key;
        Ok(())
    }
}

fn registry_key_exists(key: &str) -> bool {
    #[cfg(windows)]
    {
        windows::key_exists(key)
    }
    #[cfg(not(windows))]
    {
        let _ = key;
        false
    }
}
