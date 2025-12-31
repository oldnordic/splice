//! Backup and undo support for Splice operations.
//!
//! This module provides the ability to create backups before patching
//! and restore from those backups later. Backups are stored in
//! `.splice-backup/<operation_id>/` directories with a manifest
//! tracking the original file locations and hashes.

use crate::error::{Result, SpliceError};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::fs;
use std::path::{Path, PathBuf};

/// Metadata about a backed-up file.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BackupEntry {
    /// Original path of the file (relative to workspace root).
    pub original_path: PathBuf,
    /// SHA-256 hash of the original file content.
    pub hash: String,
    /// Byte count of the original file.
    pub size: u64,
}

/// Manifest describing a backup operation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BackupManifest {
    /// Unique identifier for this operation.
    pub operation_id: String,
    /// Timestamp when the backup was created (ISO 8601).
    pub timestamp: String,
    /// Files that were backed up.
    pub files: Vec<BackupEntry>,
    /// Absolute path to the backup directory.
    #[serde(skip)]
    pub backup_dir: PathBuf,
}

impl BackupManifest {
    /// Create a new backup manifest.
    pub fn new(operation_id: String, backup_dir: PathBuf) -> Self {
        let timestamp = chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Secs, true);
        BackupManifest {
            operation_id,
            timestamp,
            files: Vec::new(),
            backup_dir,
        }
    }

    /// Add a file entry to the manifest.
    pub fn add_file(&mut self, original_path: PathBuf, hash: String, size: u64) {
        self.files.push(BackupEntry {
            original_path,
            hash,
            size,
        });
    }

    /// Save the manifest to a file in the backup directory.
    pub fn save(&self) -> Result<()> {
        let manifest_path = self.backup_dir.join("manifest.json");
        let json = serde_json::to_string_pretty(self)
            .map_err(|e| SpliceError::Other(format!("Failed to serialize manifest: {}", e)))?;
        fs::write(&manifest_path, json)
            .map_err(|e| SpliceError::Io {
                path: manifest_path,
                source: e,
            })?;
        Ok(())
    }

    /// Load a manifest from a file.
    pub fn load(manifest_path: &Path) -> Result<Self> {
        let json = fs::read_to_string(manifest_path).map_err(|e| SpliceError::Io {
            path: manifest_path.to_path_buf(),
            source: e,
        })?;

        let mut manifest: BackupManifest = serde_json::from_str(&json)
            .map_err(|e| SpliceError::Other(format!("Failed to parse manifest: {}", e)))?;

        manifest.backup_dir = manifest_path
            .parent()
            .ok_or_else(|| SpliceError::Other("Manifest has no parent directory".to_string()))?
            .to_path_buf();

        Ok(manifest)
    }
}

/// Writer for creating backups of files before patching.
pub struct BackupWriter {
    manifest: BackupManifest,
    workspace_root: PathBuf,
}

impl BackupWriter {
    /// Create a new backup writer.
    ///
    /// # Arguments
    /// * `workspace_root` - Root directory of the workspace
    /// * `operation_id` - Unique identifier for the operation (or UUID v4 if None)
    pub fn new(workspace_root: &Path, operation_id: Option<String>) -> Result<Self> {
        let op_id = operation_id.unwrap_or_else(|| {
            uuid::Uuid::new_v4().to_string()
        });

        let backup_dir = workspace_root.join(".splice-backup").join(&op_id);

        // Create backup directory
        fs::create_dir_all(&backup_dir).map_err(|e| SpliceError::Io {
            path: backup_dir.clone(),
            source: e,
        })?;

        let manifest = BackupManifest::new(op_id, backup_dir);

        Ok(BackupWriter {
            manifest,
            workspace_root: workspace_root.to_path_buf(),
        })
    }

    /// Get the operation ID for this backup.
    pub fn operation_id(&self) -> &str {
        &self.manifest.operation_id
    }

    /// Get the path to the manifest file.
    pub fn manifest_path(&self) -> PathBuf {
        self.manifest.backup_dir.join("manifest.json")
    }

