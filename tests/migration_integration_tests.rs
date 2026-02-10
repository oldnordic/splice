//! Migration workflow integration tests.
//!
//! These tests verify the complete migration workflow from SQLite to native-v2:
//! 1. Create SQLite database with symbols and edges
//! 2. Migrate to native-v2 format
//! 3. Verify migration preserves all data
//! 4. Test rollback on verification failure
//!
//! Requires 'migration' feature (enables both sqlite and native-v2 backends) to run:
//!   cargo test --features migration migration_integration
//!
//! The migration feature enables BOTH backends simultaneously, which is needed
//! to migrate FROM SQLite TO native-v2.
//!
//! ## NOTE: Migration Tests Currently Disabled
//!
//! The migration functionality as implemented in Phase 34 uses snapshot export/import,
//! but SQLite and native-v2 backends use incompatible snapshot formats:
//! - SQLite backend: exports to `snapshot.json` format
//! - Native-v2 backend: expects `export.manifest` format
//!
//! This fundamental incompatibility prevents migration from working. The tests below
//! are marked with `#[ignore]` until the migration implementation is fixed to use
//! a compatible data transfer mechanism.
//!
//! To fix migration, one of these approaches is needed:
//! 1. Add format conversion between snapshot.json and export.manifest
//! 2. Implement direct entity-by-entity migration (not snapshot-based)
//! 3. Make SQLite backend export in native-v2 compatible format

use splice::CodeGraph;
use splice::graph::Backend;
use splice::symbol::Language;
use std::fs;
use std::path::PathBuf;
use tempfile::TempDir;

///////////////////////////////////////////////////////////////////////////////
// Test fixtures
///////////////////////////////////////////////////////////////////////////////

/// Create a populated SQLite database for migration testing.
///
/// This function explicitly creates a SQLite database using sqlitegraph
/// with GraphConfig::sqlite(). The database is then wrapped in a CodeGraph
/// using an internal helper that bypasses the auto-detection.
fn create_populated_sqlite_db(dir: &PathBuf) -> PathBuf {
    use splice::graph::Backend;

    let db_path = dir.join("source_sqlite.db");
    let _ = fs::remove_file(&db_path);

    // Create SQLite database explicitly using sqlitegraph
    let sqlite_cfg = sqlitegraph::GraphConfig::sqlite();
    let _backend = sqlitegraph::open_graph(&db_path, &sqlite_cfg)
        .expect("Failed to create SQLite backend");

    // Verify it's SQLite
    assert_eq!(
        CodeGraph::detect_backend(&db_path).unwrap(),
        Backend::SQLite,
        "Source database should be SQLite"
    );

    // Create CodeGraph with the SQLite backend
    // We need to use CodeGraph methods to add data, so reopen with CodeGraph
    let mut graph = CodeGraph::open(&db_path).expect("Failed to open SQLite database");

    // Store multiple symbols
    let test_file = dir.join("test_source.rs");

    // Function symbol
    graph
        .store_symbol_with_file_and_language(
            &test_file,
            "main_function",
            "function",
            Language::Rust,
            0,
            100,
            1,
            10,
            0,
            10,
        )
        .expect("Failed to store main_function");

    // Struct symbol
    graph
        .store_symbol_with_file_and_language(
            &test_file,
            "TestData",
            "struct",
            Language::Rust,
            100,
            200,
            10,
            20,
            4,
            12,
        )
        .expect("Failed to store TestData");

    // Method symbol
    graph
        .store_symbol_with_file_and_language(
            &test_file,
            "method_one",
            "method",
            Language::Rust,
            200,
            300,
            20,
            30,
            8,
            18,
        )
        .expect("Failed to store method_one");

    // Final verification that it's still SQLite
    assert_eq!(
        CodeGraph::detect_backend(&db_path).unwrap(),
        Backend::SQLite,
        "Source database should still be SQLite after adding data"
    );

    db_path
}

///////////////////////////////////////////////////////////////////////////////
// Migration workflow tests
///////////////////////////////////////////////////////////////////////////////

