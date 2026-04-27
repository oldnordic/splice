//! REAL .GEO WORKFLOW VERIFICATION TESTS
//!
//! These tests verify that Splice can actually use .geo files for real edit-related workflows.
//! They use the real ./code.geo database built from the Splice source code.
//!
//! CRITICAL: These tests require:
//! 1. SPLICE_GEO_TEST_DB to point to a valid .geo database
//! 2. --features geometric to be enabled

#![cfg(feature = "geometric")]

use splice::graph::CodeGraph;
use std::path::PathBuf;

fn geo_test_db() -> Option<PathBuf> {
    let path = match std::env::var_os("SPLICE_GEO_TEST_DB") {
        Some(path) => PathBuf::from(path),
        None => {
            eprintln!(
                "SKIP: SPLICE_GEO_TEST_DB is not set. Build a .geo database and set this env var to run real geometric workflow tests."
            );
            return None;
        }
    };

    if !path.exists() {
        eprintln!(
            "SKIP: SPLICE_GEO_TEST_DB does not exist: {}",
            path.display()
        );
        return None;
    }

    Some(path)
}

/// PHASE 3.1: Verify Splice can open .geo backend
///
/// Test: splice_can_open_geo_backend
/// Expected: CodeGraph::open() succeeds on .geo file without sqlitegraph errors
#[test]
fn splice_can_open_geo_backend() {
    let Some(geo_path) = geo_test_db() else {
        return;
    };

    // This should NOT fail with sqlitegraph "unsupported database type" error
    let result = CodeGraph::open(&geo_path);
    assert!(
        result.is_ok(),
        "CodeGraph::open() should succeed on .geo file, got: {:?}",
        result.err()
    );

    let graph = result.unwrap();

    // Verify geometric backend is active
    assert!(
        graph.geometric().is_ok(),
        "geometric() should return Ok when using .geo backend"
    );

    // Verify sqlitegraph backend is NOT active
    assert!(
        graph.inner().is_err(),
        "inner() should return Err when using .geo backend"
    );
}

/// PHASE 3.2: Verify Splice can resolve real symbols from .geo
///
/// Test: splice_can_resolve_real_symbol_from_geo
/// Expected: find_symbol_in_file returns valid NodeId for real symbol
#[test]
fn splice_can_resolve_real_symbol_from_geo() {
    let Some(geo_path) = geo_test_db() else {
        return;
    };

    let graph = CodeGraph::open(&geo_path).expect("Should open .geo file");

    // Try to find a known real symbol from src/graph/mod.rs
    // Using absolute path as indexed by magellan
    let file_path = "/home/feanor/Projects/splice/src/graph/mod.rs";
    let symbol_name = "get_span";

    let node_id = graph.find_symbol_in_file(file_path, symbol_name);

    assert!(
        node_id.is_some(),
        "Should find symbol '{}' in file '{}'",
        symbol_name,
        file_path
    );

    // Verify the NodeId is valid (non-zero)
    let id = node_id.unwrap();
    assert!(id.0 > 0, "NodeId should be positive, got {}", id.0);
}

/// PHASE 3.3: Verify Splice can get real span from .geo
///
/// Test: splice_can_get_real_span_from_geo
/// Expected: get_span returns valid (byte_start, byte_end) for real symbol
#[test]
fn splice_can_get_real_span_from_geo() {
    let Some(geo_path) = geo_test_db() else {
        return;
    };

    let graph = CodeGraph::open(&geo_path).expect("Should open .geo file");

    let file_path = "/home/feanor/Projects/splice/src/graph/mod.rs";
    let symbol_name = "get_span";

    let node_id = graph
        .find_symbol_in_file(file_path, symbol_name)
        .expect("Should find symbol");

    // This is the critical workflow: getting span from geometric backend
    let span_result = graph.get_span(node_id);

    assert!(
        span_result.is_ok(),
        "get_span() should work on geometric backend, got: {:?}",
        span_result.err()
    );

    let (byte_start, byte_end) = span_result.unwrap();

    // Verify span is reasonable (non-zero, end > start)
    assert!(
        byte_start > 0,
        "byte_start should be positive, got {}",
        byte_start
    );
    assert!(
        byte_end > byte_start,
        "byte_end ({}) should be > byte_start ({})",
        byte_end,
        byte_start
    );

    eprintln!(
        "SUCCESS: Symbol '{}' in '{}' has span {}..{}",
        symbol_name, file_path, byte_start, byte_end
    );
}

