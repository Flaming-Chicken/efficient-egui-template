//! Main egui application view controller, shell layout, and cross-platform runners.
#![warn(missing_docs)]
#![warn(clippy::unwrap_used)]
#![allow(clippy::collapsible_if)]
#![allow(clippy::if_same_then_else)]

pub mod components;
pub mod storage_manager;

pub use components::*;
pub use storage_manager::*;

use eframe::egui;
use shared::{
    AppState, ItemCollection, Priority, ThemeMode, decode_shareable_url_hash,
    encode_shareable_url_hash, export_to_compressed_bson, export_to_csv, export_to_json,
    export_to_printable_html, export_to_svg, import_from_compressed_bson, import_from_csv,
    import_from_json,
};
pub use spodeian_ui::{CommandEntry, CommandPaletteWidget, ScreenConstraints, UndoStack};
use std::collections::HashMap;
#[allow(unused_imports)]
use tracing::{error, info, warn};

/// Returns a human-readable description of the compiled `eframe` GPU graphics backend.
pub const fn active_renderer_label() -> &'static str {
    if cfg!(feature = "wgpu") {
        "wgpu (WebGPU / Vulkan / Metal / DX12)"
    } else {
        "glow (WebGL2 / OpenGL)"
    }
}

/// Supported data interchange and report export formats.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub enum ExportFormat {
    /// Pretty-printed JSON state document.
    #[default]
    Json,
    /// RFC 4180 CSV item spreadsheet.
    Csv,
    /// Zlib-compressed binary BSON backup.
    Bson,
    /// Standalone vector SVG priority distribution chart.
    Svg,
    /// Self-contained printable HTML5 report with inline SVG.
    Html,
}

impl ExportFormat {
    /// Every supported export format in display order.
    pub const ALL: [Self; 5] = [Self::Json, Self::Csv, Self::Bson, Self::Svg, Self::Html];

    /// Full human-facing format description.
    pub fn label(self) -> &'static str {
        match self {
            Self::Json => "JSON File",
            Self::Csv => "CSV File",
            Self::Bson => "Compressed BSON (.bson)",
            Self::Svg => "SVG Vector Chart (.svg)",
            Self::Html => "Printable HTML Report (.html)",
        }
    }

    /// Compact selector tab label.
    pub fn short_label(self) -> &'static str {
        match self {
            Self::Json => "JSON",
            Self::Csv => "CSV",
            Self::Bson => "BSON",
            Self::Svg => "SVG",
            Self::Html => "HTML",
        }
    }
}

