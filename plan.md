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
- [x] 1.7 Implement **DOCX → PDF** — **decided:** delegate to headless LibreOffice (`soffice --headless --convert-to`, throwaway user profile per run); converter reports unavailable when soffice is missing (lookup: `BOINC_SOFFICE` env → PATH → known install paths). **Verified end-to-end against LibreOffice 26.2.4** (round-trip test + CLI produce real PDF 1.7 / Word 2007+ files); Linux CI now installs libreoffice-writer so the test runs on every push
- [x] 1.8 Implement **PDF → DOCX** — **decided:** same LibreOffice backend with `--infilter=writer_pdf_import`; output is draw-frame text (lossy), acceptable for v1 per risk #1. **Verified end-to-end** (see 1.7)
- [x] 1.9 Unit + integration tests with fixture files for every converter; golden-file round-trip tests where lossless (fixtures generated at test time; LibreOffice tests self-skip when soffice is absent)
- [x] 1.10 Document "how to add a new converter" in `boinc-core/README.md`

**Exit criteria:** all four conversions work via the library API with tests; adding a mock converter requires no changes outside its own module + registration.

---

## Phase 2 — CLI (`boinc-cli`)

**Goal: scriptable interface and the target for OS context-menu commands.**

- [x] 2.1 `boinc convert <input> --to <format> [--out <path>]` with clear exit codes (0 = all succeeded, 1 = any failed, 2 = usage; plus `--out-dir`, `--quality`, `--background RRGGBB`)
- [x] 2.2 `boinc list-conversions [<file>]` — enumerate registry (used by integration layer and docs); `--all` includes tool-unavailable pairs
- [x] 2.3 Progress output on stderr (percent), machine-readable `--json` mode (JSON lines: progress/converted/error events)
- [x] 2.4 Batch mode: accept multiple input files (continues past per-file failures, exit 1 at end)
- [x] 2.5 CLI integration tests (assert_cmd) on fixture files

**Exit criteria:** a context-menu entry could be a one-line command invoking `boinc-cli`; conversions runnable in scripts on all platforms.

---

## Phase 3 — Tray app + Floem UI (`boinc-app`)

**Goal: resident tray application with a minimal Floem UI.**

- [x] 3.1 Tray icon with menu — **spike outcome:** Linux runs tray-icon on a dedicated GTK thread (independent of Floem/winit); Windows/macOS create the tray on the main thread before Floem's loop starts. Menu: Open / Pause conversions (check item) / Quit. *Deviations:* "Recent conversions" submenu deferred; Floem 0.2 exits when the last window closes (no close-interception), so v1 residency = tray lives while the window is open, closing the window quits — revisit in Phase 7 (see `tray.rs` doc comment).
- [x] 3.2 Main Floem window: drag-and-drop a file → pick target format (from registry) → convert with progress bar (verified end-to-end via screenshot + programmatic click)
- [x] 3.3 Conversion queue: worker thread owns the job list, streams snapshots to the UI via channel-backed signal; cancelation of *queued* jobs (cancel-while-running deferred); pause via tray toggle
- [x] 3.4 Notifications on completion/failure via notify-rust (*click-to-reveal deferred* — body includes the output path)
- [x] 3.5 Settings screen: output directory, JPEG quality default, launch-at-login toggle (stored only; actual registration is task 4.5)
- [x] 3.6 Single-instance guard + local IPC (interprocess local socket, JSON lines: open/pick/convert); second instance forwards args and exits (measured 13 ms)
- [x] 3.7 Persist settings (`config_dir/settings.json`) + conversion history (`data_dir/history.json`, capped at 100) via `directories`

**Exit criteria:** app runs in tray on all three platforms; a file dropped on the window or sent via IPC converts with visible progress and a notification. *(Verified on Linux; Windows/macOS compile in CI but need manual verification — tracked under 4.6/7.3.)*

---

## Phase 4 — OS integration (`boinc-integration`)

**Goal: right-click a file in the native file browser → Boinc submenu with valid conversions.**

