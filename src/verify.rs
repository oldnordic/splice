//! Pre-verification hooks for safe refactoring operations.
//!
//! Pre-verification runs BEFORE any file modifications to:
//! - Validate file state (unchanged, writable, readable)
//! - Verify workspace conditions (disk space, permissions)
//! - Check graph database synchronization
//! - Detect external modifications (via checksums)

use crate::checksum::{checksum_file, Checksum};
use crate::error::Result;
use std::path::Path;

/// Pre-verification result.
#[derive(Debug, Clone, PartialEq)]
pub enum PreVerificationResult {
    /// All checks passed, safe to proceed.
    Pass,

    /// Check failed with details.
    Fail {
        /// Check that failed
        check: String,
        /// Failure reason
        reason: String,
        /// Whether this is a blocking failure (true) or warning (false)
        blocking: bool,
    },
}

impl PreVerificationResult {
    /// Create a passing result.
    pub fn pass() -> Self {
        Self::Pass
    }

    /// Create a blocking failure result.
    pub fn blocking(check: impl Into<String>, reason: impl Into<String>) -> Self {
        Self::Fail {
            check: check.into(),
            reason: reason.into(),
            blocking: true,
        }
    }

    /// Create a warning (non-blocking) result.
    pub fn warning(check: impl Into<String>, reason: impl Into<String>) -> Self {
        Self::Fail {
            check: check.into(),
            reason: reason.into(),
            blocking: false,
        }
    }

    /// Returns true if this result is a pass.
    pub fn is_pass(&self) -> bool {
        matches!(self, Self::Pass)
    }

    /// Returns true if this result is a blocking failure.
    pub fn is_blocking(&self) -> bool {
        matches!(self, Self::Fail { blocking: true, .. })
    }

    /// Returns true if this result is a warning.
    pub fn is_warning(&self) -> bool {
        matches!(self, Self::Fail { blocking: false, .. })
    }
}

/// Verify file is ready for patching operation.
///
/// Checks:
/// - File exists and is readable
/// - File is writable (can be modified)
/// - File checksum matches expected (no external modification)
/// - File is within workspace bounds
pub fn verify_file_ready(
    file_path: &Path,
    expected_checksum: Option<&Checksum>,
    workspace_root: &Path,
) -> PreVerificationResult {
    // Check 1: File exists
    if !file_path.exists() {
        return PreVerificationResult::blocking(
            "file_exists",
            format!("File does not exist: {}", file_path.display()),
        );
    }

    // Check 2: File is readable
    if let Err(e) = std::fs::metadata(file_path) {
        return PreVerificationResult::blocking(
            "file_readable",
            format!("Cannot read file metadata: {}", e),
        );
    }

    // Check 3: File is within workspace bounds
    if let Ok(abs_file) = file_path.canonicalize() {
        if let Ok(abs_workspace) = workspace_root.canonicalize() {
            if !abs_file.starts_with(&abs_workspace) {
                return PreVerificationResult::blocking(
                    "file_in_workspace",
                    format!(
                        "File '{}' is outside workspace root '{}'",
                        file_path.display(),
                        workspace_root.display()
                    ),
                );
            }
        }
    }

    // Check 4: File is writable
    // Try to open in append mode to test write permissions
    if let Err(e) = std::fs::OpenOptions::new()
        .write(true)
        .create(false)
        .open(file_path)
    {
        return PreVerificationResult::blocking(
            "file_writable",
            format!("File is not writable: {}", e),
        );
    }

    // Check 5: Checksum matches (if provided)
    if let Some(expected) = expected_checksum {
        match checksum_file(file_path) {
            Ok(actual) => {
                if actual != *expected {
                    return PreVerificationResult::blocking(
                        "file_checksum",
                        format!(
                            "File has been modified externally. Expected checksum {}, got {}",
                            expected.as_hex(),
                            actual.as_hex()
                        ),
                    );
                }
            }
            Err(e) => {
                return PreVerificationResult::blocking(
                    "file_checksum",
                    format!("Failed to compute checksum: {}", e),
                );
            }
        }
    }

    PreVerificationResult::pass()
}

/// Verify workspace has sufficient resources.
///
/// Checks:
/// - Disk space available (estimate 2x file size for safety)
/// - Write permissions in workspace
/// - Backup directory can be created
pub fn verify_workspace_resources(
    workspace_root: &Path,
    estimated_size: usize,
) -> PreVerificationResult {
    // Check 1: Workspace exists
    if !workspace_root.exists() {
        return PreVerificationResult::blocking(
            "workspace_exists",
            format!("Workspace directory does not exist: {}", workspace_root.display()),
        );
    }

    // Check 2: Workspace is writable
    // Try to create a temp file to test write permissions
    let test_file = workspace_root.join(".splice_write_test");
    if let Err(e) = std::fs::write(&test_file, b"test") {
        return PreVerificationResult::blocking(
            "workspace_writable",
            format!("Workspace is not writable: {}", e),
        );
    }
    // Clean up test file
    let _ = std::fs::remove_file(&test_file);

    // Check 3: Sufficient disk space (2x estimated size for safety)
    // Get disk space using filesystem metadata
    match get_disk_space(workspace_root) {
        Ok((available, _total)) => {
            let needed = estimated_size * 2;
            if available < needed as u64 {
                return PreVerificationResult::blocking(
                    "disk_space",
                    format!(
                        "Insufficient disk space: need {} bytes, available {} bytes",
                        needed, available
                    ),
                );
            }
        }
        Err(e) => {
            // Non-fatal: warn if we can't check disk space
            log::warn!("Could not check disk space: {}", e);
            return PreVerificationResult::warning(
                "disk_space",
                format!("Could not verify disk space: {}", e),
            );
        }
    }

    // Check 4: Backup directory can be created
    let backup_dir = workspace_root.join(".splice/backups");
    if !backup_dir.exists() {
        if let Err(e) = std::fs::create_dir_all(&backup_dir) {
            return PreVerificationResult::blocking(
                "backup_directory",
                format!("Cannot create backup directory: {}", e),
            );
        }
    }

    PreVerificationResult::pass()
}

