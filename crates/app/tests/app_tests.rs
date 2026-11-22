use app::{
    CommandEntry, CommandPaletteWidget, TemplateApp, UndoStack, default_template_commands,
    detect_shortcut_conflicts,
};
use eframe::{App, Storage};
use proptest::prelude::*;
use shared::{AppPage, AppState, Item, Priority, ThemeMode};
use std::collections::{HashMap, HashSet, VecDeque};

#[derive(Default)]
struct MockStorage {
    data: HashMap<String, String>,
}

impl Storage for MockStorage {
    fn get_string(&self, key: &str) -> Option<String> {
        self.data.get(key).cloned()
    }
    fn set_string(&mut self, key: &str, value: String) {
        self.data.insert(key.to_owned(), value);
    }
    fn remove_string(&mut self, key: &str) {
        self.data.remove(key);
    }
    fn flush(&mut self) {}
}

#[test]
fn test_template_app_initialization() {
    let app = TemplateApp::default();
    assert_eq!(app.state.collection.total_count(), 4);
    assert_eq!(app.state.collection.completed_count(), 1);
}

#[test]
fn test_template_app_save_and_load() {
    let mut storage = MockStorage::default();
    let mut app = TemplateApp::default();

    // Add item and toggle
    let new_item = Item::new(
        "custom-persistent-1",
        "Persistent Task",
        "Should be saved in storage",
        Priority::High,
    );
    app.state.collection.add(new_item);
    app.state.collection.toggle("custom-persistent-1");

    // Save
    app.save(&mut storage);

    // Verify storage contents
    let serialized = storage
        .get_string(eframe::APP_KEY)
        .expect("App key must exist in storage");
    assert!(serialized.contains("Persistent Task"));

    // Simulate reload from storage
    let loaded_state: AppState = eframe::get_value(&storage, eframe::APP_KEY).unwrap();
    assert_eq!(loaded_state.collection.total_count(), 5);
    assert_eq!(loaded_state.collection.completed_count(), 2);
}

#[test]
fn test_template_app_export_dialog() {
    let mut app = TemplateApp::default();
    app.open_export_dialog(app::ExportFormat::Json);

    assert!(app.show_export_dialog.is_some());
    assert!(app.export_text_buffer.contains("Deploy Serverless WebApp"));
}

#[test]
fn test_template_app_load_multi_tier_json_and_ron() {
    use app::storage_manager::{
        DEDICATED_STORAGE_KEY, deserialize_app_state, load_state_multi_tier,
    };

    let mut original_state = AppState::default();
    original_state.collection.add(Item::new(
        "task-a",
        "Task A",
        "Description A",
        Priority::High,
    ));

    // 1. JSON roundtrip
    let json_str = serde_json::to_string(&original_state).unwrap();
    let from_json = deserialize_app_state(&json_str).expect("JSON should deserialize");
    assert_eq!(from_json.collection.total_count(), 5);

    // 2. RON roundtrip
    let ron_str = ron::to_string(&original_state).unwrap();
    let from_ron = deserialize_app_state(&ron_str).expect("RON should deserialize");
    assert_eq!(from_ron.collection.total_count(), 5);

    // 3. Load from dedicated key in storage (JSON)
    let mut storage_dedicated = MockStorage::default();
    storage_dedicated.set_string(DEDICATED_STORAGE_KEY, json_str.clone());
    let loaded_dedicated =
        load_state_multi_tier(Some(&storage_dedicated)).expect("Should load from dedicated key");
    assert_eq!(loaded_dedicated.collection.total_count(), 5);

    // 4. Load from legacy app key in storage (JSON)
    let mut storage_json_app = MockStorage::default();
    storage_json_app.set_string(eframe::APP_KEY, json_str);
    let loaded_json_app =
        load_state_multi_tier(Some(&storage_json_app)).expect("Should load from app key (JSON)");
    assert_eq!(loaded_json_app.collection.total_count(), 5);

    // 5. Load from legacy app key in storage (RON)
    let mut storage_ron_app = MockStorage::default();
    storage_ron_app.set_string(eframe::APP_KEY, ron_str);
    let loaded_ron_app =
        load_state_multi_tier(Some(&storage_ron_app)).expect("Should load from app key (RON)");
    assert_eq!(loaded_ron_app.collection.total_count(), 5);
}

