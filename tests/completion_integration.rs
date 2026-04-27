//! Integration tests for import-aware completion.

use rusqlite::{params, Connection};
use splice::completion::engine::CompletionEngine;
use splice::completion::types::{CompletionRequest, SuggestionSource};
use splice::graph::MagellanIntegration;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use tempfile::TempDir;

static CWD_LOCK: Mutex<()> = Mutex::new(());

fn create_completion_test_db() -> (TempDir, PathBuf, PathBuf, PathBuf) {
    create_completion_test_db_with_token("cal")
}

fn create_completion_test_db_with_token(token: &str) -> (TempDir, PathBuf, PathBuf, PathBuf) {
    let temp_dir = TempDir::new().unwrap();
    let db_path = temp_dir.path().join("splice.db");
    let current_file = temp_dir.path().join("src/main.rs");
    let imported_file = temp_dir.path().join("src/action.rs");

    std::fs::create_dir_all(current_file.parent().unwrap()).unwrap();
    std::fs::write(
        &current_file,
        format!("use crate::action::calculate_score;\nfn main() {{\n    {token}\n}}\n"),
    )
    .unwrap();
    std::fs::write(&imported_file, "pub fn calculate_score() -> i32 { 42 }\n").unwrap();

    // Let Magellan create the current sqlitegraph schema, then seed only the
    // small set of facts completion needs for deterministic tests.
    drop(MagellanIntegration::open(&db_path).unwrap());
    let conn = Connection::open(&db_path).unwrap();

    conn.execute(
        "INSERT INTO graph_entities (id, name, kind, file_path, data) VALUES (?1, ?2, ?3, ?4, ?5)",
        params![
            1_i64,
            "main.rs",
            "File",
            current_file.to_string_lossy().as_ref(),
            "{}"
        ],
    )
    .unwrap();

    conn.execute(
        "INSERT INTO graph_entities (id, name, kind, file_path, data) VALUES (?1, ?2, ?3, ?4, ?5)",
        params![
            2_i64,
            "main",
            "Symbol",
            current_file.to_string_lossy().as_ref(),
            r#"{"kind":"Function","start_line":2,"end_line":4,"start_col":4,"display_fqn":"test::main"}"#
        ],
    )
    .unwrap();

    conn.execute(
        "INSERT INTO graph_entities (id, name, kind, file_path, data) VALUES (?1, ?2, ?3, ?4, ?5)",
        params![
            3_i64,
            "calculate_score",
            "Symbol",
            imported_file.to_string_lossy().as_ref(),
            r#"{"kind":"Function","start_line":1,"end_line":1,"start_col":8,"visibility":"public","display_fqn":"test::action::calculate_score"}"#
        ],
    )
    .unwrap();

    conn.execute(
        "INSERT INTO graph_entities (id, name, kind, file_path, data) VALUES (?1, ?2, ?3, ?4, ?5)",
        params![
            4_i64,
            "use crate::action::calculate_score",
            "Import",
            current_file.to_string_lossy().as_ref(),
            r#"{"import_kind":"plain_use","import_path":["crate","action"],"imported_names":["calculate_score"],"is_glob":false,"is_reexport":false}"#
        ],
    )
    .unwrap();

    conn.execute(
        "INSERT INTO graph_edges (id, from_id, to_id, edge_type, data) VALUES (?1, ?2, ?3, ?4, ?5)",
        params![1_i64, 1_i64, 4_i64, "IMPORTS", "{}"],
    )
    .unwrap();

    (temp_dir, db_path, current_file, imported_file)
}

#[test]
fn test_completion_with_imports() {
    let _guard = CWD_LOCK.lock().unwrap();
    let (_temp_dir, db_path, current_file, imported_file) = create_completion_test_db();
    let magellan = Arc::new(MagellanIntegration::open(&db_path).unwrap());
    let engine = CompletionEngine::new(magellan, &db_path);

    let request = CompletionRequest {
        file_path: current_file,
        line: 3,
        column: 8,
        max_results: Some(10),
    };

    let response = engine.complete_at_cursor(request).unwrap();

    let suggestion = response
        .suggestions
        .iter()
        .find(|suggestion| suggestion.label == "calculate_score")
        .expect("expected imported calculate_score suggestion");

    assert!(matches!(suggestion.source, SuggestionSource::Imported));
    assert_eq!(suggestion.grounded_in, vec!["3"]);
    assert_eq!(
        suggestion.source_file.as_deref(),
        Some(imported_file.to_string_lossy().as_ref())
    );
    assert_eq!(suggestion.via_import.as_deref(), Some("use crate::action"));
}

#[test]
fn test_completion_token_filtering() {
    let _guard = CWD_LOCK.lock().unwrap();
    let (_temp_dir, db_path, current_file, _imported_file) = create_completion_test_db();
    let magellan = Arc::new(MagellanIntegration::open(&db_path).unwrap());
    let engine = CompletionEngine::new(magellan, &db_path);

    let request = CompletionRequest {
        file_path: current_file,
        line: 3,
        column: 8,
        max_results: Some(10),
    };

    let response = engine.complete_at_cursor(request).unwrap();

    assert!(!response.suggestions.is_empty());
    assert!(response
        .suggestions
        .iter()
        .all(|suggestion| suggestion.label.contains("cal")));
}

#[test]
fn test_relative_file_path_keeps_local_symbols_as_database_source() {
    let _guard = CWD_LOCK.lock().unwrap();
    let original_dir = std::env::current_dir().unwrap();
    let (_temp_dir, db_path, _current_file, _imported_file) =
        create_completion_test_db_with_token("mai");

    std::env::set_current_dir(db_path.parent().unwrap()).unwrap();

    let result = (|| {
        let magellan = Arc::new(MagellanIntegration::open(&db_path).unwrap());
        let engine = CompletionEngine::new(magellan, &db_path);

        let request = CompletionRequest {
            file_path: PathBuf::from("src/main.rs"),
            line: 3,
            column: 8,
            max_results: Some(10),
        };

        engine.complete_at_cursor(request)
    })();

    std::env::set_current_dir(original_dir).unwrap();

    let response = result.unwrap();
    let suggestion = response
        .suggestions
        .iter()
        .find(|suggestion| suggestion.label == "main")
        .expect("expected local main suggestion");

    assert!(matches!(suggestion.source, SuggestionSource::Database));
    assert!(suggestion.source_file.is_none());
    assert!(suggestion.via_import.is_none());
}
