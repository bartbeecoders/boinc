# Changelog

All notable changes to Boinc are documented here. The format follows
[Keep a Changelog](https://keepachangelog.com/en/1.1.0/) and versions follow
[Semantic Versioning](https://semver.org/).

Releases are cut by bumping `workspace.package.version` in `Cargo.toml`,
moving the Unreleased notes under the new version heading, and pushing a
`v<version>` tag — the release workflow builds and attaches deb/rpm/msi/dmg
artifacts.

## [Unreleased]

## [0.2.0] - 2026-07-14

### Added
- PDF ↔ Markdown conversion. PDF → Markdown extracts the text layer
  in-process (no external tools); Markdown → PDF renders through headless
  LibreOffice like the DOCX conversions. Existing installs need a
  `boinc integrate install` re-run (or an app launch) to grow the new
  context-menu entries.
- The tray icon now shows a spinning arc while a conversion is running.
- `boinc convert --app`: queue the conversion in the running Boinc app
  (tray progress, notifications) instead of converting in-process; falls
  back to a local conversion when the app is not running.

- Linux aarch64 packages (`.aarch64.rpm`, `_arm64.deb`), built natively on
  arm64 runners — covers Fedora Asahi Remix on Apple Silicon. The portal
  offers the aarch64 RPM on a dedicated download card, and CI now builds and
  tests on arm64 Linux.

### Changed
- Context-menu hooks now invoke `boinc convert --app`, so right-click
  conversions run through the app's queue when it is open — with tray
  progress and completion notifications — instead of in a detached CLI
  process. Re-run `boinc integrate install` to update existing hooks.

### Fixed
- The `.deb` now declares its `libxdo3` and `libxkbcommon0` runtime
  dependencies (and accepts Ubuntu 24.04's `libgtk-3-0t64`), so installs on
  clean systems no longer produce an app that fails to start. The RPM already
  carried the equivalent soname requires automatically.

## [0.1.0] - 2026-07-13

### Added
- Core conversion engine with pluggable converter registry: PNG ↔ JPG
  (image crate) and PDF ↔ DOCX (via headless LibreOffice when installed).
- `boinc` CLI: `convert` (batch-capable, `--json` mode), `list-conversions`,
  `integrate install/uninstall/status`.
- Tray application with drop-to-convert window, conversion queue,
  notifications, settings, and single-instance IPC.
- File-browser context menus: KDE service menus, Nautilus scripts, and Nemo
  actions (Linux), cascading registry verbs (Windows), Finder Quick Actions
  (macOS).
- Launch-at-login registration on all three platforms.
- Web portal (React + Vite) with OS-detected downloads resolved from the
  GitHub releases API.
