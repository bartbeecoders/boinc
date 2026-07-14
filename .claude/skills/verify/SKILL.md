---
name: verify
description: Build, launch, and observe the Boinc tray app / CLI for runtime verification of changes in this repo.
---

# Verifying Boinc changes

## Build

```sh
cargo build -p boinc-app        # tray app -> target/debug/boinc-app
cargo build -p boinc-cli        # CLI      -> target/debug/boinc
```

## Launching the app without touching real state

Always sandbox, otherwise the run forwards to a real running instance over
IPC and exits, and first-run writes context-menu hooks into the real
`~/.local/share` / `~/.config`:

```sh
env XDG_CONFIG_HOME="$SB/xdg-config" XDG_DATA_HOME="$SB/xdg-data" \
    BOINC_SOCK=boinc-verify-$$.sock \
    ./target/debug/boinc-app >"$SB/app.log" 2>&1 &
```

- `BOINC_SOCK` isolates the single-instance IPC socket (else the launch
  becomes a no-op forward to a running app).
- First run prints `boinc: installed N context-menu hook(s)` — that lands in
  the sandboxed XDG dirs, harmless.
- Debug builds skip the startup update check unless `BOINC_UPDATE_URL` is set.

## Observing the window (X11 sessions)

Available here: `xdotool`, ImageMagick `import`. No Xvfb — runs appear
briefly on the live display.

```sh
xdotool search --name '^Boinc$'      # exact match! plain "Boinc" also hits
                                     # browser/editor windows with Boinc in
                                     # the title
xprop -id <wid> WM_CLASS _NET_WM_PID # confirm it's the app: "boinc-app" + pid
import -display :0 -window <wid> shot.png
```

Window title is `Boinc` (WindowConfig in `main.rs`), WM_CLASS `boinc-app`.
Give it ~3-4s to map before searching.

## Backend selection (Wayland vs X11)

The app force-redirects to XWayland when both `WAYLAND_DISPLAY` and `DISPLAY`
are set (`force_xwayland()` in `crates/boinc-app/src/main.rs`) because the
winit-0.29 Wayland backend has no drag-and-drop. To exercise each branch on
an X11 box:

- `WAYLAND_DISPLAY=wayland-fake DISPLAY=:0` → must open an X11 window
  (redirect worked; without it, winit picks Wayland and dies on connect).
- `env -u DISPLAY WAYLAND_DISPLAY=wayland-fake` → must panic inside
  `platform_impl/linux/wayland/` (proves Wayland is kept when X11 is absent;
  the panic is just the fake socket).

## Gaps / gotchas

- No drag-and-drop source tool on this box (`dragon`, `python-gi` absent), so
  a real synthetic file drag can't be scripted; nearest observable is the
  pick row that `ui::add_pick` renders — reachable by passing a file path as
  a CLI arg to the app.
- CLI surface: `./target/debug/boinc convert ...`, exit codes 0/1/2; the
  integrate CLI test sandboxes the same way (`XDG_*` overrides).
