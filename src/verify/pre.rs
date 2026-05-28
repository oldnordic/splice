//! Pre-verification hooks for safe refactoring operations.
//!
//! Pre-verification runs BEFORE any file modifications to:
//! - Validate file state (unchanged, writable, readable)
//! - Verify workspace conditions (disk space, permissions)
//! - Check graph database synchronization
//! - Detect external modifications (via checksums)

use crate::checksum::{checksum_file, Checksum};
use crate::error::{Result, SpliceError};
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
pub(crate) fn get_disk_space(path: &Path) -> Result<(u64, u64)> {
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
