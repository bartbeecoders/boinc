# Boinc — Implementation Plan

A cross-platform (Linux/Windows/macOS) file conversion utility written in Rust with a Floem UI, installed as an OS service with tray icon and file-browser context-menu integration, plus a download portal at boinc.hideterms.com.

Source spec: `Vibecoding/Instructions.md`

---

## Guiding architecture

- **Cargo workspace** with separate crates so the engine stays UI- and OS-agnostic:
  - `boinc-core` — converter trait, registry, conversion pipeline (no UI deps)
  - `boinc-cli` — thin CLI over core; also the entry point the OS context menus invoke
  - `boinc-app` — Floem UI + tray application (settings, progress, history)
  - `boinc-integration` — per-platform install/uninstall of context-menu and service hooks
- **Extensibility first:** every conversion is a `Converter` implementation registered in a registry keyed by (input format, output format). Adding a format pair = adding one module + one registration line.
- **Context menu strategy:** the OS context-menu entries invoke `boinc-cli` (or send an IPC message to the running tray app). This avoids writing native shell-extension binaries (COM DLLs, Finder Sync extensions) in v1.

---

## Phase 0 — Project scaffolding

**Goal: a building, tested, CI-verified empty workspace.**

- [x] 0.1 `git init`, `.gitignore` (Rust template), initial commit of existing docs
- [x] 0.2 Create Cargo workspace with `boinc-core`, `boinc-cli`, `boinc-app`, `boinc-integration` crates
- [x] 0.3 Set up tooling: `rustfmt.toml`, `clippy` config, `cargo deny` (license/advisory audit)
- [x] 0.4 CI (GitHub Actions): build + test + clippy on Linux, Windows, macOS runners
- [x] 0.5 Update `CLAUDE.md` with real build/test/run commands once they exist

**Exit criteria:** `cargo build && cargo test && cargo clippy` pass on all three platforms in CI.

---

## Phase 1 — Core conversion engine (`boinc-core`)

**Goal: a pluggable conversion engine, fully usable as a library.**