#[test]
fn test_storage_diagnostics_defaults() {
    let diag = app::storage_manager::StorageDiagnostics::default();
    assert_eq!(diag.memory_items_count, 0);
    assert_eq!(
        diag.memory_capacity_limit,
        app::storage_manager::MEMORY_WORKING_SET_LIMIT
    );
    assert!(!diag.fallback_tier_active);
    assert!(!diag.quota_exceeded);
}

#[test]
fn test_default_commands_have_no_shortcut_conflicts_and_detector_catches_duplicates() {
    let defaults = default_template_commands();
    assert!(
        detect_shortcut_conflicts(&defaults).is_empty(),
        "Default command palette entries must have unique shortcut hints"
    );

    let mut with_conflict = defaults;
    with_conflict.push(CommandEntry::new(
        "custom.duplicate",
        "Conflicting Shortcut",
        "1 + ctrl",
        "Test",
    ));
    let conflicts = detect_shortcut_conflicts(&with_conflict);
    assert_eq!(conflicts.len(), 1);
    assert_eq!(conflicts[0].0, "ctrl+1");
    assert_eq!(conflicts[0].1, "nav.dashboard");
    assert_eq!(conflicts[0].2, "custom.duplicate");
}

#[test]
fn test_command_palette_execution_and_undo_redo_workflow() {
    let mut app = TemplateApp::default();
    assert_eq!(app.state.config.active_page, AppPage::Dashboard);
    assert!(app.execute_command("nav.about"));
    assert_eq!(app.state.config.active_page, AppPage::About);

    let initial_theme = app.state.config.theme;
    assert!(app.execute_command("theme.cycle"));
    assert_ne!(app.state.config.theme, initial_theme);
    assert_eq!(app.state.config.theme, initial_theme.next());

    assert!(!app.state.config.dyslexia_friendly);
    assert!(app.execute_command("a11y.dyslexia"));
    assert!(app.state.config.dyslexia_friendly);

    let base_count = app.state.collection.total_count();
    app.record_undo_snapshot();
    app.state.collection.add(Item::new(
        "undo-item",
        "Temporary Item",
        "Will be undone",
        Priority::High,
    ));
    assert_eq!(app.state.collection.total_count(), base_count + 1);

    assert!(app.execute_command("edit.undo"));
    assert_eq!(app.state.collection.total_count(), base_count);
    assert!(app.execute_command("edit.redo"));
    assert_eq!(app.state.collection.total_count(), base_count + 1);

    assert!(app.execute_command("nav.analytics"));
    assert_eq!(app.state.config.active_page, AppPage::Analytics);

    assert!(!app.state.config.zen_mode);
    assert!(app.execute_command("view.zen_mode"));
    assert!(app.state.config.zen_mode);
    assert!(app.execute_command("view.zen_mode"));
    assert!(!app.state.config.zen_mode);

    assert!(!app.execute_command("nonexistent.command"));
}

#[test]
fn test_shared_url_hash_protection_and_restore_workflow() {
    let mut app = TemplateApp::default();
    let local_count = app.state.collection.total_count();

    let mut remote_state = AppState::default();
    remote_state.collection.reset();
    remote_state.collection.add(Item::new(
        "shared-1",
        "Shared Only Item",
        "From URL hash",
        Priority::Critical,
    ));
    let hash = shared::encode_shareable_url_hash(&remote_state).expect("encode share hash");

    app.load_shared_url_hash(&hash)
        .expect("load shared hash should succeed");
    assert!(app.is_viewing_shared_link);
    assert_eq!(app.state.collection.total_count(), 1);
    assert_eq!(app.state.collection.items[0].id, "shared-1");

    // Restoring local state discards the preview without touching local data.
    app.restore_local_state_from_shared();
    assert!(!app.is_viewing_shared_link);
    assert_eq!(app.state.collection.total_count(), local_count);

    // Accepting a shared state promotes it to local state.
    app.load_shared_url_hash(&hash)
        .expect("reload shared hash should succeed");
    app.accept_shared_state_as_local();
    assert!(!app.is_viewing_shared_link);
    assert_eq!(app.state.collection.total_count(), 1);
}

