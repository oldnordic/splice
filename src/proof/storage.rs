//! Snapshot storage management.
//!
//! This module provides persistent storage for code graph snapshots,
//! with support for metadata tracking, listing, and cleanup.

use crate::error::{Result, SpliceError};
use crate::proof::data_structures::GraphSnapshot;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};

/// Snapshots directory relative to project root.
const SNAPSHOTS_DIR: &str = ".splice/snapshots/";

/// Metadata about a stored snapshot.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SnapshotMetadata {
    /// Operation type that created this snapshot (e.g., "patch", "rename", "delete")
    pub operation: String,
    /// Timestamp when snapshot was created (Unix epoch)
    pub timestamp: i64,
    /// Path to the database this snapshot was taken from
    pub database_path: PathBuf,
    /// Path to the snapshot file on disk
    pub snapshot_path: PathBuf,
    /// Number of symbols in the snapshot
    pub symbols_count: usize,
    /// Number of edges in the snapshot
    pub edges_count: usize,
}

/// Result of a database restore operation.
#[derive(Debug, Clone)]
pub struct RestoreResult {
    /// Path to the backup file created before restore
    pub backup_path: PathBuf,
    /// Number of symbols restored
    pub symbols_restored: usize,
    /// Number of edges restored
    pub edges_restored: usize,
}

/// Snapshot storage manager.
///
/// Handles creating, loading, listing, and cleaning up snapshots
/// in the `.splice/snapshots/` directory.
pub struct SnapshotStorage {
    /// Base directory for snapshot storage
    base_dir: PathBuf,
}

impl SnapshotStorage {
    /// Create a new snapshot storage manager.
    ///
    /// Creates the `.splice/snapshots/` directory if it doesn't exist.
    ///
    /// # Errors
    ///
    /// Returns `SpliceError::SnapshotDirNotCreated` if directory creation fails.
    pub fn new() -> Result<Self> {
        let base_dir = PathBuf::from(SNAPSHOTS_DIR);

        // Create directory if it doesn't exist
        if !base_dir.exists() {
            fs::create_dir_all(&base_dir).map_err(|e| SpliceError::Io {
                path: base_dir.clone(),
                source: e,
            })?;
        }

        Ok(Self { base_dir })
    }

    /// Get the base snapshots directory.
    pub fn base_dir(&self) -> &Path {
        &self.base_dir
    }

    /// Save a snapshot to disk.
    ///
    /// # Arguments
    ///
    /// * `operation` - The operation type (e.g., "patch", "rename")
    /// * `db_path` - Path to the database this snapshot was taken from
    /// * `snapshot` - The graph snapshot to save
    ///
    /// # Returns
    ///
    /// Metadata about the saved snapshot including its file path.
    ///
    /// # Errors
    ///
    /// Returns `SpliceError::Io` if file writing fails.
    pub fn save_snapshot(
        &self,
        operation: &str,
        db_path: &Path,
        snapshot: GraphSnapshot,
    ) -> Result<SnapshotMetadata> {
        // Generate filename: {operation}-{timestamp}.json
        let timestamp = chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Secs, true);
        let filename = format!("{}-{}.json", operation, timestamp);
        let snapshot_path = self.base_dir.join(&filename);

        // Create metadata
        let symbols_count = snapshot.symbols.len();
        let edges_count = snapshot.edges.len();
        let metadata = SnapshotMetadata {
            operation: operation.to_string(),
            timestamp: snapshot.timestamp,
            database_path: db_path.to_path_buf(),
            snapshot_path: snapshot_path.clone(),
            symbols_count,
            edges_count,
        };

        // Serialize snapshot to JSON
        let json = serde_json::to_string_pretty(&snapshot)
            .map_err(|e| SpliceError::Other(format!("Failed to serialize snapshot: {}", e)))?;

        // Write to file
        fs::write(&snapshot_path, json).map_err(|e| SpliceError::Io {
            path: snapshot_path.clone(),
            source: e,
        })?;

