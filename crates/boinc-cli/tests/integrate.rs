//! End-to-end test of `boinc integrate` on Linux, sandboxed via XDG
//! environment overrides so nothing touches the real home directory.

#![cfg(target_os = "linux")]
#![allow(clippy::unwrap_used)]

use assert_cmd::Command;
use predicates::prelude::*;

fn boinc(data: &std::path::Path, config: &std::path::Path) -> Command {
    let mut cmd = Command::cargo_bin("boinc").unwrap();
    cmd.env("XDG_DATA_HOME", data)
        .env("XDG_CONFIG_HOME", config);
    cmd
}

#[test]
fn install_status_uninstall_cycle() {
    let data = tempfile::tempdir().unwrap();
    let config = tempfile::tempdir().unwrap();

    boinc(data.path(), config.path())
        .args(["integrate", "install"])
        .assert()
        .success()
        .stdout(predicate::str::contains("hook(s) installed"));

    // KDE service menu: MIME-scoped, executable, one file per source format.
    let kde = data.path().join("kio/servicemenus/boinc-png.desktop");
    assert!(kde.is_file(), "missing {}", kde.display());
    let body = std::fs::read_to_string(&kde).unwrap();
    assert!(body.contains("MimeType=image/png;"));
    assert!(body.contains("X-KDE-Submenu=Boinc"));
    assert!(body.contains("--to jpg %F"));
    use std::os::unix::fs::PermissionsExt as _;
    let mode = std::fs::metadata(&kde).unwrap().permissions().mode();
    assert_eq!(mode & 0o111, 0o111);

    // Nautilus script for the same conversion.
    let script = data.path().join("nautilus/scripts/PNG to JPG (Boinc)");
    assert!(script.is_file(), "missing {}", script.display());

    // Nemo (Cinnamon) action, MIME-scoped; label has no "(Boinc)" suffix —
    // grouping is done via actions-tree.json.
    let action = data
        .path()
        .join("nemo/actions/boinc-png-to-jpg.nemo_action");
    assert!(action.is_file(), "missing {}", action.display());
    let action_body = std::fs::read_to_string(&action).unwrap();
    assert!(action_body.contains("Mimetypes=image/png;"));
    assert!(action_body.contains("Name=Convert to JPG\n"));
    assert!(!action_body.contains("(Boinc)"));

    // Nemo 6+ layout nests every boinc action under a "Boinc" submenu.
    let layout_path = config.path().join("nemo/actions-tree.json");
    assert!(
        layout_path.is_file(),
        "missing Nemo layout {}",
        layout_path.display()
    );
    let layout: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(&layout_path).unwrap()).unwrap();
    let toplevel = layout["toplevel"].as_array().expect("toplevel array");
    let boinc_menu = toplevel
        .iter()
        .find(|n| n["uuid"] == "Boinc" && n["type"] == "submenu")
        .expect("Boinc submenu");
    let kids = boinc_menu["children"].as_array().expect("submenu children");
    assert!(
        kids.iter()
            .any(|k| k["uuid"] == "boinc-png-to-jpg.nemo_action"),
        "png→jpg action missing from Boinc submenu: {kids:?}"
    );

    // Manifest recorded in the config dir.
    assert!(config.path().join("boinc/integration.json").is_file());

    boinc(data.path(), config.path())
        .args(["integrate", "status"])
        .assert()
        .success()
        .stdout(predicate::str::contains("ok"))
        .stdout(predicate::str::contains("boinc-png.desktop"));

    boinc(data.path(), config.path())
        .args(["integrate", "uninstall"])
        .assert()
        .success()
        .stdout(predicate::str::contains("hook(s) removed"));

    assert!(!kde.exists(), "uninstall must remove {}", kde.display());
    assert!(!script.exists());
    assert!(!action.exists());
    assert!(!config.path().join("boinc/integration.json").exists());
    // Layout file is removed when it only held our Boinc submenu.
    assert!(
        !layout_path.exists(),
        "uninstall should clear empty Nemo layout"
    );

    boinc(data.path(), config.path())
        .args(["integrate", "status"])
        .assert()
        .success()
        .stdout(predicate::str::contains("not installed"));
}
