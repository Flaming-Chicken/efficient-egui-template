//! Telemetry dashboard modal and live tracing log console wrapper.

use crate::TemplateApp;
use eframe::egui;
pub use spodeian_ui::render_kpi_card;
use spodeian_ui::render_telemetry_dashboard_modal;

/// Updates live telemetry metrics and renders the shared telemetry dashboard modal alongside
/// the live tracing log console window.
pub fn render_telemetry_modal(app: &mut TemplateApp, ctx: &egui::Context) {
    if app.show_telemetry_modal {
        shared::refresh_telemetry_dashboard(
            &mut app.telemetry_data,
            app.storage_diag.backend.label(),
            app.state.collection.items.len(),
        );

        render_telemetry_dashboard_modal(ctx, &mut app.show_telemetry_modal, &app.telemetry_data);
    }

    if app.show_log_console {
        let mut open = app.show_log_console;
        egui::Window::new("Live Telemetry Log Console")
            .open(&mut open)
            .default_size(egui::vec2(620.0, 260.0))
            .show(ctx, |ui| {
                app.log_console.ui(ui);
            });
        app.show_log_console = open;
    }
}
