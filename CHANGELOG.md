# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.2.2] - 2026-11-18

### Added
- `spodeian-parallel` integration in `crates/shared` powering Rayon work-stealing sweeps on native desktops and seamless SIMD128 sequential execution on `wasm32` (`sample_points`, `batch_harmonic_sweep`).
- Bundled WebGPU WGSL compute kernel (`PARAMETRIC_HARMONIC_WGSL_SHADER`), CPU parity helper (`evaluate_harmonic_wgsl_cpu_reference`), `ComputeRuntimeSummary`, and `app::active_renderer_label()` (`wgpu` vs. `glow`) surfaced in Analytics & About views.
- Dedicated [`ARCHITECTURE.md`](ARCHITECTURE.md) and 3-template comparison matrix in [`README.md`](README.md).

## [0.2.1] - 2026-11-18

### Added
- `AppPage::Analytics` view with KPI `PerformanceMetricCard`s, a colorblind-safe priority distribution chart, and an interactive 2D pan/zoom parametric signal explorer (`ParametricPreviewConfig`).
- Self-contained SVG (`export_to_svg`), printable HTML (`export_to_printable_html`), and compressed `#state=<base64url>` shareable URL hash (`encode_shareable_url_hash` / `decode_shareable_url_hash`) interchange, plus a non-destructive shared-link preview banner (`is_viewing_shared_link`, restore/keep actions) and drag-and-drop file import (`.json`, `.csv`, `.bson`).
- `AccessibilityPreferences` (`ColorblindMode`, `reduced_motion`, `large_touch_targets`), embedded `ThemePickerWidget`, `Zen Mode` (`F11`), `ReactiveFramePacer`, and `LogConsoleWidget` inside the Telemetry modal.
- Runnable `crates/shared/examples/state_interchange_and_report.rs` example and expanded unit/`proptest` coverage for Analytics routing, SVG/HTML escaping, share-link round-trips, dropped-file imports, and Zen Mode.

### Changed
- Collapsed secondary navbar controls into an overflow `Tools` menu with inline status toasts, lowering horizontal clutter on narrow/mobile viewports, and lowered native desktop minimum window size to `320x240`.
- Hardened offline-first web builds by removing external CDN font/CSS `@import`s from `crates/web/index.scss`, adding safe-area insets and `100dvh`, enforcing strict `isHostOrSubdomain` checks in `crates/web/sw.js`, and stamping build IDs / precache lists in `deploy.sh`.
- Switched workspace `members` to `["crates/*"]` for clean `cargo-generate` scaffolding with `include_desktop = false`, enabled `#![warn(missing_docs)]` and `#![warn(clippy::unwrap_used)]` on `shared` and `app`, and expanded CI (`wasm32` clippy, `RUSTDOCFLAGS="-D warnings"`, examples/benches, >2 MB tracked-file check, and `template-scaffold-check`).

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
