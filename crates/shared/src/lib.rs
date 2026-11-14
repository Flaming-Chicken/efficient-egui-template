//! Core shared domain models, serialization, state management, and export logic.

pub mod export;
pub mod models;
pub mod telemetry;

pub use export::{
    ExportError, export_to_compressed_bson, export_to_csv, export_to_json,
    import_from_compressed_bson, import_from_csv, import_from_json,
};
pub use models::{
    AppConfig, AppPage, AppState, CURRENT_SCHEMA_VERSION, Item, ItemCollection, Priority,
    ThemeMode, default_schema_version, default_ui_scale_percent,
};
pub use telemetry::{
    TelemetryChannelMetric, TelemetryDashboardData, default_template_telemetry,
    refresh_telemetry_dashboard,
};