#[test]
fn test_dropped_file_import_json_csv_and_bson() {
    let mut app = TemplateApp::default();

    let mut custom = AppState::default();
    custom.collection.reset();
    custom.collection.add(Item::new(
        "drop-1",
        "Dropped Task",
        "Via drag and drop",
        Priority::High,
    ));

    let json = shared::export_to_json(&custom).expect("json");
    let imported_json = app
        .import_dropped_bytes("backup.json", json.as_bytes())
        .expect("drop json");
    assert_eq!(imported_json, 1);
    assert_eq!(app.state.collection.items[0].id, "drop-1");

    let mut csv_col = shared::ItemCollection::new();
    csv_col.add(Item::new("csv-1", "CSV 1", "Desc 1", Priority::Low));
    csv_col.add(Item::new("csv-2", "CSV 2", "Desc 2", Priority::Medium));
    let csv = shared::export_to_csv(&csv_col).expect("csv");
    let imported_csv = app
        .import_dropped_bytes("tasks.csv", csv.as_bytes())
        .expect("drop csv");
    assert_eq!(imported_csv, 2);
    assert_eq!(app.state.collection.total_count(), 2);

    let bson = shared::export_to_compressed_bson(&custom).expect("bson");
    let imported_bson = app
        .import_dropped_bytes("state.bson", &bson)
        .expect("drop bson");
    assert_eq!(imported_bson, 1);
    assert_eq!(app.state.collection.items[0].id, "drop-1");
}

#[test]
fn test_svg_html_and_share_link_export_commands() {
    let mut app = TemplateApp::default();

    app.open_export_dialog(app::ExportFormat::Svg);
    assert_eq!(app.show_export_dialog, Some(app::ExportFormat::Svg));
    assert!(app.export_text_buffer.starts_with("<svg "));

    app.open_export_dialog(app::ExportFormat::Html);
    assert_eq!(app.show_export_dialog, Some(app::ExportFormat::Html));
    assert!(app.export_text_buffer.contains("<!DOCTYPE html>"));
    assert!(app.export_text_buffer.contains("<svg "));

    assert!(app.execute_command("data.copy_share_link"));
    let copied = app
        .last_copied_share_hash
        .as_deref()
        .expect("share hash captured");
    assert!(copied.starts_with(shared::SHARE_HASH_PREFIX));
}

#[test]
fn test_recoverable_panic_boundary_captures_and_recovers() {
    let mut app = TemplateApp::default();
    let ok = app.run_recoverable("faulty_import", |_| {
        panic!("simulated malformed input");
    });
    assert!(!ok);
    let err = app
        .last_recovered_error
        .as_deref()
        .expect("Recovered error message should be populated");
    assert!(err.contains("faulty_import"));
    assert!(err.contains("simulated malformed input"));

    let ok_after = app.run_recoverable("healthy_toggle", |a| {
        a.state.config.theme = ThemeMode::Light;
    });
    assert!(ok_after);
    assert_eq!(app.state.config.theme, ThemeMode::Light);
}

#[derive(Debug, Clone)]
enum UndoOp {
    Push(usize),
    Undo,
    Redo,
    Clear,
}

fn arb_undo_op() -> impl Strategy<Value = UndoOp> {
    prop_oneof![
        (0usize..1000usize).prop_map(UndoOp::Push),
        Just(UndoOp::Undo),
        Just(UndoOp::Redo),
        Just(UndoOp::Clear),
    ]
}

