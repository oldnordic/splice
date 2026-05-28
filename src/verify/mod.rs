//! Pre and post-verification hooks for safe refactoring operations.
//!
//! Pre-verification runs BEFORE any file modifications to:
//! - Validate file state (unchanged, writable, readable)
//! - Verify workspace conditions (disk space, permissions)
//! - Check graph database synchronization
//! - Detect external modifications (via checksums)
//!
//! Post-verification runs AFTER file modifications to:
//! - Validate syntax (tree-sitter reparse)
//! - Validate compilation (language-specific)
//! - Verify semantic preservation
//! - Check for unintended side effects
//! - Compare checksums to document actual changes

pub mod post;
pub mod pre;

pub use post::*;
pub use pre::*;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::checksum::Checksum;
    use std::fs::{self, File};
    use std::io::Write;
    use tempfile::TempDir;

    #[test]
    fn test_verify_file_ready_pass() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("test.rs");
        let mut file = File::create(&file_path).unwrap();
        writeln!(file, "fn test() {{}}").unwrap();

        let result = verify_file_ready(&file_path, None, temp_dir.path());
        assert!(result.is_pass());
    }

    #[test]
    fn test_verify_file_not_found() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("nonexistent.rs");

        let result = verify_file_ready(&file_path, None, temp_dir.path());
        assert!(result.is_blocking());
        assert!(
            matches!(result, PreVerificationResult::Fail { check, .. } if check == "file_exists")
        );
    }

    #[test]
    fn test_verify_file_not_writable() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("readonly.rs");
        let mut file = File::create(&file_path).unwrap();
        writeln!(file, "fn test() {{}}").unwrap();

        // Make file read-only
        let mut perms = fs::metadata(&file_path).unwrap().permissions();
        perms.set_readonly(true);
        fs::set_permissions(&file_path, perms.clone()).unwrap();

        let result = verify_file_ready(&file_path, None, temp_dir.path());
        assert!(result.is_blocking());
        assert!(
            matches!(result, PreVerificationResult::Fail { check, .. } if check == "file_writable")
        );

        // Cleanup: restore write permissions
        #[allow(
            clippy::permissions_set_readonly_false,
            reason = "test cleanup: restore write so TempDir can clean up"
        )]
        perms.set_readonly(false);
        fs::set_permissions(&file_path, perms).unwrap();
    }

    #[test]
    fn test_verify_checksum_mismatch() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("test.rs");
        let mut file = File::create(&file_path).unwrap();
        writeln!(file, "fn test() {{}}").unwrap();

        // Create a checksum that won't match
        let wrong_checksum = Checksum::new(
            "wrong".to_string(),
            crate::checksum::ChecksumAlgorithm::Sha256,
            100,
        );

        let result = verify_file_ready(&file_path, Some(&wrong_checksum), temp_dir.path());
        assert!(result.is_blocking());
        assert!(
            matches!(result, PreVerificationResult::Fail { check, .. } if check == "file_checksum")
        );
    }

    #[test]
    fn test_verify_workspace_resources_pass() {
        let temp_dir = TempDir::new().unwrap();

        let result = verify_workspace_resources(temp_dir.path(), 1000);
        assert!(result.is_pass());
    }

    #[test]
    fn test_verify_workspace_not_writable() {
        let temp_dir = TempDir::new().unwrap();
        let readonly_dir = temp_dir.path().join("readonly");
        fs::create_dir(&readonly_dir).unwrap();

        // Make directory read-only
        let mut perms = fs::metadata(&readonly_dir).unwrap().permissions();
        perms.set_readonly(true);
        fs::set_permissions(&readonly_dir, perms.clone()).unwrap();

        let result = verify_workspace_resources(&readonly_dir, 1000);
        assert!(result.is_blocking());

        // Cleanup: restore write permissions
        #[allow(
            clippy::permissions_set_readonly_false,
            reason = "test cleanup: restore write so TempDir can clean up"
        )]
        perms.set_readonly(false);
        fs::set_permissions(&readonly_dir, perms).unwrap();
    }

    #[test]
    fn test_pre_verify_all_pass() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("test.rs");
        let mut file = File::create(&file_path).unwrap();
        writeln!(file, "fn test() {{}}").unwrap();

        let db_path = temp_dir.path().join("codegraph.db");
        let mut db = File::create(&db_path).unwrap();
        writeln!(db, "dummy db").unwrap();

        let results =
            pre_verify_patch(&file_path, None, temp_dir.path(), &db_path, false, false).unwrap();
        assert!(results.len() == 3);
        assert!(results.iter().all(|r| r.is_pass()));
    }

    #[test]
    fn test_pre_verify_blocking_failure() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("nonexistent.rs");
        let db_path = temp_dir.path().join("codegraph.db");

        let results =
            pre_verify_patch(&file_path, None, temp_dir.path(), &db_path, false, false).unwrap();
        assert!(results.iter().any(|r| r.is_blocking()));
    }

    #[test]
    fn test_pre_verify_skip_mode() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("nonexistent.rs");
        let db_path = temp_dir.path().join("codegraph.db");

        // Skip mode should pass even though file doesn't exist
        let results =
            pre_verify_patch(&file_path, None, temp_dir.path(), &db_path, false, true).unwrap();
        assert!(results.len() == 1);
        assert!(results.iter().all(|r| r.is_pass()));
    }

    #[test]
    fn test_file_outside_workspace() {
        let temp_dir = TempDir::new().unwrap();
        let workspace = temp_dir.path().join("workspace");
        fs::create_dir(&workspace).unwrap();

        let outside_file = temp_dir.path().join("outside.rs");
        File::create(&outside_file).unwrap();

        let result = verify_file_ready(&outside_file, None, &workspace);
        assert!(result.is_blocking());
        assert!(
            matches!(result, PreVerificationResult::Fail { check, .. } if check == "file_in_workspace")
        );
    }

    #[test]
    fn test_verify_result_methods() {
        let pass = PreVerificationResult::pass();
        assert!(pass.is_pass());
        assert!(!pass.is_blocking());
        assert!(!pass.is_warning());

        let blocking = PreVerificationResult::blocking("test", "failed");
        assert!(!blocking.is_pass());
        assert!(blocking.is_blocking());
        assert!(!blocking.is_warning());

        let warning = PreVerificationResult::warning("test", "warning");
        assert!(!warning.is_pass());
        assert!(!warning.is_blocking());
        assert!(warning.is_warning());
    }

    // Post-verification tests

    #[test]
    fn test_verify_localized_change_pass() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("test.rs");
        let replaced = b"fn test() {\n    let x = 1;\n}";
        std::fs::write(&file_path, replaced).unwrap();

        // Modify only within the target span
        let target_span = (10, 20); // Within "let x = 1;"
        let modified = b"fn test() {\n    let y = 2;\n}";
        std::fs::write(&file_path, modified).unwrap();

        // Check should fail because we changed bytes outside target span
        // (we changed the whole file, not just the span)
        let result = verify_localized_change(&file_path, replaced, target_span);
        assert!(result.is_ok());
        // This should be false because we changed more than just the span
        assert!(!result.unwrap());
    }

    #[test]
    fn test_verify_localized_change_fail() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("test.rs");
        let replaced = b"fn test() {\n    let x = 1;\n}";
        std::fs::write(&file_path, replaced).unwrap();

        // Modify bytes before target span
        let modified = b"fn modified() {\n    let x = 1;\n}";
        std::fs::write(&file_path, modified).unwrap();

        let target_span = (20, 30); // After "fn test() {"
        let result = verify_localized_change(&file_path, replaced, target_span);
        assert!(result.is_ok());
        assert!(!result.unwrap()); // Should detect change before span
    }

    #[test]
    fn test_checksum_diff_changed() {
        let before = "abc123";
        let after = "def456";
        let diff = checksum_diff(before, after);
        assert!(diff.changed);
    }

    #[test]
    fn test_checksum_diff_unchanged() {
        let before = "abc123";
        let after = "abc123";
        let diff = checksum_diff(before, after);
        assert!(!diff.changed);
    }

    #[test]
    fn test_post_verify_all_pass() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("test.rs");
        std::fs::write(&file_path, b"fn test() {}").unwrap();

        let before_checksum = "some_checksum";
        let result = verify_after_patch(
            &file_path,
            temp_dir.path(),
            before_checksum,
            crate::validate::AnalyzerMode::Off,
        );
        assert!(result.is_ok());
        let verify_result = result.unwrap();
        assert!(verify_result.syntax_ok);
        assert!(verify_result.compiler_ok);
        assert!(verify_result.semantic_ok);
        assert!(verify_result.file_changed()); // Different checksum
    }

    #[test]
    fn test_post_verify_result_methods() {
        let mut result =
            PostVerificationResult::new(true, true, "before".to_string(), "after".to_string());

        assert!(result.syntax_ok);
        assert!(result.compiler_ok);
        assert!(result.semantic_ok);
        assert!(result.file_changed());
        assert!(result.warnings.is_empty());
        assert!(result.errors.is_empty());

        result.add_warning("test warning");
        assert_eq!(result.warnings.len(), 1);
        assert_eq!(result.warnings[0], "test warning");

        result.add_error("test error");
        assert_eq!(result.errors.len(), 1);
        assert_eq!(result.errors[0], "test error");
    }

    // TDD test for actual disk space checking
    #[test]
    fn test_get_disk_space_returns_actual_values() {
        let temp_dir = TempDir::new().unwrap();

        // Test that get_disk_space returns actual values, not hardcoded 1TB stub
        let result = get_disk_space(temp_dir.path());

        assert!(result.is_ok(), "get_disk_space should succeed");
        let (available, total) = result.unwrap();

        // Actual disk space should be non-zero
        assert!(available > 0, "Available space should be greater than 0");
        assert!(total > 0, "Total space should be greater than 0");

        // Available should be <= total
        assert!(available <= total, "Available should not exceed total");

        // CRITICAL: The stub returns exactly 1_000_000_000_000 (1TB)
        // This should NOT match actual disk space
        // If available is exactly 1TB, it's the stub
        assert_ne!(
            available, 1_000_000_000_000,
            "get_disk_space returned stub value (1TB). Implement actual disk space checking!"
        );

        // Also check total isn't the stub value
        assert_ne!(
            total,
            1_000_000_000_000,
            "get_disk_space returned stub value (1TB) for total. Implement actual disk space checking!"
        );
    }

    #[test]
    fn test_get_disk_space_on_tmp_is_not_stub() {
        use std::path::Path;

        // Test on /tmp which should have actual disk space, not 1TB stub
        let result = get_disk_space(Path::new("/tmp"));

        assert!(result.is_ok(), "get_disk_space on /tmp should succeed");
        let (available, total) = result.unwrap();

        // Should have some space
        assert!(available > 0, "Available should be greater than 0");
        assert!(total > 0, "Total should be greater than 0");

        // /tmp on this system has ~30GB, NOT 1TB
        // The stub returns 1TB which is wrong
        assert_ne!(
            available, 1_000_000_000_000,
            "get_disk_space on /tmp returned stub value (1TB). /tmp typically has less space."
        );
    }
}
