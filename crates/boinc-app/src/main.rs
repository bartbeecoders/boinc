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
use crate::state::{JobView, PendingPick, UiMsg};
use crate::ui::AppCtx;

fn main() {
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
    tray::init(paused, ui_tx.clone());

    let ctx = AppCtx {
        registry: registry.clone(),
        settings,
        worker_tx,
    };

    // Reactive state shared by all windows, fed from the channel.
    let jobs: RwSignal<Vec<JobView>> = create_rw_signal(Vec::new());
    let picks: RwSignal<Vec<PendingPick>> = create_rw_signal(Vec::new());
    for file in files {
        ui::add_pick(&registry, picks, file);
    }

    let ui_msg = create_signal_from_channel(ui_rx);
    create_effect(move |_| {
        let Some(msg) = ui_msg.get() else { return };
        match msg {
            UiMsg::Jobs(list) => jobs.set(list),
            UiMsg::Pick(path) => ui::add_pick(&registry, picks, path),
            UiMsg::OpenWindow => {
                // v1: the main window is always open while the app runs (see
                // tray.rs on Floem's last-window-closed behavior).
            }
            UiMsg::Quit => floem::quit_app(),
        }
    });

    Application::new()
        .window(
            move |_| ui::main_view(ctx, jobs, picks),
            Some(WindowConfig::default().size((520.0, 480.0)).title("Boinc")),
        )
        .run();
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
