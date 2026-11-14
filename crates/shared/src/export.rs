//! Serialization and data interchange engines for JSON, CSV, and compressed BSON.

use crate::models::{AppConfig, AppState, CURRENT_SCHEMA_VERSION, Item, ItemCollection, Priority};
use spodeian_persist::{StateMigrator, VersionedSnapshot};
use thiserror::Error;

#[derive(Error, Debug)]
pub enum ExportError {
    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),
    #[error("CSV parse error: {0}")]
    Csv(String),
    #[error("Invalid priority format: {0}")]
    InvalidPriority(String),
    #[error("Empty data provided")]
    EmptyData,
    #[error("Export error: {0}")]
    Spodeian(#[from] spodeian_export::ExportError),
}

fn normalize_state_version(mut state: AppState) -> AppState {
    if state.schema_version == 0 {
        state.schema_version = CURRENT_SCHEMA_VERSION;
    }
    state
}

/// Serializes application state to formatted JSON.
pub fn export_to_json(state: &AppState) -> Result<String, ExportError> {
    spodeian_export::export_to_json(state).map_err(ExportError::Spodeian)
}

/// Deserializes application state from a JSON string.
/// Transparently handles [`VersionedSnapshot<AppState>`], full [`AppState`] payloads,
/// and standalone [`ItemCollection`] payloads, upgrading `schema_version == 0`.
pub fn import_from_json(json_str: &str) -> Result<AppState, ExportError> {
    let trimmed = json_str.trim();
    if trimmed.is_empty() {
        return Err(ExportError::EmptyData);
    }

    let value: serde_json::Value = serde_json::from_str(trimmed)?;
    if let Some(obj) = value.as_object() {
        if obj.contains_key("payload")
            && obj.contains_key("schema_version")
            && let Ok(snapshot) = serde_json::from_value::<VersionedSnapshot<AppState>>(value.clone())
        {
            return Ok(normalize_state_version(snapshot.payload));
        }
        if !obj.contains_key("collection")
            && !obj.contains_key("config")
            && obj.contains_key("items")
            && let Ok(col) = serde_json::from_value::<ItemCollection>(value.clone())
        {
            return Ok(AppState {
                schema_version: CURRENT_SCHEMA_VERSION,
                collection: col,
                config: AppConfig::default(),
            });
        }
    }

    StateMigrator::deserialize_versioned_or_raw::<AppState>(trimmed)
        .map(normalize_state_version)
        .map_err(|e| ExportError::Csv(e.to_string()))
}

/// Serializes an item collection to RFC 4180-compliant CSV via `spodeian_export`.
pub fn export_to_csv(collection: &ItemCollection) -> Result<String, ExportError> {
    let headers = [
        "id",
        "title",
        "description",
        "priority",
        "completed",
        "created_at",
    ];
    let rows: Vec<Vec<String>> = collection
        .items
        .iter()
        .map(|item| {
            vec![
                item.id.clone(),
                item.title.clone(),
                item.description.clone(),
                item.priority.label().to_string(),
                item.completed.to_string(),
                item.created_at.to_string(),
            ]
        })
        .collect();
    Ok(spodeian_export::export_to_csv(&headers, &rows))
}

/// Deserializes an item collection from a CSV string using `spodeian_export::parse_csv_line`.
pub fn import_from_csv(csv_str: &str) -> Result<ItemCollection, ExportError> {
    let trimmed = csv_str.trim();
    if trimmed.is_empty() {
        return Err(ExportError::EmptyData);
    }

    let mut items = Vec::new();
    for (idx, line) in trimmed.lines().enumerate() {
        let trimmed_line = line.trim();
        if trimmed_line.is_empty() {
            continue;
        }
        if idx == 0 && trimmed_line.to_lowercase().starts_with("id,") {
            continue;
        }

        let fields = spodeian_export::parse_csv_line(trimmed_line);
        if fields.len() < 5 {
            return Err(ExportError::Csv(format!(
                "Line {}: Expected at least 5 columns (id,title,description,priority,completed[,created_at]), found {}",
                idx + 1,
                fields.len()
            )));
        }

        let id = fields[0].clone();
        let title = fields[1].clone();
        let description = fields[2].clone();
        let priority = match fields[3].to_lowercase().as_str() {
            "low" => Priority::Low,
            "medium" => Priority::Medium,
            "high" => Priority::High,
            "critical" => Priority::Critical,
            other => return Err(ExportError::InvalidPriority(other.to_string())),
        };
        let completed = fields[4].eq_ignore_ascii_case("true") || fields[4] == "1";
        let created_at = fields
            .get(5)
            .and_then(|s| s.parse::<u64>().ok())
            .unwrap_or(0);

        items.push(Item {
            id,
            title,
            description,
            priority,
            completed,
            created_at,
        });
    }

    if items.is_empty() {
        return Err(ExportError::Csv(
            "No valid items parsed from CSV".to_string(),
        ));
    }

    Ok(ItemCollection { items })
}

/// Exports the application state into compressed BSON binary bytes (Zlib-compressed BSON).
pub fn export_to_compressed_bson(state: &AppState) -> Result<Vec<u8>, String> {
    spodeian_export::export_to_compressed_bson(state)
        .map_err(|e| format!("BSON serialization failed: {e}"))
}

/// Imports and restores an [`AppState`] from a compressed (or raw) BSON slice.
pub fn import_from_compressed_bson(bytes: &[u8]) -> Result<AppState, String> {
    if bytes.is_empty() {
        return Err("BSON input is empty".to_string());
    }
    let bson_bytes =
        miniz_oxide::inflate::decompress_to_vec_zlib(bytes).unwrap_or_else(|_| bytes.to_vec());

    if let Ok(doc) = bson::from_slice::<bson::Document>(&bson_bytes)
        && !doc.contains_key("collection")
        && !doc.contains_key("config")
        && doc.contains_key("items")
        && let Ok(col) = bson::from_slice::<ItemCollection>(&bson_bytes)
    {
        return Ok(AppState {
            schema_version: CURRENT_SCHEMA_VERSION,
            collection: col,
            config: AppConfig::default(),
        });
    }

    if let Ok(state) = spodeian_export::import_from_compressed_bson::<AppState>(bytes) {
        return Ok(normalize_state_version(state));
    }
    Err("BSON deserialization failed: incompatible payload".to_string())
}
