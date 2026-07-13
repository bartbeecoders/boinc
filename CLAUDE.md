# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Status

The product spec lives in `Vibecoding/Instructions.md`; the phased implementation plan (with task checkboxes and key risk decisions) lives in `plan.md`. Phases 0–1 are done: the core engine with all four converters works via the library API. CLI subcommands (Phase 2), the Floem UI (Phase 3), and OS integration (Phase 4) are not yet implemented.

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
scripts/dev.sh                                       # same, from anywhere; extra args go to the app
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