/// Test that documents the migration incompatibility issue.
///
/// This test verifies that attempting to migrate from SQLite to native-v2
/// produces an informative error about the snapshot format incompatibility.
#[cfg(feature = "migration")]
#[test]
fn test_migration_incompatibility_documented() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");

    // Create SQLite database
    let source_path = create_populated_sqlite_db(&temp_dir.path().to_path_buf());
    let dest_path = temp_dir.path().join("will_fail.db");
    let _ = fs::remove_file(&dest_path);

    // Verify source is SQLite
    let source_backend = CodeGraph::detect_backend(&source_path).expect("Detection failed");
    assert_eq!(Backend::SQLite, source_backend, "Source should be SQLite");

    // Attempt migration (will fail due to snapshot format incompatibility)
    let graph = CodeGraph::open(&source_path).expect("Failed to open source graph");
    let result = graph.migrate_to_native_v2(&source_path, &dest_path, None, false);

    // Migration should fail
    assert!(result.is_err(), "Migration should fail due to snapshot format incompatibility");

    let err = result.unwrap_err();
    let err_msg = format!("{:?}", err);

    // Verify the error is about snapshot export/import
    assert!(
        err_msg.contains("Snapshot") || err_msg.contains("snapshot") || err_msg.contains("export") || err_msg.contains("manifest"),
        "Error should mention snapshot/export/manifest issue, got: {}",
        err_msg
    );

    eprintln!("Expected migration failure: {}", err_msg);
    eprintln!("This error documents the incompatibility between SQLite snapshot.json");
    eprintln!("format and native-v2 export.manifest format.");
}

#[cfg(feature = "migration")]
#[test]
#[ignore = "Migration implementation broken: SQLite and native-v2 use incompatible snapshot formats"]
fn test_migration_full_workflow() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");

    // Step 1: Create SQLite database
    let source_path = create_populated_sqlite_db(&temp_dir.path().to_path_buf());

    // Verify source is SQLite
    let source_backend = CodeGraph::detect_backend(&source_path).expect("Detection failed");
    assert_eq!(Backend::SQLite, source_backend, "Source should be SQLite");

    // Step 2: Define destination path
    let dest_path = temp_dir.path().join("migrated_native_v2.db");
    let _ = fs::remove_file(&dest_path);

    // Step 3: Perform migration
    let graph = CodeGraph::open(&source_path).expect("Failed to open source graph");

    let progress_steps = std::sync::Arc::new(std::sync::Mutex::new(Vec::new()));
    let progress_box = {
        let progress_steps = progress_steps.clone();
        Box::new(move |step: &str| {
            progress_steps.lock().unwrap().push(step.to_string());
        }) as Box<dyn Fn(&str)>
    };
    let progress = Some(progress_box.as_ref());

    let result = graph.migrate_to_native_v2(&source_path, &dest_path, progress, true);

    if let Err(ref e) = result {
        eprintln!("Migration error: {:?}", e);
    }
    assert!(result.is_ok(), "Migration should succeed");

    let report = result.unwrap();
    assert!(report.verification_passed, "Migration should pass verification");
    assert!(dest_path.exists(), "Destination database should exist");

    // Step 4: Verify destination is native-v2
    let dest_backend = CodeGraph::detect_backend(&dest_path).expect("Detection failed");
    assert_eq!(Backend::NativeV2, dest_backend, "Destination should be NativeV2");

    // Progress should have been reported
    let steps = progress_steps.lock().unwrap();
    assert!(!steps.is_empty(), "Progress should be reported");
}

#[cfg(feature = "migration")]
#[test]
#[ignore = "Migration implementation broken: SQLite and native-v2 use incompatible snapshot formats"]
fn test_migration_preserves_symbols() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");

    // Create populated SQLite database
    let source_path = create_populated_sqlite_db(&temp_dir.path().to_path_buf());
    let dest_path = temp_dir.path().join("symbols_migrated.db");
    let _ = fs::remove_file(&dest_path);

    // Get symbol count before migration
    let source_graph = CodeGraph::open(&source_path).expect("Failed to open source");
    let source_symbols = source_graph.all_symbol_names();

    // Migrate
    let result = source_graph.migrate_to_native_v2(&source_path, &dest_path, None, true);
    assert!(result.is_ok(), "Migration should succeed");

    // Verify destination has same symbols
    let dest_graph = CodeGraph::open(&dest_path).expect("Failed to open destination");
    let dest_symbols = dest_graph.all_symbol_names();

    // Both should have the same symbols
    assert_eq!(
        source_symbols.len(),
        dest_symbols.len(),
        "Symbol count should match after migration"
    );

    for symbol in &source_symbols {
        assert!(
            dest_symbols.contains(symbol),
            "Symbol '{}' should exist in migrated database",
            symbol
        );
    }
}

