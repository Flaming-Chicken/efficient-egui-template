//! Main egui application view controller, shell layout, and cross-platform runners.
#![allow(clippy::collapsible_if)]
#![allow(clippy::if_same_then_else)]

pub mod components;
pub mod storage_manager;

pub use components::*;
pub use storage_manager::*;

use eframe::egui;
use shared::{
    AppState, ItemCollection, Priority, ThemeMode, export_to_compressed_bson, export_to_csv,
    export_to_json,
};
pub use spodeian_ui::{CommandEntry, CommandPaletteWidget, ScreenConstraints, UndoStack};
use std::collections::HashMap;
#[allow(unused_imports)]
use tracing::{error, info, warn};

#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub enum ExportFormat {
    #[default]
    Json,
    Csv,
    Bson,
}

impl ExportFormat {
    pub fn label(self) -> &'static str {
        match self {
            Self::Json => "JSON File",
            Self::Csv => "CSV File",
            Self::Bson => "Compressed BSON (.bson)",
        }
    }
}

/// Returns the default command catalog wired into [`TemplateApp::command_palette`].
pub fn default_template_commands() -> Vec<CommandEntry> {
    vec![
        CommandEntry::new("nav.dashboard", "Go to Dashboard", "Ctrl+1", "Navigation"),
        CommandEntry::new("nav.about", "Go to About & Diagnostics", "Ctrl+2", "Navigation"),
        CommandEntry::new("edit.undo", "Undo Last Collection Change", "Ctrl+Z", "Edit"),
        CommandEntry::new("edit.redo", "Redo Collection Change", "Ctrl+Y", "Edit"),
        CommandEntry::new(
            "edit.clear_completed",
            "Clear Completed Items",
            "Ctrl+Shift+C",
            "Edit",
        ),
        CommandEntry::new(
            "theme.cycle",
            "Cycle Visual Theme (Dark / Light / High Contrast)",
            "Ctrl+Shift+T",
            "Appearance",
        ),
        CommandEntry::new(
            "a11y.dyslexia",
            "Toggle Dyslexia-Friendly Reading Mode",
            "Ctrl+Shift+D",
            "Accessibility",
        ),
        CommandEntry::new("data.export_json", "Export State as JSON", "Ctrl+E", "Data"),
        CommandEntry::new("data.export_csv", "Export Items as CSV", "", "Data"),
        CommandEntry::new("data.export_bson", "Export Compressed BSON Backup", "", "Data"),
        CommandEntry::new("data.import", "Open Import Dialog", "Ctrl+I", "Data"),
        CommandEntry::new("system.storage", "Inspect Storage & Persistence", "", "System"),
        CommandEntry::new("system.telemetry", "Open System Telemetry", "", "System"),
        CommandEntry::new("system.help", "Open Help & Accessibility Settings", "F1", "Help"),
    ]
}

fn normalize_shortcut_hint(hint: &str) -> String {
    let mut modifiers: Vec<&str> = Vec::new();
    let mut keys: Vec<String> = Vec::new();
    for raw in hint
        .split('+')
        .map(|s| s.trim().to_ascii_lowercase())
        .filter(|s| !s.is_empty())
    {
        match raw.as_str() {
            "ctrl" | "control" => modifiers.push("ctrl"),
            "shift" => modifiers.push("shift"),
            "alt" | "option" => modifiers.push("alt"),
            "cmd" | "command" | "meta" | "super" | "win" => modifiers.push("cmd"),
            _ => keys.push(raw),
        }
    }
    modifiers.sort_unstable();
    modifiers.dedup();
    keys.sort();
    let mut combined: Vec<String> = modifiers.into_iter().map(str::to_string).collect();
    combined.extend(keys);
    combined.join("+")
}

/// Detects duplicate non-empty keyboard shortcut bindings across `commands`, returning
/// `(normalized_shortcut, first_command_id, conflicting_command_id)` tuples.
pub fn detect_shortcut_conflicts(commands: &[CommandEntry]) -> Vec<(String, String, String)> {
    let mut seen: HashMap<String, String> = HashMap::new();
    let mut conflicts = Vec::new();

    for cmd in commands {
        let norm = normalize_shortcut_hint(&cmd.shortcut_hint);
        if norm.is_empty() {
            continue;
        }
        if let Some(first_id) = seen.get(&norm) {
            conflicts.push((norm, first_id.clone(), cmd.id.clone()));
        } else {
            seen.insert(norm, cmd.id.clone());
        }
    }

    conflicts
}

