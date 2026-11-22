//! Core domain entities, priorities, and collection management logic.

use serde::{Deserialize, Serialize};
use spodeian_parallel::prelude::*;

pub use spodeian_tokens::{AccessibilityPreferences, ColorblindMode, ThemeMode};

/// Current persisted application state schema version.
pub const CURRENT_SCHEMA_VERSION: u32 = 1;

/// Returns [`CURRENT_SCHEMA_VERSION`] for serde default initialization.
pub fn default_schema_version() -> u32 {
    CURRENT_SCHEMA_VERSION
}

/// Returns the default UI scale percentage (`100` = `1.0x`).
pub fn default_ui_scale_percent() -> u16 {
    100
}

/// Task item priority level.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, Default)]
pub enum Priority {
    /// Low-urgency backlog work.
    Low,
    /// Standard priority task.
    #[default]
    Medium,
    /// Important near-term deliverable.
    High,
    /// Blocking or release-critical item.
    Critical,
}

impl Priority {
    /// Human-readable label for badges and exports.
    pub fn label(self) -> &'static str {
        match self {
            Self::Low => "Low",
            Self::Medium => "Medium",
            Self::High => "High",
            Self::Critical => "Critical",
        }
    }

    /// CSS-friendly priority badge class identifier.
    pub fn badge_class(self) -> &'static str {
        match self {
            Self::Low => "priority-low",
            Self::Medium => "priority-medium",
            Self::High => "priority-high",
            Self::Critical => "priority-critical",
        }
    }
}

/// Core domain item.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Item {
    /// Unique identifier for the item.
    pub id: String,
    /// Short display title.
    pub title: String,
    /// Optional extended description.
    pub description: String,
    /// Urgency classification.
    pub priority: Priority,
    /// Completion flag.
    pub completed: bool,
    /// Creation timestamp in Unix milliseconds (bit-preserved across BSON `i64` limits).
    #[serde(default, with = "spodeian_export::u64_i64_compat")]
    pub created_at: u64,
}

impl Item {
    /// Constructs a new uncompleted [`Item`] with `created_at = 0`.
    pub fn new(
        id: impl Into<String>,
        title: impl Into<String>,
        description: impl Into<String>,
        priority: Priority,
    ) -> Self {
        Self {
            id: id.into(),
            title: title.into(),
            description: description.into(),
            priority,
            completed: false,
            created_at: 0,
        }
    }
}

/// Collection wrapper for item operations and metrics.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct ItemCollection {
    /// Ordered items managed in the working set.
    pub items: Vec<Item>,
}

impl Default for ItemCollection {
    fn default() -> Self {
        Self::with_sample_data()
    }
}

impl ItemCollection {
    /// Creates an empty collection.
    pub fn new() -> Self {
        Self { items: Vec::new() }
    }

    /// Creates a collection pre-populated with four representative template items.
    pub fn with_sample_data() -> Self {
        Self {
            items: vec![
                Item {
                    id: "item-1".to_string(),
                    title: "Deploy Serverless WebApp".to_string(),
                    description: "Publish high-performance WASM application directly to Cloudflare Pages or GitHub Pages".to_string(),
                    priority: Priority::Critical,
                    completed: false,
                    created_at: 1,
                },
                Item {
                    id: "item-2".to_string(),
                    title: "Configure Native Desktop".to_string(),
                    description: "Verify cross-platform native desktop compilation and local asset loading".to_string(),
                    priority: Priority::High,
                    completed: false,
                    created_at: 2,
                },
                Item {
                    id: "item-3".to_string(),
                    title: "Verify Offline PWA Caching".to_string(),
                    description: "Check Service Worker caching strategy for offline web support".to_string(),
                    priority: Priority::Medium,
                    completed: true,
                    created_at: 3,
                },
                Item {
                    id: "item-4".to_string(),
                    title: "Test Data Interchange".to_string(),
                    description: "Import and export data via formatted JSON, RFC 4180 CSV, and compressed BSON".to_string(),
                    priority: Priority::Low,
                    completed: false,
                    created_at: 4,
                },
            ],
        }
    }

