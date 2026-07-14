//! Tray icon with Open / Pause / Quit menu and a busy indicator.
//!
//! Spike outcome (plan.md task 3.1): tray-icon and Floem's winit event loop
//! coexist, but differently per platform. On Linux, tray-icon requires GTK,
//! so the tray runs on its own thread with a private `gtk::main()` loop —
//! fully independent of Floem. On Windows and macOS the tray icon is created
//! on the main thread before Floem's event loop starts, which then pumps its
//! native events. Menu clicks arrive on muda's global channel and are
//! forwarded to the UI/worker from a small forwarder thread.
//!
//! While a conversion is running (e.g. a slow LibreOffice PDF↔DOCX job) the
//! icon gains a rotating white spinner arc and the tooltip changes;
//! [`TrayHandle`] carries that state from the UI thread to wherever the tray
//! lives. Animation frames are driven by the GTK timer on Linux and by
//! self-rescheduling `exec_after` timers on the Floem event loop elsewhere.
//!
//! v1 residency limitation: Floem 0.2 exits its event loop when the last
//! window closes (except on macOS) and offers no close-interception, so
//! closing the main window quits the app. True close-to-tray needs a Floem
//! change or a hidden keep-alive window; revisit in Phase 7.

use std::cell::Cell;
#[cfg(not(target_os = "linux"))]
use std::rc::Rc;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

use tray_icon::menu::{CheckMenuItem, Menu, MenuEvent, MenuItem, PredefinedMenuItem};
use tray_icon::{Icon, TrayIcon, TrayIconBuilder};

use crate::state::UiMsg;

const OPEN_ID: &str = "open";
const PAUSE_ID: &str = "pause";
const QUIT_ID: &str = "quit";

fn tooltip(busy: bool) -> String {
    let version = boinc_core::version();
    if busy {
        format!("Boinc v{version} — converting…")
    } else {
        format!("Boinc file converter v{version}")
    }
}

/// One full spinner revolution takes `SPINNER_STEPS * SPINNER_TICK` (1s).
const SPINNER_STEPS: u8 = 8;
const SPINNER_TICK: Duration = Duration::from_millis(125);

#[cfg(not(target_os = "linux"))]
thread_local! {
    // The tray must only be touched from the thread that created it (the
    // main thread, which also runs the Floem event loop and thus set_busy).
    static TRAY: std::cell::RefCell<Option<TrayIcon>> =
        const { std::cell::RefCell::new(None) };
}

/// Pushes the busy/idle state onto the tray icon. On Linux the update is
/// forwarded to the GTK tray thread; on Windows/macOS it is applied directly,
/// so `set_busy` must be called on the thread that ran [`init`].
pub struct TrayHandle {
    #[cfg(target_os = "linux")]
    busy_tx: crossbeam_channel::Sender<bool>,
    #[cfg(not(target_os = "linux"))]
    busy: Rc<Cell<bool>>,
    #[cfg(not(target_os = "linux"))]
    spinning: Rc<Cell<bool>>,
    last: Cell<Option<bool>>,
}

impl TrayHandle {
    pub fn set_busy(&self, busy: bool) {
        if self.last.get() == Some(busy) {
            return;
        }
        self.last.set(Some(busy));

        #[cfg(target_os = "linux")]
        {
            let _ = self.busy_tx.send(busy);
        }

        #[cfg(not(target_os = "linux"))]
        {
            self.busy.set(busy);
            with_tray(|tray| {
                let _ = tray.set_tooltip(Some(tooltip(busy)));
            });
            // Going idle needs no action here: the running spin loop notices
            // on its next tick, restores the idle icon, and stops itself.
            if busy && !self.spinning.get() {
                self.spinning.set(true);
                spin(self.busy.clone(), self.spinning.clone(), 0);
            }
        }
    }
}

/// One animation frame on Windows/macOS, self-rescheduling on the Floem
/// event loop until the busy flag clears.
#[cfg(not(target_os = "linux"))]
fn spin(busy: Rc<Cell<bool>>, spinning: Rc<Cell<bool>>, phase: u8) {
    if !busy.get() {
        spinning.set(false);
        with_tray(|tray| set_frame(tray, None));
        return;
    }
    with_tray(|tray| set_frame(tray, Some(phase)));
    floem::action::exec_after(SPINNER_TICK, move |_| {
        spin(busy, spinning, (phase + 1) % SPINNER_STEPS);
    });
}

#[cfg(not(target_os = "linux"))]
fn with_tray(f: impl FnOnce(&TrayIcon)) {
    TRAY.with(|slot| {
        if let Some(tray) = slot.borrow().as_ref() {
            f(tray);
        }
    });
}

