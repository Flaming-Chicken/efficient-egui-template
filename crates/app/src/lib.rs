#![allow(clippy::collapsible_if)]
#![allow(clippy::if_same_then_else)]

pub mod components;
pub mod storage_manager;

pub use components::*;
pub use storage_manager::*;

use eframe::egui;
use shared::{
    AppState, Priority, ThemeMode, export_to_compressed_bson, export_to_csv, export_to_json,
};
pub use spodeian_ui::ScreenConstraints;
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

pub struct TemplateApp {
    pub state: AppState,
    pub current_theme: Option<ThemeMode>,
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
        Self {
            state: AppState::default(),
            current_theme: None,
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
            storage_diag: query_storage_diagnostics(),
            show_storage_modal: false,
            show_telemetry_modal: false,
            telemetry_data: shared::TelemetryDashboardData::default(),
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

    /// Immediately persist current state to multi-tier storage (active persistence)
    pub fn persist_state(&mut self) {
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
            if let Ok(bytes) = export_to_compressed_bson(&self.state.collection) {
                use base64::{Engine as _, engine::general_purpose};
                self.export_text_buffer = general_purpose::STANDARD.encode(&bytes);
                trigger_binary_download("data_backup.bson", &bytes, "application/octet-stream");
            }
        } else {
            self.export_text_buffer = match format {
                ExportFormat::Json => export_to_json(&self.state.collection).unwrap_or_default(),
                ExportFormat::Csv => export_to_csv(&self.state.collection),
                ExportFormat::Bson => unreachable!(),
            };
        }
        self.show_export_dialog = Some(format);
        self.export_copied_notification = None;
    }

    fn apply_theme(&mut self, ctx: &egui::Context) {
        if self.current_theme == Some(self.state.config.theme) {
            return;
        }
        self.current_theme = Some(self.state.config.theme);
        spodeian_ui::apply_theme(ctx, self.state.config.theme);
    }

    fn handle_keyboard_shortcuts(&mut self, ctx: &egui::Context) {
        if ctx.input(|i| i.key_pressed(egui::Key::Escape)) {
            if self.show_help_dialog {
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
        }
    }
}

impl eframe::App for TemplateApp {
    fn save(&mut self, storage: &mut dyn eframe::Storage) {
        // 1. Write standard RON state to eframe::APP_KEY
        eframe::set_value(storage, eframe::APP_KEY, &self.state);

        // 2. Write JSON to dedicated key in storage
        if let Ok(json_str) = serde_json::to_string(&self.state) {
            storage.set_string(DEDICATED_STORAGE_KEY, json_str);
        }
        storage.flush();

        // 3. Persist to multi-tier engine
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

        // Periodic diagnostics poll (every 2 seconds)
        let cur_time = ui.input(|i| i.time);
        if cur_time - self.last_diag_poll_time > 2.0 {
            self.last_diag_poll_time = cur_time;
            let queried = query_storage_diagnostics();
            self.storage_diag.is_persisted = queried.is_persisted;
            self.storage_diag.pwa_install_available = queried.pwa_install_available;
            self.storage_diag.is_pwa_installed = queried.is_pwa_installed;
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

        components::modals::render_dialogs(self, ui);
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
    eframe::run_native(
        "egui Template",
        options,
        Box::new(|cc| Ok(Box::new(TemplateApp::new(cc)))),
    )
    .unwrap();
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
