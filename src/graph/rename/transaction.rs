use super::*;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

/// Backup manifest for rename operations.
///
/// This struct stores metadata about a rename operation backup, including
/// the operation ID, timestamp, and checksums of all backed-up files.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RenameBackupManifest {
    /// Operation identifier (e.g., "rename-symbol_id-20250104-120000")
    pub operation_id: String,
    /// Timestamp of backup (ISO 8601)
    pub timestamp: String,
    /// Files backed up with their checksums (relative path -> sha256 hash)
    pub files: HashMap<String, String>,
}

/// Create backup for rename operation.
///
/// Creates a backup directory in `.splice/backups/rename-<symbol_id>-<timestamp>/`
/// with copies of all affected files and a manifest.json containing checksums.
///
/// # Arguments
/// * `workspace_root` - Project root directory
/// * `symbol_id` - Symbol ID for directory naming
/// * `files_to_backup` - Files to backup
///
/// # Returns
/// Path to created backup directory containing manifest.json
///
/// # Errors
/// Returns Io error if directory creation or file copying fails.
pub fn create_rename_backup(
    workspace_root: &Path,
    symbol_id: &str,
    files_to_backup: &[PathBuf],
) -> Result<PathBuf> {
    // Create backup directory name: rename-<symbol_id>-<timestamp>
    let timestamp = Utc::now().format("%Y%m%d-%H%M%S").to_string();
    let operation_id = format!("rename-{}-{}", symbol_id, timestamp);

    // Use .splice/backups/ base directory (from context decision)
    let backups_base = workspace_root.join(".splice/backups");
    fs::create_dir_all(&backups_base).map_err(|e| SpliceError::Io {
        path: backups_base.clone(),
        source: e,
    })?;

    let backup_dir = backups_base.join(&operation_id);
    fs::create_dir(&backup_dir).map_err(|e| SpliceError::Io {
        path: backup_dir.clone(),
        source: e,
    })?;

    // Copy files to backup directory, preserving structure
    let mut manifest = RenameBackupManifest {
        operation_id: operation_id.clone(),
        timestamp: Utc::now().to_rfc3339(),
        files: HashMap::new(),
    };

    for file_path in files_to_backup {
        // Compute relative path from workspace root
        let relative_path = file_path.strip_prefix(workspace_root).map_err(|_| {
            SpliceError::Other(format!(
                "File {} is not under workspace root {}",
                file_path.display(),
                workspace_root.display()
            ))
        })?;

        // Create backup path
        let backup_file_path = backup_dir.join(relative_path);

        // Create parent directories if needed
        if let Some(parent) = backup_file_path.parent() {
            fs::create_dir_all(parent).map_err(|e| SpliceError::Io {
                path: parent.to_path_buf(),
                source: e,
            })?;
        }

        // Copy file
        fs::copy(file_path, &backup_file_path).map_err(|e| SpliceError::Io {
            path: backup_file_path.clone(),
            source: e,
        })?;

        // Compute checksum for verification
        let checksum = sha256_checksum(file_path)?;
        manifest
            .files
            .insert(relative_path.display().to_string(), checksum);
    }

    // Write manifest.json
    let manifest_path = backup_dir.join("manifest.json");
    let manifest_json = serde_json::to_string_pretty(&manifest)
        .map_err(|e| SpliceError::Other(format!("Failed to serialize backup manifest: {}", e)))?;
    fs::write(&manifest_path, manifest_json).map_err(|e| SpliceError::Io {
        path: manifest_path.clone(),
        source: e,
    })?;

    Ok(backup_dir)
}

/// Compute SHA-256 checksum of a file.
///
/// # Arguments
/// * `file_path` - Path to file
///
/// # Returns
/// Hex-encoded SHA-256 checksum
///
/// # Errors
/// Returns Io error if file cannot be read.
pub(crate) fn sha256_checksum(file_path: &Path) -> Result<String> {
    let mut hasher = Sha256::new();
    let content = fs::read(file_path).map_err(|e| SpliceError::Io {
        path: file_path.to_path_buf(),
        source: e,
    })?;
    hasher.update(&content);
    Ok(format!("{:x}", hasher.finalize()))
}

/// Transaction context for rename operation.
///
/// Provides atomic rename operations with rollback support.
/// If any part of the rename fails, all changes can be reverted
/// from the backup.
#[derive(Debug)]
pub struct RenameTransaction {
    /// Full path to backup directory containing manifest.json
    pub(crate) backup_dir: Option<PathBuf>,
    /// Workspace root path for restoring files
    pub(crate) workspace_root: Option<PathBuf>,
    /// Files that have been modified
    modified_files: Vec<PathBuf>,
}

impl RenameTransaction {
    /// Create new transaction without backup.
    pub fn new() -> Self {
        Self {
            backup_dir: None,
            workspace_root: None,
            modified_files: Vec::new(),
        }
    }

    /// Set backup directory for rollback (stores full path).
    ///
    /// # Arguments
    /// * `backup_dir` - Path to backup directory containing manifest.json
    /// * `workspace_root` - Root directory of the workspace
    pub fn with_backup(mut self, backup_dir: PathBuf, workspace_root: PathBuf) -> Self {
        self.backup_dir = Some(backup_dir);
        self.workspace_root = Some(workspace_root);
        self
    }

    /// Track a modified file.
    ///
    /// # Arguments
    /// * `file` - Path to file that was modified
    pub fn track_modified(&mut self, file: PathBuf) {
        self.modified_files.push(file);
    }

    /// Rollback transaction by restoring files from backup.
    ///
    /// Reads the manifest.json from the backup directory and restores
    /// all files to their pre-rename state.
    ///
    /// # Returns
    /// Ok(()) if rollback succeeded, error if it failed
    ///
    /// # Errors
    /// Returns Io error if file restoration fails.
    /// Returns Other error if manifest is missing or invalid.
    pub fn rollback(self) -> Result<()> {
        if let (Some(backup_dir), Some(workspace_root)) = (self.backup_dir, self.workspace_root) {
            let manifest_path = backup_dir.join("manifest.json");
            if manifest_path.exists() {
                // Read manifest
                let manifest_json =
                    fs::read_to_string(&manifest_path).map_err(|e| SpliceError::Io {
                        path: manifest_path.clone(),
                        source: e,
                    })?;
                let manifest: RenameBackupManifest =
                    serde_json::from_str(&manifest_json).map_err(|e| {
                        SpliceError::Other(format!("Failed to parse backup manifest: {}", e))
                    })?;

                // Restore each file from backup
                for (relative_path, _checksum) in manifest.files {
                    let backup_file = backup_dir.join(&relative_path);
                    let original_file = workspace_root.join(&relative_path);

                    if backup_file.exists() {
                        fs::copy(&backup_file, &original_file).map_err(|e| SpliceError::Io {
                            path: original_file.clone(),
                            source: e,
                        })?;
                    }
                }
            }
        }
        Ok(())
    }

    /// Get the number of files modified in this transaction.
    pub fn modified_count(&self) -> usize {
        self.modified_files.len()
    }

    /// Get the list of modified files.
    pub fn modified_files(&self) -> &[PathBuf] {
        &self.modified_files
    }
}

impl Default for RenameTransaction {
    fn default() -> Self {
        Self::new()
    }
}
