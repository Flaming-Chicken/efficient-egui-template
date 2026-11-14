//! Unified multi-tiered storage engine, persistence manager, PWA install bridge, and diagnostics.
//!
//! Core storage routing, diagnostics, versioned migration, and download triggers are delegated
//! to `spodeian_persist`, while retaining backwards-compatible RON fallback for legacy
//! desktop and `eframe::Storage` entries.

pub use spodeian_persist::{
    KeyValueStorage, MEMORY_WORKING_SET_LIMIT, StorageBackend, StorageDiagnostics, StorageError,
    query_storage_diagnostics, request_persistent_storage, trigger_binary_download,
    trigger_pwa_install, trigger_text_download,
};

use tracing::{info, warn};

/// Dedicated storage key used across `eframe::Storage`, `localStorage`, and native CAS routing.
pub const DEDICATED_STORAGE_KEY: &str = "serverless_template_app_state";

/// Read-only adapter exposing an `&dyn eframe::Storage` as [`spodeian_persist::KeyValueStorage`].
pub struct EframeStorageAdapter<'a>(pub &'a dyn eframe::Storage);

impl KeyValueStorage for EframeStorageAdapter<'_> {
    fn get_string(&self, key: &str) -> Option<String> {
        self.0.get_string(key)
    }

    fn set_string(&mut self, _key: &str, _value: String) {}
}

fn finalize_loaded_state(mut state: shared::AppState) -> shared::AppState {
    if state.schema_version == 0 {
        state.schema_version = shared::CURRENT_SCHEMA_VERSION;
    }
    state
}

/// Dual-format deserializer for [`shared::AppState`], delegating versioned/raw JSON parsing to
/// `spodeian_persist::StateMigrator` and falling back to legacy RON payloads.
pub fn deserialize_app_state(content: &str) -> Result<shared::AppState, StorageError> {
    spodeian_persist::StateMigrator::deserialize_with_fallback(content, |trimmed| {
        ron::from_str::<shared::AppState>(trimmed).map_err(|e| e.to_string())
    })
    .map(finalize_loaded_state)
}

/// Multi-tiered loader for [`shared::AppState`].
/// Checks `eframe::Storage` (dedicated and legacy keys, JSON or RON) and browser `localStorage`
/// on `wasm32` via `spodeian_persist::load_state_multi_tier_keys` with RON fallback.
pub fn load_state_multi_tier(storage: Option<&dyn eframe::Storage>) -> Option<shared::AppState> {
    let adapter = storage.map(EframeStorageAdapter);
    if let Some(state) = spodeian_persist::load_state_multi_tier_keys::<shared::AppState>(
        adapter.as_ref().map(|a| a as &dyn KeyValueStorage),
        &[DEDICATED_STORAGE_KEY, eframe::APP_KEY],
        None,
    ) {
        return Some(finalize_loaded_state(state));
    }

    #[cfg(target_arch = "wasm32")]
    if let Some(window) = web_sys::window()
        && let Ok(Some(local_storage)) = window.local_storage()
    {
        for &key in &[DEDICATED_STORAGE_KEY, eframe::APP_KEY] {
            if let Ok(Some(content)) = local_storage.get_item(key) {
                match deserialize_app_state(&content) {
                    Ok(state) => {
                        info!("Restored legacy RON state from localStorage [{key}]");
                        return Some(state);
                    }
                    Err(e) => {
                        warn!("Failed to parse state from localStorage [{key}]: {e}");
                    }
                }
            }
        }
    }

    if let Some(storage) = storage {
        for &key in &[DEDICATED_STORAGE_KEY, eframe::APP_KEY] {
            if let Some(raw) = storage.get_string(key) {
                match deserialize_app_state(&raw) {
                    Ok(state) => {
                        info!("Restored legacy state from eframe::Storage [{key}]");
                        return Some(state);
                    }
                    Err(e) => {
                        warn!("Failed to parse state from eframe::Storage [{key}]: {e}");
                    }
                }
            }
        }

        if let Some(state) = eframe::get_value::<shared::AppState>(storage, eframe::APP_KEY) {
            info!("Restored AppState from eframe::get_value (RON).");
            return Some(finalize_loaded_state(state));
        }
    }

    None
}

/// Saves serialized state via `spodeian_persist`'s preferential multi-tier router:
/// - `wasm32`: Cache API (> 512 KB), `localStorage` (< 16 KB), and `IndexedDB` fallback.
/// - native: `ContentAddressedStorage` under `<temp_dir>/egui_template_cas_store`.
pub fn save_state_multi_tier(key: &str, json_str: &str) -> Result<StorageBackend, StorageError> {
    spodeian_persist::save_state_multi_tier(key, json_str, "egui_template_cas_store", None)
}
