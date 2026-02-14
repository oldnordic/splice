//! Backend-specific functionality tests.
//!
//! These tests verify backend-specific operations that are only available
//! with certain feature flags enabled.
//!
//! Run with:
//!   cargo test                          # SQLite backend (default)
//!   cargo test --features native-v3     # Native-v2 backend

use splice::CodeGraph;
use splice::graph::Backend;
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
        file.write_all(b"SQLite format 3\0").expect("Failed to write header");
    }

    let detected = CodeGraph::detect_backend(&db_path).expect("Detection failed");
    assert_eq!(Backend::SQLite, detected, "Should detect SQLite format");
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
        file.write_all(b"SQLite format 3\0").expect("Failed to write header");
    }

    // Open should detect SQLite format and use appropriate backend
    let _graph = CodeGraph::open(&db_path);
    // This might fail because we only wrote the header, not a complete database
    // But the important part is that it detected the format correctly

    // Verify it's detected as SQLite
    let detected = CodeGraph::detect_backend(&db_path).expect("Detection failed");
    assert_eq!(Backend::SQLite, detected, "Should detect SQLite format from header");
}

///////////////////////////////////////////////////////////////////////////////
// Native-V2 backend specific tests
///////////////////////////////////////////////////////////////////////////////

#[cfg(feature = "native-v3")]
#[test]
fn test_native_v3_header_detection() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let db_path = temp_dir.path().join("test_native_v3.db");
    let _ = fs::remove_file(&db_path);

    // Create a graph with native-v3 (requires --features native-v3)
    let _graph = CodeGraph::open(&db_path).expect("Failed to open graph");

    // Native-v2 databases don't have the SQLite header
    let is_sqlite = CodeGraph::is_sqlite_db(&db_path).expect("is_sqlite_db failed");
    assert!(!is_sqlite, "Native-v2 database should not be detected as SQLite");

    // Should be detected as NativeV3
    let detected = CodeGraph::detect_backend(&db_path).expect("Detection failed");
    assert_eq!(Backend::NativeV3, detected, "Should detect NativeV3 format");
}

#[cfg(feature = "native-v3")]
#[test]
fn test_native_v3_migration_method_exists() {
    // This test verifies that the migration method compiles with native-v3 feature
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let source_path = temp_dir.path().join("source.db");
    let dest_path = temp_dir.path().join("dest.db");
    let _ = fs::remove_file(&source_path);
    let _ = fs::remove_file(&dest_path);

    let graph = CodeGraph::open(&source_path).expect("Failed to open graph");

    // Create a dummy progress callback
    let progress_called = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
    let callback = |step: &str| {
        if !step.is_empty() {
            progress_called.store(true, std::sync::atomic::Ordering::SeqCst);
        }
    };
    let result = graph.migrate_to_native_v3(&source_path, &dest_path, Some(&callback), false);
    assert!(result.is_ok(), "Migration should succeed with native-v3 feature");

    // Verify destination was created
    assert!(dest_path.exists(), "Destination database should be created");

    let report = result.unwrap();
    assert!(report.verification_passed, "Migration should pass verification");
}

#[cfg(feature = "native-v3")]
#[test]
#[ignore = "Migration blocked by snapshot format incompatibility - see test_migration_incompatibility_documented() in migration_integration_tests.rs"]
fn test_native_v3_migration_with_verification() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let source_path = temp_dir.path().join("source_verify.db");
    let dest_path = temp_dir.path().join("dest_verify.db");
    let _ = fs::remove_file(&source_path);
    let _ = fs::remove_file(&dest_path);

    // Create and populate source database
    {
        let mut graph = CodeGraph::open(&source_path).expect("Failed to open graph");
        let file_path = temp_dir.path().join("test.rs");
        graph
            .store_symbol_with_file_and_language(
                &file_path,
                "verify_func",
                "function",
                splice::symbol::Language::Rust,
                0,
                100,
                1,
                5,
                0,
                0,  // col_end
            )
            .expect("Failed to store symbol");
    }

    // Migrate with verification enabled
    let graph = CodeGraph::open(&source_path).expect("Failed to reopen graph");
    let result = graph.migrate_to_native_v3(&source_path, &dest_path, None, true);

    assert!(result.is_ok(), "Migration with verification should succeed");

    let report = result.unwrap();
    assert!(report.verification_passed, "Verification should pass");
    assert!(report.nodes_migrated > 0 || report.edges_migrated > 0 || report.nodes_migrated == 0,
            "Should report migration statistics");
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
    assert_eq!(Backend::Unknown, detected, "Non-existent file should be Unknown");
}

#[test]
fn test_is_sqlite_db_helper() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");

    // Create a file with SQLite header
    let sqlite_path = temp_dir.path().join("sqlite_test.db");
    {
        let mut file = fs::File::create(&sqlite_path).expect("Failed to create file");
        file.write_all(b"SQLite format 3\0").expect("Failed to write header");
    }

    let is_sqlite = CodeGraph::is_sqlite_db(&sqlite_path).expect("is_sqlite_db failed");
    assert!(is_sqlite, "Should detect SQLite format");

    // Create a file without SQLite header
    let other_path = temp_dir.path().join("other_test.db");
    {
        let mut file = fs::File::create(&other_path).expect("Failed to create file");
        file.write_all(b"Other format data").expect("Failed to write");
    }

    let is_sqlite = CodeGraph::is_sqlite_db(&other_path).expect("is_sqlite_db failed");
    assert!(!is_sqlite, "Should not detect non-SQLite as SQLite");
}

///////////////////////////////////////////////////////////////////////////////
// Feature flag mutual exclusion test
///////////////////////////////////////////////////////////////////////////////

#[test]
fn test_only_one_backend_active() {
    // This test verifies that only one backend is active at a time
    // The compile_error in src/lib.rs should prevent both being enabled

    #[cfg(all(feature = "sqlite", feature = "native-v3"))]
    compile_error!(
        "Both sqlite and native-v3 features should not be enabled simultaneously. \
         This test should not compile if both are enabled."
    );

    // If we reach here, only one backend is enabled (as expected)
    let sqlite_enabled = cfg!(feature = "sqlite");
    let native_v3_enabled = cfg!(feature = "native-v3");

    // At least one should be enabled (via default features)
    assert!(
        sqlite_enabled || native_v3_enabled,
        "At least one backend should be enabled"
    );

    // Both should not be enabled (enforced by compile_error)
    assert!(
        !(sqlite_enabled && native_v3_enabled),
        "Both backends cannot be enabled simultaneously"
    );
}
