#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod app;
mod config;
mod logging;
mod serial_types;
mod serial_worker;
mod terminal_buffer;

use crossbeam_channel::bounded;
use eframe::egui;

fn main() -> eframe::Result {
    // Initialize logging
    logging::init();

    // Set up bounded crossbeam channels for thread communication
    let (cmd_tx, cmd_rx) = bounded(500);
    let (event_tx, event_rx) = bounded(500);

    let mut viewport = egui::ViewportBuilder::default()
        .with_title("AA Rust Serial Monitor")
        .with_inner_size([1250.0, 700.0]);

    if let Some(icon) = load_icon() {
        viewport = viewport.with_icon(icon);
    }

    let options = eframe::NativeOptions {
        viewport,
        persist_window: false,
        ..Default::default()
    };

    eframe::run_native(
        "aa_rust_serial_monitor",
        options,
        Box::new(move |cc| {
            // Spawn background worker thread
            serial_worker::SerialWorker::spawn(cmd_rx, event_tx, cc.egui_ctx.clone());

            let app = app::SerialApp::new(cc, cmd_tx, event_rx);
            Ok(Box::new(app))
        }),
    )
}

fn load_icon() -> Option<egui::IconData> {
    let icon_bytes = include_bytes!("../favicon.png");
    match image::load_from_memory_with_format(icon_bytes, image::ImageFormat::Png) {
        Ok(img) => {
            let rgba = img.to_rgba8().into_raw();
            let width = img.width();
            let height = img.height();
            Some(egui::IconData {
                rgba,
                width,
                height,
            })
        }
        Err(_) => None,
    }
}
