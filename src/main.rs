#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod api;
mod app;
mod audio;
mod autostart;
mod config;
mod hotkeys;
mod notification;
mod tray;

use windows_sys::core::w;
use windows_sys::Win32::Foundation::{GetLastError, ERROR_ALREADY_EXISTS};
use windows_sys::Win32::System::Threading::CreateMutexW;

fn load_app_icon() -> egui::IconData {
    let icon_bytes = include_bytes!("../assets/icon.ico");
    let icon_dir = ico::IconDir::read(std::io::Cursor::new(icon_bytes.as_ref()))
        .expect("assets/icon.ico is not a valid ICO container");

    // Prefer the largest available frame for best on-screen quality.
    let entry = icon_dir
        .entries()
        .iter()
        .max_by_key(|e| e.width() * e.height())
        .expect("assets/icon.ico contains no image entries");

    let image = entry
        .decode()
        .expect("failed to decode the largest frame in assets/icon.ico");

    egui::IconData {
        rgba: image.rgba_data().to_vec(),
        width: image.width(),
        height: image.height(),
    }
}

fn acquire_single_instance_lock() -> bool {
    unsafe {
        let handle = CreateMutexW(std::ptr::null(), 1, w!("Global\\PearDesktopHotkeys_Mutex"));
        if handle.is_null() {
            return true;
        }
        if GetLastError() == ERROR_ALREADY_EXISTS {
            false
        } else {
            let _ = handle;
            true
        }
    }
}

fn init_logging() {
    if let Ok(log_dir) = config::Config::dir() {
        let file_appender = tracing_appender::rolling::never(&log_dir, "pear.log");
        let (non_blocking, guard) = tracing_appender::non_blocking(file_appender);
        std::mem::forget(guard);
        tracing_subscriber::fmt()
            .with_writer(non_blocking)
            .with_ansi(false)
            .init();
    } else {
        tracing_subscriber::fmt().init();
    }
}

fn main() {
    init_logging();

    if !acquire_single_instance_lock() {
        tracing::warn!("Already running — exiting.");
        std::process::exit(0);
    }

    if let Err(e) = notification::register_aumid() {
        tracing::warn!("failed to register AUMID: {e:#}");
    }
    notification::set_process_aumid();

    let config = config::Config::load();

    let native_options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([480.0, 460.0])
            .with_min_inner_size([420.0, 400.0])
            .with_icon(load_app_icon()),
        // with_visible(false) is intentionally NOT used here — it's a known,
        // unreliable eframe/winit behavior where the window shows anyway
        // regardless of this setting. Hiding on startup is instead handled
        // explicitly in app.rs via raw ShowWindow(SW_HIDE), the same
        // mechanism already proven to work for "Hide to tray".
        ..Default::default()
    };

    eframe::run_native(
        "Pear Desktop — Settings",
        native_options,
        Box::new(|cc| Ok(Box::new(app::PearApp::new(cc, config)))),
    )
    .expect("eframe failed to run");
}
