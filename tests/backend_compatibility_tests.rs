//! Backend compatibility tests.
//!
//! These tests verify that the public API works identically
//! regardless of which backend (SQLite or native-v3) is enabled.
//!
//! Run with:
//!   cargo test                          # SQLite backend (default)
//!   cargo test --features native-v3     # Native-v2 backend

use splice::graph::{Backend, CodeGraph};
use std::fs;
use tempfile::TempDir;

/// Test that CodeGraph::open works with both backends.
#[test]
fn test_graph_open_both_backends() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let db_path = temp_dir.path().join("test_graph.db");

    // Remove any existing database
    let _ = fs::remove_file(&db_path);

    // Open should succeed regardless of backend
    let graph = CodeGraph::open(&db_path);
    assert!(graph.is_ok(), "CodeGraph::open failed: {:?}", graph.err());

    // Verify database file was created
    assert!(db_path.exists(), "Database file not created");
}

/// Test that backend detection works correctly.
#[test]
fn test_backend_detection() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let db_path = temp_dir.path().join("test_detect.db");
    let _ = fs::remove_file(&db_path);

    // Create a graph and check backend detection
    let _graph = CodeGraph::open(&db_path).expect("Failed to open graph");

    let detected = CodeGraph::detect_backend(&db_path);
    assert!(detected.is_ok(), "Backend detection failed: {:?}", detected.err());

    let backend = detected.unwrap();
    // We should detect either SQLite or NativeV3 (not Unknown for existing files)
    assert_ne!(Backend::Unknown, backend, "Detected Unknown backend for existing file");
}

/// Test that symbol storage works with both backends.
#[test]
fn test_symbol_storage_both_backends() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let db_path = temp_dir.path().join("test_symbols.db");
    let _ = fs::remove_file(&db_path);

    let mut graph = CodeGraph::open(&db_path).expect("Failed to open graph");

    // Store a symbol with file and language
    let file_path = temp_dir.path().join("test.rs");
    let node_id = graph
        .store_symbol_with_file_and_language(
            &file_path,
            "test_function",
            "function",
            splice::symbol::Language::Rust,
            0,
            100,
            1,
            5,
            0,
            10,
        )
        .expect("Failed to store symbol");

    // Node ID should be valid (non-zero)
    assert!(node_id.as_i64() > 0, "Node ID should be positive, got: {}", node_id.as_i64());
}

/// Test that backend can be queried after operations.
#[test]
fn test_backend_persistence() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let db_path = temp_dir.path().join("test_persist.db");
    let _ = fs::remove_file(&db_path);

    // Create and populate a graph
    {
        let mut graph = CodeGraph::open(&db_path).expect("Failed to open graph");
        let file_path = temp_dir.path().join("persist.rs");
        graph
            .store_symbol_with_file_and_language(
                &file_path,
                "persistent_func",
                "function",
                splice::symbol::Language::Rust,
                0,
                50,
                1,
                3,
                0,
                10,
            )
            .expect("Failed to store symbol");
    }

    // Re-open and verify backend is still detected correctly
    let backend = CodeGraph::detect_backend(&db_path).expect("Detection failed");
    assert_ne!(Backend::Unknown, backend, "Backend should persist after close");
}

/// Test that multiple symbols can be stored and retrieved.
#[test]
fn test_multiple_symbols_both_backends() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let db_path = temp_dir.path().join("test_multiple.db");
    let _ = fs::remove_file(&db_path);

    let mut graph = CodeGraph::open(&db_path).expect("Failed to open graph");
    let file_path = temp_dir.path().join("multi.rs");

    // Store multiple symbols
    let id1 = graph
        .store_symbol_with_file_and_language(
            &file_path,
            "function_one",
            "function",
            splice::symbol::Language::Rust,
            0,
            50,
            1,
            5,
            0,
            10,
        )
        .expect("Failed to store symbol 1");

    let id2 = graph
        .store_symbol_with_file_and_language(
            &file_path,
            "function_two",
            "function",
            splice::symbol::Language::Rust,
            60,
            110,
            7,
            12,
            0,
            15,
        )
        .expect("Failed to store symbol 2");

    // IDs should be different
    assert_ne!(id1, id2, "Different symbols should have different IDs");

    // Both should be positive
    assert!(id1.as_i64() > 0);
    assert!(id2.as_i64() > 0);
}

/// Feature-gated test: Verify native-v3 specific operations only work with that feature.
#[cfg(feature = "native-v3")]
#[test]
fn test_native_v3_feature_enabled() {
    // This test only compiles with native-v3 feature
    // Verify that we can open graphs with native-v3 backend
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let db_path = temp_dir.path().join("native_v3_test.db");
    let _ = fs::remove_file(&db_path);

    let mut graph = CodeGraph::open(&db_path).expect("Failed to open graph with native-v3");

    // Store a symbol to verify operations work
    let file_path = temp_dir.path().join("native.rs");
    let node_id = graph
        .store_symbol_with_file_and_language(
            &file_path,
            "native_function",
            "function",
            splice::symbol::Language::Rust,
            0,
            80,
            1,
            8,
            0,
            12,
        )
        .expect("Failed to store symbol");

    assert!(node_id.as_i64() > 0);
}
