use proptest::prelude::*;
use shared::{
    AppConfig, AppPage, AppState, CURRENT_SCHEMA_VERSION, ComputeRuntimeSummary, Item,
    ItemCollection, PARAMETRIC_HARMONIC_WGSL_SHADER, ParametricPreviewConfig, Priority,
    SHARE_HASH_PREFIX, ThemeMode, decode_shareable_url_hash, encode_shareable_url_hash,
    escape_html, export_to_compressed_bson, export_to_csv, export_to_json,
    export_to_printable_html, export_to_svg, import_from_compressed_bson, import_from_csv,
    import_from_json,
};
use spodeian_persist::StateMigrator;

#[test]
fn test_priority_levels_and_labels() {
    assert_eq!(Priority::Low.label(), "Low");
    assert_eq!(Priority::Medium.label(), "Medium");
    assert_eq!(Priority::High.label(), "High");
    assert_eq!(Priority::Critical.label(), "Critical");

    assert_eq!(Priority::Low.badge_class(), "priority-low");
    assert_eq!(Priority::Medium.badge_class(), "priority-medium");
    assert_eq!(Priority::High.badge_class(), "priority-high");
    assert_eq!(Priority::Critical.badge_class(), "priority-critical");
}

#[test]
fn test_theme_mode_toggle() {
    let theme = ThemeMode::Dark;
    assert!(theme.is_dark());
    let toggled = theme.toggle();
    assert_eq!(toggled, ThemeMode::Light);
    assert!(!toggled.is_dark());
}

#[test]
fn test_item_collection_operations() {
    let mut collection = ItemCollection::new();
    assert_eq!(collection.total_count(), 0);
    assert_eq!(collection.completion_ratio(), 0.0);

    let item = Item::new("test-1", "Test Title", "Test Description", Priority::High);
    collection.add(item);
    assert_eq!(collection.total_count(), 1);
    assert_eq!(collection.pending_count(), 1);
    assert_eq!(collection.completed_count(), 0);
    assert_eq!(collection.high_priority_count(), 1);
    assert_eq!(collection.completion_ratio(), 0.0);

    collection.toggle("test-1");
    assert_eq!(collection.completed_count(), 1);
    assert_eq!(collection.pending_count(), 0);
    assert_eq!(collection.high_priority_count(), 0);
    assert_eq!(collection.completion_ratio(), 1.0);

    let item2 = Item::new(
        "test-2",
        "Critical Bug",
        "Must fix immediately",
        Priority::Critical,
    );
    collection.add(item2);
    assert_eq!(collection.total_count(), 2);
    assert_eq!(collection.high_priority_count(), 1);

    collection.clear_completed();
    assert_eq!(collection.total_count(), 1);
    assert_eq!(collection.items[0].id, "test-2");

    assert!(collection.remove("test-2"));
    assert_eq!(collection.total_count(), 0);
}

#[test]
fn test_json_roundtrip_full_state() {
    let state = AppState::new();
    let json_str = export_to_json(&state).expect("Export JSON failed");
    let imported: AppState = import_from_json(&json_str).expect("Import JSON failed");
    assert_eq!(state, imported);
}

#[test]
fn test_json_roundtrip_collection_fallback() {
    let collection = ItemCollection::with_sample_data();
    let json_str = serde_json::to_string_pretty(&collection).expect("Serialize collection failed");
    let imported: AppState = import_from_json(&json_str).expect("Import JSON fallback failed");
    assert_eq!(imported.collection.total_count(), collection.total_count());
}

#[test]
fn test_bson_roundtrip() {
    let state = AppState::new();
    let bytes = export_to_compressed_bson(&state).expect("Export BSON failed");
    assert!(!bytes.is_empty());
    let imported: AppState = import_from_compressed_bson(&bytes).expect("Import BSON failed");
    assert_eq!(
        state.collection.items.len(),
        imported.collection.items.len()
    );
}

