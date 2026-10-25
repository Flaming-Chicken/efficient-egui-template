//! Native desktop application runner for the Serverless & Desktop Template.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

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
            .with_title("Serverless & Desktop Template")
            .with_inner_size([1100.0, 750.0])
            .with_min_inner_size([700.0, 500.0]),
        ..Default::default()
    };

    eframe::run_native(
        "Serverless & Desktop Template",
        options,
        Box::new(|cc| Ok(Box::new(TemplateApp::new(cc)))),
    )
}
