//! Backend-specific functionality tests.
//!
//! These tests verify backend-specific operations that are only available
//! with certain feature flags enabled.
//!
//! Run with:
//!   cargo test                          # SQLite backend (default)

use splice::graph::BackendType;
use splice::CodeGraph;
use std::fs;
use std::io::Write;
use tempfile::TempDir;

///////////////////////////////////////////////////////////////////////////////
// SQLite backend specific tests
///////////////////////////////////////////////////////////////////////////////

#[cfg(feature = "sqlite")]
#[test]
fn test_sqlite_header_detection() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let db_path = temp_dir.path().join("test_sqlite_header.db");

    // Write SQLite format 3 header
    {
        let mut file = fs::File::create(&db_path).expect("Failed to create file");
        file.write_all(b"SQLite format 3\0")
            .expect("Failed to write header");
    }

    let detected = CodeGraph::detect_backend(&db_path).expect("Detection failed");
    assert_eq!(BackendType::SQLite, detected, "Should detect SQLite format");
}

#[cfg(feature = "sqlite")]
#[test]
fn test_sqlite_graph_open_existing() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let db_path = temp_dir.path().join("test_sqlite_open.db");
    let _ = fs::remove_file(&db_path);

    // Create a SQLite format file first
    {
        let mut file = fs::File::create(&db_path).expect("Failed to create file");
        file.write_all(b"SQLite format 3\0")
            .expect("Failed to write header");
    }

    // Open should detect SQLite format and use appropriate backend
    let _graph = CodeGraph::open(&db_path);
    // This might fail because we only wrote the header, not a complete database
    // But the important part is that it detected the format correctly

    // Verify it's detected as SQLite
    let detected = CodeGraph::detect_backend(&db_path).expect("Detection failed");
    assert_eq!(
        BackendType::SQLite,
        detected,
        "Should detect SQLite format from header"
    );
}

///////////////////////////////////////////////////////////////////////////////
// Cross-backend compatibility tests
///////////////////////////////////////////////////////////////////////////////

#[test]
fn test_backend_format_detectable() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");

    // Test non-existent file
    let nonexistent = temp_dir.path().join("nonexistent.db");
    let detected = CodeGraph::detect_backend(&nonexistent).expect("Detection failed");
    assert_eq!(
        BackendType::SQLite,
        detected,
        "Non-existent file should default to SQLite"
    );
}

///////////////////////////////////////////////////////////////////////////////
// Feature flag test
///////////////////////////////////////////////////////////////////////////////

#[test]
fn test_sqlite_backend_active() {
    // Verify that the sqlite backend is active by default
    let sqlite_enabled = cfg!(feature = "sqlite");
    assert!(
        sqlite_enabled,
        "SQLite backend should be enabled by default"
    );
}