#[test]
fn test_backward_compatibility() {
    let legacy_json = r#"{"collection":{"items":[]},"config":{}}"#;
    let imported: Result<AppState, _> = serde_json::from_str(legacy_json);
    assert!(imported.is_ok());
    let state = imported.unwrap();
    assert_eq!(state.config.active_page, AppPage::Dashboard);
}

#[test]
fn test_csv_roundtrip() {
    let collection = ItemCollection::with_sample_data();
    let csv_str = export_to_csv(&collection).expect("Export CSV failed");
    let imported = import_from_csv(&csv_str).expect("Import CSV failed");
    assert_eq!(collection.items.len(), imported.items.len());
    for (orig, imp) in collection.items.iter().zip(imported.items.iter()) {
        assert_eq!(orig.id, imp.id);
        assert_eq!(orig.title, imp.title);
        assert_eq!(orig.description, imp.description);
        assert_eq!(orig.priority, imp.priority);
        assert_eq!(orig.completed, imp.completed);
    }
}

#[test]
fn test_reset_data() {
    let mut state = AppState::new();
    assert!(!state.collection.items.is_empty());
    state.config.theme = ThemeMode::Light;

    state.reset_data();
    assert!(state.collection.items.is_empty());
    assert_eq!(state.collection.total_count(), 0);
    // Preserves configuration
    assert_eq!(state.config.theme, ThemeMode::Light);
}

#[test]
fn test_app_page_routing() {
    let mut state = AppState::new();
    assert_eq!(state.config.active_page, AppPage::Dashboard);
    assert_eq!(state.config.active_page.label(), "Dashboard");
    assert_eq!(AppPage::ALL.len(), 3);
    for page in AppPage::ALL {
        assert!(!page.description().is_empty());
    }

    state.config.active_page = AppPage::Analytics;
    assert_eq!(state.config.active_page.label(), "Analytics");

    state.config.active_page = AppPage::About;
    assert_eq!(state.config.active_page.label(), "About");

    let serialized = serde_json::to_string(&state).expect("Serialization failed");
    let deserialized: AppState = serde_json::from_str(&serialized).expect("Deserialization failed");
    assert_eq!(deserialized.config.active_page, AppPage::About);
}

#[test]
fn test_svg_html_and_shareable_url_hash_exports() {
    let mut state = AppState::new();
    state.collection.add(Item::new(
        "xss-1",
        "<script>alert('xss')</script>",
        "5 > 3 & 2 < 4 \"quoted\"",
        Priority::Critical,
    ));

    let svg = export_to_svg(&state);
    assert!(svg.starts_with("<svg "));
    assert!(svg.contains("Priority Breakdown (5 items)"));
    assert!(svg.contains("Critical"));

    let html = export_to_printable_html(&state);
    assert!(html.contains("<!DOCTYPE html>"));
    assert!(html.contains("Content-Security-Policy"));
    assert!(!html.contains("<script>alert('xss')</script>"));
    assert!(html.contains("&lt;script&gt;alert(&#39;xss&#39;)&lt;/script&gt;"));
    assert!(html.contains("5 &gt; 3 &amp; 2 &lt; 4 &quot;quoted&quot;"));

    let hash = encode_shareable_url_hash(&state).expect("Encode share hash failed");
    assert!(hash.starts_with(SHARE_HASH_PREFIX));
    let decoded = decode_shareable_url_hash(&hash).expect("Decode share hash failed");
    assert_eq!(decoded, state);

    let full_url = format!("https://example.spodeian.trade/{hash}");
    let decoded_url = decode_shareable_url_hash(&full_url).expect("Decode full URL failed");
    assert_eq!(decoded_url, state);
}