    /// Appends an item to the collection.
    pub fn add(&mut self, item: Item) {
        self.items.push(item);
    }

    /// Removes the first item matching `id`, returning `true` if found.
    pub fn remove(&mut self, id: &str) -> bool {
        if let Some(pos) = self.items.iter().position(|i| i.id == id) {
            self.items.remove(pos);
            true
        } else {
            false
        }
    }

    /// Toggles the `completed` status of the item matching `id`.
    pub fn toggle(&mut self, id: &str) {
        if let Some(item) = self.items.iter_mut().find(|i| i.id == id) {
            item.completed = !item.completed;
        }
    }

    /// Alias for [`ItemCollection::toggle`].
    pub fn toggle_completed(&mut self, id: &str) {
        self.toggle(id);
    }

    /// Drops all completed items from the collection.
    pub fn clear_completed(&mut self) {
        self.items.retain(|i| !i.completed);
    }

    /// Removes all items from the collection.
    pub fn reset(&mut self) {
        self.items.clear();
    }

    /// Total number of items in the collection.
    pub fn total_count(&self) -> usize {
        self.items.len()
    }

    /// Number of completed items.
    pub fn completed_count(&self) -> usize {
        self.items.iter().filter(|i| i.completed).count()
    }

    /// Number of pending (uncompleted) items.
    pub fn pending_count(&self) -> usize {
        self.items.iter().filter(|i| !i.completed).count()
    }

    /// Completion fraction in `[0.0, 1.0]`.
    pub fn completion_ratio(&self) -> f32 {
        if self.items.is_empty() {
            0.0
        } else {
            self.completed_count() as f32 / self.total_count() as f32
        }
    }

    /// Number of pending items with [`Priority::High`] or [`Priority::Critical`].
    pub fn high_priority_count(&self) -> usize {
        self.items
            .iter()
            .filter(|i| !i.completed && matches!(i.priority, Priority::High | Priority::Critical))
            .count()
    }

    /// Returns counts per priority level ordered `(Critical, High, Medium, Low)`.
    pub fn priority_counts(&self) -> [usize; 4] {
        let mut counts = [0usize; 4];
        for item in &self.items {
            match item.priority {
                Priority::Critical => counts[0] += 1,
                Priority::High => counts[1] += 1,
                Priority::Medium => counts[2] += 1,
                Priority::Low => counts[3] += 1,
            }
        }
        counts
    }
}

/// Active application view page.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum AppPage {
    /// Interactive item management dashboard.
    #[default]
    Dashboard,
    /// Priority distribution analytics and interactive 2D parametric canvas.
    Analytics,
    /// Architecture overview, storage diagnostics, and shortcuts.
    About,
}

impl AppPage {
    /// Every page in navigation order.
    pub const ALL: [AppPage; 3] = [AppPage::Dashboard, AppPage::Analytics, AppPage::About];

    /// Display label for tabs and navigation links.
    pub fn label(self) -> &'static str {
        match self {
            Self::Dashboard => "Dashboard",
            Self::Analytics => "Analytics",
            Self::About => "About",
        }
    }

    /// Concise description of the page's purpose.
    pub fn description(self) -> &'static str {
        match self {
            Self::Dashboard => {
                "Manage items with search, priority filters, undo/redo, and multi-tier persistence."
            }
            Self::Analytics => {
                "Inspect priority breakdowns, interactive 2D pan/zoom parametric plots, and vector/HTML reports."
            }
            Self::About => {
                "Inspect crate architecture, live diagnostics, accessibility controls, and keyboard shortcuts."
            }
        }
    }
}

/// Parameters controlling the deterministic 2D parametric signal/curve preview in [`AppPage::Analytics`].
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct ParametricPreviewConfig {
    /// Fundamental frequency multiplier in Hz (`0.5..=12.0`).
    pub frequency_hz: f32,
    /// Exponential decay damping factor (`0.0..=2.5`).
    pub damping: f32,
    /// Second-harmonic amplitude ratio (`0.0..=1.0`).
    pub harmonic_ratio: f32,
}

