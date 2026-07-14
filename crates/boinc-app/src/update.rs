//! Startup update check and one-click self-update.
//!
//! On launch a background thread asks the portal for the latest release
//! (`/api/app-version` — an nginx proxy of the GitHub releases API with a
//! short cache), falling back to GitHub directly. When the release is newer
//! than this build, the user gets a notification and an in-window banner
//! whose button installs the platform-appropriate package: `pkexec
//! apt-get/dnf/zypper` for deb/rpm installs, `msiexec` on Windows, and the
//! mounted disk image on macOS. Installs are click-driven rather than fully
//! silent because Linux package files are root-owned — an unprompted polkit
//! password dialog at every login (the app autostarts) would be hostile.
//!
//! HTTP goes through the system `curl` (shipped with Linux distros, macOS,
//! and Windows 10+) so the app carries no TLS stack; without curl the check
//! silently skips. `BOINC_UPDATE_URL` overrides the endpoint, which also
//! forces the check in debug builds — that keeps the flow testable against a
//! local server.

use std::path::Path;
use std::process::Command;

use crate::state::UiMsg;

const UPDATE_URL: &str = "https://boinc.hideterms.com/api/app-version";
const FALLBACK_URL: &str = "https://api.github.com/repos/bartbeecoders/boinc/releases/latest";
const RELEASES_PAGE: &str = "https://github.com/bartbeecoders/boinc/releases/latest";

/// A newer release, as shown in the update banner.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct UpdateInfo {
    /// Bare version, e.g. "0.3.0".
    pub version: String,
    /// Release page for humans (used when no installable asset matches).
    pub release_url: String,
    /// Download URL of the asset matching this platform and install kind.
    pub asset_url: Option<String>,
    /// File name of that asset.
    pub asset_name: Option<String>,
}

/// Check for a newer release on a background thread; posts
/// [`UiMsg::UpdateAvailable`] and a notification when one exists.
pub fn spawn_check(ui_tx: crossbeam_channel::Sender<UiMsg>) {
    // Dev builds don't self-update; an explicit BOINC_UPDATE_URL still runs
    // the check so the flow can be exercised against a test server.
    if cfg!(debug_assertions) && std::env::var_os("BOINC_UPDATE_URL").is_none() {
        return;
    }
    std::thread::Builder::new()
        .name("boinc-update".into())
        .spawn(move || {
            let Some(info) = check_once() else { return };
            crate::queue::notify(
                &format!("Boinc v{} is available", info.version),
                "Open the Boinc window to install the update.",
            );
            let _ = ui_tx.send(UiMsg::UpdateAvailable(info));
        })
        .expect("update thread can be spawned");
}

fn check_once() -> Option<UpdateInfo> {
    let release = match std::env::var("BOINC_UPDATE_URL") {
        Ok(url) => fetch_json(&url)?,
        Err(_) => fetch_json(UPDATE_URL).or_else(|| fetch_json(FALLBACK_URL))?,
    };
    let tag = release["tag_name"].as_str()?;
    let remote = parse_version(tag)?;
    let current = parse_version(boinc_core::version())?;
    if remote <= current {
        return None;
    }

    let suffix = asset_suffix(install_kind());
    let asset = suffix.and_then(|suffix| {
        release["assets"].as_array()?.iter().find_map(|a| {
            let name = a["name"].as_str()?;
            if !name.ends_with(&suffix) {
                return None;
            }
            let url = a["browser_download_url"].as_str()?;
            Some((name.to_string(), url.to_string()))
        })
    });
    Some(UpdateInfo {
        version: format!("{}.{}.{}", remote.0, remote.1, remote.2),
        release_url: release["html_url"]
            .as_str()
            .unwrap_or(RELEASES_PAGE)
            .to_string(),
        asset_url: asset.as_ref().map(|(_, url)| url.clone()),
        asset_name: asset.map(|(name, _)| name),
    })
}

