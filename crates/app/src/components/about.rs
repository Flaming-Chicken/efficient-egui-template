//! About & Architecture view page.

use crate::{ScreenConstraints, TemplateApp};
use eframe::egui;

pub fn render_about_page(
    app: &mut TemplateApp,
    ui: &mut egui::Ui,
    constraints: &ScreenConstraints,
) {
    egui::ScrollArea::vertical()
        .auto_shrink([false, false])
        .show(ui, |ui| {
            ui.add_space(8.0);

            // Hero / Title
            ui.vertical_centered(|ui| {
                ui.heading("Serverless & Desktop Template");
                ui.label(
                    egui::RichText::new("A modular, high-performance foundation for Rust WebAssembly and Native Desktop apps")
                        .color(ui.visuals().weak_text_color()),
                );
                ui.add_space(4.0);
                ui.label(
                    egui::RichText::new("Version 0.1.0 • Rust 2024 • egui 0.36")
                        .small()
                        .monospace(),
                );
            });

            ui.add_space(16.0);

            // Card 1: Architecture Overview
            ui.group(|ui| {
                ui.set_width(ui.available_width());
                ui.heading("🏛️ Four-Tier Architecture");
                ui.add_space(4.0);
                ui.label("This template is cleanly decoupled into four single-responsibility crates:");
                ui.add_space(6.0);

                egui::Grid::new("arch_grid")
                    .num_columns(2)
                    .spacing([16.0, 8.0])
                    .striped(true)
                    .show(ui, |ui| {
                        ui.label(egui::RichText::new("crates/shared").strong().monospace());
                        ui.label("Domain entities, state models, JSON/CSV/BSON serialization, and business rules.");
                        ui.end_row();

                        ui.label(egui::RichText::new("crates/app").strong().monospace());
                        ui.label("Universal UI view-controller, responsive layout, theme engine, and persistence manager.");
                        ui.end_row();

                        ui.label(egui::RichText::new("crates/desktop").strong().monospace());
                        ui.label("Native desktop runner via eframe/winit with viewport configuration and mimalloc.");
                        ui.end_row();

                        ui.label(egui::RichText::new("crates/web").strong().monospace());
                        ui.label("WebAssembly client entrypoint, Trunk bundling, PWA service worker, and edge headers.");
                        ui.end_row();
                    });
            });

            ui.add_space(12.0);

            // Card 2: System & Storage Status
            ui.group(|ui| {
                ui.set_width(ui.available_width());
                ui.heading("📊 Live Diagnostics & Storage Status");
                ui.add_space(4.0);

                let backend_label = app.storage_diag.backend.label();
                let persistence_status = match app.storage_diag.is_persisted {
                    Some(true) => "Persistent (Browser storage manager guaranteed)",
                    Some(false) => "Ephemeral (May be purged under storage pressure)",
                    None => "Native / Unknown",
                };
                let total_items = app.state.collection.total_count();
                let completed_items = app.state.collection.completed_count();

                egui::Grid::new("diag_grid")
                    .num_columns(2)
                    .spacing([16.0, 6.0])
                    .show(ui, |ui| {
                        ui.label("Active Storage Tier:");
                        ui.label(egui::RichText::new(backend_label).monospace());
                        ui.end_row();

                        ui.label("Persistence Mode:");
                        ui.label(egui::RichText::new(persistence_status).monospace());
                        ui.end_row();

                        ui.label("Active Visual Theme:");
                        ui.label(format!("{} {}", app.state.config.theme.icon(), app.state.config.theme.label()));
                        ui.end_row();

                        ui.label("Memory Tier (RAM):");
                        ui.label(format!("{} active items (Working limit: {} items)", app.state.collection.items.len(), app.storage_diag.memory_capacity_limit));
                        ui.end_row();

                        ui.label("IndexedDB Fallback:");
                        let idb_text = if app.storage_diag.idb_active || app.storage_diag.fallback_tier_active {
                            "Active (Protected against RAM overfill)"
                        } else {
                            "Synchronized & Ready (Durable Backing)"
                        };
                        ui.label(egui::RichText::new(idb_text).color(egui::Color32::from_rgb(80, 200, 100)).monospace());
                        ui.end_row();

                        ui.label("Total Managed Items:");
                        ui.label(format!("{} items ({} completed)", total_items, completed_items));
                        ui.end_row();

                        ui.label("Viewport Layout:");
                        let mode = if constraints.is_mobile { "Mobile Portrait" } else { "Widescreen Desktop" };
                        let size = ui.available_size();
                        ui.label(format!("{} (~{}x{})", mode, size.x as i32, size.y as i32));
                        ui.end_row();
                    });

                ui.add_space(8.0);
                if ui.button("Inspect Full Storage Details").clicked() {
                    app.show_storage_modal = true;
                }
            });

            ui.add_space(12.0);

            // Card 3: Shortcuts & User Guide
            ui.group(|ui| {
                ui.set_width(ui.available_width());
                ui.heading("⌨️ Keyboard Shortcuts & Tips");
                ui.add_space(4.0);

                egui::Grid::new("shortcuts_grid")
                    .num_columns(2)
                    .spacing([16.0, 6.0])
                    .show(ui, |ui| {
                        ui.label(egui::RichText::new("Enter").monospace().strong());
                        ui.label("Quickly add new task/item from the input form.");
                        ui.end_row();

                        ui.label(egui::RichText::new("Escape").monospace().strong());
                        ui.label("Close any active modal dialog or overlay.");
                        ui.end_row();

                        ui.label(egui::RichText::new("Data Import/Export").strong());
                        ui.label("Supports JSON, CSV (RFC 4180), and Compressed BSON backups.");
                        ui.end_row();
                    });
            });

            ui.add_space(20.0);
        });
}