/// Returns the default command catalog wired into [`TemplateApp::command_palette`].
pub fn default_template_commands() -> Vec<CommandEntry> {
    vec![
        CommandEntry::new("nav.dashboard", "Go to Dashboard", "Ctrl+1", "Navigation"),
        CommandEntry::new(
            "nav.about",
            "Go to About & Diagnostics",
            "Ctrl+2",
            "Navigation",
        ),
        CommandEntry::new(
            "nav.analytics",
            "Go to Analytics & 2D Canvas",
            "Ctrl+3",
            "Navigation",
        ),
        CommandEntry::new(
            "view.zen_mode",
            "Toggle Distraction-Free Zen Mode",
            "F11",
            "View",
        ),
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
        CommandEntry::new(
            "a11y.reduced_motion",
            "Toggle Reduced Motion",
            "",
            "Accessibility",
        ),
        CommandEntry::new(
            "a11y.large_targets",
            "Toggle Large Touch Targets (44pt)",
            "",
            "Accessibility",
        ),
        CommandEntry::new("data.export_json", "Export State as JSON", "Ctrl+E", "Data"),
        CommandEntry::new("data.export_csv", "Export Items as CSV", "", "Data"),
        CommandEntry::new(
            "data.export_bson",
            "Export Compressed BSON Backup",
            "",
            "Data",
        ),
        CommandEntry::new("data.export_svg", "Export Standalone SVG Chart", "", "Data"),
        CommandEntry::new(
            "data.export_html",
            "Export Printable HTML5 Report",
            "",
            "Data",
        ),
        CommandEntry::new(
            "data.copy_share_link",
            "Copy Shareable State URL Hash",
            "Ctrl+Shift+L",
            "Data",
        ),
        CommandEntry::new("data.import", "Open Import Dialog", "Ctrl+I", "Data"),
        CommandEntry::new(
            "system.storage",
            "Inspect Storage & Persistence",
            "",
            "System",
        ),
        CommandEntry::new("system.telemetry", "Open System Telemetry", "", "System"),
        CommandEntry::new("system.logs", "Toggle Live Log Console", "", "System"),
        CommandEntry::new(
            "system.help",
            "Open Help & Accessibility Settings",
            "F1",
            "Help",
        ),
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

/// Thread-safe staging slot for an asynchronously read dropped file `(bytes, filename)`.
pub type PendingDroppedFile = std::sync::Arc<std::sync::Mutex<Option<(Vec<u8>, String)>>>;

/// Primary `eframe::App` state container and view controller.
pub struct TemplateApp {
    /// Persistent root application state.
    pub state: AppState,
    /// Last applied theme mode for change detection.
    pub current_theme: Option<ThemeMode>,
    /// Last applied dyslexia reading mode for change detection.
    pub current_dyslexia_mode: Option<bool>,
    /// Last applied UI scale percentage for change detection.
    pub current_ui_scale_percent: Option<u16>,
    /// Last applied accessibility preferences for change detection.
    pub current_accessibility: Option<spodeian_tokens::AccessibilityPreferences>,
    /// Searchable command palette modal (`Ctrl+K` / `Cmd+K`).
    pub command_palette: CommandPaletteWidget,
    /// Bounded 64-step undo/redo collection history.
    pub undo_stack: UndoStack<ItemCollection>,
    /// Reactive idle-throttling repaint pacer.
    pub frame_pacer: spodeian_ui::ReactiveFramePacer,
    /// Live tracing log ring-buffer widget.
    pub log_console: spodeian_ui::LogConsoleWidget,
    /// True while previewing state loaded from a `#state=...` URL hash.
    pub is_viewing_shared_link: bool,
    /// Local state snapshot preserved while inspecting a shared link.
    pub saved_local_state_backup: Option<AppState>,
    /// Transient inline header status toast `(message, timestamp_secs)`.
    pub status_toast: Option<(String, f64)>,
    /// Last observed egui input timestamp in seconds.
    pub last_ui_time: f64,
    /// Parameters for the interactive 2D parametric viewport in [`shared::AppPage::Analytics`].
    pub preview_config: shared::ParametricPreviewConfig,
    /// Current 2D pan offset in the analytics viewport.
    pub canvas_pan: egui::Vec2,
    /// Current zoom factor (`0.25..=4.0`) in the analytics viewport.
    pub canvas_zoom: f32,
    /// Most recent panic message captured by [`TemplateApp::run_recoverable`].
    pub last_recovered_error: Option<String>,
    /// Controls visibility of the Reset Data confirmation modal.
    pub show_reset_dialog: bool,
    /// Controls visibility of the Help & Accessibility modal.
    pub show_help_dialog: bool,
    /// Controls visibility of the Data Import modal.
    pub show_import_dialog: bool,
    /// Text buffer inside the Import modal.
    pub import_text_buffer: String,
    /// Validation/result feedback displayed in the Import modal.
    pub import_result_message: Option<Result<String, String>>,
    /// Active Export modal format when open.
    pub show_export_dialog: Option<ExportFormat>,
    /// Serialized preview buffer inside the Export modal.
    pub export_text_buffer: String,
    /// Timestamp when export text was copied to clipboard.
    pub export_copied_notification: Option<f64>,
    /// Last selected export format.
    pub selected_export_format: ExportFormat,
    /// Draft title in the Dashboard add-item form.
    pub new_item_title: String,
    /// Draft description in the Dashboard add-item form.
    pub new_item_description: String,
    /// Draft priority in the Dashboard add-item form.
    pub new_item_priority: Priority,
    /// Optional priority filter in the Dashboard item list.
    pub filter_priority: Option<Priority>,
    /// Search query in the Dashboard item list.
    pub search_query: String,
    /// Queried persistence backend & PWA diagnostics.
    pub storage_diag: StorageDiagnostics,
    /// Controls visibility of the Storage & Data Management modal.
    pub show_storage_modal: bool,
    /// Controls visibility of the System Telemetry modal.
    pub show_telemetry_modal: bool,
    /// Controls visibility of the Live Telemetry Log Console window.
    pub show_log_console: bool,
    /// Telemetry KPI & channel data displayed in the Telemetry modal.
    pub telemetry_data: shared::TelemetryDashboardData,
    /// Dismissal flag for the ephemeral storage warning banner.
    pub dismissed_ephemeral_warning: bool,
    /// Dismissal flag for the quota exceeded warning banner.
    pub dismissed_quota_warning: bool,
    /// Dismissal flag for the combined warning banner.
    pub dismissed_combined_warning: bool,
    /// Last timestamp when storage diagnostics were refreshed.
    pub last_diag_poll_time: f64,
    /// Latest shareable hash copied by `"data.copy_share_link"` (for clipboard & tests).
    pub last_copied_share_hash: Option<String>,
    /// Asynchronous dropped-file payload staged on `wasm32` targets before UI ingestion.
    pub pending_dropped_file: PendingDroppedFile,
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
            current_accessibility: None,
            command_palette: CommandPaletteWidget::new(default_template_commands()),
            undo_stack: UndoStack::new(64),
            frame_pacer: spodeian_ui::ReactiveFramePacer::default(),
            log_console: spodeian_ui::LogConsoleWidget::new(
                spodeian_telemetry::global_ring_buffer(),
            ),
            is_viewing_shared_link: false,
            saved_local_state_backup: None,
            status_toast: None,
            last_ui_time: 0.0,
            preview_config: shared::ParametricPreviewConfig::default(),
            canvas_pan: egui::Vec2::ZERO,
            canvas_zoom: 1.0,
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
            show_log_console: false,
            telemetry_data,
            dismissed_ephemeral_warning: false,
            dismissed_quota_warning: false,
            dismissed_combined_warning: false,
            last_diag_poll_time: 0.0,
            last_copied_share_hash: None,
            pending_dropped_file: std::sync::Arc::new(std::sync::Mutex::new(None)),
        }
    }
}