pub fn init(paused: Arc<AtomicBool>, ui_tx: crossbeam_channel::Sender<UiMsg>) -> TrayHandle {
    #[cfg(target_os = "linux")]
    let handle = {
        let (busy_tx, busy_rx) = crossbeam_channel::unbounded::<bool>();
        std::thread::Builder::new()
            .name("boinc-tray".into())
            .spawn(move || {
                if gtk::init().is_err() {
                    eprintln!("boinc: GTK unavailable; running without tray icon");
                    return;
                }
                match build_tray() {
                    Ok(tray) => {
                        // Poll busy updates from the UI thread and, while
                        // busy, advance the spinner one frame per tick; the
                        // timer closure keeps the tray alive for the app's
                        // lifetime.
                        let mut busy = false;
                        let mut phase: u8 = 0;
                        gtk::glib::timeout_add_local(SPINNER_TICK, move || {
                            if let Some(next) = busy_rx.try_iter().last() {
                                if next != busy {
                                    busy = next;
                                    phase = 0;
                                    let _ = tray.set_tooltip(Some(tooltip(busy)));
                                    if !busy {
                                        set_frame(&tray, None);
                                    }
                                }
                            }
                            if busy {
                                set_frame(&tray, Some(phase));
                                phase = (phase + 1) % SPINNER_STEPS;
                            }
                            gtk::glib::ControlFlow::Continue
                        });
                        gtk::main();
                    }
                    Err(err) => eprintln!("boinc: tray unavailable: {err}"),
                }
            })
            .expect("tray thread can be spawned");
        TrayHandle {
            busy_tx,
            last: Cell::new(None),
        }
    };

    #[cfg(not(target_os = "linux"))]
    let handle = {
        // Must be created on the thread running the (winit) event loop.
        match build_tray() {
            Ok(tray) => TRAY.with(|slot| *slot.borrow_mut() = Some(tray)),
            Err(err) => eprintln!("boinc: tray unavailable: {err}"),
        }
        TrayHandle {
            busy: Rc::new(Cell::new(false)),
            spinning: Rc::new(Cell::new(false)),
            last: Cell::new(None),
        }
    };

    spawn_menu_forwarder(paused, ui_tx);
    handle
}

fn build_tray() -> Result<TrayIcon, Box<dyn std::error::Error>> {
    let menu = Menu::new();
    menu.append(&MenuItem::with_id(OPEN_ID, "Open Boinc", true, None))?;
    menu.append(&CheckMenuItem::with_id(
        PAUSE_ID,
        "Pause conversions",
        true,
        false,
        None,
    ))?;
    menu.append(&PredefinedMenuItem::separator())?;
    menu.append(&MenuItem::with_id(QUIT_ID, "Quit", true, None))?;

    let (rgba, width, height) = icon_rgba(None);
    let tray = TrayIconBuilder::new()
        .with_menu(Box::new(menu))
        .with_tooltip(tooltip(false))
        .with_icon(Icon::from_rgba(rgba, width, height)?)
        .build()?;
    Ok(tray)
}

/// Swap the tray icon to one animation frame (`Some(phase)`) or back to the
/// idle disc (`None`). Tooltips are set on busy/idle transitions, not here.
fn set_frame(tray: &TrayIcon, spinner: Option<u8>) {
    let (rgba, width, height) = icon_rgba(spinner);
    match Icon::from_rgba(rgba, width, height) {
        Ok(icon) => {
            if let Err(err) = tray.set_icon(Some(icon)) {
                eprintln!("boinc: could not update tray icon: {err}");
            }
        }
        Err(err) => eprintln!("boinc: could not build tray icon: {err}"),
    }
}

fn spawn_menu_forwarder(paused: Arc<AtomicBool>, ui_tx: crossbeam_channel::Sender<UiMsg>) {
    std::thread::Builder::new()
        .name("boinc-tray-menu".into())
        .spawn(move || {
            let receiver = MenuEvent::receiver();
            while let Ok(event) = receiver.recv() {
                match event.id.0.as_str() {
                    OPEN_ID => {
                        let _ = ui_tx.send(UiMsg::OpenWindow);
                    }
                    // The check mark itself is toggled by muda.
                    PAUSE_ID => {
                        paused.fetch_xor(true, Ordering::Relaxed);
                    }
                    QUIT_ID => {
                        let _ = ui_tx.send(UiMsg::Quit);
                    }
                    _ => {}
                }
            }
        })
        .expect("tray menu thread can be spawned");
}

/// A simple generated icon (filled orange disc) so no binary asset is needed.
/// `spinner` selects one frame of the busy animation: a white three-quarter
/// ring punched into the disc, with its gap rotated one step per frame so a
/// running conversion is visible at a glance.
fn icon_rgba(spinner: Option<u8>) -> (Vec<u8>, u32, u32) {
    use std::f32::consts::{FRAC_PI_2, TAU};

    let (width, height) = (32u32, 32u32);
    let mut data = vec![0u8; (width * height * 4) as usize];
    let center = 15.5f32;
    let radius = 14.0f32;
    let gap_start =
        spinner.map(|phase| f32::from(phase % SPINNER_STEPS) / f32::from(SPINNER_STEPS) * TAU);
    for y in 0..height {
        for x in 0..width {
            let dx = x as f32 - center;
            let dy = y as f32 - center;
            let dist = (dx * dx + dy * dy).sqrt();
            let i = ((y * width + x) * 4) as usize;
            if dist <= radius {
                data[i..i + 4].copy_from_slice(&[255, 140, 0, 255]);
            }
            let on_ring = (5.5..=8.5).contains(&dist);
            if on_ring
                && gap_start.is_some_and(|gap| (dy.atan2(dx) - gap).rem_euclid(TAU) >= FRAC_PI_2)
            {
                data[i..i + 4].copy_from_slice(&[255, 255, 255, 255]);
            }
        }
    }
    (data, width, height)
}
