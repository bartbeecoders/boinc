//! Boinc tray application: resident converter with a Floem UI.
//!
//! Startup: if another instance is already running, forward our arguments to
//! it over IPC and exit. Otherwise start the worker, IPC server, and tray,
//! then run the Floem event loop. File arguments become pending target picks
//! in the window (same flow as drag-and-drop).

// Windows: hide the console window in release builds.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod history;
mod ipc;
mod queue;
mod settings;
mod state;
mod tray;
mod ui;
mod update;

use std::path::PathBuf;
use std::sync::atomic::AtomicBool;
use std::sync::{Arc, Mutex};

use boinc_core::ConverterRegistry;
use floem::Application;
use floem::ext_event::create_signal_from_channel;
use floem::reactive::{RwSignal, SignalGet, SignalUpdate, create_effect, create_rw_signal};
use floem::window::WindowConfig;

use crate::ipc::IpcMsg;
use crate::settings::Settings;
use crate::state::{JobStatus, JobView, PendingPick, UiMsg};
use crate::ui::AppCtx;

fn main() {
    force_xwayland();

    let files: Vec<PathBuf> = std::env::args_os()
        .skip(1)
        .filter(|a| !a.to_string_lossy().starts_with('-'))
        .map(|a| absolute(PathBuf::from(a)))
        .collect();

    // Second instance: hand everything to the running app and exit.
    let mut forward = vec![IpcMsg::Open];
    forward.extend(files.iter().map(|f| IpcMsg::Pick { input: f.clone() }));
    if ipc::try_forward(&forward) {
        return;
    }

    let registry = Arc::new(ConverterRegistry::with_defaults());
    let settings = Arc::new(Mutex::new(Settings::load()));
    let paused = Arc::new(AtomicBool::new(false));

    // First run: best-effort context-menu integration (idempotent; skipped
    // once a manifest exists). Lets installers stay free of post-install
    // hooks on every platform.
    if boinc_integration::status().is_empty() {
        if let Some(cli) = sibling_cli() {
            match boinc_integration::install(&registry, &cli) {
                Ok(report) => eprintln!(
                    "boinc: installed {} context-menu hook(s)",
                    report.created.len()
                ),
                Err(err) => eprintln!("boinc: context-menu integration failed: {err}"),
            }
        }
    }

    let (ui_tx, ui_rx) = crossbeam_channel::unbounded::<UiMsg>();
    let (worker_tx, worker_rx) = std::sync::mpsc::channel();

    queue::spawn_worker(
        worker_rx,
        ui_tx.clone(),
        registry.clone(),
        settings.clone(),
        paused.clone(),
    );
    ipc::spawn_server(ui_tx.clone(), worker_tx.clone());
    let tray = tray::init(paused, ui_tx.clone());
    if settings
        .lock()
        .expect("settings mutex not poisoned")
        .check_updates
    {
        update::spawn_check(ui_tx.clone());
    }

    let ctx = AppCtx {
        registry: registry.clone(),
        settings,
        worker_tx,
        ui_tx: ui_tx.clone(),
    };

    // Reactive state shared by all windows, fed from the channel.
    let jobs: RwSignal<Vec<JobView>> = create_rw_signal(Vec::new());
    let picks: RwSignal<Vec<PendingPick>> = create_rw_signal(Vec::new());
    let update: RwSignal<Option<update::UpdateInfo>> = create_rw_signal(None);
    let update_status: RwSignal<String> = create_rw_signal(String::new());
    for file in files {
        ui::add_pick(&registry, picks, file);
    }

    let ui_msg = create_signal_from_channel(ui_rx);
    create_effect(move |_| {
        let Some(msg) = ui_msg.get() else { return };
        match msg {
            UiMsg::Jobs(list) => {
                // Reflect an active conversion (e.g. a slow LibreOffice
                // PDF↔DOCX job) in the tray icon.
                tray.set_busy(
                    list.iter()
                        .any(|j| matches!(j.status, JobStatus::Running(_))),
                );
                jobs.set(list);
            }
            UiMsg::Pick(path) => ui::add_pick(&registry, picks, path),
            UiMsg::UpdateAvailable(info) => update.set(Some(info)),
            UiMsg::UpdateStatus(text) => update_status.set(text),
            UiMsg::OpenWindow => {
                // v1: the main window is always open while the app runs (see
                // tray.rs on Floem's last-window-closed behavior).
            }
            UiMsg::Quit => floem::quit_app(),
        }
    });

    Application::new()
        .window(
            move |_| ui::main_view(ctx, jobs, picks, update, update_status),
            Some(WindowConfig::default().size((520.0, 480.0)).title("Boinc")),
        )
        .run();
}

/// Wayland sessions: run under XWayland instead, because Floem 0.2's winit
/// fork (0.29) never emits `DroppedFile` on its Wayland backend, which would
/// silently break drag-and-drop into the window. Winit picks Wayland whenever
/// `WAYLAND_DISPLAY` is set, so drop it — but only when `DISPLAY` offers an
/// X11 path; otherwise keep the working Wayland window over no window at all.
#[allow(unsafe_code)] // env mutation is unsafe in edition 2024; single-threaded here
fn force_xwayland() {
    #[cfg(target_os = "linux")]
    {
        let set = |var: &str| std::env::var_os(var).is_some_and(|v| !v.is_empty());
        if set("WAYLAND_DISPLAY") && set("DISPLAY") {
            // SAFETY: called at the top of main, before any threads exist.
            unsafe { std::env::remove_var("WAYLAND_DISPLAY") };
        }
    }
}

/// The `boinc` CLI installed next to this binary, if any.
fn sibling_cli() -> Option<PathBuf> {
    let exe = std::env::current_exe().ok()?;
    let name = if cfg!(windows) { "boinc.exe" } else { "boinc" };
    let cli = exe.parent()?.join(name);
    cli.is_file().then_some(cli)
}

fn absolute(path: PathBuf) -> PathBuf {
    if path.is_absolute() {
        path
    } else {
        std::env::current_dir()
            .map(|cwd| cwd.join(&path))
            .unwrap_or(path)
    }
}