impl Default for ParametricPreviewConfig {
    fn default() -> Self {
        Self {
            frequency_hz: 2.5,
            damping: 0.45,
            harmonic_ratio: 0.35,
        }
    }
}

/// WGSL compute shader (`@compute @workgroup_size(64)`) evaluating the same damped harmonic
/// waveform as [`ParametricPreviewConfig::sample_points`] for WebGPU (`sml-kernel` / `wgpu`) dispatch.
pub const PARAMETRIC_HARMONIC_WGSL_SHADER: &str = r#"struct HarmonicParams {
    frequency_hz: f32,
    damping: f32,
    harmonic_ratio: f32,
    count: u32,
};

@group(0) @binding(0) var<storage, read_write> output_samples: array<f32>;
@group(0) @binding(1) var<uniform> params: HarmonicParams;

@compute @workgroup_size(64)
fn main(@builtin(global_invocation_id) gid: vec3<u32>) {
    let idx = gid.x;
    if (idx >= params.count) {
        return;
    }
    let denom = f32(max(params.count - 1u, 1u));
    let t = f32(idx) / denom;
    let angle = 6.28318530718 * params.frequency_hz * t;
    let envelope = exp(-params.damping * t);
    output_samples[idx] = envelope * (sin(angle) + params.harmonic_ratio * cos(2.0 * angle));
}"#;

/// Portable compute runtime capability summary shared across native Rayon desktop and browser WASM.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ComputeRuntimeSummary {
    /// Active concurrency backend description.
    pub runtime_label: &'static str,
    /// Available hardware thread parallelism (`>= 1`).
    pub parallel_threads: usize,
    /// Whether `wasm32` SIMD128 instructions are active at compile time.
    pub simd128_enabled: bool,
    /// Whether bundled WGSL compute shaders are compiled and ready for WebGPU dispatch.
    pub wgsl_shader_ready: bool,
}

impl ComputeRuntimeSummary {
    /// Captures the compute runtime summary for the active compilation target.
    pub fn current() -> Self {
        let simd128_enabled = cfg!(all(target_arch = "wasm32", target_feature = "simd128"));
        let parallel_threads = {
            #[cfg(not(target_arch = "wasm32"))]
            {
                std::thread::available_parallelism()
                    .map(|n| n.get())
                    .unwrap_or(1)
            }
            #[cfg(target_arch = "wasm32")]
            {
                1usize
            }
        };
        let runtime_label = if cfg!(not(target_arch = "wasm32")) {
            "Native Desktop (Rayon Work-Stealing Pool)"
        } else if simd128_enabled {
            "WASM32 Browser + SIMD128 Sequential"
        } else {
            "WASM32 Portable Sequential"
        };
        Self {
            runtime_label,
            parallel_threads,
            simd128_enabled,
            wgsl_shader_ready: PARAMETRIC_HARMONIC_WGSL_SHADER.contains("@compute"),
        }
    }
}

impl ParametricPreviewConfig {
    /// Generates `count` evenly spaced `(t, y)` samples across `t in [0.0, 1.0]` using
    /// `spodeian_parallel` (Rayon thread pool on native; zero-overhead sequential on `wasm32`).
    pub fn sample_points(&self, count: usize) -> Vec<(f32, f32)> {
        if count == 0 {
            return Vec::new();
        }
        let freq = if self.frequency_hz.is_finite() {
            self.frequency_hz.clamp(0.25, 20.0)
        } else {
            2.5
        };
        let damp = if self.damping.is_finite() {
            self.damping.clamp(0.0, 5.0)
        } else {
            0.45
        };
        let harmonic = if self.harmonic_ratio.is_finite() {
            self.harmonic_ratio.clamp(0.0, 1.0)
        } else {
            0.35
        };

        let denom = (count.saturating_sub(1)).max(1) as f32;
        (0..count)
            .into_par_iter()
            .map(|idx| {
                let t = idx as f32 / denom;
                let angle = std::f32::consts::TAU * freq * t;
                let envelope = (-damp * t).exp();
                let y = envelope * (angle.sin() + harmonic * (2.0 * angle).cos());
                (t, y)
            })
            .collect()
    }

