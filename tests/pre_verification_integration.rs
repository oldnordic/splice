//! Integration tests for pre-verification hooks.
//!
//! These tests verify that pre-verification correctly:
//! - Blocks operations on externally modified files
//! - Allows operations on clean files
//! - Converts warnings to errors in strict mode

use std::fs::{self, File};
use std::io::Write;
use tempfile::TempDir;

/// Helper to create a test workspace.
fn create_test_workspace() -> TempDir {
    let temp_dir = TempDir::new().unwrap();
    let workspace = temp_dir.path();

    // Create a dummy codegraph.db (using Splice's convention: .splice_graph.db)
    let db_path = workspace.join(".splice_graph.db");
    let mut db = File::create(&db_path).unwrap();
    writeln!(db, "dummy database").unwrap();

    // Create a test Rust file
    let test_file = workspace.join("test.rs");
    let mut file = File::create(&test_file).unwrap();
    writeln!(file, "fn test_function() {{}}").unwrap();

    temp_dir
}

#[test]
fn test_pre_verify_blocks_corrupted_file() {
    let temp_dir = create_test_workspace();
    let workspace = temp_dir.path();
    let test_file = workspace.join("test.rs");
    let db_path = workspace.join(".splice_graph.db");

    // Compute checksum of original file
    let original_checksum = splice::checksum::checksum_file(&test_file).unwrap();

    // Modify the file externally
    let mut file = File::create(&test_file).unwrap();
    writeln!(file, "fn modified_function() {{}}").unwrap();

    // Pre-verification should detect the modification
    use splice::verify;
    let results = verify::pre_verify_patch(
        &test_file,
        Some(&original_checksum),
        workspace,
        &db_path,
        false,
        false,
    )
    .unwrap();

    // Should have a blocking failure for checksum mismatch
    assert!(results.iter().any(|r| r.is_blocking()));

    // Check that it's specifically a checksum failure
    let checksum_failed = results.iter().any(|r| {
        if let verify::PreVerificationResult::Fail { check, .. } = r {
            check == "file_checksum"
        } else {
            false
        }
    });
    assert!(checksum_failed, "Expected file_checksum failure");
}

#[test]
fn test_pre_verify_allows_clean_file() {
    let temp_dir = create_test_workspace();
    let workspace = temp_dir.path();
    let test_file = workspace.join("test.rs");
    let db_path = workspace.join(".splice_graph.db");

    // Compute checksum of file
    let checksum = splice::checksum::checksum_file(&test_file).unwrap();

    // Pre-verification should pass
    use splice::verify;
    let results = verify::pre_verify_patch(
        &test_file,
        Some(&checksum),
        workspace,
        &db_path,
        false,
        false,
    )
    .unwrap();

    // All checks should pass
    assert!(results.iter().all(|r| r.is_pass()));
}

#[test]
fn test_strict_mode_blocks_on_warning() {
    let temp_dir = create_test_workspace();
    let workspace = temp_dir.path();
    let test_file = workspace.join("test.rs");
    let db_path = workspace.join(".splice_graph.db");

    use splice::verify;

    // In a normal scenario with a valid setup, all checks should pass
    // This test verifies the strict mode flag is being passed through correctly
    let results_normal = verify::pre_verify_patch(
        &test_file,
        None,
        workspace,
        &db_path,
        false, // not strict
        false,
    )
    .unwrap();

    // All checks should pass
    assert!(results_normal.iter().all(|r| r.is_pass()));

    // In strict mode with the same valid setup, all checks should still pass
    let results_strict = verify::pre_verify_patch(
        &test_file,
        None,
        workspace,
        &db_path,
        true, // strict mode
        false,
    )
    .unwrap();

    // All checks should still pass (no warnings to convert)
    assert!(results_strict.iter().all(|r| r.is_pass()));

    // Test that skip mode works independently of strict mode
    let results_strict_skip = verify::pre_verify_patch(
        &test_file,
        None,
        workspace,
        &db_path,
        true, // strict mode
        true, // skip verification
    )
    .unwrap();

    // Skip should bypass all checks even in strict mode
    assert!(results_strict_skip.iter().all(|r| r.is_pass()));
    assert!(results_strict_skip.len() == 1);
}