impl TemplateApp {
    /// Initializes [`TemplateApp`] from `eframe::CreationContext`, restoring any persisted state.
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

    /// Shows a brief status badge (`✓ Saved`, `↺ Undone`, etc.) in the top bar for 2.5 seconds.
    pub fn set_status_toast(&mut self, msg: impl Into<String>) {
        self.status_toast = Some((msg.into(), self.last_ui_time));
    }

    /// Loads state encoded in a `#state=...` URL fragment while backing up local saved state so
    /// opening a shared link never overwrites local user data.
    pub fn load_shared_url_hash(&mut self, hash: &str) -> Result<(), String> {
        let shared_state = decode_shareable_url_hash(hash).map_err(|e| e.to_string())?;
        if !self.is_viewing_shared_link {
            self.saved_local_state_backup = Some(self.state.clone());
        }
        self.state = shared_state;
        self.is_viewing_shared_link = true;
        self.undo_stack.clear();
        self.set_status_toast("Loaded shared link");
        Ok(())
    }

    /// Discards the shared link preview and restores the local user state snapshot.
    pub fn restore_local_state_from_shared(&mut self) {
        if let Some(backup) = self.saved_local_state_backup.take() {
            self.state = backup;
        }
        self.is_viewing_shared_link = false;
        self.undo_stack.clear();
        self.set_status_toast("Restored local state");
    }

    /// Accepts the currently previewed shared link state as the user's new local state and saves it.
    pub fn accept_shared_state_as_local(&mut self) {
        self.is_viewing_shared_link = false;
        self.saved_local_state_backup = None;
        self.persist_state();
        self.set_status_toast("Saved shared state locally");
    }