    /// Evaluates a parallel harmonic sweep across `multipliers` using `spodeian_parallel`.
    pub fn batch_harmonic_sweep(&self, multipliers: &[f32], count: usize) -> Vec<Vec<(f32, f32)>> {
        (0..multipliers.len())
            .into_par_iter()
            .map(|idx| {
                let scale = multipliers[idx];
                let scaled = Self {
                    frequency_hz: if scale.is_finite() {
                        (self.frequency_hz * scale).clamp(0.25, 20.0)
                    } else {
                        self.frequency_hz
                    },
                    damping: self.damping,
                    harmonic_ratio: self.harmonic_ratio,
                };
                scaled.sample_points(count)
            })
            .collect()
    }

    /// Returns scalar `y` values matching [`PARAMETRIC_HARMONIC_WGSL_SHADER`]'s output buffer layout.
    pub fn evaluate_harmonic_wgsl_cpu_reference(&self, count: usize) -> Vec<f32> {
        self.sample_points(count)
            .into_iter()
            .map(|(_, y)| y)
            .collect()
    }
}

/// Global application configuration.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct AppConfig {
    /// Active visual theme (Dark, Light, High Contrast Dark/Light).
    pub theme: ThemeMode,
    /// Currently selected application page.
    pub active_page: AppPage,
    /// Whether edits automatically persist to durable tiers.
    pub auto_save: bool,
    /// Whether completed items remain visible in the dashboard list.
    pub show_completed: bool,
    /// Optional priority filter.
    pub filter_priority: Option<Priority>,
    /// Case-insensitive text filter across item titles and descriptions.
    pub search_query: String,
    /// Enlarged reading typography and wider spacing for dyslexia accessibility.
    #[serde(default)]
    pub dyslexia_friendly: bool,
    /// UI zoom percentage (`80..=150`).
    #[serde(default = "default_ui_scale_percent")]
    pub ui_scale_percent: u16,
    /// Colorblind, reduced-motion, and large touch target accessibility settings.
    #[serde(default)]
    pub accessibility: AccessibilityPreferences,
    /// Distraction-free mode hiding top navigation and bottom footer chrome.
    #[serde(default)]
    pub zen_mode: bool,
}

impl AppConfig {
    /// Returns the clamped zoom/UI scale factor in `0.8..=1.5`.
    pub fn clamped_ui_scale_factor(&self) -> f32 {
        (self.ui_scale_percent.clamp(80, 150) as f32) / 100.0
    }
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            theme: ThemeMode::Dark,
            active_page: AppPage::Dashboard,
            auto_save: true,
            show_completed: true,
            filter_priority: None,
            search_query: String::new(),
            dyslexia_friendly: false,
            ui_scale_percent: default_ui_scale_percent(),
            accessibility: AccessibilityPreferences::default(),
            zen_mode: false,
        }
    }
}

/// Root state container.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct AppState {
    /// Schema revision for backwards-compatible persistence migration.
    #[serde(default = "default_schema_version")]
    pub schema_version: u32,
    /// Managed domain item collection.
    pub collection: ItemCollection,
    /// User preferences and navigation state.
    pub config: AppConfig,
}

impl Default for AppState {
    fn default() -> Self {
        Self {
            schema_version: CURRENT_SCHEMA_VERSION,
            collection: ItemCollection::default(),
            config: AppConfig::default(),
        }
    }
}

impl AppState {
    /// Creates a new [`AppState`] with sample items and default configuration.
    pub fn new() -> Self {
        Self::default()
    }

    /// Resets app domain data while preserving configuration preferences like theme.
    pub fn reset_data(&mut self) {
        self.collection.reset();
    }
}
