# Boinc

Convert files from one format to another — from the file browser's
right-click menu, a tray application, or the command line.

Supported conversions: **PNG ↔ JPG** out of the box; **PDF ↔ DOCX** when
[LibreOffice](https://www.libreoffice.org/) is installed. The conversion
engine is pluggable — see `crates/boinc-core/README.md` for how to add a
format pair.

Downloads: https://boinc.hideterms.com

## Building

```sh
cargo build --workspace          # needs GTK3 dev libraries on Linux
cargo test --workspace
scripts/dev.sh                   # run the tray app
cargo run -p boinc-cli -- --help # the `boinc` CLI
```

## Packages

- **Linux:** `cargo deb -p boinc-app` / `cargo generate-rpm -p crates/boinc-app`
  (after `cargo build --release -p boinc-app -p boinc-cli`)
- **Windows:** `cargo wix -p boinc-app` (WiX 3 toolset required)
- **macOS:** `scripts/package-macos.sh` → `dist/Boinc-<version>.dmg`

Context menus are installed per-user on the app's first run (or manually via
`boinc integrate install`); `boinc integrate uninstall` removes them.

## Releasing

Bump `workspace.package.version` in `Cargo.toml`, update `CHANGELOG.md`, and
push a `v<version>` tag. The release workflow builds deb/rpm/msi/dmg
artifacts and attaches them to the GitHub release. In-app auto-update is
deliberately out of scope for v1 — the download portal announces new
versions instead.