    /// Backup a single file.
    ///
    /// The file is copied to the backup directory with its original
    /// filename preserved (to avoid collisions, files with the same
    /// name from different directories are stored in subdirectories).
    pub fn backup_file(&mut self, file_path: &Path) -> Result<()> {
        // Read original file
        let content = fs::read(file_path).map_err(|e| SpliceError::Io {
            path: file_path.to_path_buf(),
            source: e,
        })?;

        // Compute hash
        let hash = compute_hash(&content);
        let size = content.len() as u64;

        // Compute relative path from workspace root
        let relative = file_path
            .strip_prefix(&self.workspace_root)
            .map_err(|_| SpliceError::Other(format!(
                "File '{}' is not under workspace root '{}'",
                file_path.display(),
                self.workspace_root.display()
            )))?;

        // Create backup path preserving directory structure
        let backup_path = self.manifest.backup_dir.join(relative);

        // Create parent directories if needed
        if let Some(parent) = backup_path.parent() {
            fs::create_dir_all(parent).map_err(|e| SpliceError::Io {
                path: parent.to_path_buf(),
                source: e,
            })?;
        }

        // Copy file to backup location
        fs::write(&backup_path, &content).map_err(|e| SpliceError::Io {
            path: backup_path.clone(),
            source: e,
        })?;

        // Add entry to manifest
        self.manifest.add_file(relative.to_path_buf(), hash, size);

        Ok(())
    }

    /// Finalize the backup by writing the manifest file.
    ///
    /// Returns the path to the manifest file.
    pub fn finalize(self) -> Result<PathBuf> {
        self.manifest.save()?;
        Ok(self.manifest_path())
    }

    /// Get the backup directory path.
    pub fn backup_dir(&self) -> &Path {
        &self.manifest.backup_dir
    }
}

/// Restore files from a backup manifest.
pub fn restore_from_manifest(manifest_path: &Path, workspace_root: &Path) -> Result<usize> {
    let manifest = BackupManifest::load(manifest_path)?;

    let mut restored = 0;

    for entry in &manifest.files {
        let original_path = workspace_root.join(&entry.original_path);
        let backup_path = manifest.backup_dir.join(&entry.original_path);

        // Verify backup file exists
        if !backup_path.exists() {
            return Err(SpliceError::Other(format!(
                "Backup file missing: {}",
                backup_path.display()
            )));
        }

        // Read backup content
        let content = fs::read(&backup_path).map_err(|e| SpliceError::Io {
            path: backup_path.clone(),
            source: e,
        })?;

        // Verify hash matches
        let actual_hash = compute_hash(&content);
        if actual_hash != entry.hash {
            return Err(SpliceError::Other(format!(
                "Hash mismatch for {}: expected {}, got {}",
                entry.original_path.display(),
                entry.hash,
                actual_hash
            )));
        }

        // Create parent directory if needed
        if let Some(parent) = original_path.parent() {
            fs::create_dir_all(parent).map_err(|e| SpliceError::Io {
                path: parent.to_path_buf(),
                source: e,
            })?;
        }

        // Write to original location
        fs::write(&original_path, &content).map_err(|e| SpliceError::Io {
            path: original_path.clone(),
            source: e,
        })?;

        restored += 1;
    }

    Ok(restored)
}

