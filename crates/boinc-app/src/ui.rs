//! Floem views: main window (drop zone, target pickers, job list) and the
//! settings window.

use std::path::{Path, PathBuf};
use std::sync::mpsc::Sender;
use std::sync::{Arc, Mutex};

use boinc_core::{ConverterRegistry, detect_format};
use floem::event::{Event, EventListener};
use floem::peniko::Color;
use floem::reactive::{RwSignal, SignalGet, SignalUpdate, create_rw_signal};
use floem::views::{
    Decorators, button, container, dyn_stack, empty, h_stack, label, scroll, text_input, v_stack,
};
use floem::{IntoView, new_window, window::WindowConfig};

use crate::settings::Settings;
use crate::state::{JobStatus, JobView, PendingPick, WorkerCmd};

/// Everything the views need; cheap to clone.
pub struct AppCtx {
    pub registry: Arc<ConverterRegistry>,
    pub settings: Arc<Mutex<Settings>>,
    pub worker_tx: Sender<WorkerCmd>,
}

impl Clone for AppCtx {
    fn clone(&self) -> Self {
        Self {
            registry: self.registry.clone(),
            settings: self.settings.clone(),
            worker_tx: self.worker_tx.clone(),
        }
    }
}

/// Turn an incoming file into a pending pick row (or an error row).
pub fn add_pick(registry: &ConverterRegistry, picks: RwSignal<Vec<PendingPick>>, input: PathBuf) {
    let pick = match detect_format(&input) {
        Ok(from) => {
            let targets = registry.available_targets(from);
            if targets.is_empty() {
                PendingPick {
                    input,
                    targets,
                    error: Some(format!("no conversions available for {from}")),
                }
            } else {
                PendingPick {
                    input,
                    targets,
                    error: None,
                }
            }
        }
        Err(err) => PendingPick {
            input,
            targets: Vec::new(),
            error: Some(err.to_string()),
        },
    };
    picks.update(|list| {
        list.retain(|p| p.input != pick.input);
        list.push(pick);
    });
}

/// Register or unregister this binary as a login item.
fn apply_autostart(enabled: bool) -> std::io::Result<()> {
    let app = std::env::current_exe()?;
    boinc_integration::set_autostart(enabled, &app)
}

fn file_name(path: &Path) -> String {
    path.file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_else(|| path.display().to_string())
}

pub fn main_view(
    ctx: AppCtx,
    jobs: RwSignal<Vec<JobView>>,
    picks: RwSignal<Vec<PendingPick>>,
) -> impl IntoView {
    let drop_ctx = ctx.clone();

    let header = {
        let ctx = ctx.clone();
        h_stack((
            label(|| "Boinc").style(|s| s.font_size(20.0).font_bold()),
            empty().style(|s| s.flex_grow(1.0_f32)),
            button("Settings").action(move || open_settings(ctx.clone())),
        ))
        .style(|s| s.items_center().width_full())
    };

    let drop_zone = container(
        label(|| "Drop files here to convert").style(|s| s.color(Color::rgb8(0x60, 0x60, 0x60))),
    )
    .style(|s| {
        s.width_full()
            .height(90.0)
            .items_center()
            .justify_center()
            .border(2.0)
            .border_color(Color::rgb8(0xb0, 0xb0, 0xb0))
            .border_radius(8.0)
            .background(Color::rgb8(0xf5, 0xf5, 0xf5))
    });

    let pick_ctx = ctx.clone();
    let picks_list = dyn_stack(
        move || picks.get(),
        |pick| pick.input.clone(),
        move |pick| pick_row(pick_ctx.clone(), picks, pick),
    )
    .style(|s| s.flex_col().width_full().gap(4.0));

    let job_ctx = ctx;
    let jobs_list = scroll(
        dyn_stack(
            move || jobs.get(),
            |job| (job.id, job.status.clone()),
            move |job| job_row(job_ctx.clone(), job),
        )
        .style(|s| s.flex_col().width_full().gap(4.0)),
    )
    .style(|s| s.width_full().flex_grow(1.0_f32));

    v_stack((header, drop_zone, picks_list, jobs_list))
        .style(|s| {
            s.flex_col()
                .width_full()
                .height_full()
                .padding(12.0)
                .gap(10.0)
        })
        .on_event_stop(EventListener::DroppedFile, move |event| {
            if let Event::DroppedFile(e) = event {
                add_pick(&drop_ctx.registry, picks, e.path.clone());
            }
        })
}

