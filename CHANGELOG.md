# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.2.0] - 2026-11-14

### Added
- `AppState::schema_version` (`CURRENT_SCHEMA_VERSION = 1`) backed by `spodeian_persist::StateMigrator` for forward-compatible JSON snapshots and legacy unversioned / RON state upgrades.
- Searchable `spodeian_ui::CommandPaletteWidget` (`Ctrl+K` / `Cmd+K`), keyboard shortcut conflict detector (`detect_shortcut_conflicts`), and bounded 64-step `spodeian_ui::UndoStack` (`Ctrl+Z` / `Ctrl+Y`) across item edits, resets, and imports.
- Accessible `dyslexia_friendly` reading mode (`Aa` navbar toggle + `Ctrl+Shift+D`) and `ui_scale_percent` (`80%`–`150%` zoom factor) persisted in `AppConfig`.
- Recoverable panic/fault capture helper (`TemplateApp::run_recoverable`) and non-crashing recovery overlay banner (`render_error_recovery_banner`).
- Interactive `cargo-generate.toml` configuration for optional desktop runner, offline PWA, Cloudflare Pages, and mobile build scripts, plus `.github/dependabot.yml`.
- Property-based tests (`proptest`) covering versioned JSON/BSON round-trips (including `u64 >= 2^63` via `spodeian_export::u64_i64_compat`), RFC 4180 CSV round-trips, UI scale/theme normalization, `UndoStack` state invariants, `CommandPaletteWidget` search indexing, and shortcut conflict detection.

### Changed
- Deduplicated storage routing/diagnostics (`crates/app/src/storage_manager.rs`), telemetry dashboard DTOs and modal (`crates/shared/src/telemetry.rs`, `crates/app/src/components/telemetry_viewer.rs`), and CSV/JSON/BSON interchange (`crates/shared/src/export.rs`) onto `spodeian-core` (`spodeian-persist`, `spodeian-ui`, `spodeian-telemetry`, `spodeian-export`).
- Replaced the hardcoded `"Version 0.1.0"` label in the About view with dynamic `env!("CARGO_PKG_VERSION")`.

## [0.1.0] - 2026-08-18

### Added
- Initial 4-crate `egui 0.36` + `eframe 0.36` serverless WASM & native desktop template with AccessKit, multi-tier persistence, offline PWA service worker, JSON/CSV/BSON export, and Criterion serialization benchmarks.