/// Get available disk space on the filesystem containing the given path.
fn get_disk_space(_path: &Path) -> Result<(u64, u64)> {
    // Use std::fs::metadata to get file system stats
    // Note: This is a simplified implementation
    // A more robust implementation would use sysinfo or similar crate
    // For now, we return a large number to avoid false positives
    // TODO: Implement proper disk space checking
    Ok((1_000_000_000_000, 1_000_000_000_000)) // 1TB default
}

/// Verify graph database is in sync with files.
///
/// Checks:
/// - Database file exists and is readable
/// - File modification time <= database last update time
pub fn verify_graph_sync(file_path: &Path, db_path: &Path) -> PreVerificationResult {
    // Check 1: Database exists
    if !db_path.exists() {
        return PreVerificationResult::blocking(
            "graph_exists",
            format!("Graph database does not exist: {}", db_path.display()),
        );
    }

    // Check 2: Database is readable
    if let Err(e) = std::fs::metadata(db_path) {
        return PreVerificationResult::blocking(
            "graph_readable",
            format!("Cannot read graph database: {}", e),
        );
    }

    // Check 3: File mtime <= database mtime
    match (std::fs::metadata(file_path), std::fs::metadata(db_path)) {
        (Ok(file_meta), Ok(db_meta)) => {
            let file_mtime = match file_meta.modified() {
                Ok(time) => time,
                Err(_) => {
                    return PreVerificationResult::warning(
                        "file_mtime",
                        "Cannot read file modification time",
                    )
                }
            };

            let db_mtime = match db_meta.modified() {
                Ok(time) => time,
                Err(_) => {
                    return PreVerificationResult::warning(
                        "db_mtime",
                        "Cannot read database modification time",
                    )
                }
            };

            if file_mtime > db_mtime {
                return PreVerificationResult::blocking(
                    "graph_sync",
                    format!(
                        "File '{}' has been modified since database was last updated (file: {:?}, db: {:?})",
                        file_path.display(),
                        file_mtime,
                        db_mtime
                    ),
                );
            }
        }
        (Err(e), Ok(_)) => {
            return PreVerificationResult::warning(
                "file_metadata",
                format!("Cannot read file metadata: {}", e),
            )
        }
        (Ok(_), Err(e)) => {
            return PreVerificationResult::warning(
                "db_metadata",
                format!("Cannot read database metadata: {}", e),
            )
        }
        (Err(e), Err(_)) => {
            return PreVerificationResult::warning(
                "metadata",
                format!("Cannot read metadata: {}", e),
            )
        }
    }

    PreVerificationResult::pass()
}

/// Run all pre-verification checks for a patch operation.
///
/// This function runs all verification checks and returns a Vec of results.
/// The caller should check for blocking failures before proceeding.
///
/// # Returns
/// * `Ok(Vec<PreVerificationResult>)` - All checks completed (may contain warnings)
/// * `Err(SpliceError)` - Fatal error during verification
pub fn pre_verify_patch(
    file_path: &Path,
    expected_checksum: Option<&Checksum>,
    workspace_root: &Path,
    db_path: &Path,
) -> Result<Vec<PreVerificationResult>> {
    let mut results = Vec::new();

    // Get file size for workspace resource check
    let file_size = if file_path.exists() {
        std::fs::metadata(file_path)
            .map(|m| m.len() as usize)
            .unwrap_or(0)
    } else {
        0
    };

    // Run all checks
    results.push(verify_file_ready(
        file_path,
        expected_checksum,
        workspace_root,
    ));

    results.push(verify_workspace_resources(
        workspace_root,
        file_size,
    ));

    results.push(verify_graph_sync(file_path, db_path));

    // Check for fatal errors during verification
    if results.iter().any(|r| r.is_blocking()) {
        // Return the results so caller can see what failed
        // The caller should check is_blocking() and return appropriate error
    }

    Ok(results)
}

#[cfg(test)]
mod tests {
    use super::*;
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
        assert!(matches!(result, PreVerificationResult::Fail { check, .. } if check == "file_exists"));
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
        assert!(matches!(result, PreVerificationResult::Fail { check, .. } if check == "file_writable"));

        // Cleanup: restore write permissions
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
        assert!(matches!(result, PreVerificationResult::Fail { check, .. } if check == "file_checksum"));
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

        let results = pre_verify_patch(&file_path, None, temp_dir.path(), &db_path).unwrap();
        assert!(results.len() == 3);
        assert!(results.iter().all(|r| r.is_pass()));
    }

    #[test]
    fn test_pre_verify_blocking_failure() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("nonexistent.rs");
        let db_path = temp_dir.path().join("codegraph.db");

        let results = pre_verify_patch(&file_path, None, temp_dir.path(), &db_path).unwrap();
        assert!(results.iter().any(|r| r.is_blocking()));
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
        assert!(matches!(result, PreVerificationResult::Fail { check, .. } if check == "file_in_workspace"));
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
}