fn pick_row(ctx: AppCtx, picks: RwSignal<Vec<PendingPick>>, pick: PendingPick) -> impl IntoView {
    let input = pick.input.clone();
    let name = file_name(&input);
    let error = pick.error.clone();

    let dismiss_input = input.clone();
    let dismiss = button("✕").action(move || {
        let dismiss_input = dismiss_input.clone();
        picks.update(move |list| list.retain(|p| p.input != dismiss_input));
    });

    let targets = dyn_stack(
        move || pick.targets.clone(),
        |to| *to,
        move |to| {
            let ctx = ctx.clone();
            let input = input.clone();
            button(format!("→ {to}")).action(move || {
                let _ = ctx.worker_tx.send(WorkerCmd::Enqueue {
                    input: input.clone(),
                    to,
                });
                let input = input.clone();
                picks.update(move |list| list.retain(|p| p.input != input));
            })
        },
    )
    .style(|s| s.gap(6.0));

    let row = match error {
        Some(message) => h_stack((
            label(move || name.clone()),
            label(move || message.clone()).style(|s| s.color(Color::rgb8(0xc0, 0x30, 0x30))),
            empty().style(|s| s.flex_grow(1.0_f32)),
            dismiss,
        ))
        .into_any(),
        None => h_stack((
            label(move || name.clone()),
            targets,
            empty().style(|s| s.flex_grow(1.0_f32)),
            dismiss,
        ))
        .into_any(),
    };
    row.style(|s| s.items_center().width_full().gap(8.0))
}

fn job_row(ctx: AppCtx, job: JobView) -> impl IntoView {
    let name = format!("{} → {}", file_name(&job.input), job.to);
    let id = job.id;

    let status = match job.status {
        JobStatus::Queued => {
            let cancel = button("Cancel").action(move || {
                let _ = ctx.worker_tx.send(WorkerCmd::Cancel(id));
            });
            h_stack((label(|| "queued"), cancel))
                .style(|s| s.gap(8.0).items_center())
                .into_any()
        }
        JobStatus::Running(pct) => h_stack((progress_bar(pct), label(move || format!("{pct}%"))))
            .style(|s| s.gap(8.0).items_center())
            .into_any(),
        JobStatus::Done(output) => label(move || format!("done: {}", output.display()))
            .style(|s| s.color(Color::rgb8(0x20, 0x80, 0x20)))
            .into_any(),
        JobStatus::Failed(message) => label(move || format!("failed: {message}"))
            .style(|s| s.color(Color::rgb8(0xc0, 0x30, 0x30)))
            .into_any(),
        JobStatus::Canceled => label(|| "canceled")
            .style(|s| s.color(Color::rgb8(0x80, 0x80, 0x80)))
            .into_any(),
    };

    h_stack((
        label(move || name.clone()),
        empty().style(|s| s.flex_grow(1.0_f32)),
        status,
    ))
    .style(|s| s.items_center().width_full().gap(8.0))
}

fn progress_bar(pct: i32) -> impl IntoView {
    container(empty().style(move |s| {
        s.height_full()
            .width_pct(f64::from(pct.clamp(0, 100)))
            .background(Color::rgb8(0x30, 0x70, 0xd0))
            .border_radius(4.0)
    }))
    .style(|s| {
        s.width(140.0)
            .height(8.0)
            .background(Color::rgb8(0xd8, 0xd8, 0xd8))
            .border_radius(4.0)
    })
}

fn open_settings(ctx: AppCtx) {
    new_window(
        move |_| settings_view(ctx),
        Some(
            WindowConfig::default()
                .size((420.0, 260.0))
                .title("Boinc Settings"),
        ),
    );
}

fn settings_view(ctx: AppCtx) -> impl IntoView {
    let current = ctx
        .settings
        .lock()
        .expect("settings mutex not poisoned")
        .clone();

    let output_dir = create_rw_signal(
        current
            .output_dir
            .as_ref()
            .map(|p| p.display().to_string())
            .unwrap_or_default(),
    );
    let quality = create_rw_signal(current.jpeg_quality.to_string());
    let launch = create_rw_signal(current.launch_at_login);
    let status = create_rw_signal(String::new());

    let save = {
        let ctx = ctx.clone();
        button("Save").action(move || {
            let Ok(parsed_quality) = quality.get().trim().parse::<u8>() else {
                status.set("JPEG quality must be a number 1-100".into());
                return;
            };
            if !(1..=100).contains(&parsed_quality) {
                status.set("JPEG quality must be 1-100".into());
                return;
            }
            let dir = output_dir.get().trim().to_string();
            let new_settings = Settings {
                output_dir: (!dir.is_empty()).then(|| PathBuf::from(dir)),
                jpeg_quality: parsed_quality,
                launch_at_login: launch.get(),
            };
            match new_settings.save() {
                Ok(()) => {
                    let autostart = apply_autostart(new_settings.launch_at_login);
                    *ctx.settings.lock().expect("settings mutex not poisoned") = new_settings;
                    match autostart {
                        Ok(()) => status.set("Saved".into()),
                        Err(err) => status.set(format!("Saved (autostart failed: {err})")),
                    }
                }
                Err(err) => status.set(format!("Could not save: {err}")),
            }
        })
    };

    v_stack((
        label(|| "Output directory (empty = next to input)"),
        text_input(output_dir).style(|s| s.width_full()),
        label(|| "Default JPEG quality (1-100)"),
        text_input(quality).style(|s| s.width(80.0)),
        label(move || {
            if launch.get() {
                "[x] Launch at login".to_string()
            } else {
                "[ ] Launch at login".to_string()
            }
        })
        .on_click_stop(move |_| launch.update(|b| *b = !*b)),
        h_stack((save, label(move || status.get()))).style(|s| s.gap(10.0).items_center()),
    ))
    .style(|s| {
        s.flex_col()
            .width_full()
            .height_full()
            .padding(14.0)
            .gap(8.0)
    })
}