    /// Imports dropped `.json`, `.csv`, or `.bson` file bytes into [`TemplateApp::state`].
    pub fn import_dropped_bytes(&mut self, filename: &str, bytes: &[u8]) -> Result<usize, String> {
        if bytes.is_empty() {
            return Err("Dropped file is empty".to_string());
        }
        let lower = filename.to_ascii_lowercase();
        if lower.ends_with(".bson") {
            let state = import_from_compressed_bson(bytes)?;
            self.record_undo_snapshot();
            let count = state.collection.total_count();
            self.state = state;
            self.persist_state();
            self.set_status_toast(format!("Imported {count} items (.bson)"));
            return Ok(count);
        }

        let text = std::str::from_utf8(bytes)
            .map_err(|e| format!("Dropped file is not valid UTF-8 or BSON: {e}"))?;
        let trimmed = text.trim();
        if lower.ends_with(".json") || trimmed.starts_with('{') {
            let state = import_from_json(trimmed).map_err(|e| e.to_string())?;
            self.record_undo_snapshot();
            let count = state.collection.total_count();
            self.state = state;
            self.persist_state();
            self.set_status_toast(format!("Imported {count} items (.json)"));
            return Ok(count);
        }

        let col = import_from_csv(trimmed).map_err(|e| e.to_string())?;
        self.record_undo_snapshot();
        let count = col.total_count();
        self.state.collection = col;
        self.persist_state();
        self.set_status_toast(format!("Imported {count} items (.csv)"));
        Ok(count)
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
            self.set_status_toast("↺ Undone");
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
            self.set_status_toast("↻ Redone");
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
            "nav.analytics" => {
                self.state.config.active_page = shared::AppPage::Analytics;
                self.persist_state();
                true
            }
            "nav.about" => {
                self.state.config.active_page = shared::AppPage::About;
                self.persist_state();
                true
            }
            "view.zen_mode" => {
                self.state.config.zen_mode = !self.state.config.zen_mode;
                self.persist_state();
                self.set_status_toast(if self.state.config.zen_mode {
                    "Zen Mode enabled"
                } else {
                    "Zen Mode disabled"
                });
                true
            }
            "edit.undo" => self.undo(),
            "edit.redo" => self.redo(),
            "edit.clear_completed" => {
                self.record_undo_snapshot();
                self.state.collection.clear_completed();
                self.persist_state();
                self.set_status_toast("Cleared completed");
                true
            }
            "theme.cycle" => {
                self.state.config.theme = self.state.config.theme.next();
                self.persist_state();
                self.set_status_toast(format!(
                    "Theme: {}",
                    self.state.config.theme.display_label()
                ));
                true
            }
            "a11y.dyslexia" => {
                self.state.config.dyslexia_friendly = !self.state.config.dyslexia_friendly;
                self.persist_state();
                true
            }
            "a11y.reduced_motion" => {
                self.state.config.accessibility.reduced_motion =
                    !self.state.config.accessibility.reduced_motion;
                self.persist_state();
                true
            }
            "a11y.large_targets" => {
                self.state.config.accessibility.large_touch_targets =
                    !self.state.config.accessibility.large_touch_targets;
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
            "data.export_svg" => {
                self.selected_export_format = ExportFormat::Svg;
                self.open_export_dialog(ExportFormat::Svg);
                true
            }
            "data.export_html" => {
                self.selected_export_format = ExportFormat::Html;
                self.open_export_dialog(ExportFormat::Html);
                true
            }
            "data.copy_share_link" => match encode_shareable_url_hash(&self.state) {
                Ok(hash) => {
                    self.last_copied_share_hash = Some(hash.clone());
                    spodeian_web_utils::copy_to_clipboard(&hash);
                    self.set_status_toast("Copied #state link");
                    true
                }
                Err(e) => {
                    self.last_recovered_error = Some(format!("Share link encode failed: {e}"));
                    false
                }
            },
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
            "system.logs" => {
                self.show_log_console = !self.show_log_console;
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

    /// Immediately persists current state to multi-tier storage unless previewing a shared link.
    pub fn persist_state(&mut self) {
        if self.is_viewing_shared_link {
            return;
        }
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
                    self.set_status_toast("✓ Saved");
                }
                Err(_) => {
                    self.storage_diag.quota_exceeded = true;
                }
            }
        }
    }

    /// Populates the export dialog buffer for `format` and opens the export window.
    pub fn open_export_dialog(&mut self, format: ExportFormat) {
        self.export_text_buffer = match format {
            ExportFormat::Json => export_to_json(&self.state).unwrap_or_default(),
            ExportFormat::Csv => export_to_csv(&self.state.collection).unwrap_or_default(),
            ExportFormat::Bson => match export_to_compressed_bson(&self.state) {
                Ok(bytes) => {
                    use base64::{Engine as _, engine::general_purpose};
                    general_purpose::STANDARD.encode(&bytes)
                }
                Err(e) => format!("BSON export error: {e}"),
            },
            ExportFormat::Svg => export_to_svg(&self.state),
            ExportFormat::Html => export_to_printable_html(&self.state),
        };
        self.show_export_dialog = Some(format);
        self.export_copied_notification = None;
    }

    fn apply_theme(&mut self, ctx: &egui::Context) {
        let theme = self.state.config.theme;
        let dyslexia = self.state.config.dyslexia_friendly;
        let scale_percent = self.state.config.ui_scale_percent.clamp(80, 150);
        let mut a11y = self.state.config.accessibility;
        a11y.high_contrast = a11y.high_contrast || theme.is_high_contrast();

        if self.current_theme == Some(theme)
            && self.current_dyslexia_mode == Some(dyslexia)
            && self.current_ui_scale_percent == Some(scale_percent)
            && self.current_accessibility == Some(a11y)
        {
            return;
        }
        self.current_theme = Some(theme);
        self.current_dyslexia_mode = Some(dyslexia);
        self.current_ui_scale_percent = Some(scale_percent);
        self.current_accessibility = Some(a11y);

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
        spodeian_ui::configure_accessible_focus(ctx, &a11y);
        ctx.set_zoom_factor(self.state.config.clamped_ui_scale_factor());
    }

