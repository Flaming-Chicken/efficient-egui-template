//! Serialization and data interchange engines for JSON, CSV, compressed BSON, SVG, HTML, and URL hashes.

use crate::models::{
    AppConfig, AppState, CURRENT_SCHEMA_VERSION, ColorblindMode, Item, ItemCollection, Priority,
};
use base64::Engine as _;
use spodeian_persist::{StateMigrator, VersionedSnapshot};
use thiserror::Error;

/// Prefix used for compact shareable state fragments in URLs.
pub const SHARE_HASH_PREFIX: &str = "#state=";

/// Errors produced when exporting or importing [`AppState`] or [`ItemCollection`] payloads.
#[derive(Error, Debug)]
pub enum ExportError {
    /// JSON serialization or parsing failure.
    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),
    /// CSV format or column validation error.
    #[error("CSV parse error: {0}")]
    Csv(String),
    /// Unrecognized priority cell value in CSV input.
    #[error("Invalid priority format: {0}")]
    InvalidPriority(String),
    /// Compressed BSON or share-link decoding failure.
    #[error("Share link / BSON error: {0}")]
    Bson(String),
    /// Empty payload passed to an importer.
    #[error("Empty data provided")]
    EmptyData,
    /// Underlying `spodeian-export` error.
    #[error("Export error: {0}")]
    Spodeian(#[from] spodeian_export::ExportError),
}

fn normalize_state_version(mut state: AppState) -> AppState {
    if state.schema_version == 0 {
        state.schema_version = CURRENT_SCHEMA_VERSION;
    }
    state
}

