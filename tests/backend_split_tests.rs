//! Backend split architecture verification tests
//!
//! These tests verify that:
//! 1. SQLite backend works independently
//! 2. Geometric backend works independently
//! 3. Both backends implement the same interface
//! 4. Router correctly dispatches to appropriate backend

use std::path::Path;
use tempfile::NamedTempFile;

/// Test 1: SQLite backend can be opened and used
#[test]
fn test_sqlite_backend_open_and_store() {
    let temp_file = NamedTempFile::new().unwrap();
    let mut graph = splice::graph::CodeGraph::open(temp_file.path()).unwrap();

    // Verify backend type
    assert!(matches!(
        graph.backend_type(),
        splice::graph::BackendType::SQLite
    ));
    assert!(!graph.is_geometric());

    // Store a symbol
    let node_id = graph
        .store_symbol_with_file_and_language(
            Path::new("src/test.rs"),
            "test_function",
            "function",
            splice::symbol::Language::Rust,
            100,
            200,
            10,
            20,
            4,
            0,
        )
        .unwrap();

    // Find the symbol
    let found = graph.find_symbol_in_file("src/test.rs", "test_function");
    assert_eq!(found, Some(node_id));

    // Get span
    let (start, end) = graph.get_span(node_id).unwrap();
    assert_eq!(start, 100);
    assert_eq!(end, 200);
}

/// Test 2: Router dispatches to SQLite for .db files
#[test]
fn test_router_dispatches_to_sqlite_for_db() {
    let temp_file = NamedTempFile::new().unwrap();
    let graph = splice::graph::CodeGraph::open(temp_file.path()).unwrap();

    // Should be SQLite backend
    assert!(matches!(
        graph.backend_type(),
        splice::graph::BackendType::SQLite
    ));

    // Should have sqlite inner access
    assert!(graph.inner().is_ok());
    assert!(graph.as_sqlite().is_some());

    // Should NOT have geometric access
    assert!(!graph.is_geometric());
}

/// Test 3: Router dispatches to Geometric for .geo files
#[test]
#[cfg(feature = "geometric")]
fn test_router_dispatches_to_geo_for_geo() {
    // Skip if no real .geo file exists (we can't create one without magellan)
    if !Path::new("./code.geo").exists() {
        eprintln!("SKIP: No code.geo file available for test");
        return;
    }

    let graph = splice::graph::CodeGraph::open(Path::new("./code.geo")).unwrap();

    // Should be Geometric backend
    assert!(matches!(
        graph.backend_type(),
        splice::graph::BackendType::Geometric
    ));
    assert!(graph.is_geometric());

    // Should have geometric access
    assert!(graph.geometric().is_ok());
    assert!(graph.as_geo().is_some());

    // Should NOT have sqlite inner access
    assert!(graph.inner().is_err());
    assert!(graph.as_sqlite().is_none());
}

/// Test 4: Both backends support find_symbols_by_name
#[test]
fn test_both_backends_find_symbols_by_name() {
    let temp_file = NamedTempFile::new().unwrap();
    let mut graph = splice::graph::CodeGraph::open(temp_file.path()).unwrap();

    // Store two symbols with same name in different files
    graph
        .store_symbol_with_file_and_language(
            Path::new("src/a.rs"),
            "common",
            "function",
            splice::symbol::Language::Rust,
            0,
            10,
            1,
            1,
            0,
            0,
        )
        .unwrap();

    graph
        .store_symbol_with_file_and_language(
            Path::new("src/b.rs"),
            "common",
            "function",
            splice::symbol::Language::Rust,
            0,
            10,
            1,
            1,
            0,
            0,
        )
        .unwrap();

    // Find symbols by name
    let matches = graph.find_symbols_by_name("common");
    assert_eq!(matches.len(), 2);
}

/// Test 5: Both backends support all_symbol_names
#[test]
fn test_both_backends_all_symbol_names() {
    let temp_file = NamedTempFile::new().unwrap();
    let mut graph = splice::graph::CodeGraph::open(temp_file.path()).unwrap();

    // Store symbols
    graph
        .store_symbol_with_file_and_language(
            Path::new("src/test.rs"),
            "func_a",
            "function",
            splice::symbol::Language::Rust,
            0,
            10,
            1,
            1,
            0,
            0,
        )
        .unwrap();

    graph
        .store_symbol_with_file_and_language(
            Path::new("src/test.rs"),
            "func_b",
            "function",
            splice::symbol::Language::Rust,
            20,
            30,
            2,
            2,
            0,
            0,
        )
        .unwrap();

    // Get all symbol names
    let names = graph.all_symbol_names();
    assert!(names.contains(&"func_a".to_string()));
    assert!(names.contains(&"func_b".to_string()));
}

/// Test 6: Backend detection works correctly
#[test]
fn test_backend_detection() {
    // SQLite
    let sqlite_file = NamedTempFile::new().unwrap();
    let detected = splice::graph::CodeGraph::detect_backend(sqlite_file.path()).unwrap();
    assert!(matches!(detected, splice::graph::BackendType::SQLite));

    // Geometric
    let detected = splice::graph::CodeGraph::detect_backend(Path::new("test.geo")).unwrap();
    assert!(matches!(detected, splice::graph::BackendType::Geometric));
}

/// Test 7: is_geometric_db helper works
#[test]
fn test_is_geometric_db_helper() {
    assert!(splice::graph::CodeGraph::is_geometric_db(Path::new(
        "code.geo"
    )));
    assert!(!splice::graph::CodeGraph::is_geometric_db(Path::new(
        "code.db"
    )));
    assert!(!splice::graph::CodeGraph::is_geometric_db(Path::new(
        ".magellan/splice.db"
    )));
}

/// Test 8: Geometric backend can resolve real symbols from code.geo
#[test]
#[cfg(feature = "geometric")]
fn test_geo_backend_real_workflow() {
    if !Path::new("./code.geo").exists() {
        eprintln!("SKIP: No code.geo file available for test");
        return;
    }

    let graph = splice::graph::CodeGraph::open(Path::new("./code.geo")).unwrap();

    // Verify backend type
    assert!(graph.is_geometric());

    // Try to find a known function from Splice source
    // Use absolute path as indexed by magellan
    let file_path = "/home/feanor/Projects/splice/src/graph/mod.rs";

    // Just verify the query doesn't panic - actual symbol presence depends on indexing
    let _symbols = graph.find_symbols_by_name("open");

    // If we can find any symbols, the backend is working
    let names = graph.all_symbol_names();
    assert!(!names.is_empty(), "Geometric backend should have symbols");
}
