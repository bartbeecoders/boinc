//! The local-socket protocol between the `boinc` CLI (which context-menu
//! hooks invoke) and a running tray app.
//!
//! The app listens on the socket; the CLI and later app instances forward
//! JSON lines and exit. Messages: `{"cmd":"open"}`,
//! `{"cmd":"pick","input":"/f.png"}`,
//! `{"cmd":"convert","input":"/f.png","to":"jpg"}`. Living here keeps both
//! sides of the contract in one crate they already share.

use std::io::Write;
use std::path::PathBuf;

use interprocess::local_socket::prelude::*;
use interprocess::local_socket::{
    GenericFilePath, GenericNamespaced, ListenerOptions, Name, Stream,
};
use serde::{Deserialize, Serialize};

/// Re-exported so listener callers can iterate `.incoming()` without a
/// direct `interprocess` dependency.
pub use interprocess::local_socket::traits::ListenerExt;

#[derive(Debug, Serialize, Deserialize)]
#[serde(tag = "cmd", rename_all = "lowercase")]
pub enum IpcMsg {
    Open,
    Pick { input: PathBuf },
    Convert { input: PathBuf, to: String },
}

/// The app's socket. `BOINC_SOCK` overrides the name so tests (and parallel
/// dev instances) can isolate themselves from a real running app.
fn socket_name() -> std::io::Result<Name<'static>> {
    let name = std::env::var("BOINC_SOCK").unwrap_or_else(|_| "boinc.sock".into());
    to_name(name)
}

fn to_name(name: String) -> std::io::Result<Name<'static>> {
    if GenericNamespaced::is_supported() {
        let leaked: &'static str = Box::leak(name.into_boxed_str());
        leaked.to_ns_name::<GenericNamespaced>()
    } else {
        let path = std::env::temp_dir().join(name);
        let leaked: &'static str = Box::leak(path.to_string_lossy().into_owned().into_boxed_str());
        leaked.to_fs_name::<GenericFilePath>()
    }
}

/// Try to hand `msgs` to a running app instance. Returns `true` when one
/// accepted them (the caller should not convert locally).
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

/// Bind the listener side (the running app).
pub fn bind() -> std::io::Result<LocalSocketListener> {
    bind_at(socket_name()?)
}

/// Bind a specific socket name, bypassing `BOINC_SOCK`. Lets tests stand in
/// for the app on an isolated socket.
pub fn bind_named(name: &str) -> std::io::Result<LocalSocketListener> {
    bind_at(to_name(name.to_string())?)
}

fn bind_at(name: Name<'static>) -> std::io::Result<LocalSocketListener> {
    ListenerOptions::new().name(name).create_sync()
}