pub struct TemplateApp {
    pub state: AppState,
    pub current_theme: Option<ThemeMode>,
    pub current_dyslexia_mode: Option<bool>,
    pub current_ui_scale_percent: Option<u16>,
    pub command_palette: CommandPaletteWidget,
    pub undo_stack: UndoStack<ItemCollection>,
    pub last_recovered_error: Option<String>,
    pub show_reset_dialog: bool,
    pub show_help_dialog: bool,
    pub show_import_dialog: bool,
    pub import_text_buffer: String,
    pub import_result_message: Option<Result<String, String>>,
    pub show_export_dialog: Option<ExportFormat>,
    pub export_text_buffer: String,
    pub export_copied_notification: Option<f64>,
    pub selected_export_format: ExportFormat,
    pub new_item_title: String,
    pub new_item_description: String,
    pub new_item_priority: Priority,
    pub filter_priority: Option<Priority>,
    pub search_query: String,
    pub storage_diag: StorageDiagnostics,
    pub show_storage_modal: bool,
    pub show_telemetry_modal: bool,
    pub telemetry_data: shared::TelemetryDashboardData,
    pub dismissed_ephemeral_warning: bool,
    pub dismissed_quota_warning: bool,
    pub dismissed_combined_warning: bool,
    pub last_diag_poll_time: f64,
}

impl Default for TemplateApp {
    fn default() -> Self {
        let state = AppState::default();
        let mut telemetry_data = shared::default_template_telemetry();
        let storage_diag = query_storage_diagnostics();
        shared::refresh_telemetry_dashboard(
            &mut telemetry_data,
            storage_diag.backend.label(),
            state.collection.items.len(),
        );
        Self {
            state,
            current_theme: None,
            current_dyslexia_mode: None,
            current_ui_scale_percent: None,
            command_palette: CommandPaletteWidget::new(default_template_commands()),
            undo_stack: UndoStack::new(64),
            last_recovered_error: None,
            show_reset_dialog: false,
            show_help_dialog: false,
            show_import_dialog: false,
            import_text_buffer: String::new(),
            import_result_message: None,
            show_export_dialog: None,
            export_text_buffer: String::new(),
            export_copied_notification: None,
            selected_export_format: ExportFormat::default(),
            new_item_title: String::new(),
            new_item_description: String::new(),
            new_item_priority: Priority::Medium,
            filter_priority: None,
            search_query: String::new(),
            storage_diag,
            show_storage_modal: false,
            show_telemetry_modal: false,
            telemetry_data,
            dismissed_ephemeral_warning: false,
            dismissed_quota_warning: false,
            dismissed_combined_warning: false,
            last_diag_poll_time: 0.0,
        }
    }
}

impl TemplateApp {
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        info!("Initializing Serverless & Desktop Template App...");

        let state = load_state_multi_tier(cc.storage).unwrap_or_else(|| {
            warn!("No saved state found in storage, initializing fresh defaults.");
            AppState::default()
        });

