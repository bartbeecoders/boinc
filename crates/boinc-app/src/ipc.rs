//! Single-instance guard and local IPC.
//!
//! The first instance listens on a local socket; later instances (and, in
//! Phase 4, context-menu invocations) forward their arguments as JSON lines
//! and exit. Messages: `{"cmd":"open"}`, `{"cmd":"pick","input":"/f.png"}`,
//! `{"cmd":"convert","input":"/f.png","to":"jpg"}`.

use std::io::{BufRead, BufReader, Write};
use std::path::PathBuf;
use std::sync::mpsc::Sender;

use boinc_core::Format;
use interprocess::local_socket::prelude::*;
use interprocess::local_socket::{
    GenericFilePath, GenericNamespaced, ListenerOptions, Name, Stream,
};
use serde::{Deserialize, Serialize};

use crate::state::{UiMsg, WorkerCmd};

#[derive(Debug, Serialize, Deserialize)]
#[serde(tag = "cmd", rename_all = "lowercase")]
pub enum IpcMsg {
    Open,
    Pick { input: PathBuf },
    Convert { input: PathBuf, to: String },
}

fn socket_name() -> std::io::Result<Name<'static>> {
    if GenericNamespaced::is_supported() {
        "boinc.sock".to_ns_name::<GenericNamespaced>()
    } else {
        let path = std::env::temp_dir().join("boinc.sock");
        let leaked: &'static str = Box::leak(path.to_string_lossy().into_owned().into_boxed_str());
        leaked.to_fs_name::<GenericFilePath>()
    }
}

/// Try to hand `msgs` to an already-running instance. Returns `true` when a
/// running instance accepted them (the caller should exit).
pub fn try_forward(msgs: &[IpcMsg]) -> bool {
    let Ok(name) = socket_name() else {
        return false;
    };
    let Ok(mut conn) = Stream::connect(name) else {
        return false;
    };
    for msg in msgs {
        let Ok(mut line) = serde_json::to_string(msg) else {
            continue;
        };
        line.push('\n');
        if conn.write_all(line.as_bytes()).is_err() {
            return false;
        }
    }
    true
}

/// Listen for messages from later instances and dispatch them.
pub fn spawn_server(ui_tx: crossbeam_channel::Sender<UiMsg>, worker_tx: Sender<WorkerCmd>) {
    let listener = match bind() {
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

fn bind() -> std::io::Result<LocalSocketListener> {
    let name = socket_name()?;
    ListenerOptions::new().name(name).create_sync()
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
