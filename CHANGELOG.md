# Changelog

All notable changes to Boinc are documented here. The format follows
[Keep a Changelog](https://keepachangelog.com/en/1.1.0/) and versions follow
[Semantic Versioning](https://semver.org/).

Releases are cut by bumping `workspace.package.version` in `Cargo.toml`,
moving the Unreleased notes under the new version heading, and pushing a
`v<version>` tag — the release workflow builds and attaches deb/rpm/msi/dmg
artifacts.

## [Unreleased]

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