- [x] 4.1 Design the integration layer: `boinc integrate install/uninstall/status` subcommands; entries generated from the converter registry (available conversions only — re-run install after adding LibreOffice); every created hook recorded in a JSON manifest so uninstall/status touch exactly what we wrote
- [x] 4.2 **Linux:** KDE Dolphin ServiceMenus (`kio/servicemenus`, one MIME-scoped `.desktop` per source format, executable bit set, `%F` batch) + Nautilus scripts (no MIME scoping possible — unsupported files fail with a notify-send; documented deviation) + Nemo/Cinnamon actions (`nemo/actions`, MIME-scoped `.nemo_action` per conversion — added for EndeavourOS/Cinnamon, installed live). Verified live incl. spaces/unicode filenames
- [x] 4.3 **Windows:** cascading HKCU `SystemFileAssociations\<ext>\shell\Boinc` verbs via winreg (`MUIVerb` + empty `SubCommands` + nested shell keys); cross-checked with `cargo check --target x86_64-pc-windows-msvc`
- [x] 4.4 **macOS:** generated Finder Quick Action `.workflow` bundles in `~/Library/Services`, UTI-scoped via `NSSendFileTypes`; cross-checked with `cargo check --target aarch64-apple-darwin`
- [x] 4.5 Launch-at-login per platform (XDG autostart / HKCU Run key / LaunchAgent); the app's settings toggle now actually registers/unregisters on save
- [ ] 4.6 Manual test matrix: **Linux done** (KDE exec line + Nautilus script, spaces/unicode); Windows + macOS pending real-machine QA (folded into 7.3)

**Exit criteria:** on each OS, right-clicking a PNG shows "Boinc → Convert to JPG" (and only valid targets), and selecting it produces the converted file with a notification. *(Met on Linux; Windows/macOS hooks are written per platform docs but unverified on real machines — the CLI conversion they invoke shows no notification yet, see 7.1.)*

---

## Phase 5 — Packaging & installers

**Goal: one-click installers per platform that set up app, CLI, tray autostart, and context menus.**

- [x] 5.1 **Linux:** `.deb` + `.rpm` via cargo-deb/cargo-generate-rpm — built and inspected locally (binaries, desktop entry, icon, LICENSE; gtk3 dependency, libreoffice recommends). *Deviations:* AppImage/Flatpak deferred; instead of a root post-install hook, the app performs **per-user context-menu integration on first run** (idempotent, manifest-guarded) — cleaner than root-time hooks and shared by all three platforms
- [x] 5.2 **Windows:** hand-written WiX 3 config (`wix/main.wxs`), per-user scope, both binaries + Start Menu shortcut, MajorUpgrade handling; built in CI via cargo-wix (context menus/autostart via first-run integration, no custom actions). *Not yet run on a real Windows machine*
- [x] 5.3 **macOS:** `scripts/package-macos.sh` builds Boinc.app + dmg; codesign hook via `APPLE_SIGNING_IDENTITY`. **Unsigned until an Apple Developer account exists — notarization still open (risk #3)**
- [x] 5.4 CI release pipeline (`release.yml`): `v*` tag → deb/rpm/msi/dmg attached to the GitHub release — **proven: v0.1.0 shipped with all four artifacts** (github.com/bartbeecoders/boinc/releases/tag/v0.1.0)
- [x] 5.5 Auto-update: **decided** — no in-app updater in v1; the portal announces new versions (documented in README)
- [x] 5.6 Versioning + changelog: single workspace version, semver, Keep-a-Changelog `CHANGELOG.md`; release steps in README

**Exit criteria:** fresh VM per OS: download installer from CI artifacts → install → context-menu conversion works → clean uninstall. *(Package contents verified on Linux; the full fresh-VM pass on all three OSes remains for 7.3, and needs a GitHub remote for the release workflow to run at all.)*

---

## Phase 6 — Web portal (boinc.hideterms.com)

**Goal: landing page with downloads.**

- [x] 6.1 Landing page (`site/`, React 19 + Vite; originally static, converted on request): hero is an interactive recreation of the context menu (click "Convert to JPG" and the demo file converts, with an undo toast that converts it back); real app screenshot; verified via headless-Chrome screenshots at desktop + mobile widths, light + dark
- [x] 6.2 Download section: per-OS detection highlights the visitor's cards and retargets the primary button; asset links resolved client-side from the GitHub releases API (so releases don't require a site redeploy), with the releases page as no-JS fallback
- [x] 6.3 Hosting — **revised decision: VPS K3S instead of Cloudflare Pages.** `scripts/deploy-k3s.sh` builds the site image (node → nginx, `site/Dockerfile`), pushes to the ACR registry, and applies `k8s/boinc/` on the VPS: namespace, acr-secret copy, NodePort **32087** service, deployment with health probes. **Deployed and verified live** (`http://212.47.77.32:32087/healthz` → ok). Cloudflare Tunnel route `boinc.hideterms.com → localhost:32087` is owner-managed (pending); runbook in `Vibecoding/deploy.md`
- [x] 6.4 Docs on-page: numbered how-it-works, per-OS install one-liners, FAQ (uploads, LibreOffice requirement, overwrite policy, batch, license)

**Exit criteria:** boinc.hideterms.com is live, serves correct installer per visitor OS, updates automatically on release. *(Met except the hostname: the deployed site resolves v0.1.0's four artifacts live from the GitHub API — verified end-to-end via headless Chrome against the running pod. Public hostname blocks only on the owner's Cloudflare Tunnel route.)*

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
