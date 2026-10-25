//! System Telemetry Dashboard Viewer Component.
//!
//! Provides a standardized telemetry dashboard modal surfacing real-time system performance,
//! operations throughput, error rate, memory footprint, and channel-level health metrics.

use crate::TemplateApp;
use eframe::egui;
use spodeian_web_utils::copy_to_clipboard;

pub fn render_telemetry_modal(app: &mut TemplateApp, ctx: &egui::Context) {
    if !app.show_telemetry_modal {
        return;
    }

    let mut is_open = app.show_telemetry_modal;
    egui::Window::new("📊 System Performance Telemetry")
        .open(&mut is_open)
        .collapsible(false)
        .resizable(true)
        .default_width(580.0)
        .show(ctx, |ui| {
            ui.add_space(4.0);

            // Header status banner
            ui.horizontal(|ui| {
                ui.heading(&app.telemetry_data.title);
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    let badge_text = if app.telemetry_data.success_rate > 99.0 {
                        "● System Operational"
                    } else {
                        "▲ Degraded"
                    };
                    let badge_color = if app.telemetry_data.success_rate > 99.0 {
                        egui::Color32::from_rgb(70, 190, 110)
                    } else {
                        egui::Color32::from_rgb(220, 150, 40)
                    };
                    ui.colored_label(badge_color, badge_text);
                });
            });

            ui.separator();
            ui.add_space(4.0);

            // KPI Metric Cards Grid
            egui::Grid::new("telemetry_kpi_grid")
                .num_columns(4)
                .spacing([12.0, 10.0])
                .show(ui, |ui| {
                    render_kpi_card(
                        ui,
                        "Uptime",
                        &format!("{}s", app.telemetry_data.uptime_secs),
                        "Online Duration",
                    );
                    render_kpi_card(
                        ui,
                        "Operations",
                        &format!("{}", app.telemetry_data.total_operations),
                        "Total Invocations",
                    );
                    render_kpi_card(
                        ui,
                        "Success Rate",
                        &format!("{:.2}%", app.telemetry_data.success_rate),
                        "Zero-Fault SLA",
                    );
                    render_kpi_card(
                        ui,
                        "Throughput",
                        &format!("{:.0} ops/s", app.telemetry_data.throughput_ops_sec),
                        "Peak Velocity",
                    );
                    ui.end_row();
                });

            ui.add_space(8.0);
            ui.separator();
            ui.add_space(4.0);

            // Channel Telemetry Table
            ui.label(egui::RichText::new("Subsystem Channels & Health Telemetry").strong());
            ui.add_space(2.0);

            egui::ScrollArea::vertical()
                .max_height(200.0)
                .show(ui, |ui| {
                    egui::Grid::new("telemetry_channels_grid")
                        .striped(true)
                        .num_columns(4)
                        .spacing([24.0, 6.0])
                        .show(ui, |ui| {
                            ui.label(egui::RichText::new("Channel").strong());
                            ui.label(egui::RichText::new("Processed").strong());
                            ui.label(egui::RichText::new("Latency").strong());
                            ui.label(egui::RichText::new("Status").strong());
                            ui.end_row();

                            for ch in &app.telemetry_data.channels {
                                ui.label(&ch.name);
                                ui.label(format!("{}", ch.count));
                                ui.label(format!("{:.2} ms", ch.latency_ms));
                                let color = if ch.status == "Healthy" || ch.status == "Optimal" {
                                    egui::Color32::from_rgb(70, 190, 110)
                                } else {
                                    egui::Color32::from_rgb(220, 150, 40)
                                };
                                ui.colored_label(color, &ch.status);
                                ui.end_row();
                            }
                        });
                });

            ui.add_space(10.0);
            ui.separator();

            // Actions: Copy Snapshot JSON / Close
            ui.horizontal(|ui| {
                if ui.button("📋 Copy Telemetry Snapshot").clicked() {
                    if let Ok(json) = serde_json::to_string_pretty(&app.telemetry_data) {
                        copy_to_clipboard(&json);
                    }
                }

                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui.button("Close").clicked() {
                        app.show_telemetry_modal = false;
                    }
                });
            });
        });

    app.show_telemetry_modal = is_open;
}

fn render_kpi_card(ui: &mut egui::Ui, title: &str, value: &str, subtitle: &str) {
    egui::Frame::group(ui.style())
        .inner_margin(egui::Margin::symmetric(10, 8))
        .show(ui, |ui| {
            ui.vertical(|ui| {
                ui.label(egui::RichText::new(title).size(11.0).weak());
                ui.label(egui::RichText::new(value).size(17.0).strong());
                ui.label(egui::RichText::new(subtitle).size(10.0).weak());
            });
        });
}