    fn handle_keyboard_shortcuts(&mut self, ctx: &egui::Context) {
        let (
            escape,
            cmd_palette,
            f1,
            f11,
            ctrl_z,
            ctrl_y,
            ctrl_1,
            ctrl_2,
            ctrl_3,
            ctrl_e,
            ctrl_i,
            ctrl_shift_c,
            ctrl_shift_t,
            ctrl_shift_d,
            ctrl_shift_l,
        ) = ctx.input(|i| {
            let ctrl = i.modifiers.ctrl || i.modifiers.mac_cmd;
            (
                i.key_pressed(egui::Key::Escape),
                ctrl && i.key_pressed(egui::Key::K),
                i.key_pressed(egui::Key::F1),
                i.key_pressed(egui::Key::F11),
                ctrl && !i.modifiers.shift && i.key_pressed(egui::Key::Z),
                (ctrl && i.key_pressed(egui::Key::Y))
                    || (ctrl && i.modifiers.shift && i.key_pressed(egui::Key::Z)),
                ctrl && i.key_pressed(egui::Key::Num1),
                ctrl && i.key_pressed(egui::Key::Num2),
                ctrl && i.key_pressed(egui::Key::Num3),
                ctrl && !i.modifiers.shift && i.key_pressed(egui::Key::E),
                ctrl && !i.modifiers.shift && i.key_pressed(egui::Key::I),
                ctrl && i.modifiers.shift && i.key_pressed(egui::Key::C),
                ctrl && i.modifiers.shift && i.key_pressed(egui::Key::T),
                ctrl && i.modifiers.shift && i.key_pressed(egui::Key::D),
                ctrl && i.modifiers.shift && i.key_pressed(egui::Key::L),
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

        if f11 {
            let _ = self.execute_command("view.zen_mode");
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
            } else if self.show_log_console {
                self.show_log_console = false;
            } else if self.show_export_dialog.is_some() {
                self.show_export_dialog = None;
                self.export_text_buffer.clear();
            } else if self.show_import_dialog {
                self.show_import_dialog = false;
                self.import_text_buffer.clear();
                self.import_result_message = None;
            } else if self.state.config.zen_mode {
                self.state.config.zen_mode = false;
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
        } else if ctrl_shift_l {
            if let Ok(hash) = encode_shareable_url_hash(&self.state) {
                self.last_copied_share_hash = Some(hash.clone());
                ctx.copy_text(hash);
                self.set_status_toast("Copied #state link");
            }
        } else if ctrl_1 {
            let _ = self.execute_command("nav.dashboard");
        } else if ctrl_2 {
            let _ = self.execute_command("nav.about");
        } else if ctrl_3 {
            let _ = self.execute_command("nav.analytics");
        } else if ctrl_e {
            let _ = self.execute_command("data.export_json");
        } else if ctrl_i {
            let _ = self.execute_command("data.import");
        }
    }

    fn handle_dropped_files(&mut self, ctx: &egui::Context) {
        let pending_payload = self
            .pending_dropped_file
            .lock()
            .ok()
            .and_then(|mut guard| guard.take());
        if let Some((bytes, name)) = pending_payload
            && let Err(err) = self.import_dropped_bytes(&name, &bytes)
        {
            self.last_recovered_error = Some(format!("Import '{name}': {err}"));
        }

        let dropped = ctx.input(|i| i.raw.dropped_files.clone());
        for file in dropped {
            let path = file.path();
            let name = if !path.as_os_str().is_empty() {
                path.file_name()
                    .and_then(|n| n.to_str())
                    .unwrap_or("dropped_file")
                    .to_string()
            } else {
                "dropped_file".to_string()
            };

            #[cfg(not(target_arch = "wasm32"))]
            {
                let loaded_bytes = if let Ok(bytes) = file.bytes() {
                    Some(bytes)
                } else if !path.as_os_str().is_empty() {
                    std::fs::read(path).ok()
                } else {
                    None
                };
                if let Some(bytes) = loaded_bytes {
                    if let Err(err) = self.import_dropped_bytes(&name, &bytes) {
                        self.last_recovered_error = Some(format!("Import '{name}': {err}"));
                    }
                } else {
                    self.last_recovered_error =
                        Some(format!("Could not read dropped file '{name}'"));
                }
            }

            #[cfg(target_arch = "wasm32")]
            {
                let pending = self.pending_dropped_file.clone();
                let file_clone = file.clone();
                let ctx_clone = ctx.clone();
                wasm_bindgen_futures::spawn_local(async move {
                    if let Ok(bytes) = file_clone.bytes_async().await {
                        if let Ok(mut guard) = pending.lock() {
                            *guard = Some((bytes, name));
                        }
                        ctx_clone.request_repaint();
                    }
                });
            }
        }
    }
}

impl eframe::App for TemplateApp {
    fn save(&mut self, storage: &mut dyn eframe::Storage) {
        let state_to_save = if self.is_viewing_shared_link {
            self.saved_local_state_backup
                .as_ref()
                .unwrap_or(&self.state)
        } else {
            &self.state
        };

        eframe::set_value(storage, eframe::APP_KEY, state_to_save);

        if let Ok(json_str) = serde_json::to_string(state_to_save) {
            storage.set_string(DEDICATED_STORAGE_KEY, json_str);
        }
        storage.flush();

        self.persist_state();
    }

    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        self.apply_theme(ui.ctx());
        if self.state.config.theme.is_high_contrast()
            || self.state.config.accessibility.large_touch_targets
        {
            ui.spacing_mut().interact_size = egui::vec2(44.0, 44.0);
            ui.spacing_mut().button_padding = egui::vec2(14.0, 10.0);
        } else {
            ui.spacing_mut().interact_size.y = ui.spacing_mut().interact_size.y.max(32.0);
            ui.spacing_mut().button_padding = egui::vec2(12.0, 8.0);
        }

        let cur_time = ui.input(|i| i.time);
        self.last_ui_time = cur_time;
        if let Some((_, ts)) = self.status_toast
            && cur_time - ts > 2.5
        {
            self.status_toast = None;
        }

        self.handle_keyboard_shortcuts(ui.ctx());
        self.handle_dropped_files(ui.ctx());

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
        if !self.state.config.zen_mode {
            components::navbar::render_navbar(self, ui, &constraints);

            egui::Panel::bottom("bottom_footer").show(ui, |ui| {
                ui.add_space(3.0);
                ui.horizontal_wrapped(|ui| {
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
                            let _ = self.execute_command("nav.about");
                        }

                        if ui.link("Analytics").clicked() {
                            let _ = self.execute_command("nav.analytics");
                        }

                        if ui.link("Dashboard").clicked() {
                            let _ = self.execute_command("nav.dashboard");
                        }
                    });
                });
                ui.add_space(3.0);
            });
        }

