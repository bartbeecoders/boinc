# Changelog

All notable changes to Boinc are documented here. The format follows
[Keep a Changelog](https://keepachangelog.com/en/1.1.0/) and versions follow
[Semantic Versioning](https://semver.org/).

Releases are cut by bumping `workspace.package.version` in `Cargo.toml`,
moving the Unreleased notes under the new version heading, and pushing a
`v<version>` tag — the release workflow builds and attaches deb/rpm/msi/dmg
artifacts.

## [Unreleased]

## [0.4.1] - 2026-07-15

### Added
- GitHub self-hosted runner package under `runner/` (host systemd install
  recommended, optional K3s manifests, setup docs).

### Changed
- Cinnamon/Nemo context menus: conversions are grouped under a single
  **Boinc** submenu (via Nemo 6 `actions-tree.json`) instead of one top-level
  item per conversion. Re-run `boinc integrate install` to refresh.

### Fixed
- CI `cargo-deny`: ignore unmaintained transitive crates pulled in by
  vtracer 0.6 (`adler`, `ansi_term`, `atty`).
- `cargo fmt` cleanups so CI format checks pass.

## [0.4.0] - 2026-07-15

### Added
- BMP, GIF, and WebP as first-class formats: convert among all raster types
  (PNG, JPG, BMP, GIF, WebP — any ↔ any) and vectorize any of them to SVG via
  [vtracer](https://github.com/visioncortex/vtracer) (in-process; no external
  tools). Re-run `boinc integrate install` (or launch the app) to refresh
  context-menu entries.
- Sample files under `examples/` for manual/CLI smoke tests (see
  `examples/README.md` for sources).
- Portal “What it converts” section updated for the new image and SVG pairs.

## [0.3.1] - 2026-07-14

### Fixed
- Drag-and-drop into the app window did nothing on Wayland sessions (e.g.
  Fedora Asahi Remix): the windowing backend only delivers file-drop events
  on X11, so the app now runs under XWayland when an X11 path is available.
  Pure Wayland sessions without XWayland keep the Wayland window (drops
  still unavailable there).

## [0.3.0] - 2026-07-14

### Added
- The app shows its version in the window header and tray tooltip.
- Update check on startup (disable in Settings): the app asks
  `boinc.hideterms.com/api/app-version` (a cached proxy of the GitHub
  releases API) for the latest release and, when newer, shows a notification
  plus an in-window banner whose button downloads and installs the matching
  package — `pkexec apt-get/dnf/zypper` for deb/rpm installs, `msiexec` on
  Windows, the mounted disk image on macOS. Source builds are pointed at the
  releases page instead.

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