        Self {
            state,
            ..Default::default()
        }
    }

    /// Captures the current item collection onto [`TemplateApp::undo_stack`] before a mutation.
    pub fn record_undo_snapshot(&mut self) {
        self.undo_stack.push(self.state.collection.clone());
    }

    /// Reverts the last item collection mutation, returning `true` if a state was restored.
    pub fn undo(&mut self) -> bool {
        if let Some(prev) = self.undo_stack.undo(self.state.collection.clone()) {
            self.state.collection = prev;
            self.persist_state();
            true
        } else {
            false
        }
    }

    /// Reapplies the most recently undone collection state, returning `true` if restored.
    pub fn redo(&mut self) -> bool {
        if let Some(next) = self.undo_stack.redo(self.state.collection.clone()) {
            self.state.collection = next;
            self.persist_state();
            true
        } else {
            false
        }
    }

    /// Executes a registered command ID from the command palette or shortcut handler.
    pub fn execute_command(&mut self, command_id: &str) -> bool {
        match command_id {
            "nav.dashboard" => {
                self.state.config.active_page = shared::AppPage::Dashboard;
                self.persist_state();
                true
            }
            "nav.about" => {
                self.state.config.active_page = shared::AppPage::About;
                self.persist_state();
                true
            }
            "edit.undo" => self.undo(),
            "edit.redo" => self.redo(),
            "edit.clear_completed" => {
                self.record_undo_snapshot();
                self.state.collection.clear_completed();
                self.persist_state();
                true
            }
            "theme.cycle" => {
                self.state.config.theme = self.state.config.theme.next();
                self.persist_state();
                true
            }
            "a11y.dyslexia" => {
                self.state.config.dyslexia_friendly = !self.state.config.dyslexia_friendly;
                self.persist_state();
                true
            }
            "data.export_json" => {
                self.selected_export_format = ExportFormat::Json;
                self.open_export_dialog(ExportFormat::Json);
                true
            }
            "data.export_csv" => {
                self.selected_export_format = ExportFormat::Csv;
                self.open_export_dialog(ExportFormat::Csv);
                true
            }
            "data.export_bson" => {
                self.selected_export_format = ExportFormat::Bson;
                self.open_export_dialog(ExportFormat::Bson);
                true
            }
            "data.import" => {
                self.show_import_dialog = true;
                self.import_text_buffer.clear();
                self.import_result_message = None;
                true
            }
            "system.storage" => {
                self.show_storage_modal = true;
                true
            }
            "system.telemetry" => {
                self.show_telemetry_modal = true;
                true
            }
            "system.help" => {
                self.show_help_dialog = true;
                true
            }
            _ => false,
        }
    }

    /// Executes `action` behind `catch_unwind`, storing any panic message into
    /// [`TemplateApp::last_recovered_error`] so the UI stays responsive.
    pub fn run_recoverable<F>(&mut self, label: &str, action: F) -> bool
    where
        F: FnOnce(&mut Self),
    {
        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            action(self);
        }));
        match result {
            Ok(()) => true,
            Err(payload) => {
                let detail = if let Some(s) = payload.downcast_ref::<&str>() {
                    (*s).to_string()
                } else if let Some(s) = payload.downcast_ref::<String>() {
                    s.clone()
                } else {
                    "Unexpected panic payload".to_string()
                };
                error!("Recovered from panic in {label}: {detail}");
                self.last_recovered_error = Some(format!("{label}: {detail}"));
                false
            }
        }
    }

    /// Immediately persist current state to multi-tier storage (active persistence)
    pub fn persist_state(&mut self) {
        let items_len = self.state.collection.items.len();
        self.storage_diag.memory_items_count = items_len;
        self.storage_diag.fallback_tier_active =
            items_len > MEMORY_WORKING_SET_LIMIT || self.storage_diag.idb_active;

        if let Ok(json_str) = serde_json::to_string(&self.state) {
            match save_state_multi_tier(DEDICATED_STORAGE_KEY, &json_str) {
                Ok(backend) => {
                    self.storage_diag.backend = backend;
                    if backend == StorageBackend::IndexedDb {
                        self.storage_diag.quota_exceeded = true;
                        self.storage_diag.idb_active = true;
                    } else {
                        self.storage_diag.quota_exceeded = false;
                    }
                }
                Err(_) => {
                    self.storage_diag.quota_exceeded = true;
                }
            }
        }
    }

    pub fn open_export_dialog(&mut self, format: ExportFormat) {
        if format == ExportFormat::Bson {
            if let Ok(bytes) = export_to_compressed_bson(&self.state) {
                use base64::{Engine as _, engine::general_purpose};
                self.export_text_buffer = general_purpose::STANDARD.encode(&bytes);
                trigger_binary_download("data_backup.bson", &bytes, "application/octet-stream");
            }
        } else {
            self.export_text_buffer = match format {
                ExportFormat::Json => export_to_json(&self.state).unwrap_or_default(),
                ExportFormat::Csv => export_to_csv(&self.state.collection).unwrap_or_default(),
                ExportFormat::Bson => unreachable!(),
            };
        }
        self.show_export_dialog = Some(format);
        self.export_copied_notification = None;
    }

    fn apply_theme(&mut self, ctx: &egui::Context) {
        let theme = self.state.config.theme;
        let dyslexia = self.state.config.dyslexia_friendly;
        let scale_percent = self.state.config.ui_scale_percent.clamp(80, 150);

        if self.current_theme == Some(theme)
            && self.current_dyslexia_mode == Some(dyslexia)
            && self.current_ui_scale_percent == Some(scale_percent)
        {
            return;
        }
        self.current_theme = Some(theme);
        self.current_dyslexia_mode = Some(dyslexia);
        self.current_ui_scale_percent = Some(scale_percent);

        ctx.all_styles_mut(|style| {
            let defaults = egui::Style::default();
            style.text_styles = defaults.text_styles;
            style.spacing = defaults.spacing;
            if dyslexia {
                for font_id in style.text_styles.values_mut() {
                    font_id.size = (font_id.size * 1.12).round();
                }
                style.spacing.item_spacing = egui::vec2(10.0, 8.0);
                style.spacing.button_padding = egui::vec2(14.0, 10.0);
                style.spacing.indent = 22.0;
            }
        });
        spodeian_ui::apply_theme(ctx, theme);
        ctx.set_zoom_factor(self.state.config.clamped_ui_scale_factor());
    }

    fn handle_keyboard_shortcuts(&mut self, ctx: &egui::Context) {
        let (
            escape,
            cmd_palette,
            f1,
            ctrl_z,
            ctrl_y,
            ctrl_1,
            ctrl_2,
            ctrl_e,
            ctrl_i,
            ctrl_shift_c,
            ctrl_shift_t,
            ctrl_shift_d,
        ) = ctx.input(|i| {
            let ctrl = i.modifiers.ctrl || i.modifiers.mac_cmd;
            (
                i.key_pressed(egui::Key::Escape),
                ctrl && i.key_pressed(egui::Key::K),
                i.key_pressed(egui::Key::F1),
                ctrl && !i.modifiers.shift && i.key_pressed(egui::Key::Z),
                (ctrl && i.key_pressed(egui::Key::Y))
                    || (ctrl && i.modifiers.shift && i.key_pressed(egui::Key::Z)),
                ctrl && i.key_pressed(egui::Key::Num1),
                ctrl && i.key_pressed(egui::Key::Num2),
                ctrl && !i.modifiers.shift && i.key_pressed(egui::Key::E),
                ctrl && !i.modifiers.shift && i.key_pressed(egui::Key::I),
                ctrl && i.modifiers.shift && i.key_pressed(egui::Key::C),
                ctrl && i.modifiers.shift && i.key_pressed(egui::Key::T),
                ctrl && i.modifiers.shift && i.key_pressed(egui::Key::D),
            )
        });

        if cmd_palette {
            if self.command_palette.is_open() {
                self.command_palette.close();
            } else {
                self.command_palette.open();
            }
            return;
        }

        if escape {
            if self.command_palette.is_open() {
                self.command_palette.close();
            } else if self.show_help_dialog {
                self.show_help_dialog = false;
            } else if self.show_reset_dialog {
                self.show_reset_dialog = false;
            } else if self.show_storage_modal {
                self.show_storage_modal = false;
            } else if self.show_telemetry_modal {
                self.show_telemetry_modal = false;
            } else if self.show_export_dialog.is_some() {
                self.show_export_dialog = None;
                self.export_text_buffer.clear();
            } else if self.show_import_dialog {
                self.show_import_dialog = false;
                self.import_text_buffer.clear();
                self.import_result_message = None;
            }
            return;
        }

        if f1 {
            let _ = self.execute_command("system.help");
            return;
        }

        if ctx.egui_wants_keyboard_input() || self.command_palette.is_open() {
            return;
        }

        if ctrl_z {
            let _ = self.execute_command("edit.undo");
        } else if ctrl_y {
            let _ = self.execute_command("edit.redo");
        } else if ctrl_shift_c {
            let _ = self.execute_command("edit.clear_completed");
        } else if ctrl_shift_t {
            let _ = self.execute_command("theme.cycle");
        } else if ctrl_shift_d {
            let _ = self.execute_command("a11y.dyslexia");
        } else if ctrl_1 {
            let _ = self.execute_command("nav.dashboard");
        } else if ctrl_2 {
            let _ = self.execute_command("nav.about");
        } else if ctrl_e {
            let _ = self.execute_command("data.export_json");
        } else if ctrl_i {
            let _ = self.execute_command("data.import");
        }
    }
}

