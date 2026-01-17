//! End-to-end refactoring workflow tests.
//!
//! These tests validate complete refactoring workflows including:
//! - Patch operations with validation and rollback
//! - Delete operations with reference finding
//! - Multi-step plan execution
//! - Batch operations with atomicity
//! - Pattern-based apply-files operations
//!
//! All tests use real CLI invocation via std::process::Command.

#[cfg(test)]
mod tests {
    use sha2::{Digest, Sha256};
    use std::fs;
    use std::path::{Path, PathBuf};
    use std::process::{Command, Output};
    use tempfile::TempDir;
    #[cfg(unix)]
    use std::os::unix::fs::PermissionsExt;

    /// Create a minimal Rust workspace with Cargo.toml and src/lib.rs
    ///
    /// Returns a temporary directory that will be cleaned up when dropped.
    fn create_rust_workspace() -> TempDir {
        let workspace_dir = TempDir::new().expect("Failed to create temp workspace");
        let workspace_path = workspace_dir.path();

        // Create Cargo.toml
        let cargo_toml = r#"[package]
name = "test-workspace"
version = "0.1.0"
edition = "2021"

[lib]
name = "test_workspace"
path = "src/lib.rs"
"#;
        fs::write(workspace_path.join("Cargo.toml"), cargo_toml)
            .expect("Failed to write Cargo.toml");

        // Create src directory
        let src_dir = workspace_path.join("src");
        fs::create_dir(&src_dir).expect("Failed to create src directory");

        // Create minimal src/lib.rs
        let lib_rs = r#"//! Test library

pub fn greet(name: &str) -> String {
    format!("Hello, {}!", name)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_greet() {
        assert_eq!(greet("World"), "Hello, World!");
    }
}
"#;
        fs::write(src_dir.join("lib.rs"), lib_rs).expect("Failed to write lib.rs");

        workspace_dir
    }

    /// Create a workspace with multiple files for cross-file tests
    ///
    /// Creates:
    /// - src/lib.rs with public function
    /// - src/a.rs with another function
    /// - src/b.rs that calls functions from both
    fn create_multi_file_workspace() -> TempDir {
        let workspace_dir = TempDir::new().expect("Failed to create temp workspace");
        let workspace_path = workspace_dir.path();

        // Create Cargo.toml
        let cargo_toml = r#"[package]
name = "multi-file-test"
version = "0.1.0"
edition = "2021"

[lib]
name = "multi_file_test"
path = "src/lib.rs"
"#;
        fs::write(workspace_path.join("Cargo.toml"), cargo_toml)
            .expect("Failed to write Cargo.toml");

        // Create src directory
        let src_dir = workspace_path.join("src");
        fs::create_dir(&src_dir).expect("Failed to create src directory");

        // Create src/lib.rs
        let lib_rs = r#"//! Multi-file test library

mod a;
mod b;

pub use a::helper_function;

pub fn main_function() -> String {
    format!("Main: {}", b::caller_function())
}
"#;
        fs::write(src_dir.join("lib.rs"), lib_rs).expect("Failed to write lib.rs");

        // Create src/a.rs
        let a_rs = r#"//! Module A

pub fn helper_function() -> String {
    String::from("Helper A")
}
"#;
        fs::write(src_dir.join("a.rs"), a_rs).expect("Failed to write a.rs");

        // Create src/b.rs
        let b_rs = r#"//! Module B

use crate::a::helper_function;

pub fn caller_function() -> String {
    format!("Caller: {}", helper_function())
}
"#;
        fs::write(src_dir.join("b.rs"), b_rs).expect("Failed to write b.rs");

        workspace_dir
    }

    /// Get the path to the splice binary (cross-platform)
    ///
    /// Looks for:
    /// 1. SPLICE_TEST_BIN environment variable
    /// 2. CARGO_BIN_EXE_splice environment variable
    /// 3. Finds newest splice-* binary in target/debug/deps/
    fn get_splice_binary() -> PathBuf {
        if let Ok(path) = std::env::var("SPLICE_TEST_BIN") {
            return PathBuf::from(path);
        }

        if let Ok(path) = std::env::var("CARGO_BIN_EXE_splice") {
            return PathBuf::from(path);
        }

        // During testing, use cargo to build/run the binary
        let mut path = std::env::current_exe().unwrap();
        // This test binary is in target/debug/deps/
        // The splice binary is in target/debug/
        path.pop(); // deps
        let deps_dir = path.clone();
        path.pop(); // debug
        let bin_path = path.join("splice");

        let bin_mtime = std::fs::metadata(&bin_path)
            .and_then(|meta| meta.modified())
            .ok();

        let mut newest: Option<(std::time::SystemTime, PathBuf)> = None;
        if let Ok(entries) = std::fs::read_dir(deps_dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                let name = path.file_name().and_then(|s| s.to_str()).unwrap_or("");
                if !name.starts_with("splice-") || !path.is_file() {
                    continue;
                }

                if let Ok(metadata) = entry.metadata() {
                    #[cfg(unix)]
                    let is_executable = metadata.permissions().mode() & 0o111 != 0;
                    #[cfg(not(unix))]
                    let is_executable = true;

                    if !is_executable {
                        continue;
                    }

                    if let Ok(modified) = metadata.modified() {
                        if newest
                            .as_ref()
                            .map(|(time, _)| modified > *time)
                            .unwrap_or(true)
                        {
                            newest = Some((modified, path));
                        }
                    }
                }
            }
        }