proptest! {
    #[test]
    fn prop_undo_stack_state_invariants(
        cap in 1usize..=16usize,
        ops in proptest::collection::vec(arb_undo_op(), 0..64),
    ) {
        let mut stack = UndoStack::new(cap);
        let mut current = 0usize;
        let mut ref_undo: VecDeque<usize> = VecDeque::new();
        let mut ref_redo: Vec<usize> = Vec::new();
        let mut ref_current = 0usize;

        for op in ops {
            match op {
                UndoOp::Push(next_val) => {
                    stack.push(current);
                    current = next_val;

                    if ref_undo.len() >= cap {
                        ref_undo.pop_front();
                    }
                    ref_undo.push_back(ref_current);
                    ref_redo.clear();
                    ref_current = next_val;
                }
                UndoOp::Undo => {
                    let got = stack.undo(current);
                    let expected = if let Some(prev) = ref_undo.pop_back() {
                        ref_redo.push(ref_current);
                        ref_current = prev;
                        Some(prev)
                    } else {
                        None
                    };
                    prop_assert_eq!(got, expected);
                    if let Some(prev) = got {
                        current = prev;
                    }
                }
                UndoOp::Redo => {
                    let got = stack.redo(current);
                    let expected = if let Some(next) = ref_redo.pop() {
                        if ref_undo.len() >= cap {
                            ref_undo.pop_front();
                        }
                        ref_undo.push_back(ref_current);
                        ref_current = next;
                        Some(next)
                    } else {
                        None
                    };
                    prop_assert_eq!(got, expected);
                    if let Some(next) = got {
                        current = next;
                    }
                }
                UndoOp::Clear => {
                    stack.clear();
                    ref_undo.clear();
                    ref_redo.clear();
                }
            }

            prop_assert_eq!(current, ref_current);
            prop_assert!(stack.undo_len() <= cap);
            prop_assert_eq!(stack.can_undo(), !ref_undo.is_empty());
            prop_assert_eq!(stack.can_redo(), !ref_redo.is_empty());
            prop_assert_eq!(stack.undo_len(), ref_undo.len());
            prop_assert_eq!(stack.redo_len(), ref_redo.len());
        }
    }

    #[test]
    fn prop_command_palette_fuzzy_search_never_out_of_bounds(
        queries in proptest::collection::vec(".{0,24}", 1..20),
    ) {
        let commands = default_template_commands();
        let count = commands.len();
        let mut palette = CommandPaletteWidget::new(commands);
        palette.open();

        for query in queries {
            palette.set_query(query);
            let filtered = palette.filtered_indices();
            prop_assert!(filtered.iter().all(|&idx| idx < count));
            if filtered.is_empty() {
                prop_assert_eq!(palette.selected_index(), 0);
            } else {
                prop_assert!(palette.selected_index() < filtered.len());
            }
        }
    }

    #[test]
    fn prop_shortcut_conflict_detection(
        bindings in proptest::collection::vec(
            ("[a-z.]{2,10}", prop_oneof![Just(""), Just("Ctrl+A"), Just("Ctrl+B"), Just("Shift+X"), Just("F1")]),
            0..12
        ),
    ) {
        let entries: Vec<CommandEntry> = bindings
            .iter()
            .enumerate()
            .map(|(i, (id, key))| CommandEntry::new(format!("{id}_{i}"), "Cmd", *key, "Cat"))
            .collect();
        let conflicts = detect_shortcut_conflicts(&entries);

        let non_empty: Vec<&str> = bindings
            .iter()
            .map(|(_, k)| *k)
            .filter(|k| !k.is_empty())
            .collect();
        let unique: HashSet<&str> = non_empty.iter().copied().collect();
        prop_assert_eq!(conflicts.is_empty(), non_empty.len() == unique.len());
    }
}