impl eframe::App for TemplateApp {
    fn save(&mut self, storage: &mut dyn eframe::Storage) {
        eframe::set_value(storage, eframe::APP_KEY, &self.state);

        if let Ok(json_str) = serde_json::to_string(&self.state) {
            storage.set_string(DEDICATED_STORAGE_KEY, json_str);
        }
        storage.flush();

        self.persist_state();
    }

    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        self.apply_theme(ui.ctx());
        if self.state.config.theme.is_high_contrast() {
            ui.spacing_mut().interact_size = egui::vec2(44.0, 44.0);
            ui.spacing_mut().button_padding = egui::vec2(14.0, 10.0);
        } else {
            ui.spacing_mut().interact_size.y = ui.spacing_mut().interact_size.y.max(32.0);
            ui.spacing_mut().button_padding = egui::vec2(12.0, 8.0);
        }
        self.handle_keyboard_shortcuts(ui.ctx());

        let cur_time = ui.input(|i| i.time);
        if cur_time - self.last_diag_poll_time > 2.0 {
            self.last_diag_poll_time = cur_time;
            let queried = query_storage_diagnostics();
            self.storage_diag.is_persisted = queried.is_persisted;
            self.storage_diag.pwa_install_available = queried.pwa_install_available;
            self.storage_diag.is_pwa_installed = queried.is_pwa_installed;
            self.storage_diag.memory_items_count = self.state.collection.items.len();
            self.storage_diag.fallback_tier_active = self.state.collection.items.len()
                > MEMORY_WORKING_SET_LIMIT
                || self.storage_diag.idb_active;
        }

