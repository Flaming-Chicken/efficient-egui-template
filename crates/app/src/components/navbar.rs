//! Top navigation bar and global header controls.

use crate::{ScreenConstraints, TemplateApp, storage_manager::*};
use eframe::egui;

/// Renders the responsive top navigation bar, secondary overflow menu, and action toast badge.
pub fn render_navbar(app: &mut TemplateApp, ui: &mut egui::Ui, constraints: &ScreenConstraints) {
    egui::Panel::top("top_panel").show(ui, |ui| {
        ui.add_space(4.0);
        let title_text = if constraints.is_mobile {
            "Serverless & Desktop"
        } else {
            "Serverless & Desktop Template"
        };
        let header_row_height = if constraints.is_mobile { 44.0 } else { 32.0 };
        let use_compact_menu = constraints.is_mobile || ui.available_width() < 1120.0;

        ui.horizontal(|ui| {
            ui.set_height(header_row_height);

            if constraints.is_mobile {
                ui.label(egui::RichText::new(title_text).size(17.0).strong());
            } else {
                ui.heading(title_text);
            }

            ui.add_space(6.0);
            if ui
                .selectable_label(
                    app.state.config.active_page == shared::AppPage::Dashboard,
                    if constraints.is_mobile {
                        "Dash"
                    } else {
                        "📊 Dashboard"
                    },
                )
                .on_hover_text("Task and Item Management View (Ctrl+1)")
                .clicked()
            {
                let _ = app.execute_command("nav.dashboard");
            }

            if ui
                .selectable_label(
                    app.state.config.active_page == shared::AppPage::Analytics,
                    if constraints.is_mobile {
                        "Stats"
                    } else {
                        "📈 Analytics"
                    },
                )
                .on_hover_text("Priority Distribution & 2D Parametric Viewport (Ctrl+3)")
                .clicked()
            {
                let _ = app.execute_command("nav.analytics");
            }

            if ui
                .selectable_label(
                    app.state.config.active_page == shared::AppPage::About,
                    if constraints.is_mobile {
                        "About"
                    } else {
                        "ℹ About"
                    },
                )
                .on_hover_text("Template Architecture, Diagnostics, and Documentation (Ctrl+2)")
                .clicked()
            {
                let _ = app.execute_command("nav.about");
            }

            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                let theme_text = if constraints.is_mobile {
                    app.state.config.theme.icon().to_string()
                } else {
                    format!(
                        "{} {}",
                        app.state.config.theme.icon(),
                        app.state.config.theme.display_label()
                    )
                };

                if ui
                    .button(theme_text)
                    .on_hover_text("Cycle visual themes (Ctrl+Shift+T)")
                    .clicked()
                {
                    let _ = app.execute_command("theme.cycle");
                }

                if use_compact_menu {
                    ui.menu_button("☰ Menu", |ui| {
                        render_overflow_menu_items(app, ui);
                    });
                } else {
                    if ui
                        .selectable_label(app.state.config.dyslexia_friendly, "Aa")
                        .on_hover_text("Toggle Dyslexia-Friendly Reading Mode (Ctrl+Shift+D)")
                        .clicked()
                    {
                        let _ = app.execute_command("a11y.dyslexia");
                    }

                    if ui
                        .button("⌘K Commands")
                        .on_hover_text("Open Command Palette (Ctrl+K / Cmd+K)")
                        .clicked()
                    {
                        app.command_palette.open();
                    }

                    if ui
                        .add_enabled(app.undo_stack.can_redo(), egui::Button::new("Redo"))
                        .on_hover_text("Redo collection change (Ctrl+Y)")
                        .clicked()
                    {
                        let _ = app.execute_command("edit.redo");
                    }

                    if ui
                        .add_enabled(app.undo_stack.can_undo(), egui::Button::new("Undo"))
                        .on_hover_text("Undo last collection change (Ctrl+Z)")
                        .clicked()
                    {
                        let _ = app.execute_command("edit.undo");
                    }

                    ui.menu_button("Tools", |ui| {
                        render_overflow_menu_items(app, ui);
                    });
                }

                if let Some((ref msg, _)) = app.status_toast {
                    ui.label(
                        egui::RichText::new(msg)
                            .small()
                            .strong()
                            .color(ui.visuals().hyperlink_color),
                    );
                }
            });
        });
        ui.add_space(4.0);
    });
}

fn render_overflow_menu_items(app: &mut TemplateApp, ui: &mut egui::Ui) {
    if ui.button("⌘ Command Palette (Ctrl+K)").clicked() {
        app.command_palette.open();
        ui.close();
    }
    if ui
        .add_enabled(
            app.undo_stack.can_undo(),
            egui::Button::new("↺ Undo (Ctrl+Z)"),
        )
        .clicked()
    {
        let _ = app.execute_command("edit.undo");
        ui.close();
    }
    if ui
        .add_enabled(
            app.undo_stack.can_redo(),
            egui::Button::new("↻ Redo (Ctrl+Y)"),
        )
        .clicked()
    {
        let _ = app.execute_command("edit.redo");
        ui.close();
    }
    ui.separator();
    if ui
        .selectable_label(
            app.state.config.dyslexia_friendly,
            "Aa Dyslexia-Friendly Mode (Ctrl+Shift+D)",
        )
        .clicked()
    {
        let _ = app.execute_command("a11y.dyslexia");
        ui.close();
    }
    if ui
        .selectable_label(app.state.config.zen_mode, "⛶ Zen Mode (F11)")
        .clicked()
    {
        let _ = app.execute_command("view.zen_mode");
        ui.close();
    }
    ui.separator();
    if ui
        .button("Export Data (JSON / CSV / BSON / SVG / HTML)")
        .clicked()
    {
        let format = app.selected_export_format;
        app.open_export_dialog(format);
        ui.close();
    }
    if ui
        .button("Copy Shareable State Link (Ctrl+Shift+L)")
        .clicked()
    {
        let _ = app.execute_command("data.copy_share_link");
        ui.close();
    }
    if ui.button("Import Data (Ctrl+I)").clicked() {
        let _ = app.execute_command("data.import");
        ui.close();
    }
    if app.storage_diag.pwa_install_available && !app.storage_diag.is_pwa_installed {
        if ui.button("Install Web App").clicked() {
            trigger_pwa_install();
            ui.close();
        }
    }
    ui.separator();
    let storage_label = match app.storage_diag.is_persisted {
        Some(true) => "Storage: Persistent",
        Some(false) => "Storage: Ephemeral",
        None => "Storage Diagnostics",
    };
    if ui.button(storage_label).clicked() {
        let _ = app.execute_command("system.storage");
        ui.close();
    }
    if ui.button("Telemetry & Live Logs").clicked() {
        let _ = app.execute_command("system.telemetry");
        ui.close();
    }
    if ui.button("Help & Accessibility (F1)").clicked() {
        let _ = app.execute_command("system.help");
        ui.close();
    }
    if ui.button("Reset Sample Items").clicked() {
        app.show_reset_dialog = true;
        ui.close();
    }
}
