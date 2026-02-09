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
        let json = serde_json::to_string_pretty(&snapshot).map_err(|e| SpliceError::Other(format!(
            "Failed to serialize snapshot: {}",
            e
        )))?;

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

        serde_json::from_str(&json).map_err(|e| SpliceError::Other(format!(
            "Snapshot corrupted: {} - {}",
            path.display(),
            e
        )))
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
                    let filename = path.file_stem()
                        .and_then(|s| s.to_str())
                        .unwrap_or("");

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
        snapshots.sort_by(|a, b| b.timestamp.cmp(&a.timestamp));

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
    /// # Errors
    ///
    /// Returns `SpliceError::Io` if file deletion fails.
    pub fn cleanup_old_snapshots(&self, keep_count: usize) -> Result<()> {
        let snapshots = self.list_snapshots()?;

        if snapshots.len() <= keep_count {
            return Ok(());
        }

        // Delete snapshots beyond keep_count
        for snapshot in snapshots.into_iter().skip(keep_count) {
            fs::remove_file(&snapshot.snapshot_path).map_err(|e| SpliceError::Io {
                path: snapshot.snapshot_path.clone(),
                source: e,
            })?;
        }

        Ok(())
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

    #[test]
    fn test_snapshot_storage_creation() {
        let storage = SnapshotStorage::new().unwrap();
        assert!(storage.base_dir().exists());
    }

    #[test]
    fn test_save_and_load_snapshot() {
        let storage = SnapshotStorage::new().unwrap();

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
            .save_snapshot("test", Path::new(".codemcp/codegraph.db"), snapshot.clone())
            .unwrap();

        assert!(metadata.snapshot_path.exists());
        assert_eq!(metadata.operation, "test");

        // Load snapshot
        let loaded = storage.load_snapshot(&metadata.snapshot_path).unwrap();
        assert_eq!(loaded.timestamp, snapshot.timestamp);
    }

    #[test]
    fn test_list_snapshots() {
        let storage = SnapshotStorage::new().unwrap();

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
                .save_snapshot(&format!("test_{}", i), Path::new(".codemcp/codegraph.db"), snapshot)
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
        let storage = SnapshotStorage::new().unwrap();

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
                .save_snapshot(&format!("cleanup_{}", i), Path::new(".codemcp/codegraph.db"), snapshot)
                .unwrap();
        }

        // Keep only 3 most recent
        storage.cleanup_old_snapshots(3).unwrap();

        let snapshots = storage.list_snapshots().unwrap();
        assert!(snapshots.len() <= 3);
    }

    #[test]
    fn test_get_latest_snapshot() {
        let storage = SnapshotStorage::new().unwrap();

        // No snapshots initially
        let latest = storage.get_latest_snapshot().unwrap();
        assert!(latest.is_none());

        // Create a snapshot
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

        storage
            .save_snapshot("latest_test", Path::new(".codemcp/codegraph.db"), snapshot)
            .unwrap();

        // Should have a latest snapshot now
        let latest = storage.get_latest_snapshot().unwrap();
        assert!(latest.is_some());
        assert_eq!(latest.unwrap().operation, "latest_test");
    }
}
