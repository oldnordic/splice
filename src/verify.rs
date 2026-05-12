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

use crate::checksum::{checksum_file, Checksum};
use crate::error::{Result, SpliceError};
use crate::io_ext;
use std::path::Path;

/// Multiplier for disk space estimation to account for filesystem overhead.
/// Atomic writes need space for both original and new file simultaneously,
/// plus additional buffer for journaling and metadata.
const DISK_SPACE_MULTIPLIER: usize = 3;

/// Additional overhead per file for metadata and filesystem structures (in bytes).
/// Accounts for typical filesystem block size (4KB) and inode/table overhead.
const DISK_OVERHEAD_PER_FILE: u64 = 4096;

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
        matches!(
            self,
            Self::Fail {
                blocking: false,
                ..
            }
        )
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
            format!(
                "Workspace directory does not exist: {}",
                workspace_root.display()
            ),
        );
    }

    // Check 2: Workspace is writable
    // Try to create a temp file to test write permissions
    let test_file = workspace_root.join(".splice_write_test");

    // Clean up any stale test file first (from previous crashed runs)
    let _ = std::fs::remove_file(&test_file);

    if let Err(e) = std::fs::write(&test_file, b"test") {
        return PreVerificationResult::blocking(
            "workspace_writable",
            format!("Workspace is not writable: {}", e),
        );
    }
    // Clean up test file (only if write succeeded above)
    let _ = std::fs::remove_file(&test_file);

    // Check 3: Sufficient disk space with improved heuristic
    // Get disk space using filesystem metadata
    match get_disk_space(workspace_root) {
        Ok((available, _total)) => {
            // Calculate needed space with multiplier and overhead
            // Atomic writes need space for: original + new + metadata overhead
            let needed = (estimated_size * DISK_SPACE_MULTIPLIER) as u64 + DISK_OVERHEAD_PER_FILE;
            if available < needed {
                return PreVerificationResult::blocking(
                    "disk_space",
                    format!(
                        "Insufficient disk space: need {} bytes ({}x file size + {} overhead), available {} bytes",
                        needed, DISK_SPACE_MULTIPLIER, DISK_OVERHEAD_PER_FILE, available
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
///
/// Returns (available_bytes, total_bytes) for the filesystem.
fn get_disk_space(path: &Path) -> Result<(u64, u64)> {
    #[cfg(unix)]
    {
        // Use statvfs on Unix systems
        use std::ffi::CString;
        use std::os::unix::ffi::OsStrExt;

        // Convert path to CString
        let c_path = CString::new(path.as_os_str().as_bytes()).map_err(|_| SpliceError::Io {
            path: path.to_path_buf(),
            source: std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                "path contains null byte",
            ),
        })?;

        // Unsafe block to call statvfs
        unsafe {
            let mut stat: libc::statvfs = std::mem::zeroed();

            if libc::statvfs(c_path.as_ptr(), &mut stat) != 0 {
                return Err(SpliceError::Io {
                    path: path.to_path_buf(),
                    source: std::io::Error::last_os_error(),
                });
            }

            // Calculate available and total bytes
            // statvfs.f_bsize = filesystem block size
            // statvfs.f_blocks = total blocks
            // statvfs.f_bavail = free blocks available to non-privileged user
            let frsize = stat.f_frsize as u64; // Fragment size (fundamental file system block size)
            let total = stat.f_blocks as u64 * frsize;
            let available = stat.f_bavail as u64 * frsize;

            Ok((available, total))
        }
    }

    #[cfg(windows)]
    {
        // Use GetDiskFreeSpaceExW on Windows
        use std::os::windows::ffi::OsStrExt;
        use std::ptr;

        // Convert path to UTF-16
        let path_wide: Vec<u16> = path
            .as_os_str()
            .encode_wide()
            .chain(std::iter::once(0))
            .collect();

        unsafe {
            let mut free_bytes: u64 = 0;
            let mut total_bytes: u64 = 0;
            let mut available_bytes: u64 = 0;

            // GetDiskFreeSpaceExW retrieves information about the amount of space
            // that is available on a disk volume
            if winapi::um::GetDiskFreeSpaceExW(
                path_wide.as_ptr(),
                &mut free_bytes,
                &mut total_bytes,
                ptr::null_mut(),
            ) != 0
            {
                Ok((available_bytes, total_bytes))
            } else {
                Err(std::io::Error::last_os_error().into())
            }
        }
    }
}

/// Verify graph database is in sync with files.
///
/// Checks:
/// - Database file exists and is readable
/// - File modification time <= database last update time
pub fn verify_graph_sync(file_path: &Path, db_path: &Path) -> PreVerificationResult {
    // Check 1: Database exists
    // Patches don't require a graph DB (graph is only needed for query commands).
    if !db_path.exists() {
        return PreVerificationResult::warning(
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
/// # Arguments
/// * `file_path` - Path to the file to verify
/// * `expected_checksum` - Optional expected checksum for external modification detection
/// * `workspace_root` - Workspace directory path
/// * `db_path` - Path to graph database
/// * `strict` - If true, convert warnings to blocking failures
/// * `skip` - If true, skip all verification checks (returns all Pass)
///
/// # Returns
/// * `Ok(Vec<PreVerificationResult>)` - All checks completed (may contain warnings)
/// * `Err(SpliceError)` - Fatal error during verification
pub fn pre_verify_patch(
    file_path: &Path,
    expected_checksum: Option<&Checksum>,
    workspace_root: &Path,
    db_path: &Path,
    strict: bool,
    skip: bool,
) -> Result<Vec<PreVerificationResult>> {
    let mut results = Vec::new();

    // Skip verification if requested
    if skip {
        log::warn!("Skipping pre-verification checks (dangerous!)");
        results.push(PreVerificationResult::pass());
        return Ok(results);
    }

    // Get file size for workspace resource check
    let file_size = if file_path.exists() {
        std::fs::metadata(file_path)
            .map(|m| m.len() as usize)
            .unwrap_or(0)
    } else {
        0
    };

    // Run all checks
    let mut file_result = verify_file_ready(file_path, expected_checksum, workspace_root);

    let mut workspace_result = verify_workspace_resources(workspace_root, file_size);

    let mut graph_result = verify_graph_sync(file_path, db_path);

    // In strict mode, convert warnings to blocking failures
    if strict {
        if file_result.is_warning() {
            file_result = PreVerificationResult::blocking(
                "strict_mode",
                format!("Warning treated as error: {:?}", file_result),
            );
        }
        if workspace_result.is_warning() {
            workspace_result = PreVerificationResult::blocking(
                "strict_mode",
                format!("Warning treated as error: {:?}", workspace_result),
            );
        }
        if graph_result.is_warning() {
            graph_result = PreVerificationResult::blocking(
                "strict_mode",
                format!("Warning treated as error: {:?}", graph_result),
            );
        }
    }

    results.push(file_result);
    results.push(workspace_result);
    results.push(graph_result);

    Ok(results)
}

/// Post-verification result.
#[derive(Debug, Clone, PartialEq)]
pub struct PostVerificationResult {
    /// Syntax validation passed (tree-sitter parse)
    pub syntax_ok: bool,

    /// Compiler validation passed (cargo check / rustc)
    /// NOTE: This catches semantic errors: type mismatches, borrow checker,
    /// unresolved references, missing methods, etc.
    pub compiler_ok: bool,

    /// Enhanced LSP diagnostics (rust-analyzer, advisory/non-blocking)
    /// Currently not implemented. Use compiler_ok for semantic validation.
    pub semantic_ok: bool,

    /// Checksums before and after
    pub before_checksum: String,
    /// After checksum
    pub after_checksum: String,

    /// Warnings (non-blocking issues)
    pub warnings: Vec<String>,

    /// Errors (blocking issues that would have failed validation)
    pub errors: Vec<String>,
}

impl PostVerificationResult {
    /// Create a new post-verification result.
    pub fn new(
        syntax_ok: bool,
        compiler_ok: bool,
        before_checksum: String,
        after_checksum: String,
    ) -> Self {
        Self {
            syntax_ok,
            compiler_ok,
            semantic_ok: true, // Default to true (advisory)
            before_checksum,
            after_checksum,
            warnings: Vec::new(),
            errors: Vec::new(),
        }
    }

    /// Add a warning.
    pub fn add_warning(&mut self, warning: impl Into<String>) {
        self.warnings.push(warning.into());
    }

    /// Add an error.
    pub fn add_error(&mut self, error: impl Into<String>) {
        self.errors.push(error.into());
    }

    /// Check if file changed (checksums differ).
    pub fn file_changed(&self) -> bool {
        self.before_checksum != self.after_checksum
    }
}

/// Checksum difference comparison.
#[derive(Debug, Clone, PartialEq)]
pub struct ChecksumDiff {
    /// Checksums are different (change occurred)
    pub changed: bool,
    /// Estimated size of change (bytes, negative = smaller)
    pub estimated_delta: i64,
}

/// Compare checksums to document what changed.
pub fn checksum_diff(before_checksum: &str, after_checksum: &str) -> ChecksumDiff {
    let changed = before_checksum != after_checksum;

    // Delta estimation isn't implemented — actual file sizes would be required.
    let estimated_delta = 0;

    ChecksumDiff {
        changed,
        estimated_delta,
    }
}

/// Verify file after patching.
///
/// Runs:
/// - Syntax validation (via tree-sitter if available)
/// - Compiler validation (via cargo check for Rust)
/// - Checksum verification (confirm expected changes)
/// - Semantic checks (rust-analyzer diagnostics - advisory)
///
/// Returns a PostVerificationResult with detailed status.
pub fn verify_after_patch(
    file_path: &Path,
    workspace_root: &Path,
    expected_before: &str,
    analyzer_mode: crate::validate::AnalyzerMode,
) -> Result<PostVerificationResult> {
    use crate::checksum::checksum_file;

    let mut result = PostVerificationResult::new(
        false, // syntax_ok - will be set below
        false, // compiler_ok - will be set below
        expected_before.to_string(),
        String::new(), // after_checksum - will be set below
    );

    // Compute after checksum
    match checksum_file(file_path) {
        Ok(after) => {
            result.after_checksum = after.as_hex().to_string();

            // Check if file actually changed
            if !result.file_changed() {
                result.add_warning("File checksum unchanged - no modification detected");
            }
        }
        Err(e) => {
            result.add_error(format!("Failed to compute after checksum: {}", e));
            return Ok(result); // Return with error logged
        }
    }

    // Syntax validation: parse with tree-sitter
    use crate::syntax_validator::validate_syntax;
    match std::fs::read(file_path) {
        Ok(source) => {
            result.syntax_ok = match validate_syntax(file_path, &source) {
                Ok(valid) => {
                    if !valid {
                        result.add_error(
                            "Syntax validation failed: tree-sitter detected parse errors"
                                .to_string(),
                        );
                    }
                    valid
                }
                Err(e) => {
                    log::warn!("Syntax validation error: {}", e);
                    false
                }
            };
        }
        Err(e) => {
            result.add_error(format!("Failed to read file for syntax validation: {}", e));
            result.syntax_ok = false;
        }
    }

    // Compiler validation: run cargo check for Rust projects
    result.compiler_ok = match run_cargo_check(workspace_root) {
        Ok(output) => {
            if output.status.success() {
                log::info!("Cargo check passed");
                true
            } else {
                let stderr = String::from_utf8_lossy(&output.stderr);
                result.add_error(format!("Cargo check failed: {}", stderr));
                log::warn!("Cargo check failed: {}", stderr);
                false
            }
        }
        Err(e) => {
            // Don't block if cargo check unavailable (not a Rust project or cargo not installed)
            result.add_warning(format!("Unable to run cargo check: {}", e));
            log::debug!("Unable to run cargo check: {}", e);
            true // Allow patch to proceed
        }
    };

    // Semantic validation via rust-analyzer (advisory, non-blocking for result)
    if workspace_root.join("Cargo.toml").exists()
        && analyzer_mode != crate::validate::AnalyzerMode::Off
    {
        match crate::validate::run_rust_analyzer(workspace_root, &analyzer_mode) {
            Ok(diagnostics) => {
                let (errors, warnings): (Vec<_>, Vec<_>) = diagnostics
                    .iter()
                    .partition(|d| d.level == crate::validate::ErrorLevel::Error);

                for warn in &warnings {
                    result.add_warning(format!(
                        "rust-analyzer [{}] at {}:{}: {}",
                        warn.code.as_deref().unwrap_or("warning"),
                        warn.file,
                        warn.line,
                        warn.message
                    ));
                }

                for err in &errors {
                    result.add_error(format!(
                        "rust-analyzer [{}] at {}:{}: {}",
                        err.code.as_deref().unwrap_or("error"),
                        err.file,
                        err.line,
                        err.message
                    ));
                }

                result.semantic_ok = errors.is_empty();
            }
            Err(e) => {
                result.add_warning(format!("rust-analyzer unavailable: {}", e));
                result.semantic_ok = true; // Advisory: don't block if analyzer can't run
            }
        }
    } else {
        result.semantic_ok = true;
    }

    Ok(result)
}

/// Verify that changes were localized to the target span.
///
/// Reads current file, masks out the target span region, and verifies
/// that non-target regions match the original content.
pub fn verify_localized_change(
    file_path: &Path,
    replaced_content: &[u8],
    target_span: (usize, usize),
) -> Result<bool> {
    let current = io_ext::read(file_path)?;

    // Check bytes before target span
    if target_span.0 > 0 && target_span.0 <= replaced_content.len() {
        let before_replaced = &replaced_content[..target_span.0];
        let before_current = current.get(..target_span.0);

        if before_current != Some(before_replaced) {
            log::warn!("File modified before target span");
            return Ok(false);
        }
    }

    // Check bytes after target span
    let after_start = target_span.1.min(replaced_content.len());
    if after_start < replaced_content.len() {
        let after_replaced = &replaced_content[after_start..];
        let after_current = current.get(after_start..);

        if after_current != Some(after_replaced) {
            log::warn!("File modified after target span");
            return Ok(false);
        }
    }

    Ok(true)
}

/// Run cargo check to validate Rust compilation
///
/// This helper function runs `cargo check --quiet` in the workspace directory
/// to verify that the code compiles after a patch operation.
///
/// # Arguments
/// * `workspace_dir` - Path to the workspace directory
///
/// # Returns
/// * `Ok(Output)` - Process output from cargo check
/// * `Err(SpliceError)` - Failed to run cargo check
///
/// # Notes
/// - Uses --quiet flag to suppress build output (only show errors)
/// - Non-Rust projects or missing cargo will return an error
/// - Caller should decide whether to block on failure (we don't block here)
fn run_cargo_check(workspace_dir: &Path) -> Result<std::process::Output> {
    use std::process::Command;

    if !workspace_dir.join("Cargo.toml").exists() {
        return Err(crate::error::SpliceError::IoContext {
            context: format!(
                "Cannot run cargo check outside a Rust package: {}",
                workspace_dir.display()
            ),
            source: std::io::Error::new(std::io::ErrorKind::NotFound, "Cargo.toml not found"),
        });
    }

    let output = Command::new("cargo")
        .args(["check", "--quiet", "--color=never"])
        .current_dir(workspace_dir)
        .output()
        .map_err(|e| crate::error::SpliceError::IoContext {
            context: format!("Failed to run cargo check: {}", e),
            source: e,
        })?;

    Ok(output)
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