#[test]
fn test_parametric_preview_samples_are_finite_and_bounded() {
    let empty = ParametricPreviewConfig::default().sample_points(0);
    assert!(empty.is_empty());

    let cfg = ParametricPreviewConfig {
        frequency_hz: f32::NAN,
        damping: -4.0,
        harmonic_ratio: 5.0,
    };
    let pts = cfg.sample_points(64);
    assert_eq!(pts.len(), 64);
    assert!(
        pts.iter()
            .all(|(t, y)| t.is_finite() && (0.0..=1.0).contains(t) && y.is_finite())
    );
}

#[test]
fn test_versioned_envelope_and_legacy_schema_migration() {
    let mut state = AppState {
        schema_version: CURRENT_SCHEMA_VERSION,
        collection: ItemCollection::with_sample_data(),
        config: AppConfig {
            theme: ThemeMode::HighContrastDark,
            dyslexia_friendly: true,
            ui_scale_percent: 125,
            ..Default::default()
        },
    };
    state.collection.add(Item::new(
        "extra-1",
        "Migrated Item",
        "Check",
        Priority::Critical,
    ));

    let envelope_json =
        StateMigrator::serialize_versioned(&state, CURRENT_SCHEMA_VERSION, 1_774_900_000.0)
            .expect("Serialize versioned envelope failed");
    let restored = import_from_json(&envelope_json).expect("Import versioned envelope failed");
    assert_eq!(restored, state);

    // Unversioned v0 payload upgrades to CURRENT_SCHEMA_VERSION cleanly.
    let v0_json = r#"{"schema_version":0,"collection":{"items":[{"id":"legacy-1","title":"Legacy","description":"","priority":"Low","completed":false}]},"config":{}}"#;
    let upgraded = import_from_json(v0_json).expect("Import legacy v0 payload failed");
    assert_eq!(upgraded.schema_version, CURRENT_SCHEMA_VERSION);
    assert_eq!(upgraded.collection.items.len(), 1);
    assert_eq!(upgraded.collection.items[0].id, "legacy-1");

    // Standalone custom ItemCollection preserves exact items rather than falling back to defaults.
    let custom_col = ItemCollection {
        items: vec![Item::new(
            "only-one",
            "Only Item",
            "Custom",
            Priority::Medium,
        )],
    };
    let custom_json = serde_json::to_string(&custom_col).unwrap();
    let imported_col = import_from_json(&custom_json).expect("Import custom collection failed");
    assert_eq!(imported_col.schema_version, CURRENT_SCHEMA_VERSION);
    assert_eq!(imported_col.collection.items, custom_col.items);
}

fn arb_priority() -> impl Strategy<Value = Priority> {
    prop_oneof![
        Just(Priority::Low),
        Just(Priority::Medium),
        Just(Priority::High),
        Just(Priority::Critical),
    ]
}

fn arb_theme() -> impl Strategy<Value = ThemeMode> {
    prop_oneof![
        Just(ThemeMode::Dark),
        Just(ThemeMode::Light),
        Just(ThemeMode::HighContrastDark),
        Just(ThemeMode::HighContrastLight),
    ]
}

