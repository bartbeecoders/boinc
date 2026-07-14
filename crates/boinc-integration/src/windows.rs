//! Windows integration: cascading context-menu verbs in the per-user
//! registry (no COM shell extension — plan.md task 4.3).
//!
//! Layout, per extension (e.g. `.png`):
//!
//! ```text
//! HKCU\Software\Classes\SystemFileAssociations\.png\shell\Boinc
//!     MUIVerb    = "Boinc"
//!     SubCommands = ""            (empty → read nested shell\ subkeys)
//!     shell\png-to-jpg
//!         (default) = "Convert to JPG"
//!         command\(default) = "C:\...\boinc.exe" convert "%1" --to jpg
//! ```
//!
//! Autostart uses the classic `HKCU\...\CurrentVersion\Run` value.

use std::io;
use std::path::Path;

use winreg::RegKey;
use winreg::enums::HKEY_CURRENT_USER;

use crate::entries::{MenuEntry, windows_extensions};
use crate::manifest::Hook;

const RUN_KEY: &str = r"Software\Microsoft\Windows\CurrentVersion\Run";

/// Create the cascading menu for one source format. Returns the registry
/// keys created (for the manifest).
pub fn install_menus(
    cli: &Path,
    from: boinc_core::Format,
    entries: &[MenuEntry],
) -> io::Result<Vec<Hook>> {
    let hkcu = RegKey::predef(HKEY_CURRENT_USER);
    let mut hooks = Vec::new();

    for ext in windows_extensions(from) {
        let base = format!(r"Software\Classes\SystemFileAssociations\{ext}\shell\Boinc");
        let (boinc_key, _) = hkcu.create_subkey(&base)?;
        boinc_key.set_value("MUIVerb", &"Boinc")?;
        boinc_key.set_value("SubCommands", &"")?;

        for entry in entries {
            let (verb_key, _) = boinc_key.create_subkey(format!(r"shell\{}", entry.id()))?;
            verb_key.set_value("", &entry.label())?;
            let (cmd_key, _) = verb_key.create_subkey("command")?;
            cmd_key.set_value(
                "",
                &format!(
                    "\"{}\" convert --app \"%1\" --to {}",
                    cli.display(),
                    entry.to.extension()
                ),
            )?;
        }
        hooks.push(Hook::RegistryKey {
            key: format!(r"HKCU\{base}"),
        });
    }
    Ok(hooks)
}

/// Delete a key recorded in the manifest (`HKCU\...`).
pub fn remove_key(key: &str) -> io::Result<()> {
    let Some(path) = key.strip_prefix(r"HKCU\") else {
        return Err(io::Error::other(format!(
            "unsupported registry root in {key:?}"
        )));
    };
    let hkcu = RegKey::predef(HKEY_CURRENT_USER);
    match hkcu.delete_subkey_all(path) {
        Err(err) if err.kind() != io::ErrorKind::NotFound => Err(err),
        _ => Ok(()),
    }
}

pub fn key_exists(key: &str) -> bool {
    key.strip_prefix(r"HKCU\")
        .is_some_and(|path| RegKey::predef(HKEY_CURRENT_USER).open_subkey(path).is_ok())
}

pub fn set_autostart(enabled: bool, app: &Path) -> io::Result<()> {
    let hkcu = RegKey::predef(HKEY_CURRENT_USER);
    let (run, _) = hkcu.create_subkey(RUN_KEY)?;
    if enabled {
        run.set_value("Boinc", &format!("\"{}\"", app.display()))
    } else {
        match run.delete_value("Boinc") {
            Err(err) if err.kind() != io::ErrorKind::NotFound => Err(err),
            _ => Ok(()),
        }
    }
}