        egui::CentralPanel::default().show(ui, |ui| match self.state.config.active_page {
            shared::AppPage::Dashboard => {
                components::item_list::render_summary_cards(self, ui, &constraints);
                ui.add_space(10.0);
                components::item_list::render_new_item_form(self, ui, &constraints);
                ui.add_space(10.0);
                components::item_list::render_item_list(self, ui, &constraints);
            }
            shared::AppPage::Analytics => {
                components::analytics::render_analytics_page(self, ui, &constraints);
            }
            shared::AppPage::About => {
                components::about::render_about_page(self, ui, &constraints);
            }
        });

        if self.state.config.zen_mode {
            egui::Area::new(egui::Id::new("zen_mode_exit_overlay"))
                .order(egui::Order::Foreground)
                .anchor(egui::Align2::RIGHT_TOP, egui::vec2(-12.0, 12.0))
                .show(ui.ctx(), |ui| {
                    if ui.button("Exit Zen Mode (F11)").clicked() {
                        let _ = self.execute_command("view.zen_mode");
                    }
                });
        }

        if let Some(cmd_id) = self.command_palette.show(ui.ctx()) {
            let _ = self.execute_command(&cmd_id);
        }

        components::modals::render_shared_link_banner(self, ui.ctx());
        components::modals::render_dialogs(self, ui);
        components::modals::render_error_recovery_banner(self, ui.ctx());
        components::modals::render_warning_banners(self, ui.ctx());
        components::telemetry_viewer::render_telemetry_modal(self, ui.ctx());

        self.frame_pacer.pace_frame(ui.ctx(), false);
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