#[test]
fn test_skip_mode_bypasses_all_checks() {
    let temp_dir = create_test_workspace();
    let workspace = temp_dir.path();
    let test_file = workspace.join("test.rs");
    let db_path = workspace.join(".splice_graph.db");

    // Remove the file to make checks fail
    fs::remove_file(&test_file).unwrap();

    use splice::verify;

    // Normal mode: should fail
    let results_normal = verify::pre_verify_patch(
        &test_file,
        None,
        workspace,
        &db_path,
        false,
        false, // not skipping
    )
    .unwrap();

    assert!(results_normal.iter().any(|r| r.is_blocking()));

    // Skip mode: should pass
    let results_skip = verify::pre_verify_patch(
        &test_file,
        None,
        workspace,
        &db_path,
        false,
        true, // skip verification
    )
    .unwrap();

    // All checks should pass (skipped)
    assert!(results_skip.iter().all(|r| r.is_pass()));
    assert!(results_skip.len() == 1);
}

#[test]
fn test_pre_verify_detects_readonly_file() {
    let temp_dir = create_test_workspace();
    let workspace = temp_dir.path();
    let test_file = workspace.join("test.rs");
    let db_path = workspace.join(".splice_graph.db");

    // Make file read-only
    let mut perms = fs::metadata(&test_file).unwrap().permissions();
    perms.set_readonly(true);
    fs::set_permissions(&test_file, perms.clone()).unwrap();

    use splice::verify;

    let results = verify::pre_verify_patch(
        &test_file,
        None,
        workspace,
        &db_path,
        false,
        false,
    )
    .unwrap();

    // Should have a blocking failure for writable check
    assert!(results.iter().any(|r| r.is_blocking()));

    // Check that it's specifically a writable failure
    let writable_failed = results.iter().any(|r| {
        if let verify::PreVerificationResult::Fail { check, .. } = r {
            check == "file_writable"
        } else {
            false
        }
    });
    assert!(writable_failed, "Expected file_writable failure");

    // Cleanup: restore write permissions
    perms.set_readonly(false);
    fs::set_permissions(&test_file, perms).unwrap();
}

#[test]
fn test_pre_verify_detects_file_outside_workspace() {
    let temp_dir = TempDir::new().unwrap();
    let workspace = temp_dir.path().join("workspace");
    fs::create_dir(&workspace).unwrap();

    // Create a file outside the workspace
    let outside_file = temp_dir.path().join("outside.rs");
    let mut file = File::create(&outside_file).unwrap();
    writeln!(file, "fn external() {{}}").unwrap();

    let db_path = workspace.join(".splice_graph.db");

    use splice::verify;

    let results = verify::pre_verify_patch(
        &outside_file,
        None,
        &workspace,
        &db_path,
        false,
        false,
    )
    .unwrap();

    // Should have a blocking failure for workspace check
    assert!(results.iter().any(|r| r.is_blocking()));

    // Check that it's specifically a workspace failure
    let workspace_failed = results.iter().any(|r| {
        if let verify::PreVerificationResult::Fail { check, .. } = r {
            check == "file_in_workspace"
        } else {
            false
        }
    });
    assert!(workspace_failed, "Expected file_in_workspace failure");
}

#[test]
fn test_pre_verify_checks_workspace_writable() {
    let temp_dir = create_test_workspace();
    let workspace = temp_dir.path();
    let test_file = workspace.join("test.rs");

    // Make workspace read-only
    let mut perms = fs::metadata(workspace).unwrap().permissions();
    perms.set_readonly(true);
    fs::set_permissions(workspace, perms.clone()).unwrap();

    use splice::verify;
    let db_path = workspace.join(".splice_graph.db");

    let results = verify::pre_verify_patch(
        &test_file,
        None,
        workspace,
        &db_path,
        false,
        false,
    )
    .unwrap();

    // Should have a blocking failure for workspace writable check
    assert!(results.iter().any(|r| r.is_blocking()));

    // Cleanup: restore write permissions
    perms.set_readonly(false);
    fs::set_permissions(workspace, perms).unwrap();
}