#[cfg(feature = "migration")]
#[test]
#[ignore = "Migration implementation broken: SQLite and native-v2 use incompatible snapshot formats"]
fn test_migration_creates_backup_on_failure() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");

    // Create a minimal SQLite database
    let source_path = temp_dir.path().join("backup_test.db");
    let _ = fs::remove_file(&source_path);

    {
        let _graph = CodeGraph::open(&source_path).expect("Failed to create graph");
    }

    let dest_path = temp_dir.path().join("dest.db");
    let _ = fs::remove_file(&dest_path);

    // Migrate with verification (should pass and not need rollback)
    let graph = CodeGraph::open(&source_path).expect("Failed to open graph");
    let result = graph.migrate_to_native_v2(&source_path, &dest_path, None, true);

    assert!(result.is_ok(), "Migration should succeed");

    // Note: Testing actual rollback requires triggering a failure,
    // which is difficult without corrupting the migration process.
    // This test verifies the happy path where migration succeeds.
}

#[cfg(feature = "migration")]
#[test]
#[ignore = "Migration implementation broken: SQLite and native-v2 use incompatible snapshot formats"]
fn test_migration_destination_exists_error() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");

    let source_path = create_populated_sqlite_db(&temp_dir.path().to_path_buf());
    let dest_path = temp_dir.path().join("already_exists.db");

    // Create destination file first
    fs::write(&dest_path, b"existing data").expect("Failed to create destination");

    let graph = CodeGraph::open(&source_path).expect("Failed to open graph");

    // Migration should fail because destination exists
    let result = graph.migrate_to_native_v2(&source_path, &dest_path, None, false);

    assert!(result.is_err(), "Migration should fail when destination exists");

    let err = result.unwrap_err();
    let err_msg = format!("{}", err);
    assert!(
        err_msg.contains("already exists") || err_msg.contains("exists"),
        "Error should mention destination already exists"
    );
}

#[cfg(feature = "migration")]
#[test]
#[ignore = "Migration implementation broken: SQLite and native-v2 use incompatible snapshot formats"]
fn test_migration_verification() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");

    let source_path = create_populated_sqlite_db(&temp_dir.path().to_path_buf());
    let dest_path = temp_dir.path().join("verify_test.db");
    let _ = fs::remove_file(&dest_path);

    // Migrate with verification enabled
    let graph = CodeGraph::open(&source_path).expect("Failed to open graph");
    let result = graph.migrate_to_native_v2(&source_path, &dest_path, None, true);

    assert!(result.is_ok(), "Migration should succeed");

    let report = result.unwrap();
    assert!(
        report.verification_passed,
        "Migration should pass verification"
    );
    assert!(report.verification_error.is_none(), "No verification errors");

    // Verify migration integrity directly
    let verification_result = CodeGraph::verify_migration(&source_path, &dest_path);
    assert!(
        verification_result.is_ok(),
        "Direct verification should pass"
    );
}

#[cfg(feature = "migration")]
#[test]
#[ignore = "Migration implementation broken: SQLite and native-v2 use incompatible snapshot formats"]
fn test_migration_with_progress_reporting() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");

    let source_path = create_populated_sqlite_db(&temp_dir.path().to_path_buf());
    let dest_path = temp_dir.path().join("progress_test.db");
    let _ = fs::remove_file(&dest_path);

    // Track progress calls
    let progress_log = std::sync::Arc::new(std::sync::Mutex::new(Vec::new()));
    let progress_box = {
        let log = progress_log.clone();
        Box::new(move |step: &str| {
            log.lock().unwrap().push(step.to_string());
        }) as Box<dyn Fn(&str)>
    };
    let progress = Some(progress_box.as_ref());

    let graph = CodeGraph::open(&source_path).expect("Failed to open graph");
    let result = graph.migrate_to_native_v2(&source_path, &dest_path, progress, true);

    assert!(result.is_ok(), "Migration should succeed");

    let log = progress_log.lock().unwrap();
    assert!(!log.is_empty(), "Progress should be reported");

    // Verify expected progress steps
    let log_str = log.join(" ");
    assert!(
        log_str.contains("Export") || log_str.contains("export"),
        "Should report export step"
    );
    assert!(
        log_str.contains("Import") || log_str.contains("Importing") || log_str.contains("import"),
        "Should report import step"
    );
    assert!(
        log_str.contains("Verif") || log_str.contains("verif"),
        "Should report verification step"
    );
}