        match (bin_mtime, newest) {
            (Some(bin_time), Some((deps_time, deps_path))) => {
                if deps_time > bin_time {
                    deps_path
                } else {
                    bin_path
                }
            }
            (None, Some((_, deps_path))) => deps_path,
            _ => bin_path,
        }
    }

    /// Run splice CLI and return output
    ///
    /// # Arguments
    /// * `args` - Command-line arguments to pass to splice
    /// * `workspace` - Working directory for the command
    fn run_splice(args: &[&str], workspace: &Path) -> Output {
        let binary = get_splice_binary();
        Command::new(binary)
            .args(args)
            .current_dir(workspace)
            .output()
            .expect("Failed to execute splice command")
    }

    /// Verify file content matches expected string
    ///
    /// # Panics
    /// Panics if file content doesn't match expected
    fn assert_file_content(path: &Path, expected: &str) {
        let actual = fs::read_to_string(path)
            .unwrap_or_else(|e| panic!("Failed to read file {}: {}", path.display(), e));
        assert_eq!(
            actual, expected,
            "File content mismatch\nExpected:\n{}\n\nActual:\n{}",
            expected, actual
        );
    }

    /// Verify file unchanged from original content
    ///
    /// # Panics
    /// Panics if file content has changed
    fn assert_file_unchanged(path: &Path, original: &str) {
        let actual = fs::read_to_string(path)
            .unwrap_or_else(|e| panic!("Failed to read file {}: {}", path.display(), e));
        assert_eq!(
            actual, original,
            "File was modified unexpectedly\nExpected (original):\n{}\n\nActual:\n{}",
            original, actual
        );
    }

    /// Compute SHA-256 checksum of file
    ///
    /// Returns hexadecimal string of checksum
    fn file_checksum(path: &Path) -> String {
        let content = fs::read(path)
            .unwrap_or_else(|e| panic!("Failed to read file {}: {}", path.display(), e));
        let mut hasher = Sha256::new();
        hasher.update(&content);
        format!("{:x}", hasher.finalize())
    }

    // ============================================================================
    // Fixture and Helper Tests (Task 1)
    // ============================================================================

    #[test]
    fn test_workspace_creation() {
        let workspace = create_rust_workspace();
        let workspace_path = workspace.path();

        // Verify Cargo.toml exists
        assert!(
            workspace_path.join("Cargo.toml").exists(),
            "Cargo.toml should exist"
        );

        // Verify src/lib.rs exists
        assert!(
            workspace_path.join("src/lib.rs").exists(),
            "src/lib.rs should exist"
        );

        // Verify cargo check succeeds
        let output = Command::new("cargo")
            .args(["check", "--quiet"])
            .current_dir(workspace_path)
            .output();

        assert!(
            output.is_ok(),
            "cargo check should succeed in workspace"
        );

        let output = output.unwrap();
        assert!(
            output.status.success(),
            "cargo check failed: {}",
            String::from_utf8_lossy(&output.stderr)
        );
    }

    #[test]
    fn test_splice_binary_exists() {
        let binary = get_splice_binary();
        assert!(
            binary.exists(),
            "splice binary should exist at: {}",
            binary.display()
        );

        // Verify it's executable by running --version
        let output = Command::new(&binary)
            .arg("--version")
            .output()
            .expect("Failed to run splice --version");

        assert!(
            output.status.success(),
            "splice --version failed: {}",
            String::from_utf8_lossy(&output.stderr)
        );
    }

    #[test]
    fn test_checksum_computation() {
        let workspace = create_rust_workspace();
        let lib_path = workspace.path().join("src/lib.rs");

        let checksum1 = file_checksum(&lib_path);
        let checksum2 = file_checksum(&lib_path);

        assert_eq!(
            checksum1, checksum2,
            "Checksum should be deterministic for same file"
        );

        // Verify checksum changes when content changes
        fs::write(&lib_path, "// modified\n").expect("Failed to modify file");
        let checksum3 = file_checksum(&lib_path);

        assert_ne!(
            checksum1, checksum3,
            "Checksum should change when file content changes"
        );
    }

    // ============================================================================
    // Patch Workflow Tests (Task 2)
    // ============================================================================

    #[test]
    fn test_e2e_patch_success() {
        let workspace = create_rust_workspace();
        let workspace_path = workspace.path();
        let lib_path = workspace_path.join("src/lib.rs");

        // Create patch file
        let patch_content = r#"pub fn greet(name: &str) -> String {
    format!("Greetings, {}!", name)
}
"#;
        let patch_file = workspace_path.join("patch.rs");
        fs::write(&patch_file, patch_content).expect("Failed to write patch file");

        // First, index the workspace
        let index_output = run_splice(&["index", "."], workspace_path);
        assert!(
            index_output.status.success(),
            "Index should succeed: {}",
            String::from_utf8_lossy(&index_output.stderr)
        );

        // Run patch command
        let output = run_splice(
            &[
                "patch",
                "--file",
                "src/lib.rs",
                "--symbol",
                "greet",
                "--with",
                "patch.rs",
            ],
            workspace_path,
        );

        // Verify exit code 0
        assert!(
            output.status.success(),
            "Patch should succeed: {}",
            String::from_utf8_lossy(&output.stderr)
        );

        // Verify file content updated
        let updated_content = fs::read_to_string(&lib_path).expect("Failed to read lib.rs");
        assert!(
            updated_content.contains("Greetings,"),
            "Updated content should contain 'Greetings,'"
        );

        // Verify cargo check passes
        let check_output = Command::new("cargo")
            .args(["check", "--quiet"])
            .current_dir(workspace_path)
            .output()
            .expect("Failed to run cargo check");

        assert!(
            check_output.status.success(),
            "cargo check should pass: {}",
            String::from_utf8_lossy(&check_output.stderr)
        );
    }

    #[test]
    fn test_e2e_patch_with_preview() {
        let workspace = create_rust_workspace();
        let workspace_path = workspace.path();

        let original_content = fs::read_to_string(workspace_path.join("src/lib.rs"))
            .expect("Failed to read original lib.rs");

        // Create patch file
        let patch_content = r#"pub fn greet(name: &str) -> String {
    format!("Greetings, {}!", name)
}
"#;
        let patch_file = workspace_path.join("patch.rs");
        fs::write(&patch_file, patch_content).expect("Failed to write patch file");

        // Index the workspace
        let _index_output = run_splice(&["index", "."], workspace_path);

        // Run patch with preview
        let output = run_splice(
            &[
                "patch",
                "--file",
                "src/lib.rs",
                "--symbol",
                "greet",
                "--with",
                "patch.rs",
                "--preview",
            ],
            workspace_path,
        );

        // Verify returns preview report (stdout should contain "greet" or "preview")
        let stdout = String::from_utf8_lossy(&output.stdout);
        assert!(
            stdout.len() > 0,
            "Preview should produce output"
        );

        // Verify original file unchanged
        let current_content = fs::read_to_string(workspace_path.join("src/lib.rs"))
            .expect("Failed to read current lib.rs");
        assert_eq!(
            current_content, original_content,
            "Original file should be unchanged in preview mode"
        );
    }

    #[test]
    fn test_e2e_patch_rollback_on_syntax_error() {
        let workspace = create_rust_workspace();
        let workspace_path = workspace.path();
        let lib_path = workspace_path.join("src/lib.rs");

        let original_content = fs::read_to_string(&lib_path).expect("Failed to read original");

        // Create patch with syntax error (missing closing brace)
        let patch_content = r#"pub fn greet(name: &str) -> String {
    format!("Greetings, {}!", name)
"#; // Missing closing brace
        let patch_file = workspace_path.join("patch.rs");
        fs::write(&patch_file, patch_content).expect("Failed to write patch file");

        // Index the workspace
        let _index_output = run_splice(&["index", "."], workspace_path);

        // Run patch command
        let output = run_splice(
            &[
                "patch",
                "--file",
                "src/lib.rs",
                "--symbol",
                "greet",
                "--with",
                "patch.rs",
                "--skip-validation", // Skip cargo check for this test
            ],
            workspace_path,
        );

        // Verify exit code != 0 (syntax error should fail)
        assert!(
            !output.status.success(),
            "Patch should fail on syntax error"
        );

        // Verify original file restored
        let current_content = fs::read_to_string(&lib_path).expect("Failed to read current lib.rs");
        assert_eq!(
            current_content, original_content,
            "Original file should be restored after syntax error"
        );
    }

    #[test]
    fn test_e2e_patch_rollback_on_compile_error() {
        let workspace = create_rust_workspace();
        let workspace_path = workspace.path();
        let lib_path = workspace_path.join("src/lib.rs");

        let original_content = fs::read_to_string(&lib_path).expect("Failed to read original");

        // Create patch with type error (returns i32 instead of String)
        let patch_content = r#"pub fn greet(name: &str) -> String {
    42 // Type error: integer literal doesn't match String return
}
"#;
        let patch_file = workspace_path.join("patch.rs");
        fs::write(&patch_file, patch_content).expect("Failed to write patch file");

        // Index the workspace
        let _index_output = run_splice(&["index", "."], workspace_path);

        // Run patch command (validation should catch compile error)
        let output = run_splice(
            &[
                "patch",
                "--file",
                "src/lib.rs",
                "--symbol",
                "greet",
                "--with",
                "patch.rs",
            ],
            workspace_path,
        );

        // Verify exit code != 0 (compile error should fail)
        assert!(
            !output.status.success(),
            "Patch should fail on compile error"
        );

        // Verify original file restored
        let current_content = fs::read_to_string(&lib_path).expect("Failed to read current lib.rs");
        assert_eq!(
            current_content, original_content,
            "Original file should be restored after compile error"
        );
    }

    #[test]
    fn test_e2e_patch_with_checksum_verification() {
        let workspace = create_rust_workspace();
        let workspace_path = workspace.path();
        let lib_path = workspace_path.join("src/lib.rs");

        // Compute pre-patch checksum
        let pre_checksum = file_checksum(&lib_path);

        // Create patch file
        let patch_content = r#"pub fn greet(name: &str) -> String {
    format!("Greetings, {}!", name)
}
"#;
        let patch_file = workspace_path.join("patch.rs");
        fs::write(&patch_file, patch_content).expect("Failed to write patch file");

        // Index the workspace
        let _index_output = run_splice(&["index", "."], workspace_path);

        // Run successful patch
        let output = run_splice(
            &[
                "patch",
                "--file",
                "src/lib.rs",
                "--symbol",
                "greet",
                "--with",
                "patch.rs",
            ],
            workspace_path,
        );

        assert!(
            output.status.success(),
            "Patch should succeed: {}",
            String::from_utf8_lossy(&output.stderr)
        );

        // Compute post-patch checksum
        let post_checksum = file_checksum(&lib_path);

        // Verify checksum changed (file was modified)
        assert_ne!(
            pre_checksum, post_checksum,
            "Checksum should change after successful patch"
        );

        // Verify post-patch checksum represents valid Rust code
        let check_output = Command::new("cargo")
            .args(["check", "--quiet"])
            .current_dir(workspace_path)
            .output()
            .expect("Failed to run cargo check");

        assert!(
            check_output.status.success(),
            "Post-patch content should pass cargo check"
        );
    }

    #[test]
    fn test_e2e_patch_execution_log_recorded() {
        let workspace = create_rust_workspace();
        let workspace_path = workspace.path();

        // Create patch file
        let patch_content = r#"pub fn greet(name: &str) -> String {
    format!("Greetings, {}!", name)
}
"#;
        let patch_file = workspace_path.join("patch.rs");
        fs::write(&patch_file, patch_content).expect("Failed to write patch file");

        // Index the workspace
        let _index_output = run_splice(&["index", "."], workspace_path);

        // Run patch command
        let output = run_splice(
            &[
                "patch",
                "--file",
                "src/lib.rs",
                "--symbol",
                "greet",
                "--with",
                "patch.rs",
            ],
            workspace_path,
        );

        assert!(
            output.status.success(),
            "Patch should succeed: {}",
            String::from_utf8_lossy(&output.stderr)
        );

        // Verify .splice/operations.db was created
        let ops_db_path = workspace_path.join(".splice/operations.db");
        assert!(
            ops_db_path.exists(),
            "operations.db should exist after patch"
        );

        // Verify execution log contains entry (check file size > 0)
        let metadata = fs::metadata(&ops_db_path).expect("Failed to read ops db metadata");
        assert!(
            metadata.len() > 0,
            "operations.db should have content"
        );

        // Parse stdout for execution_id if present
        let stdout = String::from_utf8_lossy(&output.stdout);
        if stdout.contains("execution_id") {
            assert!(
                stdout.contains("patch"),
                "Execution log should reference patch operation"
            );
        }
    }

    // ============================================================================
    // Additional tests will be added in subsequent tasks
    // ============================================================================
}