/// Download and install `info` on a background thread, reporting progress
/// back to the banner via [`UiMsg::UpdateStatus`].
pub fn spawn_install(info: UpdateInfo, ui_tx: crossbeam_channel::Sender<UiMsg>) {
    std::thread::Builder::new()
        .name("boinc-install".into())
        .spawn(move || {
            let report = |text: &str| {
                let _ = ui_tx.send(UiMsg::UpdateStatus(text.to_string()));
            };
            match install(&info, &report, &ui_tx) {
                Ok(done) => report(&done),
                Err(err) => {
                    report(&format!("Update failed: {err}"));
                    crate::queue::notify("Boinc update failed", &err);
                }
            }
        })
        .expect("install thread can be spawned");
}

fn install(
    info: &UpdateInfo,
    report: &dyn Fn(&str),
    ui_tx: &crossbeam_channel::Sender<UiMsg>,
) -> Result<String, String> {
    let (Some(url), Some(name)) = (&info.asset_url, &info.asset_name) else {
        // Nothing installable for this build (e.g. running from source):
        // hand the user the release page instead.
        open_url(&info.release_url);
        return Ok("See the releases page for the new version.".into());
    };

    report("Downloading update…");
    let path = std::env::temp_dir().join(name);
    let fetched = Command::new("curl")
        .args(["-fsSL", "--max-time", "300", "-o"])
        .arg(&path)
        .args(["-H", "User-Agent: boinc-app", url])
        .status()
        .map_err(|e| format!("could not run curl: {e}"))?;
    if !fetched.success() {
        return Err(format!("download failed ({fetched})"));
    }

    report("Installing…");
    match install_kind() {
        InstallKind::Deb => {
            run_installer("pkexec", &["apt-get", "install", "-y"], &path)?;
            relaunch(ui_tx);
            Ok(format!("Updated to v{} — restarting…", info.version))
        }
        InstallKind::Rpm => {
            let tool: &[&str] = if have("dnf") {
                &["dnf", "install", "-y"]
            } else {
                &[
                    "zypper",
                    "--non-interactive",
                    "install",
                    "--allow-unsigned-rpm",
                ]
            };
            run_installer("pkexec", tool, &path)?;
            relaunch(ui_tx);
            Ok(format!("Updated to v{} — restarting…", info.version))
        }
        InstallKind::Msi => {
            // msiexec replaces the per-user install; quit so our files are
            // not locked while it runs.
            Command::new("msiexec")
                .arg("/i")
                .arg(&path)
                .arg("/passive")
                .spawn()
                .map_err(|e| format!("could not start msiexec: {e}"))?;
            let _ = ui_tx.send(UiMsg::Quit);
            Ok("Installer started".into())
        }
        InstallKind::Dmg => {
            Command::new("open")
                .arg(&path)
                .status()
                .map_err(|e| format!("could not open the disk image: {e}"))?;
            Ok("Disk image opened — drag Boinc to Applications.".into())
        }
        InstallKind::None => {
            open_url(&info.release_url);
            Ok("See the releases page for the new version.".into())
        }
    }
}

/// `pkexec <tool…> <package>`, surfacing stderr on failure (covers both a
/// dismissed password prompt and a real package-manager error).
fn run_installer(elevate: &str, tool: &[&str], package: &Path) -> Result<(), String> {
    let out = Command::new(elevate)
        .args(tool)
        .arg(package)
        .output()
        .map_err(|e| format!("could not run {elevate}: {e}"))?;
    if out.status.success() {
        Ok(())
    } else {
        let stderr = String::from_utf8_lossy(&out.stderr);
        Err(format!("{}; {}", out.status, stderr.trim()))
    }
}

/// Start a replacement instance once this one has quit. The single-instance
/// socket only frees on exit, so the relaunch waits behind a short sleep in
/// a detached shell.
fn relaunch(ui_tx: &crossbeam_channel::Sender<UiMsg>) {
    if let Ok(exe) = std::env::current_exe() {
        let _ = Command::new("sh")
            .arg("-c")
            .arg(format!("sleep 2; exec \"{}\"", exe.display()))
            .spawn();
    }
    let _ = ui_tx.send(UiMsg::Quit);
}