/// Compute SHA-256 hash of bytes.
fn compute_hash(bytes: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    let result = hasher.finalize();
    format!("{:x}", result)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_backup_writer_creates_manifest() {
        let workspace = TempDir::new().expect("Failed to create temp dir");
        let workspace_root = workspace.path();

        // Create a test file
        let test_file = workspace_root.join("test.txt");
        fs::write(&test_file, b"hello world").expect("Failed to write test file");

        // Create backup
        let mut writer = BackupWriter::new(workspace_root, Some("test-op-123".to_string()))
            .expect("Failed to create BackupWriter");

        writer
            .backup_file(&test_file)
            .expect("Failed to backup file");

        let manifest_path = writer.finalize().expect("Failed to finalize backup");

        // Verify manifest exists
        assert!(manifest_path.exists(), "Manifest file should exist");

        // Verify backup file exists
        let backup_file = workspace_root
            .join(".splice-backup/test-op-123/test.txt");
        assert!(backup_file.exists(), "Backup file should exist");

        // Verify content matches
        let backup_content = fs::read_to_string(&backup_file)
            .expect("Failed to read backup file");
        assert_eq!(backup_content, "hello world");
    }

    #[test]
    fn test_restore_from_manifest_restores_files() {
        let workspace = TempDir::new().expect("Failed to create temp dir");
        let workspace_root = workspace.path();

        // Create test files
        let test_file = workspace_root.join("test.txt");
        fs::write(&test_file, b"original content").expect("Failed to write test file");

        // Create backup
        let mut writer = BackupWriter::new(workspace_root, Some("restore-test".to_string()))
            .expect("Failed to create BackupWriter");

        writer
            .backup_file(&test_file)
            .expect("Failed to backup file");

        let manifest_path = writer.finalize().expect("Failed to finalize backup");

        // Modify original file
        fs::write(&test_file, b"modified content").expect("Failed to modify file");

        // Restore from backup
        let restored = restore_from_manifest(&manifest_path, workspace_root)
            .expect("Failed to restore");

        assert_eq!(restored, 1, "Should restore one file");

        // Verify content was restored
        let content = fs::read_to_string(&test_file).expect("Failed to read file");
        assert_eq!(content, "original content", "Content should be restored");
    }

    #[test]
    fn test_restore_hash_mismatch_fails() {
        let workspace = TempDir::new().expect("Failed to create temp dir");
        let workspace_root = workspace.path();

        // Create test file
        let test_file = workspace_root.join("test.txt");
        fs::write(&test_file, b"original").expect("Failed to write test file");

        // Create backup
        let mut writer = BackupWriter::new(workspace_root, Some("hash-test".to_string()))
            .expect("Failed to create BackupWriter");

        writer
            .backup_file(&test_file)
            .expect("Failed to backup file");

        let manifest_path = writer.finalize().expect("Failed to finalize backup");

        // Tamper with the backup file
        let backup_file = workspace_root.join(".splice-backup/hash-test/test.txt");
        fs::write(&backup_file, b"tampered").expect("Failed to tamper with backup");

        // Attempt to restore should fail due to hash mismatch
        let result = restore_from_manifest(&manifest_path, workspace_root);

        assert!(result.is_err(), "Restore should fail on hash mismatch");
        match result {
            Err(SpliceError::Other(msg)) if msg.contains("Hash mismatch") => {
                // Expected
            }
            other => {
                panic!("Expected hash mismatch error, got: {:?}", other);
            }
        }
    }

    #[test]
    fn test_backup_with_subdirectories() {
        let workspace = TempDir::new().expect("Failed to create temp dir");
        let workspace_root = workspace.path();

        // Create directory structure
        let src_dir = workspace_root.join("src");
        fs::create_dir(&src_dir).expect("Failed to create src dir");

        let test_file = src_dir.join("lib.rs");
        fs::write(&test_file, b"fn main() {}").expect("Failed to write test file");

        // Create backup
        let mut writer = BackupWriter::new(workspace_root, Some("subdir-test".to_string()))
            .expect("Failed to create BackupWriter");

        writer
            .backup_file(&test_file)
            .expect("Failed to backup file");

        let manifest_path = writer.finalize().expect("Failed to finalize backup");

        // Verify backup preserves directory structure
        let backup_file = workspace_root.join(".splice-backup/subdir-test/src/lib.rs");
        assert!(backup_file.exists(), "Backup should preserve directory structure");

        // Verify can be restored
        fs::write(&test_file, b"modified").expect("Failed to modify");
        let restored = restore_from_manifest(&manifest_path, workspace_root)
            .expect("Failed to restore");

        assert_eq!(restored, 1);
        let content = fs::read_to_string(&test_file).expect("Failed to read");
        assert_eq!(content, "fn main() {}");
    }

    #[test]
    fn test_manifest_save_and_load() {
        let workspace = TempDir::new().expect("Failed to create temp dir");
        let workspace_root = workspace.path();

        let mut manifest = BackupManifest::new(
            "test-manifest".to_string(),
            workspace_root.join(".splice-backup").join("test-manifest"),
        );

        manifest.add_file(
            PathBuf::from("src/lib.rs"),
            "abc123".to_string(),
            1024,
        );

        let manifest_path = workspace_root.join(".splice-backup/test-manifest/manifest.json");

        // Save
        fs::create_dir_all(manifest_path.parent().unwrap())
            .expect("Failed to create dir");
        manifest.save().expect("Failed to save manifest");

        // Load
        let loaded = BackupManifest::load(&manifest_path).expect("Failed to load manifest");

        assert_eq!(loaded.operation_id, "test-manifest");
        assert_eq!(loaded.files.len(), 1);
        assert_eq!(loaded.files[0].original_path, PathBuf::from("src/lib.rs"));
        assert_eq!(loaded.files[0].hash, "abc123");
        assert_eq!(loaded.files[0].size, 1024);
    }
}
