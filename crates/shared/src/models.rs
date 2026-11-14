//! Core domain entities, priorities, and collection management logic.

use serde::{Deserialize, Serialize};

pub use spodeian_tokens::ThemeMode;

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
    Low,
    #[default]
    Medium,
    High,
    Critical,
}

impl Priority {
    pub fn label(self) -> &'static str {
        match self {
            Self::Low => "Low",
            Self::Medium => "Medium",
            Self::High => "High",
            Self::Critical => "Critical",
        }
    }

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
    pub id: String,
    pub title: String,
    pub description: String,
    pub priority: Priority,
    pub completed: bool,
    #[serde(default, with = "spodeian_export::u64_i64_compat")]
    pub created_at: u64,
}

impl Item {
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
    pub items: Vec<Item>,
}

impl Default for ItemCollection {
    fn default() -> Self {
        Self::with_sample_data()
    }
}

impl ItemCollection {
    pub fn new() -> Self {
        Self { items: Vec::new() }
    }

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

    pub fn add(&mut self, item: Item) {
        self.items.push(item);
    }

    pub fn remove(&mut self, id: &str) -> bool {
        if let Some(pos) = self.items.iter().position(|i| i.id == id) {
            self.items.remove(pos);
            true
        } else {
            false
        }
    }

    pub fn toggle(&mut self, id: &str) {
        if let Some(item) = self.items.iter_mut().find(|i| i.id == id) {
            item.completed = !item.completed;
        }
    }

    pub fn toggle_completed(&mut self, id: &str) {
        self.toggle(id);
    }

    pub fn clear_completed(&mut self) {
        self.items.retain(|i| !i.completed);
    }

    pub fn reset(&mut self) {
        self.items.clear();
    }

    pub fn total_count(&self) -> usize {
        self.items.len()
    }

    pub fn completed_count(&self) -> usize {
        self.items.iter().filter(|i| i.completed).count()
    }

    pub fn pending_count(&self) -> usize {
        self.items.iter().filter(|i| !i.completed).count()
    }

    pub fn completion_ratio(&self) -> f32 {
        if self.items.is_empty() {
            0.0
        } else {
            self.completed_count() as f32 / self.total_count() as f32
        }
    }

    pub fn high_priority_count(&self) -> usize {
        self.items
            .iter()
            .filter(|i| !i.completed && matches!(i.priority, Priority::High | Priority::Critical))
            .count()
    }
}

/// Active application view page.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum AppPage {
    #[default]
    Dashboard,
    About,
}

impl AppPage {
    pub fn label(self) -> &'static str {
        match self {
            Self::Dashboard => "Dashboard",
            Self::About => "About",
        }
    }
}

/// Global application configuration.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct AppConfig {
    pub theme: ThemeMode,
    pub active_page: AppPage,
    pub auto_save: bool,
    pub show_completed: bool,
    pub filter_priority: Option<Priority>,
    pub search_query: String,
    #[serde(default)]
    pub dyslexia_friendly: bool,
    #[serde(default = "default_ui_scale_percent")]
    pub ui_scale_percent: u16,
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
        }
    }
}

/// Root state container.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct AppState {
    #[serde(default = "default_schema_version")]
    pub schema_version: u32,
    pub collection: ItemCollection,
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
    pub fn new() -> Self {
        Self::default()
    }

    /// Resets app domain data while preserving configuration preferences like theme.
    pub fn reset_data(&mut self) {
        self.collection.reset();
    }
}