        Ok(metadata)
    }

    /// Load a snapshot from disk.
    ///
    /// # Arguments
    ///
    /// * `path` - Path to the snapshot file
    ///
    /// # Returns
    ///
    /// The loaded graph snapshot.
    ///
    /// # Errors
    ///
    /// Returns `SpliceError::SnapshotNotFound` if the file doesn't exist.
    /// Returns `SpliceError::SnapshotCorrupted` if JSON parsing fails.
    pub fn load_snapshot(&self, path: &Path) -> Result<GraphSnapshot> {
        if !path.exists() {
            return Err(SpliceError::Other(format!(
                "Snapshot not found: {}",
                path.display()
            )));
        }

        let json = fs::read_to_string(path).map_err(|e| SpliceError::Io {
            path: path.to_path_buf(),
            source: e,
        })?;

        serde_json::from_str(&json).map_err(|e| {
            SpliceError::Other(format!("Snapshot corrupted: {} - {}", path.display(), e))
        })
    }

    /// List all snapshots in the storage directory.
    ///
    /// # Returns
    ///
    /// A vector of metadata for all snapshots, ordered by timestamp
    /// (newest first).
    ///
    /// # Errors
    ///
    /// Returns `SpliceError::Io` if directory reading fails.
    pub fn list_snapshots(&self) -> Result<Vec<SnapshotMetadata>> {
        let mut snapshots = Vec::new();

        let entries = fs::read_dir(&self.base_dir).map_err(|e| SpliceError::Io {
            path: self.base_dir.clone(),
            source: e,
        })?;

        for entry in entries {
            let entry = entry.map_err(|e| SpliceError::Io {
                path: self.base_dir.clone(),
                source: e,
            })?;

            let path = entry.path();
            if path.extension().and_then(|s| s.to_str()) != Some("json") {
                continue;
            }

            // Load snapshot to get metadata
            match self.load_snapshot(&path) {
                Ok(snapshot) => {
                    // Parse operation and timestamp from filename
                    let filename = path.file_stem().and_then(|s| s.to_str()).unwrap_or("");

                    let (operation, _) = filename.split_once('-').unwrap_or((filename, ""));

                    let metadata = SnapshotMetadata {
                        operation: operation.to_string(),
                        timestamp: snapshot.timestamp,
                        database_path: PathBuf::new(), // Not stored in snapshot file
                        snapshot_path: path,
                        symbols_count: snapshot.symbols.len(),
                        edges_count: snapshot.edges.len(),
                    };
                    snapshots.push(metadata);
                }
                Err(_) => {
                    // Skip corrupted snapshots
                    continue;
                }
            }
        }

        // Sort by timestamp (newest first)
        snapshots.sort_by_key(|b| std::cmp::Reverse(b.timestamp));

        Ok(snapshots)
    }

    /// Get the most recent snapshot.
    ///
    /// # Returns
    ///
    /// `Ok(Some(metadata))` if a snapshot exists, `Ok(None)` if no snapshots exist.
    ///
    /// # Errors
    ///
    /// Returns `SpliceError::Io` if directory reading fails.
    pub fn get_latest_snapshot(&self) -> Result<Option<SnapshotMetadata>> {
        let snapshots = self.list_snapshots()?;
        Ok(snapshots.into_iter().next())
    }

    /// Clean up old snapshots, keeping only the N most recent.
    ///
    /// # Arguments
    ///
    /// * `keep_count` - Number of snapshots to keep (most recent)
    ///
    /// # Returns
    ///
    /// List of paths to deleted snapshot files.
    ///
    /// # Errors
    ///
    /// Returns `SpliceError::Io` if file deletion fails.
    pub fn cleanup_old_snapshots(&self, keep_count: usize) -> Result<Vec<PathBuf>> {
        let mut snapshots = self.list_snapshots()?;
        let mut deleted_paths = Vec::new();

        if snapshots.len() <= keep_count {
            return Ok(deleted_paths);
        }

        // Sort by timestamp descending (newest first) to keep the most recent
        snapshots.sort_by_key(|b| std::cmp::Reverse(b.timestamp));

        // Delete snapshots beyond keep_count
        for snapshot in snapshots.into_iter().skip(keep_count) {
            fs::remove_file(&snapshot.snapshot_path).map_err(|e| SpliceError::Io {
                path: snapshot.snapshot_path.clone(),
                source: e,
            })?;
            deleted_paths.push(snapshot.snapshot_path);
        }

        Ok(deleted_paths)
    }

    /// Delete a snapshot by its ID (timestamp-based filename).
    ///
    /// # Arguments
    ///
    /// * `snapshot_id` - Snapshot identifier (timestamp or filename)
    ///
    /// # Returns
    ///
    /// `Ok(true)` if snapshot was deleted, `Ok(false)` if not found.
    ///
    /// # Errors
    ///
    /// Returns `SpliceError::Io` if file deletion fails.
    pub fn delete_by_id(&self, snapshot_id: &str) -> Result<bool> {
        // First, try to find the snapshot by exact filename match
        let snapshot_path = self.find_snapshot_path(snapshot_id)?;

        match snapshot_path {
            Some(path) => {
                fs::remove_file(&path).map_err(|e| SpliceError::Io {
                    path: path.clone(),
                    source: e,
                })?;
                Ok(true)
            }
            None => Ok(false),
        }
    }

    /// Get a snapshot by its ID (timestamp-based filename).
    ///
    /// # Arguments
    ///
    /// * `snapshot_id` - Snapshot identifier (timestamp or filename)
    ///
    /// # Returns
    ///
    /// `Ok(Some((path, metadata)))` if found, `Ok(None)` if not found.
    ///
    /// # Errors
    ///
    /// Returns `SpliceError::Io` if directory reading fails.
    pub fn get_by_id(&self, snapshot_id: &str) -> Result<Option<(PathBuf, SnapshotMetadata)>> {
        let snapshot_path = self.find_snapshot_path(snapshot_id)?;

        match snapshot_path {
            Some(path) => {
                // Load the snapshot to get metadata
                let snapshot = self.load_snapshot(&path)?;

                // Parse operation and timestamp from filename
                let filename = path.file_stem().and_then(|s| s.to_str()).unwrap_or("");

                let (operation, _) = filename.split_once('-').unwrap_or((filename, ""));

                let metadata = SnapshotMetadata {
                    operation: operation.to_string(),
                    timestamp: snapshot.timestamp,
                    database_path: PathBuf::new(),
                    snapshot_path: path.clone(),
                    symbols_count: snapshot.symbols.len(),
                    edges_count: snapshot.edges.len(),
                };

                Ok(Some((path, metadata)))
            }
            None => Ok(None),
        }
    }

    /// List snapshots with optional filtering.
    ///
    /// # Arguments
    ///
    /// * `operation_filter` - Optional filter by operation type (e.g., "patch", "rename")
    /// * `limit` - Optional maximum number of snapshots to return
    ///
    /// # Returns
    ///
    /// Filtered and limited list of snapshot metadata, ordered by timestamp (newest first).
    ///
    /// # Errors
    ///
    /// Returns `SpliceError::Io` if directory reading fails.
    pub fn list_snapshots_filtered(
        &self,
        operation_filter: Option<&str>,
        limit: Option<usize>,
    ) -> Result<Vec<SnapshotMetadata>> {
        let mut snapshots = self.list_snapshots()?;

        // Apply operation filter if specified
        if let Some(filter) = operation_filter {
            snapshots.retain(|s| s.operation == filter);
        }

        // Apply limit if specified
        if let Some(limit) = limit {
            snapshots.truncate(limit);
        }

        Ok(snapshots)
    }

    /// Get the total disk usage of all snapshots.
    ///
    /// # Returns
    ///
    /// Total size in bytes of all snapshot files.
    ///
    /// # Errors
    ///
    /// Returns `SpliceError::Io` if directory reading or file metadata access fails.
    pub fn get_total_size(&self) -> Result<u64> {
        let entries = fs::read_dir(&self.base_dir).map_err(|e| SpliceError::Io {
            path: self.base_dir.clone(),
            source: e,
        })?;

        let mut total_size = 0u64;

        for entry in entries {
            let entry = entry.map_err(|e| SpliceError::Io {
                path: self.base_dir.clone(),
                source: e,
            })?;

            let path = entry.path();
            if path.extension().and_then(|s| s.to_str()) != Some("json") {
                continue;
            }

            let metadata = entry.metadata().map_err(|e| SpliceError::Io {
                path: path.clone(),
                source: e,
            })?;

            total_size += metadata.len();
        }

        Ok(total_size)
    }

    /// Find a snapshot file path by ID (timestamp or filename).
    ///
    /// # Arguments
    ///
    /// * `snapshot_id` - Snapshot identifier (timestamp or filename)
    ///
    /// # Returns
    ///
    /// `Ok(Some(path))` if found, `Ok(None)` if not found.
    ///
    /// # Errors
    ///
    /// Returns `SpliceError::Io` if directory reading fails.
    fn find_snapshot_path(&self, snapshot_id: &str) -> Result<Option<PathBuf>> {
        let entries = fs::read_dir(&self.base_dir).map_err(|e| SpliceError::Io {
            path: self.base_dir.clone(),
            source: e,
        })?;

        // Normalize the snapshot_id (remove .json extension if present)
        let normalized_id = snapshot_id.strip_suffix(".json").unwrap_or(snapshot_id);

        for entry in entries {
            let entry = entry.map_err(|e| SpliceError::Io {
                path: self.base_dir.clone(),
                source: e,
            })?;

            let path = entry.path();
            if path.extension().and_then(|s| s.to_str()) != Some("json") {
                continue;
            }

            // Check if filename matches
            let filename = path.file_stem().and_then(|s| s.to_str()).unwrap_or("");

            // Match exact filename or timestamp portion
            if filename == normalized_id || filename.ends_with(&format!("-{}", normalized_id)) {
                return Ok(Some(path));
            }
        }

        Ok(None)
    }

    /// Restore a database from a snapshot file.
    ///
    /// # Arguments
    ///
    /// * `db_path` - Path to the database file to restore
    /// * `snapshot_path` - Path to the snapshot file to restore from
    ///
    /// # Returns
    ///
    /// `Ok(RestoreResult)` with backup path and restored symbol/edge counts
    ///
    /// # Errors
    ///
    /// - If snapshot file doesn't exist or is corrupted
    pub fn restore_from_snapshot(_db_path: &Path, _snapshot_path: &Path) -> Result<RestoreResult> {
        Err(SpliceError::Other(
            "Database snapshot restore is disabled.".to_string(),
        ))
    }
}

