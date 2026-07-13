//! Tray icon with Open / Pause / Quit menu.
//!
//! Spike outcome (plan.md task 3.1): tray-icon and Floem's winit event loop
//! coexist, but differently per platform. On Linux, tray-icon requires GTK,
//! so the tray runs on its own thread with a private `gtk::main()` loop —
//! fully independent of Floem. On Windows and macOS the tray icon is created
//! on the main thread before Floem's event loop starts, which then pumps its
//! native events. Menu clicks arrive on muda's global channel and are
//! forwarded to the UI/worker from a small forwarder thread.
//!
//! v1 residency limitation: Floem 0.2 exits its event loop when the last
//! window closes (except on macOS) and offers no close-interception, so
//! closing the main window quits the app. True close-to-tray needs a Floem
//! change or a hidden keep-alive window; revisit in Phase 7.

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use tray_icon::menu::{CheckMenuItem, Menu, MenuEvent, MenuItem, PredefinedMenuItem};
use tray_icon::{Icon, TrayIcon, TrayIconBuilder};

use crate::state::UiMsg;

const OPEN_ID: &str = "open";
const PAUSE_ID: &str = "pause";
const QUIT_ID: &str = "quit";

pub fn init(paused: Arc<AtomicBool>, ui_tx: crossbeam_channel::Sender<UiMsg>) {
    #[cfg(target_os = "linux")]
    {
        std::thread::Builder::new()
            .name("boinc-tray".into())
            .spawn(|| {
                if gtk::init().is_err() {
                    eprintln!("boinc: GTK unavailable; running without tray icon");
                    return;
                }
                match build_tray() {
                    Ok(_tray) => gtk::main(), // keeps _tray alive for the app's lifetime
                    Err(err) => eprintln!("boinc: tray unavailable: {err}"),
                }
            })
            .expect("tray thread can be spawned");
    }

    #[cfg(not(target_os = "linux"))]
    {
        // Must be created on the thread running the (winit) event loop.
        match build_tray() {
            Ok(tray) => std::mem::forget(tray),
            Err(err) => eprintln!("boinc: tray unavailable: {err}"),
        }
    }

    spawn_menu_forwarder(paused, ui_tx);
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

    let (rgba, width, height) = icon_rgba();
    let tray = TrayIconBuilder::new()
        .with_menu(Box::new(menu))
        .with_tooltip("Boinc file converter")
        .with_icon(Icon::from_rgba(rgba, width, height)?)
        .build()?;
    Ok(tray)
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
fn icon_rgba() -> (Vec<u8>, u32, u32) {
    let (width, height) = (32u32, 32u32);
    let mut data = vec![0u8; (width * height * 4) as usize];
    let center = 15.5f32;
    let radius = 14.0f32;
    for y in 0..height {
        for x in 0..width {
            let dx = x as f32 - center;
            let dy = y as f32 - center;
            if (dx * dx + dy * dy).sqrt() <= radius {
                let i = ((y * width + x) * 4) as usize;
                data[i..i + 4].copy_from_slice(&[255, 140, 0, 255]);
            }
        }
    }
    (data, width, height)
}
