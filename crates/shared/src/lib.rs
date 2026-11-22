//! Core shared domain models, serialization, state management, and export logic.
#![warn(missing_docs)]
#![warn(clippy::unwrap_used)]

pub mod export;
pub mod models;
pub mod telemetry;

pub use export::{
    ExportError, SHARE_HASH_PREFIX, decode_shareable_url_hash, encode_shareable_url_hash,
    escape_html, export_to_compressed_bson, export_to_csv, export_to_json,
    export_to_printable_html, export_to_svg, import_from_compressed_bson, import_from_csv,
    import_from_json,
};
pub use models::{
    AccessibilityPreferences, AppConfig, AppPage, AppState, CURRENT_SCHEMA_VERSION, ColorblindMode,
    ComputeRuntimeSummary, Item, ItemCollection, PARAMETRIC_HARMONIC_WGSL_SHADER,
    ParametricPreviewConfig, Priority, ThemeMode, default_schema_version, default_ui_scale_percent,
};
pub use telemetry::{
    TelemetryChannelMetric, TelemetryDashboardData, default_template_telemetry,
    refresh_telemetry_dashboard,
};
