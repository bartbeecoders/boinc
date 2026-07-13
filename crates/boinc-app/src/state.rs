//! Shared types passed between the UI thread, the worker, IPC, and the tray.

use std::path::PathBuf;

use boinc_core::Format;

/// One conversion job as shown in the UI. The worker owns the authoritative
/// list and sends full snapshots to the UI on every change.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct JobView {
    pub id: u64,
    pub input: PathBuf,
    pub to: Format,
    pub status: JobStatus,
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum JobStatus {
    Queued,
    /// Running, with percent complete.
    Running(i32),
    Done(PathBuf),
    Failed(String),
    Canceled,
}

/// Commands into the worker thread.
#[derive(Debug)]
pub enum WorkerCmd {
    Enqueue {
        input: PathBuf,
        to: Format,
    },
    /// Cancel a job; only has an effect while it is still queued.
    Cancel(u64),
}

/// Messages into the UI thread (bridged into a floem signal).
#[derive(Clone, Debug)]
pub enum UiMsg {
    /// Snapshot of all jobs.
    Jobs(Vec<JobView>),
    /// A file arrived (drop, CLI arg, or IPC): show the target picker.
    Pick(PathBuf),
    OpenWindow,
    Quit,
}

/// A dropped/received file waiting for the user to pick a target format.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct PendingPick {
    pub input: PathBuf,
    pub targets: Vec<Format>,
    /// Set when the file can't be converted (unknown format, no targets).
    pub error: Option<String>,
}
