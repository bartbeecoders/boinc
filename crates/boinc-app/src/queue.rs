//! The conversion queue: a worker thread that owns the job list, runs
//! conversions sequentially, and streams snapshots to the UI.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{Receiver, RecvTimeoutError};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use boinc_core::{ConversionRequest, ConverterRegistry, OutputPolicy, convert};

use crate::history::{self, HistoryEntry};
use crate::settings::Settings;
use crate::state::{JobStatus, JobView, UiMsg, WorkerCmd};

pub fn spawn_worker(
    cmd_rx: Receiver<WorkerCmd>,
    ui_tx: crossbeam_channel::Sender<UiMsg>,
    registry: Arc<ConverterRegistry>,
    settings: Arc<Mutex<Settings>>,
    paused: Arc<AtomicBool>,
) {
    std::thread::Builder::new()
        .name("boinc-worker".into())
        .spawn(move || worker_loop(&cmd_rx, &ui_tx, &registry, &settings, &paused))
        .expect("worker thread can be spawned");
}

fn worker_loop(
    cmd_rx: &Receiver<WorkerCmd>,
    ui_tx: &crossbeam_channel::Sender<UiMsg>,
    registry: &ConverterRegistry,
    settings: &Mutex<Settings>,
    paused: &AtomicBool,
) {
    let mut jobs: Vec<JobView> = Vec::new();
    let mut next_id: u64 = 1;

    loop {
        // Wait briefly for commands so an idle worker doesn't spin; drain
        // whatever queued up.
        match cmd_rx.recv_timeout(Duration::from_millis(150)) {
            Ok(cmd) => {
                apply(&mut jobs, &mut next_id, cmd);
                while let Ok(cmd) = cmd_rx.try_recv() {
                    apply(&mut jobs, &mut next_id, cmd);
                }
                snapshot(ui_tx, &jobs);
            }
            Err(RecvTimeoutError::Timeout) => {}
            Err(RecvTimeoutError::Disconnected) => return,
        }

        if paused.load(Ordering::Relaxed) {
            continue;
        }
        let Some(idx) = jobs.iter().position(|j| j.status == JobStatus::Queued) else {
            continue;
        };

        let input = jobs[idx].input.clone();
        let to = jobs[idx].to;
        let mut request = ConversionRequest::new(&input, to);
        {
            let settings = settings.lock().expect("settings mutex not poisoned");
            request.policy = OutputPolicy {
                dir: settings.output_dir.clone(),
            };
            request.options.jpeg_quality = settings.jpeg_quality;
        }

        jobs[idx].status = JobStatus::Running(0);
        snapshot(ui_tx, &jobs);

        let mut last_pct = 0;
        let result = convert(registry, &request, &mut |p| {
            let pct = (p * 100.0).round() as i32;
            if pct != last_pct {
                last_pct = pct;
                jobs[idx].status = JobStatus::Running(pct);
                snapshot(ui_tx, &jobs);
            }
        });

        let file_name = input
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_else(|| input.display().to_string());
        match result {
            Ok(result) => {
                jobs[idx].status = JobStatus::Done(result.output.clone());
                notify(
                    &format!("Converted {file_name}"),
                    &format!("Saved as {}", result.output.display()),
                );
                history::record(HistoryEntry::now(
                    input,
                    Some(result.output),
                    to.extension().to_string(),
                    None,
                ));
            }
            Err(err) => {
                jobs[idx].status = JobStatus::Failed(err.to_string());
                notify(&format!("Could not convert {file_name}"), &err.to_string());
                history::record(HistoryEntry::now(
                    input,
                    None,
                    to.extension().to_string(),
                    Some(err.to_string()),
                ));
            }
        }
        snapshot(ui_tx, &jobs);
    }
}

fn apply(jobs: &mut Vec<JobView>, next_id: &mut u64, cmd: WorkerCmd) {
    match cmd {
        WorkerCmd::Enqueue { input, to } => {
            jobs.push(JobView {
                id: *next_id,
                input,
                to,
                status: JobStatus::Queued,
            });
            *next_id += 1;
        }
        WorkerCmd::Cancel(id) => {
            if let Some(job) = jobs
                .iter_mut()
                .find(|j| j.id == id && j.status == JobStatus::Queued)
            {
                job.status = JobStatus::Canceled;
            }
        }
    }
}

fn snapshot(ui_tx: &crossbeam_channel::Sender<UiMsg>, jobs: &[JobView]) {
    let _ = ui_tx.send(UiMsg::Jobs(jobs.to_vec()));
}

pub(crate) fn notify(summary: &str, body: &str) {
    let result = notify_rust::Notification::new()
        .appname("Boinc")
        .summary(summary)
        .body(body)
        .show();
    if let Err(err) = result {
        eprintln!("boinc: could not show notification: {err}");
    }
}