proptest! {
    #[test]
    fn prop_versioned_state_json_and_bson_roundtrip(
        items in proptest::collection::vec(
            ("[a-z0-9_-]{1,12}", ".{0,32}", ".{0,48}", arb_priority(), any::<bool>(), any::<u64>()),
            0..12
        ),
        theme in arb_theme(),
        dyslexia_friendly in any::<bool>(),
        ui_scale_percent in 50u16..=200u16,
        is_about in any::<bool>(),
    ) {
        let state = AppState {
            schema_version: CURRENT_SCHEMA_VERSION,
            collection: ItemCollection {
                items: items
                    .into_iter()
                    .map(|(id, title, description, priority, completed, created_at)| Item {
                        id,
                        title,
                        description,
                        priority,
                        completed,
                        created_at,
                    })
                    .collect(),
            },
            config: AppConfig {
                theme,
                active_page: if is_about { AppPage::About } else { AppPage::Dashboard },
                auto_save: true,
                show_completed: true,
                filter_priority: None,
                search_query: String::new(),
                dyslexia_friendly,
                ui_scale_percent,
                ..Default::default()
            },
        };

        let json = export_to_json(&state).expect("JSON export failed");
        let from_json = import_from_json(&json).expect("JSON import failed");
        prop_assert_eq!(&from_json, &state);

        let envelope = StateMigrator::serialize_versioned(&state, CURRENT_SCHEMA_VERSION, 1.0)
            .expect("Versioned serialize failed");
        let from_envelope = import_from_json(&envelope).expect("Envelope import failed");
        prop_assert_eq!(&from_envelope, &state);

        let bson_bytes = export_to_compressed_bson(&state).expect("BSON export failed");
        let from_bson = import_from_compressed_bson(&bson_bytes).expect("BSON import failed");
        prop_assert_eq!(&from_bson, &state);

        let share_hash = encode_shareable_url_hash(&state).expect("Share hash encode failed");
        let from_hash = decode_shareable_url_hash(&share_hash).expect("Share hash decode failed");
        prop_assert_eq!(&from_hash, &state);
    }

    #[test]
    fn prop_escape_html_neutralizes_raw_markup_delimiters(raw in ".*") {
        let escaped = escape_html(&raw);
        prop_assert!(!escaped.contains('<'));
        prop_assert!(!escaped.contains('>'));
    }

    #[test]
    fn prop_csv_export_import_roundtrip(
        items in proptest::collection::vec(
            (
                "[A-Za-z0-9_-]{1,12}",
                "[A-Za-z0-9,.\"_-]([A-Za-z0-9,.\" _-]{0,20}[A-Za-z0-9,.\"_-])?",
                "[A-Za-z0-9,.\"_-]([A-Za-z0-9,.\" _-]{0,30}[A-Za-z0-9,.\"_-])?",
                arb_priority(),
                any::<bool>(),
                any::<u64>(),
            ),
            1..10
        ),
    ) {
        let collection = ItemCollection {
            items: items
                .into_iter()
                .map(|(id, title, description, priority, completed, created_at)| Item {
                    id,
                    title,
                    description,
                    priority,
                    completed,
                    created_at,
                })
                .collect(),
        };

        let csv = export_to_csv(&collection).expect("CSV export failed");
        let imported = import_from_csv(&csv).expect("CSV import failed");
        prop_assert_eq!(imported, collection);
    }

    #[test]
    fn prop_ui_scale_clamping_and_theme_cycle(
        raw_scale in 0u16..=500u16,
        theme in arb_theme(),
    ) {
        let cfg = AppConfig {
            ui_scale_percent: raw_scale,
            theme,
            ..Default::default()
        };
        let factor = cfg.clamped_ui_scale_factor();
        prop_assert!((0.80..=1.50).contains(&factor));

        let cycled = theme.next().next().next().next();
        prop_assert_eq!(cycled, theme);
    }
}

#[test]
fn test_batch_harmonic_sweep_and_wgsl_parity() {
    let cfg = ParametricPreviewConfig::default();
    let curves = cfg.batch_harmonic_sweep(&[0.5, 1.0, 2.0], 64);
    assert_eq!(curves.len(), 3);
    assert!(curves.iter().all(|c| c.len() == 64));

    let cpu_ref = cfg.evaluate_harmonic_wgsl_cpu_reference(64);
    assert_eq!(cpu_ref.len(), 64);
    for (idx, (_, y)) in curves[1].iter().enumerate() {
        assert!((y - cpu_ref[idx]).abs() < 1e-6);
    }

    assert!(PARAMETRIC_HARMONIC_WGSL_SHADER.contains("@compute @workgroup_size(64)"));
    assert!(PARAMETRIC_HARMONIC_WGSL_SHADER.contains("fn main("));

    let summary = ComputeRuntimeSummary::current();
    assert!(summary.parallel_threads >= 1);
    assert!(summary.wgsl_shader_ready);
}