impl Default for SnapshotStorage {
    fn default() -> Self {
        Self::new().expect("Failed to create snapshot storage")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;
    use tempfile::TempDir;

    fn test_storage() -> (TempDir, SnapshotStorage) {
        let temp_dir = TempDir::new().unwrap();
        let base_dir = temp_dir.path().join(".splice/snapshots");
        fs::create_dir_all(&base_dir).unwrap();
        (temp_dir, SnapshotStorage { base_dir })
    }

    #[test]
    fn test_snapshot_storage_creation() {
        let (_temp_dir, storage) = test_storage();
        assert!(storage.base_dir().exists());
    }

    #[test]
    fn test_save_and_load_snapshot() {
        let (_temp_dir, storage) = test_storage();

        // Create a test snapshot
        let snapshot = GraphSnapshot {
            timestamp: chrono::Utc::now().timestamp(),
            symbols: HashMap::new(),
            edges: HashMap::new(),
            entry_points: Vec::new(),
            stats: crate::proof::data_structures::GraphStats {
                total_symbols: 0,
                total_edges: 0,
                entry_point_count: 0,
                max_complexity: None,
            },
        };

        // Save snapshot
        let metadata = storage
            .save_snapshot("test", Path::new(".magellan/splice.db"), snapshot.clone())
            .unwrap();

        assert!(metadata.snapshot_path.exists());
        assert_eq!(metadata.operation, "test");

        // Load snapshot
        let loaded = storage.load_snapshot(&metadata.snapshot_path).unwrap();
        assert_eq!(loaded.timestamp, snapshot.timestamp);
    }

    #[test]
    fn test_list_snapshots() {
        let (_temp_dir, storage) = test_storage();

        // Create test snapshots
        for i in 0..3 {
            let snapshot = GraphSnapshot {
                timestamp: chrono::Utc::now().timestamp() + i as i64,
                symbols: HashMap::new(),
                edges: HashMap::new(),
                entry_points: Vec::new(),
                stats: crate::proof::data_structures::GraphStats {
                    total_symbols: 0,
                    total_edges: 0,
                    entry_point_count: 0,
                    max_complexity: None,
                },
            };

            storage
                .save_snapshot(
                    &format!("test_{}", i),
                    Path::new(".magellan/splice.db"),
                    snapshot,
                )
                .unwrap();
        }

        // List snapshots
        let snapshots = storage.list_snapshots().unwrap();
        assert!(snapshots.len() >= 3);

        // Check ordering (newest first)
        for i in 0..snapshots.len() - 1 {
            assert!(snapshots[i].timestamp >= snapshots[i + 1].timestamp);
        }
    }

    #[test]
    fn test_cleanup_old_snapshots() {
        let (_temp_dir, storage) = test_storage();

        // Create test snapshots
        for i in 0..5 {
            let snapshot = GraphSnapshot {
                timestamp: chrono::Utc::now().timestamp() + i as i64,
                symbols: HashMap::new(),
                edges: HashMap::new(),
                entry_points: Vec::new(),
                stats: crate::proof::data_structures::GraphStats {
                    total_symbols: 0,
                    total_edges: 0,
                    entry_point_count: 0,
                    max_complexity: None,
                },
            };

            storage
                .save_snapshot(
                    &format!("cleanup_{}", i),
                    Path::new(".magellan/splice.db"),
                    snapshot,
                )
                .unwrap();
        }

        // Keep only 3 most recent
        storage.cleanup_old_snapshots(3).unwrap();

        let snapshots = storage.list_snapshots().unwrap();
        assert!(snapshots.len() <= 3);
    }

    #[test]
    fn test_get_latest_snapshot() {
        let (_temp_dir, storage) = test_storage();

        // Create a snapshot with a future timestamp to ensure it's the latest
        let future_timestamp = chrono::Utc::now().timestamp() + 1000;
        let snapshot = GraphSnapshot {
            timestamp: future_timestamp,
            symbols: HashMap::new(),
            edges: HashMap::new(),
            entry_points: Vec::new(),
            stats: crate::proof::data_structures::GraphStats {
                total_symbols: 0,
                total_edges: 0,
                entry_point_count: 0,
                max_complexity: None,
            },
        };

        storage
            .save_snapshot(
                "latest_test_unique",
                Path::new(".magellan/splice.db"),
                snapshot,
            )
            .unwrap();

        // Should have a latest snapshot now (and it should be ours)
        let latest = storage.get_latest_snapshot().unwrap();
        assert!(latest.is_some());
        assert_eq!(latest.unwrap().operation, "latest_test_unique");
    }
}
