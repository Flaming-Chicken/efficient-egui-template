//! Native desktop application runner for the Serverless & Desktop Template.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

#[cfg(not(any(target_arch = "wasm32", target_os = "android", target_os = "ios")))]
#[global_allocator]
static GLOBAL: mimalloc::MiMalloc = mimalloc::MiMalloc;

use app::TemplateApp;
use eframe::NativeOptions;
use eframe::egui;
use spodeian_telemetry::init_default;

fn main() -> eframe::Result<()> {
    // Universal telemetry & logging initialization
    init_default();

    // Native window viewport configurations
    let options = NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title("Serverless & Desktop egui Template")
            .with_inner_size([1100.0, 750.0])
            .with_min_inner_size([320.0, 240.0]),
        ..Default::default()
    };

    eframe::run_native(
        "Serverless & Desktop egui Template",
        options,
        Box::new(|cc| Ok(Box::new(TemplateApp::new(cc)))),
    )
}