/// PHASE 3.4: Verify ambiguity handling is explicit
///
/// Test: splice_geo_ambiguity_handling_is_explicit
/// Expected: find_symbol_in_file handles ambiguous names appropriately
#[test]
fn splice_geo_ambiguity_handling_is_explicit() {
    let Some(geo_path) = geo_test_db() else {
        return;
    };

    let graph = CodeGraph::open(&geo_path).expect("Should open .geo file");

    // "open" is highly ambiguous - appears in many files
    // The method should still return a result (first match or specific logic)
    let file_path = "/home/feanor/Projects/splice/src/graph/mod.rs";
    let ambiguous_name = "open";

    let node_id = graph.find_symbol_in_file(file_path, ambiguous_name);

    // Should find the symbol in the specific file (not ambiguous within one file)
    assert!(
        node_id.is_some(),
        "Should find symbol '{}' even if ambiguous across files",
        ambiguous_name
    );

    // Verify we can get span for this too
    let span = graph.get_span(node_id.unwrap());
    assert!(span.is_ok(), "Should get span for ambiguous-named symbol");
}

/// PHASE 3.5: Verify span-safe edit workflow on .geo
///
/// Test: splice_span_safe_edit_workflow_works_on_geo
/// Expected: Can retrieve source content using span from geometric backend
#[test]
fn splice_span_safe_edit_workflow_works_on_geo() {
    let Some(geo_path) = geo_test_db() else {
        return;
    };

    let graph = CodeGraph::open(&geo_path).expect("Should open .geo file");

    let file_path = "/home/feanor/Projects/splice/src/graph/mod.rs";
    let symbol_name = "get_span";

    // Step 1: Find symbol
    let node_id = graph
        .find_symbol_in_file(file_path, symbol_name)
        .expect("Should find symbol");

    // Step 2: Get span
    let (byte_start, byte_end) = graph.get_span(node_id).expect("Should get span");

    // Step 3: Use geometric backend to get code chunk (span-safe retrieval)
    let magellan = graph.geometric().expect("Should have geometric backend");

    let chunk_result = magellan.get_code_chunk_by_span(file_path, byte_start, byte_end);

    assert!(
        chunk_result.is_ok(),
        "get_code_chunk should succeed, got: {:?}",
        chunk_result.err()
    );

    let chunk = chunk_result.unwrap();
    assert!(chunk.is_some(), "Should retrieve code chunk for symbol");

    let code = chunk.unwrap();

    // Verify we got actual code content
    assert!(!code.content.is_empty(), "Code chunk should not be empty");

    // Verify it contains the function signature
    assert!(
        code.content.contains("pub fn get_span") || code.content.contains("fn get_span"),
        "Code chunk should contain function definition, got: {}",
        code.content
    );

    eprintln!(
        "SUCCESS: Retrieved {} bytes of source code for '{}'",
        code.content.len(),
        symbol_name
    );
}

/// PHASE 5: SQLite regression check
///
/// Test: sqlite_backend_still_works
/// Expected: .db files still work after geometric refactoring
#[test]
fn sqlite_backend_still_works() {
    use tempfile::NamedTempFile;

    // Create a temporary .db file using Splice's sqlitegraph backend
    let temp_file = NamedTempFile::new().expect("Should create temp file");
    let db_path = temp_file.path().to_path_buf();
    drop(temp_file);

    // Open with Splice (should use sqlitegraph for .db)
    let result = CodeGraph::open(&db_path);

    // Should succeed
    assert!(
        result.is_ok(),
        "CodeGraph::open() should still work on .db files, got: {:?}",
        result.err()
    );

    let graph = result.unwrap();

    // Should have sqlite backend, not geometric
    assert!(graph.inner().is_ok(), "inner() should work on .db files");

    #[cfg(feature = "geometric")]
    assert!(
        graph.geometric().is_err(),
        "geometric() should return Err for .db files"
    );

    // Cleanup
    let _ = std::fs::remove_file(&db_path);
}
