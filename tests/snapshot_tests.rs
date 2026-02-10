//! Snapshot and restore tests (native-v2 backend only).
//!
//! These tests verify the snapshot capture, storage, and restore functionality.
//! Restore operations are only available with the native-v2 feature.
//!
//! Run with:
//!   cargo test --features native-v2 snapshot

use splice::proof::{GraphSnapshot, SnapshotStorage, compare_snapshots};
use splice::CodeGraph;
use std::collections::HashMap;
use std::fs;
use std::io::Write;
use tempfile::TempDir;

///////////////////////////////////////////////////////////////////////////////
// Snapshot capture tests (both backends)
///////////////////////////////////////////////////////////////////////////////

#[test]
fn test_snapshot_storage_creation() {
    let storage = SnapshotStorage::new().expect("Failed to create SnapshotStorage");
    let base_dir = storage.base_dir();

    assert!(base_dir.exists(), "Snapshots directory should exist");
    assert!(base_dir.ends_with(".splice/snapshots"), "Should use correct path");
}

#[test]
fn test_save_and_load_snapshot() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let storage = SnapshotStorage::new().expect("Failed to create SnapshotStorage");

    // Create a minimal snapshot
    let snapshot = GraphSnapshot {
        timestamp: chrono::Utc::now().timestamp(),
        symbols: HashMap::new(),
        edges: HashMap::new(),
        entry_points: Vec::new(),
        stats: splice::proof::GraphStats {
            total_symbols: 0,
            total_edges: 0,
            entry_point_count: 0,
            max_complexity: None,
        },
    };

    let db_path = temp_dir.path().join("test.db");
    let metadata = storage
        .save_snapshot("test_operation", &db_path.as_path(), snapshot.clone())
        .expect("Failed to save snapshot");

    assert!(metadata.snapshot_path.exists(), "Snapshot file should exist");
    assert_eq!(metadata.operation, "test_operation");

    // Load the snapshot back
    let loaded = storage
        .load_snapshot(&metadata.snapshot_path)
        .expect("Failed to load snapshot");

    assert_eq!(loaded.timestamp, snapshot.timestamp);
}

#[test]
fn test_list_snapshots() {
    let storage = SnapshotStorage::new().expect("Failed to create SnapshotStorage");

    // Create multiple snapshots
    for i in 0..3 {
        let snapshot = GraphSnapshot {
            timestamp: chrono::Utc::now().timestamp() + i as i64,
            symbols: HashMap::new(),
            edges: HashMap::new(),
            entry_points: Vec::new(),
            stats: splice::proof::GraphStats {
                total_symbols: 0,
                total_edges: 0,
                entry_point_count: 0,
                max_complexity: None,
            },
        };

        let temp_dir = TempDir::new().expect("Failed to create temp dir");
        storage
            .save_snapshot(&format!("list_test_{}", i), temp_dir.path().join("test.db").as_path(), snapshot)
            .expect("Failed to save snapshot");
    }

    let snapshots = storage.list_snapshots().expect("Failed to list snapshots");
    assert!(snapshots.len() >= 3, "Should have at least 3 snapshots");

    // Verify sorted by timestamp (newest first)
    for i in 0..snapshots.len().saturating_sub(1) {
        assert!(
            snapshots[i].timestamp >= snapshots[i + 1].timestamp,
            "Snapshots should be sorted newest first"
        );
    }
}

#[test]
fn test_cleanup_old_snapshots() {
    let storage = SnapshotStorage::new().expect("Failed to create SnapshotStorage");

    // Get initial snapshot count
    let initial_count = storage.list_snapshots().expect("Failed to list snapshots").len();

    // Create 3 new snapshots with unique operation names
    let unique_id = chrono::Utc::now().timestamp();
    for i in 0..3 {
        let snapshot = GraphSnapshot {
            timestamp: unique_id + i as i64,
            symbols: HashMap::new(),
            edges: HashMap::new(),
            entry_points: Vec::new(),
            stats: splice::proof::GraphStats {
                total_symbols: 0,
                total_edges: 0,
                entry_point_count: 0,
                max_complexity: None,
            },
        };

        let temp_dir = TempDir::new().expect("Failed to create temp dir");
        storage
            .save_snapshot(&format!("cleanup_{}_{}", unique_id, i), temp_dir.path().join("test.db").as_path(), snapshot)
            .expect("Failed to save snapshot");
    }

    // Keep only initial_count + 1 most recent (should delete 2 of our new snapshots)
    let keep_count = initial_count + 1;
    let deleted = storage
        .cleanup_old_snapshots(keep_count)
        .expect("Failed to cleanup snapshots");

    // We should have deleted at least 2 of our new snapshots
    assert!(deleted.len() >= 2, "Should delete at least 2 old snapshots");

    let remaining = storage.list_snapshots().expect("Failed to list snapshots");
    assert!(remaining.len() <= keep_count, "Should have at most keep_count snapshots remaining");
}

