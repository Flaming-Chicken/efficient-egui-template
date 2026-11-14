//! System Telemetry Dashboard Viewer Component.
//!
//! Refreshes live `spodeian_telemetry` counters and delegates modal & KPI card rendering to
//! `spodeian_ui::render_telemetry_dashboard_modal`.

use crate::TemplateApp;
use eframe::egui;
pub use spodeian_ui::render_kpi_card;
use spodeian_ui::render_telemetry_dashboard_modal;

/// Updates live telemetry metrics and renders the shared telemetry dashboard modal.
pub fn render_telemetry_modal(app: &mut TemplateApp, ctx: &egui::Context) {
    if !app.show_telemetry_modal {
        return;
    }

    shared::refresh_telemetry_dashboard(
        &mut app.telemetry_data,
        app.storage_diag.backend.label(),
        app.state.collection.items.len(),
    );

    render_telemetry_dashboard_modal(ctx, &mut app.show_telemetry_modal, &app.telemetry_data);
}