/// How this Boinc was installed — decides the asset to fetch and the tool
/// that installs it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum InstallKind {
    Deb,
    Rpm,
    Msi,
    Dmg,
    /// Not a packaged install (source build): no self-update.
    None,
}

fn install_kind() -> InstallKind {
    if cfg!(windows) {
        return InstallKind::Msi;
    }
    if cfg!(target_os = "macos") {
        return InstallKind::Dmg;
    }
    if cmd_ok("dpkg", &["-s", "boinc"]) {
        InstallKind::Deb
    } else if cmd_ok("rpm", &["-q", "boinc"]) {
        InstallKind::Rpm
    } else {
        InstallKind::None
    }
}

/// Release-asset suffix for this platform/arch, matching the names the
/// release workflow produces.
fn asset_suffix(kind: InstallKind) -> Option<String> {
    let arch = std::env::consts::ARCH; // "x86_64" | "aarch64"
    match kind {
        InstallKind::Deb => {
            let deb_arch = if arch == "aarch64" { "arm64" } else { "amd64" };
            Some(format!("_{deb_arch}.deb"))
        }
        InstallKind::Rpm => Some(format!(".{arch}.rpm")),
        InstallKind::Msi => Some(format!("-{arch}.msi")),
        InstallKind::Dmg => Some(".dmg".into()),
        InstallKind::None => None,
    }
}

/// "v1.2.3" or "1.2.3" → (1, 2, 3).
fn parse_version(text: &str) -> Option<(u64, u64, u64)> {
    let mut parts = text.trim().trim_start_matches('v').splitn(3, '.');
    let mut next = || parts.next()?.parse::<u64>().ok();
    Some((next()?, next()?, next()?))
}

fn fetch_json(url: &str) -> Option<serde_json::Value> {
    let out = Command::new("curl")
        .args([
            "-fsSL",
            "--max-time",
            "15",
            "-H",
            "User-Agent: boinc-app",
            url,
        ])
        .output()
        .ok()?;
    if !out.status.success() {
        return None;
    }
    serde_json::from_slice(&out.stdout).ok()
}

fn cmd_ok(cmd: &str, args: &[&str]) -> bool {
    Command::new(cmd)
        .args(args)
        .output()
        .is_ok_and(|out| out.status.success())
}

fn have(cmd: &str) -> bool {
    cmd_ok(cmd, &["--version"])
}

fn open_url(url: &str) {
    let opener = if cfg!(windows) {
        ("cmd", vec!["/c", "start", "", url])
    } else if cfg!(target_os = "macos") {
        ("open", vec![url])
    } else {
        ("xdg-open", vec![url])
    };
    let _ = Command::new(opener.0).args(opener.1).spawn();
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;

    #[test]
    fn version_parsing_and_ordering() {
        assert_eq!(parse_version("v0.2.0"), Some((0, 2, 0)));
        assert_eq!(parse_version("1.12.3"), Some((1, 12, 3)));
        assert_eq!(parse_version("v0.2"), None);
        assert_eq!(parse_version("nightly"), None);
        // Tuple ordering is the semver ordering for numeric triples.
        assert!(parse_version("v0.10.0") > parse_version("v0.9.9"));
        assert!(parse_version("v1.0.0") > parse_version("v0.99.0"));
    }

    #[test]
    fn asset_suffixes_match_release_artifact_names() {
        // Names as the release workflow produces them, e.g.
        // boinc_0.2.0-1_arm64.deb / boinc-0.2.0-1.aarch64.rpm.
        let arch = std::env::consts::ARCH;
        assert_eq!(
            asset_suffix(InstallKind::Deb).unwrap(),
            if arch == "aarch64" {
                "_arm64.deb"
            } else {
                "_amd64.deb"
            }
        );
        assert_eq!(
            asset_suffix(InstallKind::Rpm).unwrap(),
            format!(".{arch}.rpm")
        );
        assert!(asset_suffix(InstallKind::Msi).unwrap().ends_with(".msi"));
        assert_eq!(asset_suffix(InstallKind::None), None);
    }
}