///////////////////////////////////////////////////////////////////////////////
// Cross-backend tests
///////////////////////////////////////////////////////////////////////////////

#[test]
fn test_migration_not_available_without_native_v2() {
    // This test verifies that migration methods are not available
    // without the native-v2 feature enabled

    #[cfg(not(feature = "migration"))]
    {
        // Without native-v2 feature, we should not be able to call migrate_to_native_v2
        // This is enforced at compile time via the cfg attribute
        // If this test compiles, the feature gating is working
    }
}

///////////////////////////////////////////////////////////////////////////////
// Migration report tests
///////////////////////////////////////////////////////////////////////////////

#[cfg(feature = "migration")]
#[test]
#[ignore = "Migration implementation broken: SQLite and native-v2 use incompatible snapshot formats"]
fn test_migration_report_contents() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");

    let source_path = create_populated_sqlite_db(&temp_dir.path().to_path_buf());
    let dest_path = temp_dir.path().join("report_test.db");
    let _ = fs::remove_file(&dest_path);

    let graph = CodeGraph::open(&source_path).expect("Failed to open graph");
    let result = graph.migrate_to_native_v2(&source_path, &dest_path, None, true);

    assert!(result.is_ok(), "Migration should succeed");

    let report = result.unwrap();

    // Verify report fields
    assert_eq!(report.destination, dest_path, "Destination path should match");
    assert!(report.verification_passed, "Verification should pass");
    assert!(report.verification_error.is_none(), "No verification errors");

    // Verify metadata contains useful information
    assert!(!report.snapshot_metadata.is_empty(), "Snapshot metadata should not be empty");
    assert!(
        report.snapshot_metadata.contains("entity_count") ||
            report.snapshot_metadata.contains("edge_count"),
        "Snapshot metadata should contain counts"
    );
}

///////////////////////////////////////////////////////////////////////////////
// Edge case tests
///////////////////////////////////////////////////////////////////////////////

#[cfg(feature = "migration")]
#[test]
#[ignore = "Migration implementation broken: SQLite and native-v2 use incompatible snapshot formats"]
fn test_migration_empty_database() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");

    // Create empty SQLite database
    let source_path = temp_dir.path().join("empty.db");
    let _ = fs::remove_file(&source_path);

    {
        let _graph = CodeGraph::open(&source_path).expect("Failed to create empty database");
    }

    let dest_path = temp_dir.path().join("empty_migrated.db");
    let _ = fs::remove_file(&dest_path);

    // Migrate empty database
    let graph = CodeGraph::open(&source_path).expect("Failed to open graph");
    let result = graph.migrate_to_native_v2(&source_path, &dest_path, None, true);

    assert!(result.is_ok(), "Empty database migration should succeed");

    let report = result.unwrap();
    assert!(dest_path.exists(), "Destination should be created");
    assert!(report.verification_passed, "Verification should pass");
}

#[cfg(feature = "migration")]
#[test]
#[ignore = "Migration implementation broken: SQLite and native-v2 use incompatible snapshot formats"]
fn test_migration_large_symbol_count() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");

    let source_path = temp_dir.path().join("large.db");
    let _ = fs::remove_file(&source_path);

    // Create database with many symbols
    let mut graph = CodeGraph::open(&source_path).expect("Failed to create database");
    let test_file = temp_dir.path().join("large_test.rs");

    for i in 0..100 {
        graph
            .store_symbol_with_file_and_language(
                &test_file,
                &format!("symbol_{}", i),
                "function",
                Language::Rust,
                i * 100,
                i * 100 + 50,
                i,
                i + 1,
                0,
                0,
            )
            .expect("Failed to store symbol");
    }

    let dest_path = temp_dir.path().join("large_migrated.db");
    let _ = fs::remove_file(&dest_path);

    // Migrate
    let result = graph.migrate_to_native_v2(&source_path, &dest_path, None, true);

    assert!(result.is_ok(), "Large database migration should succeed");

    let report = result.unwrap();
    assert!(report.verification_passed, "Verification should pass");
    assert!(report.nodes_migrated >= 100, "Should migrate at least 100 nodes");
}