/// Escapes special HTML characters (`&`, `<`, `>`, `"`, `'`) to prevent markup injection.
pub fn escape_html(input: &str) -> String {
    let mut out = String::with_capacity(input.len());
    for ch in input.chars() {
        match ch {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            '\'' => out.push_str("&#39;"),
            _ => out.push(ch),
        }
    }
    out
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
            && let Ok(snapshot) =
                serde_json::from_value::<VersionedSnapshot<AppState>>(value.clone())
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

/// Encodes `state` into a compact `#state=<url-safe-base64>` URL fragment suitable for link sharing.
pub fn encode_shareable_url_hash(state: &AppState) -> Result<String, ExportError> {
    let bytes = export_to_compressed_bson(state).map_err(ExportError::Bson)?;
    let encoded = base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(bytes);
    Ok(format!("{SHARE_HASH_PREFIX}{encoded}"))
}

/// Decodes a `#state=...` URL fragment (or full URL containing `#state=`) into [`AppState`].
pub fn decode_shareable_url_hash(hash_or_url: &str) -> Result<AppState, ExportError> {
    let trimmed = hash_or_url.trim();
    if trimmed.is_empty() {
        return Err(ExportError::EmptyData);
    }

    let fragment = trimmed
        .find('#')
        .map(|idx| &trimmed[idx..])
        .unwrap_or(trimmed);
    let payload = fragment
        .strip_prefix(SHARE_HASH_PREFIX)
        .or_else(|| fragment.strip_prefix("state="))
        .or_else(|| fragment.strip_prefix('#'))
        .unwrap_or(fragment)
        .trim();

    if payload.is_empty() {
        return Err(ExportError::EmptyData);
    }

    let bytes = base64::engine::general_purpose::URL_SAFE_NO_PAD
        .decode(payload)
        .or_else(|_| base64::engine::general_purpose::URL_SAFE.decode(payload))
        .or_else(|_| base64::engine::general_purpose::STANDARD.decode(payload))
        .map_err(|e| ExportError::Bson(format!("Invalid share link encoding: {e}")))?;

    import_from_compressed_bson(&bytes).map_err(ExportError::Bson)
}

fn priority_palette_hex(mode: ColorblindMode) -> [&'static str; 4] {
    match mode {
        ColorblindMode::None => ["#ef4444", "#f97316", "#38bdf8", "#94a3b8"],
        ColorblindMode::Deuteranopia => ["#d95f02", "#e6ab02", "#3b82f6", "#94a3b8"],
        ColorblindMode::Protanopia => ["#ea580c", "#ca8a04", "#2563eb", "#94a3b8"],
        ColorblindMode::Tritanopia => ["#f43f5e", "#fb7185", "#0ea5e9", "#94a3b8"],
        ColorblindMode::Monochrome => ["#f8fafc", "#cbd5e1", "#94a3b8", "#64748b"],
    }
}

/// Generates an accessible, standalone SVG priority breakdown and completion chart for `state`.
pub fn export_to_svg(state: &AppState) -> String {
    let counts = state.collection.priority_counts();
    let max_count = counts.iter().copied().max().unwrap_or(1).max(1);
    let total = state.collection.total_count();
    let completed = state.collection.completed_count();
    let pct = (state.collection.completion_ratio() * 100.0).round() as usize;
    let colors = priority_palette_hex(state.config.accessibility.colorblind_mode);
    let labels = ["Critical", "High", "Medium", "Low"];

    let (bg, panel, fg, muted, border) = if state.config.theme.is_dark() {
        ("#0f172a", "#1e293b", "#f8fafc", "#94a3b8", "#334155")
    } else {
        ("#f8fafc", "#ffffff", "#0f172a", "#475569", "#cbd5e1")
    };

    let mut bars = String::new();
    let chart_left = 48.0_f64;
    let chart_top = 64.0_f64;
    let bar_max_h = 132.0_f64;
    let col_w = 148.0_f64;
    let bar_w = 72.0_f64;

    for (idx, ((label, &count), color)) in labels
        .iter()
        .zip(counts.iter())
        .zip(colors.iter())
        .enumerate()
    {
        let x = chart_left + (idx as f64) * col_w;
        let h = if count == 0 {
            4.0
        } else {
            ((count as f64 / max_count as f64) * bar_max_h).max(8.0)
        };
        let y = chart_top + (bar_max_h - h);
        bars.push_str(&format!(
            "<g>\
             <rect x=\"{x:.1}\" y=\"{y:.1}\" width=\"{bar_w:.1}\" height=\"{h:.1}\" rx=\"6\" fill=\"{color}\" />\
             <text x=\"{tx:.1}\" y=\"{vy:.1}\" text-anchor=\"middle\" fill=\"{fg}\" font-family=\"system-ui, sans-serif\" font-size=\"13\" font-weight=\"700\">{count}</text>\
             <text x=\"{tx:.1}\" y=\"{ly:.1}\" text-anchor=\"middle\" fill=\"{muted}\" font-family=\"system-ui, sans-serif\" font-size=\"12\">{label}</text>\
             </g>",
            tx = x + bar_w / 2.0,
            vy = y - 8.0,
            ly = chart_top + bar_max_h + 20.0,
        ));
    }

    let progress_w = if total == 0 {
        0.0
    } else {
        (584.0 * (completed as f64 / total as f64)).clamp(0.0, 584.0)
    };

    format!(
        "<svg xmlns=\"http://www.w3.org/2000/svg\" viewBox=\"0 0 680 280\" role=\"img\" aria-labelledby=\"chart-title chart-desc\">\
         <title id=\"chart-title\">Task Priority Distribution</title>\
         <desc id=\"chart-desc\">{} total items, {} completed ({}%). Critical: {}, High: {}, Medium: {}, Low: {}.</desc>\
         <rect width=\"680\" height=\"280\" rx=\"12\" fill=\"{bg}\" stroke=\"{border}\" />\
         <rect x=\"20\" y=\"16\" width=\"640\" height=\"248\" rx=\"10\" fill=\"{panel}\" stroke=\"{border}\" />\
         <text x=\"48\" y=\"42\" fill=\"{fg}\" font-family=\"system-ui, sans-serif\" font-size=\"16\" font-weight=\"700\">Priority Breakdown ({total} items)</text>\
         <text x=\"632\" y=\"42\" text-anchor=\"end\" fill=\"{muted}\" font-family=\"system-ui, sans-serif\" font-size=\"13\">{completed}/{total} completed ({pct}%)</text>\
         {bars}\
         <rect x=\"48\" y=\"234\" width=\"584\" height=\"10\" rx=\"5\" fill=\"{border}\" />\
         <rect x=\"48\" y=\"234\" width=\"{progress_w:.1}\" height=\"10\" rx=\"5\" fill=\"{accent}\" />\
         </svg>",
        total,
        completed,
        pct,
        counts[0],
        counts[1],
        counts[2],
        counts[3],
        accent = colors[2]
    )
}

/// Generates a zero-dependency, self-contained HTML5 printable report embedding [`export_to_svg`].
pub fn export_to_printable_html(state: &AppState) -> String {
    let total = state.collection.total_count();
    let completed = state.collection.completed_count();
    let pending = state.collection.pending_count();
    let high_crit = state.collection.high_priority_count();
    let svg = export_to_svg(state);

    let (bg, card_bg, fg, muted, border) = if state.config.theme.is_dark() {
        ("#0d1117", "#161b22", "#f0f6fc", "#8b949e", "#30363d")
    } else {
        ("#f8fafc", "#ffffff", "#0f172a", "#475569", "#cbd5e1")
    };

    let mut rows = String::new();
    for item in &state.collection.items {
        let status = if item.completed {
            "Completed"
        } else {
            "Pending"
        };
        rows.push_str(&format!(
            "<tr>\
             <td><code>{}</code></td>\
             <td>{}</td>\
             <td>{}</td>\
             <td>{}</td>\
             <td>{}</td>\
             </tr>",
            escape_html(&item.id),
            escape_html(&item.title),
            escape_html(&item.description),
            escape_html(item.priority.label()),
            status
        ));
    }

    format!(
        "<!DOCTYPE html>\n\
<html lang=\"en\">\n\
<head>\n\
  <meta charset=\"utf-8\" />\n\
  <meta name=\"viewport\" content=\"width=device-width, initial-scale=1.0\" />\n\
  <meta http-equiv=\"Content-Security-Policy\" content=\"default-src 'none'; style-src 'unsafe-inline'; img-src data:;\" />\n\
  <title>Workspace Report</title>\n\
  <style>\n\
    :root {{ color-scheme: {scheme}; }}\n\
    body {{ margin: 0; padding: 24px; font-family: system-ui, -apple-system, sans-serif; background: {bg}; color: {fg}; line-height: 1.5; }}\n\
    .container {{ max-width: 920px; margin: 0 auto; }}\n\
    .kpis {{ display: grid; grid-template-columns: repeat(auto-fit, minmax(160px, 1fr)); gap: 12px; margin: 16px 0 24px; }}\n\
    .card {{ background: {card_bg}; border: 1px solid {border}; border-radius: 10px; padding: 14px 16px; }}\n\
    .label {{ font-size: 0.75rem; text-transform: uppercase; color: {muted}; letter-spacing: 0.05em; }}\n\
    .value {{ font-size: 1.5rem; font-weight: 700; margin-top: 4px; }}\n\
    .chart {{ margin: 16px 0 24px; }}\n\
    .chart svg {{ width: 100%; height: auto; display: block; }}\n\
    table {{ width: 100%; border-collapse: collapse; background: {card_bg}; border: 1px solid {border}; border-radius: 10px; overflow: hidden; }}\n\
    th, td {{ text-align: left; padding: 10px 12px; border-bottom: 1px solid {border}; font-size: 0.9rem; vertical-align: top; }}\n\
    th {{ background: rgba(127, 127, 127, 0.08); font-weight: 600; }}\n\
    @media print {{ body {{ background: #ffffff; color: #000000; padding: 0; }} .card, table {{ border-color: #cbd5e1; }} }}\n\
  </style>\n\
</head>\n\
<body>\n\
  <main class=\"container\">\n\
    <h1>Application State Report</h1>\n\
    <p style=\"color: {muted};\">Schema v{schema} &bull; Theme: {theme} &bull; Generated offline</p>\n\
    <section class=\"kpis\">\n\
      <div class=\"card\"><div class=\"label\">Total Items</div><div class=\"value\">{total}</div></div>\n\
      <div class=\"card\"><div class=\"label\">Completed</div><div class=\"value\">{completed}</div></div>\n\
      <div class=\"card\"><div class=\"label\">Pending</div><div class=\"value\">{pending}</div></div>\n\
      <div class=\"card\"><div class=\"label\">High &amp; Critical</div><div class=\"value\">{high_crit}</div></div>\n\
    </section>\n\
    <section class=\"chart\">{svg}</section>\n\
    <section>\n\
      <table>\n\
        <thead><tr><th>ID</th><th>Title</th><th>Description</th><th>Priority</th><th>Status</th></tr></thead>\n\
        <tbody>{rows}</tbody>\n\
      </table>\n\
    </section>\n\
  </main>\n\
</body>\n\
</html>\n",
        scheme = if state.config.theme.is_dark() {
            "dark"
        } else {
            "light"
        },
        schema = state.schema_version,
        theme = escape_html(state.config.theme.label()),
    )
}
