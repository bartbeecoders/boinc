//! Single-instance guard and local IPC (server side).
//!
//! The first instance listens on a local socket; later instances and
//! context-menu invocations (`boinc convert --app`) forward their arguments
//! as JSON lines and exit. The socket name and message format live in
//! `boinc_integration::ipc`, shared with the CLI.

use std::io::{BufRead, BufReader};
use std::sync::mpsc::Sender;

use boinc_core::Format;
pub use boinc_integration::ipc::IpcMsg;
use boinc_integration::ipc::ListenerExt as _;
pub use boinc_integration::ipc::try_forward;

use crate::state::{UiMsg, WorkerCmd};

/// Listen for messages from later instances and dispatch them.
pub fn spawn_server(ui_tx: crossbeam_channel::Sender<UiMsg>, worker_tx: Sender<WorkerCmd>) {
    let listener = match boinc_integration::ipc::bind() {
        Ok(listener) => listener,
        Err(err) => {
            eprintln!("boinc: IPC unavailable ({err}); single-instance guard disabled");
            return;
        }
    };
    std::thread::Builder::new()
        .name("boinc-ipc".into())
        .spawn(move || {
            for conn in listener.incoming() {
                let Ok(conn) = conn else { continue };
                for line in BufReader::new(conn).lines() {
                    let Ok(line) = line else { break };
                    match serde_json::from_str::<IpcMsg>(&line) {
                        Ok(msg) => dispatch(msg, &ui_tx, &worker_tx),
                        Err(err) => eprintln!("boinc: bad IPC message: {err}"),
                    }
                }
            }
        })
        .expect("IPC thread can be spawned");
}

fn dispatch(msg: IpcMsg, ui_tx: &crossbeam_channel::Sender<UiMsg>, worker_tx: &Sender<WorkerCmd>) {
    match msg {
        IpcMsg::Open => {
            let _ = ui_tx.send(UiMsg::OpenWindow);
        }
        IpcMsg::Pick { input } => {
            let _ = ui_tx.send(UiMsg::Pick(input));
        }
        IpcMsg::Convert { input, to } => match Format::from_extension(&to) {
            Some(to) => {
                let _ = worker_tx.send(WorkerCmd::Enqueue { input, to });
            }
            None => eprintln!("boinc: IPC convert with unknown format {to:?}"),
        },
    }
}
