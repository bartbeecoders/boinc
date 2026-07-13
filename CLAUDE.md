# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Status

The product spec lives in `Vibecoding/Instructions.md`; the phased implementation plan (with task checkboxes and key risk decisions) lives in `plan.md`. Phases 0–5 are done: the core engine with all four converters works via the library API, the `boinc` CLI (`convert`, `list-conversions`, `integrate install/uninstall/status`, `--json` mode; exit codes 0/1/2 = success/failure/usage), the Floem tray app (drop-to-convert window, worker-thread queue, notifications, settings, single-instance IPC), OS context-menu integration (KDE service menus + Nautilus scripts + Nemo/Cinnamon actions on Linux — verified; Windows registry verbs and macOS Quick Actions — compile-checked only), packaging (deb/rpm verified locally, WiX MSI + dmg script CI-only, tag-triggered `release.yml`), and the web portal (`site/` — React 19 + Vite; `npm run dev`/`build` in `site/`; deployed to the VPS K3S cluster via `scripts/deploy-k3s.sh` — podman image from `site/Dockerfile` → ACR registry → `k8s/boinc/` manifests, NodePort 32087, fronted by an owner-managed Cloudflare Tunnel for boinc.hideterms.com; runbook in `Vibecoding/deploy.md`; download links resolve from the GitHub releases API at page load, so releases need no site redeploy). Next: hardening (Phase 7).

Packaging facts: packaging metadata lives in `crates/boinc-app/Cargo.toml` (`cargo deb -p boinc-app`, `cargo generate-rpm -p crates/boinc-app`, `cargo wix -p boinc-app`); the app installs per-user context menus on first run (manifest-guarded), so installers have no post-install hooks. Releases: bump workspace version, update CHANGELOG.md, push `v*` tag. macOS artifacts are unsigned until an Apple Developer account is wired in.

Integration facts: `boinc integrate install` writes hooks for *currently available* conversions only and records them in a manifest (`~/.config/boinc/integration.json`); uninstall/status operate solely on the manifest. Hooks invoke the absolute path of the `boinc` binary that ran install (dev installs point at `target/debug/boinc`). The integrate CLI test sandboxes via `XDG_DATA_HOME`/`XDG_CONFIG_HOME` overrides.

App architecture facts: the worker thread owns the job list and streams `UiMsg::Jobs` snapshots over a crossbeam channel bridged into a floem signal (`create_signal_from_channel`). The tray runs on a dedicated GTK thread on Linux, on the main thread on Windows/macOS. Floem 0.2 exits when the last window closes — closing the window quits the app (v1 limitation, see `crates/boinc-app/src/tray.rs`). A second app instance forwards `{"cmd":"open"|"pick"|"convert"}` JSON lines over the local socket `boinc.sock` and exits; Linux CI builds need `libgtk-3-dev`.

Key engine facts: conversions are `Converter` trait impls in a `ConverterRegistry` keyed by (from, to) `Format` pair — see `crates/boinc-core/README.md` for how to add one. PDF↔DOCX delegates to headless LibreOffice (`soffice`) found via `BOINC_SOFFICE` env → PATH → known paths; those converters report `is_available() == false` without it, and their tests self-skip. Format detection trusts magic bytes over extensions. Outputs are never silently overwritten (` (1)` suffix policy).

## Commands

```sh
cargo build --workspace                              # build everything
cargo test --workspace                               # run all tests
cargo test -p boinc-core <test_name>                 # run a single test
cargo clippy --workspace --all-targets -- -D warnings  # lint (CI-enforced)
cargo fmt --all                                      # format
cargo deny check                                     # license/advisory audit (needs cargo-deny installed; CI runs it)
cargo run -p boinc-cli                               # run the CLI (binary is named `boinc`)
cargo run -p boinc-app                               # run the tray app
scripts/dev.sh                                       # tray app + portal dev server + context-menu hooks for the dev build
```

CI (`.github/workflows/ci.yml`) runs fmt/clippy/build/test on Linux, Windows, and macOS — clippy warnings are errors there.

## Workspace Layout

Cargo workspace under `crates/`, split so the engine stays UI- and OS-agnostic:

- `boinc-core` — converter trait, registry, conversion pipeline. Must never depend on Floem or platform integration code.
- `boinc-cli` — CLI over core (binary named `boinc`); also the entry point OS context menus invoke.
- `boinc-app` — Floem UI + tray application.
- `boinc-integration` — per-platform install/uninstall of context-menu and service hooks; menu entries are generated from the converter registry.

Shared version/edition/lints come from `[workspace.*]` in the root `Cargo.toml`; crates opt in via `workspace = true`. `clippy::unwrap_used` is warn — prefer `?`/`expect` with a reason.

## What Boinc Is

Boinc is a cross-platform (Linux, Windows, macOS) file conversion utility. The intended user flow:

1. User selects a file in the OS file browser; a Boinc submenu appears in the context menu.
2. User picks a conversion option and the file is converted.
3. The app runs as an OS extension/service with a tray icon.

A companion web portal (landing page at boinc.hideterms.com) will host app downloads.

### Initial conversions to support

- PDF ↔ DOCX
- PNG ↔ JPG

The conversion system must be easily extensible so new format pairs can be added — design it around a pluggable converter abstraction rather than hardcoded pairs.

## Architecture Decisions (from the spec)

- **Language:** Rust
- **UI:** Floem (https://lap.dev/floem/) — minimal UI
- **Distribution:** installable as an OS extension/service (context-menu integration + tray application), which will require per-platform integration work (e.g. Windows shell extensions, Linux file-manager integration, macOS Finder extensions)