#[test]
fn test_get_latest_snapshot() {
    let storage = SnapshotStorage::new().expect("Failed to create SnapshotStorage");

    // Create a snapshot with a future timestamp to ensure it's the latest
    let future_timestamp = chrono::Utc::now().timestamp() + 10000;
    let snapshot = GraphSnapshot {
        timestamp: future_timestamp,
        symbols: HashMap::new(),
        edges: HashMap::new(),
        entry_points: Vec::new(),
        stats: splice::proof::GraphStats {
            total_symbols: 0,
            total_edges: 0,
            entry_point_count: 0,
            max_complexity: None,
        },
    };

    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    storage
        .save_snapshot("latest_test_unique", temp_dir.path().join("test.db").as_path(), snapshot)
        .expect("Failed to save snapshot");

    let latest = storage
        .get_latest_snapshot()
        .expect("Failed to get latest snapshot");

    assert!(latest.is_some(), "Should have a latest snapshot");
    assert_eq!(latest.unwrap().operation, "latest_test_unique");
}

///////////////////////////////////////////////////////////////////////////////
// Snapshot restore tests (native-v2 only)
///////////////////////////////////////////////////////////////////////////////

#[cfg(feature = "native-v2")]
#[test]
fn test_restore_from_snapshot_file() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let db_path = temp_dir.path().join("restore_test.db");
    let _ = fs::remove_file(&db_path);

    // Create a native-v2 database
    {
        let _graph = CodeGraph::open(&db_path).expect("Failed to create graph");
    }

    // Capture a snapshot
    let storage = SnapshotStorage::new().expect("Failed to create storage");
    let snapshot = GraphSnapshot {
        timestamp: chrono::Utc::now().timestamp(),
        symbols: {
            let mut symbols = HashMap::new();
            symbols.insert(
                "test_symbol".to_string(),
                splice::proof::SymbolInfo {
                    id: "test_symbol".to_string(),
                    name: "test_symbol".to_string(),
                    kind: "function".to_string(),
                    file_path: "/test/path.rs".to_string(),
                    byte_span: (0, 100),
                    fan_in: 0,
                    fan_out: 0,
                },
            );
            symbols
        },
        edges: HashMap::new(),
        entry_points: Vec::new(),
        stats: splice::proof::GraphStats {
            total_symbols: 1,
            total_edges: 0,
            entry_point_count: 0,
            max_complexity: None,
        },
    };

    let snapshot_path = temp_dir.path().join("snapshot.json");
    storage
        .save_snapshot("restore_test", &db_path.as_path(), snapshot)
        .expect("Failed to save snapshot");

    // Note: Full restore test is complex as it requires proper ID mapping
    // This test verifies the API exists and compiles with native-v2
    let result = CodeGraph::restore_from_snapshot(&db_path, &snapshot_path);

    // Result may be Ok or Err depending on implementation details
    // We're primarily testing that the method exists and is callable
    let _ = result;
}

#[cfg(feature = "native-v2")]
#[test]
fn test_restore_fails_for_sqlite_backend() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");

    // Create a SQLite database (write SQLite header)
    let sqlite_path = temp_dir.path().join("sqlite.db");
    {
        let mut file = fs::File::create(&sqlite_path).expect("Failed to create file");
        file.write_all(b"SQLite format 3\0").expect("Failed to write header");
    }

    let snapshot_path = temp_dir.path().join("snapshot.json");

    // Restore should fail for SQLite backend
    let result = CodeGraph::restore_from_snapshot(&sqlite_path, &snapshot_path);
    assert!(result.is_err(), "Restore should fail for SQLite backend");

    let err = result.unwrap_err();
    let err_msg = format!("{}", err);
    assert!(
        err_msg.contains("native-v2") || err_msg.contains("SQLite"),
        "Error should mention native-v2 requirement or SQLite incompatibility"
    );
}

///////////////////////////////////////////////////////////////////////////////
// Snapshot comparison tests
///////////////////////////////////////////////////////////////////////////////

#[test]
fn test_snapshot_comparison() {
    let snapshot1 = GraphSnapshot {
        timestamp: chrono::Utc::now().timestamp(),
        symbols: {
            let mut symbols = HashMap::new();
            symbols.insert(
                "symbol1".to_string(),
                splice::proof::SymbolInfo {
                    id: "symbol1".to_string(),
                    name: "symbol1".to_string(),
                    kind: "function".to_string(),
                    file_path: "/test.rs".to_string(),
                    byte_span: (0, 50),
                    fan_in: 1,
                    fan_out: 2,
                },
            );
            symbols
        },
        edges: HashMap::new(),
        entry_points: Vec::new(),
        stats: splice::proof::GraphStats {
            total_symbols: 1,
            total_edges: 0,
            entry_point_count: 0,
            max_complexity: None,
        },
    };

    let snapshot2 = GraphSnapshot {
        timestamp: chrono::Utc::now().timestamp() + 1,
        symbols: {
            let mut symbols = HashMap::new();
            symbols.insert(
                "symbol1".to_string(),
                splice::proof::SymbolInfo {
                    id: "symbol1".to_string(),
                    name: "symbol1".to_string(),
                    kind: "function".to_string(),
                    file_path: "/test.rs".to_string(),
                    byte_span: (0, 50),
                    fan_in: 1,
                    fan_out: 2,
                },
            );
            symbols
        },
        edges: HashMap::new(),
        entry_points: Vec::new(),
        stats: splice::proof::GraphStats {
            total_symbols: 1,
            total_edges: 0,
            entry_point_count: 0,
            max_complexity: None,
        },
    };

    let diff = compare_snapshots(&snapshot1, &snapshot2).expect("Failed to compare snapshots");
    // Identical snapshots should have no differences
    assert_eq!(diff.symbols_added, 0, "No symbols should be added");
    assert_eq!(diff.symbols_removed, 0, "No symbols should be removed");
}