- [x] 1.1 Define core types: `Format` enum (Pdf, Docx, Png, Jpg, …), `ConversionRequest`, `ConversionResult`, structured `ConversionError`
- [x] 1.2 Define the `Converter` trait: `supports() -> (Format, Format)`, `convert(input, output, options, progress_callback)` — plus `is_available()` for tool-backed converters
- [x] 1.3 Build the `ConverterRegistry`: register converters, look up by format pair, list available conversions for a given input format (drives the context-menu entries later)
- [x] 1.4 Format detection from file extension + magic bytes (don't trust extension alone)
- [x] 1.5 Output-path policy: default to same directory with new extension, never overwrite silently (append ` (1)` etc.), configurable
- [x] 1.6 Implement **PNG → JPG** and **JPG → PNG** using the `image` crate (options: JPEG quality, background color for alpha flattening)
- [x] 1.7 Implement **DOCX → PDF** — **decided:** delegate to headless LibreOffice (`soffice --headless --convert-to`, throwaway user profile per run); converter reports unavailable when soffice is missing (lookup: `BOINC_SOFFICE` env → PATH → known install paths)
- [x] 1.8 Implement **PDF → DOCX** — **decided:** same LibreOffice backend with `--infilter=writer_pdf_import`; output is draw-frame text (lossy), acceptable for v1 per risk #1
- [x] 1.9 Unit + integration tests with fixture files for every converter; golden-file round-trip tests where lossless (fixtures generated at test time; LibreOffice tests self-skip when soffice is absent)
- [x] 1.10 Document "how to add a new converter" in `boinc-core/README.md`

**Exit criteria:** all four conversions work via the library API with tests; adding a mock converter requires no changes outside its own module + registration.

---

## Phase 2 — CLI (`boinc-cli`)

**Goal: scriptable interface and the target for OS context-menu commands.**

- [ ] 2.1 `boinc convert <input> --to <format> [--out <path>]` with clear exit codes
- [ ] 2.2 `boinc list-conversions [<file>]` — enumerate registry (used by integration layer and docs)
- [ ] 2.3 Progress output on stderr (percent), machine-readable `--json` mode
- [ ] 2.4 Batch mode: accept multiple input files
- [ ] 2.5 CLI integration tests (assert_cmd) on fixture files

**Exit criteria:** a context-menu entry could be a one-line command invoking `boinc-cli`; conversions runnable in scripts on all platforms.

---

## Phase 3 — Tray app + Floem UI (`boinc-app`)

**Goal: resident tray application with a minimal Floem UI.**

- [ ] 3.1 Tray icon with menu (Open Boinc, Recent conversions, Pause, Quit) — evaluate `tray-icon` crate alongside Floem's event loop; verify coexistence on all three platforms early (spike task)
- [ ] 3.2 Main Floem window: drag-and-drop a file → pick target format (from registry) → convert with progress bar
- [ ] 3.3 Conversion queue: run conversions on worker threads, show per-job progress/state, cancelation
- [ ] 3.4 Notifications on completion/failure (native notifications, click to reveal output file)
- [ ] 3.5 Settings screen: output-path policy, JPEG quality defaults, launch-at-login toggle
- [ ] 3.6 Single-instance guard + local IPC (e.g. unix socket / named pipe) so CLI/context-menu invocations can hand jobs to the running app instead of spawning a second process
- [ ] 3.7 Persist settings + conversion history (small on-disk store, e.g. JSON in the platform config dir via `directories` crate)

**Exit criteria:** app runs in tray on all three platforms; a file dropped on the window or sent via IPC converts with visible progress and a notification.

---

## Phase 4 — OS integration (`boinc-integration`)

**Goal: right-click a file in the native file browser → Boinc submenu with valid conversions.**

- [ ] 4.1 Design the integration layer: `boinc integrate install/uninstall/status` subcommands that write/remove the per-platform hooks; entries generated from the converter registry so new converters appear automatically
- [ ] 4.2 **Linux:** `.desktop` entries + file-manager integration — Nautilus (GNOME) scripts/actions, KDE Dolphin ServiceMenus (`.desktop` in `kio/servicemenus`), MIME-type scoping so only convertible files show the menu
- [ ] 4.3 **Windows:** registry context-menu verbs (HKCU `Software\Classes\SystemFileAssociations\<ext>\shell\Boinc\...`) with a cascading submenu invoking `boinc-cli`; no COM shell extension in v1
- [ ] 4.4 **macOS:** Finder Quick Actions / Services (`NSServices` in the app bundle or Automator workflows installed to `~/Library/Services`) scoped to supported file types
- [ ] 4.5 Launch-at-login service registration per platform (XDG autostart, Windows Run key / Task Scheduler, macOS LaunchAgent)
- [ ] 4.6 Manual test matrix: each platform × each conversion from the context menu, including filenames with spaces/unicode

**Exit criteria:** on each OS, right-clicking a PNG shows "Boinc → Convert to JPG" (and only valid targets), and selecting it produces the converted file with a notification.

---

## Phase 5 — Packaging & installers

**Goal: one-click installers per platform that set up app, CLI, tray autostart, and context menus.**

- [ ] 5.1 **Linux:** `.deb` + `.rpm` (`cargo-deb`, `cargo-generate-rpm`) and AppImage or Flatpak; post-install hook runs `boinc integrate install`
- [ ] 5.2 **Windows:** MSI or NSIS installer (`cargo-wix` / NSIS) — installs binaries, registry context menus, autostart; proper uninstall
- [ ] 5.3 **macOS:** `.app` bundle + `.dmg`; code signing & notarization pipeline (needs Apple Developer account — flag early)
- [ ] 5.4 CI release pipeline: tag → build artifacts for all platforms → attach to GitHub release
- [ ] 5.5 Auto-update strategy decision (v1 can be "portal announces new version"; defer in-app updater)
- [ ] 5.6 Versioning + changelog conventions

**Exit criteria:** fresh VM per OS: download installer from CI artifacts → install → context-menu conversion works → clean uninstall.

---

## Phase 6 — Web portal (boinc.hideterms.com)

**Goal: landing page with downloads.**

- [ ] 6.1 Landing page: what Boinc does, supported conversions, screenshots/GIF of the context-menu flow
- [ ] 6.2 Download section with per-OS detection and links to latest release artifacts
- [ ] 6.3 Static hosting + deploy pipeline (site rebuilt/updated on each release), DNS setup for boinc.hideterms.com
- [ ] 6.4 Minimal docs page: install steps per OS, how conversions work, FAQ

**Exit criteria:** boinc.hideterms.com is live, serves correct installer per visitor OS, updates automatically on release.

---

## Phase 7 — Hardening & release

**Goal: v1.0 shipped.**

- [ ] 7.1 Error-path polish: unreadable input, disk full, unsupported/corrupt files — every failure surfaces a human-readable notification
- [ ] 7.2 Large-file behavior: memory profile of each converter, streaming where possible
- [ ] 7.3 Cross-platform QA pass of the full matrix (Phase 4.6 repeated on installed builds)
- [ ] 7.4 Add one *new* conversion end-to-end (e.g. WEBP → PNG) as a proof of the extensibility promise, measuring what it took
- [ ] 7.5 Tag v1.0, publish release + portal update

---

## Key risks & decision points

| # | Risk / decision | Impact | Recommendation |
|---|---|---|---|
| 1 | **PDF ↔ DOCX quality.** Pure-Rust conversion (esp. DOCX→PDF layout, PDF→DOCX structure recovery) is a large project on its own. | Phase 1 schedule | v1: delegate to headless LibreOffice when installed (`soffice --headless --convert-to`), detect availability at runtime and hide the menu entries otherwise. Keep the `Converter` trait clean so a native implementation can replace it later. Decide in task 1.7 before building. |
| 2 | **Floem + tray-icon event-loop coexistence.** Both want to own the platform event loop. | Phase 3 feasibility | Do task 3.1 as an early spike on all three platforms before building the rest of the UI. Fallback: separate tray process talking to the Floem app over IPC. |
| 3 | **macOS signing/notarization** requires an Apple Developer account and secrets in CI. | Phase 5 timeline | Start the account/cert process when Phase 3 begins; unsigned builds are painful for users to open. |
| 4 | **Context-menu UX differs per platform** (cascading submenus are limited on some Linux file managers and on macOS Services). | Phase 4 scope | Accept flat entries ("Boinc: Convert to JPG") where submenus aren't supported; keep entry generation data-driven from the registry. |
| 5 | **"OS extension/service" scope creep** (true shell extensions, background daemons). | Overall scope | v1 = tray app + CLI + declarative context-menu hooks. Native shell extensions only if v1 UX proves insufficient. |

## Suggested build order

Phases 0 → 1 → 2 are strictly sequential. Phase 3 (app) and Phase 4 (integration) can proceed in parallel once Phase 2 lands, since context menus target the CLI. Phase 6 (portal) is independent and can start anytime; it only needs release artifacts from Phase 5 to go fully live.