        let constraints = ScreenConstraints::compute(ui);
        components::navbar::render_navbar(self, ui, &constraints);

        egui::Panel::bottom("bottom_footer").show(ui, |ui| {
            ui.add_space(3.0);
            ui.horizontal(|ui| {
                ui.label(
                    egui::RichText::new(
                        "Built with Rust & egui 0.36 • Serverless WASM & Native Desktop",
                    )
                    .small()
                    .color(ui.visuals().weak_text_color()),
                );

                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    let tier_text = format!("Storage: {}", self.storage_diag.backend.label());
                    ui.label(
                        egui::RichText::new(tier_text)
                            .small()
                            .monospace()
                            .color(ui.visuals().weak_text_color()),
                    );

                    ui.label(
                        egui::RichText::new("•")
                            .small()
                            .color(ui.visuals().weak_text_color()),
                    );

                    if ui.link("About").clicked() {
                        self.state.config.active_page = shared::AppPage::About;
                        self.persist_state();
                    }

                    if ui.link("Dashboard").clicked() {
                        self.state.config.active_page = shared::AppPage::Dashboard;
                        self.persist_state();
                    }
                });
            });
            ui.add_space(3.0);
        });

        egui::CentralPanel::default().show(ui, |ui| match self.state.config.active_page {
            shared::AppPage::Dashboard => {
                components::item_list::render_summary_cards(self, ui, &constraints);
                ui.add_space(10.0);
                components::item_list::render_new_item_form(self, ui, &constraints);
                ui.add_space(10.0);
                components::item_list::render_item_list(self, ui, &constraints);
            }
            shared::AppPage::About => {
                components::about::render_about_page(self, ui, &constraints);
            }
        });

        if let Some(cmd_id) = self.command_palette.show(ui.ctx()) {
            let _ = self.execute_command(&cmd_id);
        }

        components::modals::render_dialogs(self, ui);
        components::modals::render_error_recovery_banner(self, ui.ctx());
        components::modals::render_warning_banners(self, ui.ctx());
        components::telemetry_viewer::render_telemetry_modal(self, ui.ctx());
    }
}

#[cfg(target_os = "android")]
#[unsafe(no_mangle)]
fn android_main(app: winit::platform::android::activity::AndroidApp) {
    use eframe::NativeOptions;
    let mut options = NativeOptions::default();
    options.android_app = Some(app);
    if let Err(e) = eframe::run_native(
        "egui Template",
        options,
        Box::new(|cc| Ok(Box::new(TemplateApp::new(cc)))),
    ) {
        tracing::error!("Failed to run egui application on Android: {e}");
    }
}

#[cfg(target_os = "ios")]
#[unsafe(no_mangle)]
pub extern "C" fn ios_main() {
    use eframe::NativeOptions;
    let options = NativeOptions::default();
    let _ = eframe::run_native(
        "egui Template",
        options,
        Box::new(|cc| Ok(Box::new(TemplateApp::new(cc)))),
    );
}
