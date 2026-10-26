//! Shared business logic, domain models, and configuration.

use serde::{Deserialize, Serialize};

pub mod export;
pub mod models;
pub mod telemetry;

pub use export::*;
pub use models::*;
pub use telemetry::*;

pub use spodeian_tokens::ThemeMode;

#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq, Default)]
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

#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq)]
#[serde(default)]
pub struct AppConfig {
    pub theme: ThemeMode,
    pub active_page: AppPage,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            theme: ThemeMode::Dark,
            active_page: AppPage::Dashboard,
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub struct AppState {
    #[serde(default)]
    pub config: AppConfig,
    #[serde(default)]
    pub collection: ItemCollection,
}

impl AppState {
    /// Resets app domain data while preserving configuration preferences like theme.
    pub fn reset_data(&mut self) {
        self.collection.reset();
    }
}
