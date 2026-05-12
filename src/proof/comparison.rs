//! Snapshot comparison module.
//!
//! This module provides functionality to compare two graph snapshots
//! and detect changes in symbols, edges, and invariants.

use crate::error::Result;
use crate::proof::data_structures::{GraphSnapshot, InvariantCheck, SymbolInfo};
use crate::proof::validation;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;

/// Type of change detected between snapshots.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ChangeType {
    /// Symbol/edge was added
    Added,
    /// Symbol/edge was removed
    Removed,
    /// Symbol was modified (name, kind, file_path, or spans changed)
    Modified,
    /// No change detected
    Unchanged,
}

/// Difference information for a single symbol.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SymbolDiff {
    /// Unique symbol ID
    pub id: String,
    /// Symbol name (from the "after" snapshot if modified)
    pub name: String,
    /// Type of change detected
    pub change_type: ChangeType,
    /// Symbol info before the change (None if added)
    pub before: Option<SymbolInfo>,
    /// Symbol info after the change (None if removed)
    pub after: Option<SymbolInfo>,
}

/// Difference information for a single edge.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EdgeDiff {
    /// Caller symbol ID
    pub from: String,
    /// Callee symbol ID
    pub to: String,
    /// Type of change detected
    pub change_type: ChangeType,
}

/// Complete diff between two graph snapshots.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SnapshotDiff {
    /// Timestamp of the "before" snapshot
    pub before_timestamp: i64,
    /// Timestamp of the "after" snapshot
    pub after_timestamp: i64,
    /// Number of symbols added
    pub symbols_added: usize,
    /// Number of symbols removed
    pub symbols_removed: usize,
    /// Number of symbols modified
    pub symbols_modified: usize,
    /// Number of edges added
    pub edges_added: usize,
    /// Number of edges removed
    pub edges_removed: usize,
    /// Detailed symbol differences
    pub symbol_details: Vec<SymbolDiff>,
    /// Detailed edge differences
    pub edge_details: Vec<EdgeDiff>,
    /// Invariant validation results
    pub invariant_results: Vec<InvariantCheck>,
}

/// Compare two graph snapshots and detect differences.
///
/// This function performs a comprehensive comparison of two snapshots:
/// - Detects added/removed/modified symbols
/// - Detects added/removed call relationships (edges)
/// - Runs invariant validation on the pair
///
/// # Arguments
///
/// * `before` - Graph snapshot before the change
/// * `after` - Graph snapshot after the change
///
/// # Returns
///
/// A complete `SnapshotDiff` containing all detected changes and invariant results.
///
/// # Examples
///
/// ```no_run
/// use splice::proof::{comparison::compare_snapshots, storage::SnapshotStorage};
/// # use splice::error::Result;
///
/// # fn main() -> Result<()> {
/// let storage = SnapshotStorage::new()?;
///
/// // Load snapshots
/// let before = storage.load_snapshot(std::path::Path::new("before.json"))?;
/// let after = storage.load_snapshot(std::path::Path::new("after.json"))?;
///
/// // Compare
/// let diff = compare_snapshots(&before, &after)?;
///
/// println!("Symbols added: {}", diff.symbols_added);
/// println!("Symbols removed: {}", diff.symbols_removed);
/// # Ok(())
/// # }
/// ```
pub fn compare_snapshots(before: &GraphSnapshot, after: &GraphSnapshot) -> Result<SnapshotDiff> {
    // Compare symbols
    let (symbol_diffs, added, removed, modified) = diff_symbols(before, after);

    // Compare edges
    let (edge_diffs, edges_added, edges_removed) = diff_edges(before, after);

    // Run invariant validation
    let invariant_results = validation::validate_invariants(before, after)?;

    Ok(SnapshotDiff {
        before_timestamp: before.timestamp,
        after_timestamp: after.timestamp,
        symbols_added: added,
        symbols_removed: removed,
        symbols_modified: modified,
        edges_added,
        edges_removed,
        symbol_details: symbol_diffs,
        edge_details: edge_diffs,
        invariant_results,
    })
}

/// Compare symbols between two snapshots.
///
/// Returns a tuple of:
/// - Vector of symbol differences (sorted by ID)
/// - Count of added symbols
/// - Count of removed symbols
/// - Count of modified symbols
fn diff_symbols(
    before: &GraphSnapshot,
    after: &GraphSnapshot,
) -> (Vec<SymbolDiff>, usize, usize, usize) {
    let before_ids: HashSet<String> = before.symbols.keys().cloned().collect();
    let after_ids: HashSet<String> = after.symbols.keys().cloned().collect();

    // Find added IDs (in after but not before)
    let added_ids: Vec<_> = after_ids.difference(&before_ids).cloned().collect();

    // Find removed IDs (in before but not after)
    let removed_ids: Vec<_> = before_ids.difference(&after_ids).cloned().collect();

    // Find common IDs (in both)
    let common_ids: Vec<_> = before_ids.intersection(&after_ids).cloned().collect();

    // Check common symbols for modifications
    let mut symbol_diffs = Vec::new();
    let mut modified_count = 0;

    // Added symbols
    for id in &added_ids {
        if let Some(after_sym) = after.symbols.get(id) {
            symbol_diffs.push(SymbolDiff {
                id: id.clone(),
                name: after_sym.name.clone(),
                change_type: ChangeType::Added,
                before: None,
                after: Some(after_sym.clone()),
            });
        }
    }

    // Removed symbols
    for id in &removed_ids {
        if let Some(before_sym) = before.symbols.get(id) {
            symbol_diffs.push(SymbolDiff {
                id: id.clone(),
                name: before_sym.name.clone(),
                change_type: ChangeType::Removed,
                before: Some(before_sym.clone()),
                after: None,
            });
        }
    }

    // Check for modifications in common symbols
    for id in &common_ids {
        let before_sym = before.symbols.get(id).unwrap();
        let after_sym = after.symbols.get(id).unwrap();

        // A symbol is modified if any of these changed:
        // - name, file_path, kind, byte_span, fan_in, or fan_out
        if before_sym.name != after_sym.name
            || before_sym.file_path != after_sym.file_path
            || before_sym.kind != after_sym.kind
            || before_sym.byte_span != after_sym.byte_span
            || before_sym.fan_in != after_sym.fan_in
            || before_sym.fan_out != after_sym.fan_out
        {
            modified_count += 1;
            symbol_diffs.push(SymbolDiff {
                id: id.clone(),
                name: after_sym.name.clone(),
                change_type: ChangeType::Modified,
                before: Some(before_sym.clone()),
                after: Some(after_sym.clone()),
            });
        }
    }

    // Sort by ID for consistent output
    symbol_diffs.sort_by(|a, b| a.id.cmp(&b.id));

    (
        symbol_diffs,
        added_ids.len(),
        removed_ids.len(),
        modified_count,
    )
}

/// Compare edges between two snapshots.
///
/// Returns a tuple of:
/// - Vector of edge differences (sorted by from->to)
/// - Count of added edges
/// - Count of removed edges
fn diff_edges(before: &GraphSnapshot, after: &GraphSnapshot) -> (Vec<EdgeDiff>, usize, usize) {
    let mut edge_diffs = Vec::new();

    // Build sets of all edges in both snapshots
    // An edge is represented as (from_id, to_id)
    let mut before_edges: HashSet<(String, String)> = HashSet::new();
    for (from_id, to_ids) in &before.edges {
        for to_id in to_ids {
            before_edges.insert((from_id.clone(), to_id.clone()));
        }
    }

    let mut after_edges: HashSet<(String, String)> = HashSet::new();
    for (from_id, to_ids) in &after.edges {
        for to_id in to_ids {
            after_edges.insert((from_id.clone(), to_id.clone()));
        }
    }

    // Find added edges (in after but not before)
    let added_edges: Vec<_> = after_edges.difference(&before_edges).cloned().collect();

    // Find removed edges (in before but not after)
    let removed_edges: Vec<_> = before_edges.difference(&after_edges).cloned().collect();

    // Convert to EdgeDiff structs
    for (from, to) in &added_edges {
        edge_diffs.push(EdgeDiff {
            from: from.clone(),
            to: to.clone(),
            change_type: ChangeType::Added,
        });
    }

    for (from, to) in &removed_edges {
        edge_diffs.push(EdgeDiff {
            from: from.clone(),
            to: to.clone(),
            change_type: ChangeType::Removed,
        });
    }

    // Sort by from->to for consistent output
    edge_diffs.sort_by(|a, b| match a.from.cmp(&b.from) {
        std::cmp::Ordering::Equal => a.to.cmp(&b.to),
        other => other,
    });

    (edge_diffs, added_edges.len(), removed_edges.len())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::proof::data_structures::GraphStats;
    use std::collections::HashMap;

    fn create_test_snapshot(timestamp: i64) -> GraphSnapshot {
        let mut symbols = HashMap::new();

        symbols.insert(
            "0000000000000001".to_string(),
            SymbolInfo {
                id: "0000000000000001".to_string(),
                name: "main".to_string(),
                file_path: "src/main.rs".to_string(),
                kind: "fn".to_string(),
                byte_span: (0, 100),
                fan_in: 0,
                fan_out: 1,
            },
        );

        symbols.insert(
            "0000000000000002".to_string(),
            SymbolInfo {
                id: "0000000000000002".to_string(),
                name: "helper".to_string(),
                file_path: "src/helper.rs".to_string(),
                kind: "fn".to_string(),
                byte_span: (0, 50),
                fan_in: 1,
                fan_out: 0,
            },
        );

        let mut edges = HashMap::new();
        edges.insert(
            "0000000000000001".to_string(),
            vec!["0000000000000002".to_string()],
        );
        edges.insert("0000000000000002".to_string(), vec![]);

        GraphSnapshot {
            timestamp,
            symbols,
            edges,
            entry_points: vec!["0000000000000001".to_string()],
            stats: GraphStats {
                total_symbols: 2,
                total_edges: 1,
                entry_point_count: 1,
                max_complexity: None,
            },
        }
    }

    #[test]
    fn test_compare_identical_snapshots() {
        let snapshot = create_test_snapshot(1000);
        let diff = compare_snapshots(&snapshot, &snapshot).unwrap();

        assert_eq!(diff.symbols_added, 0);
        assert_eq!(diff.symbols_removed, 0);
        assert_eq!(diff.symbols_modified, 0);
        assert_eq!(diff.edges_added, 0);
        assert_eq!(diff.edges_removed, 0);
        assert!(diff.symbol_details.is_empty());
        assert!(diff.edge_details.is_empty());
    }

    #[test]
    fn test_detect_added_symbol() {
        let before = create_test_snapshot(1000);
        let mut after = create_test_snapshot(2000);

        // Add a new symbol
        after.symbols.insert(
            "0000000000000003".to_string(),
            SymbolInfo {
                id: "0000000000000003".to_string(),
                name: "new_func".to_string(),
                file_path: "src/new.rs".to_string(),
                kind: "fn".to_string(),
                byte_span: (0, 30),
                fan_in: 0,
                fan_out: 0,
            },
        );
        after.stats.total_symbols = 3;

        let diff = compare_snapshots(&before, &after).unwrap();

        assert_eq!(diff.symbols_added, 1);
        assert_eq!(diff.symbols_removed, 0);
        assert_eq!(diff.symbols_modified, 0);

        // Check the added symbol details
        let added = &diff.symbol_details[0];
        assert_eq!(added.id, "0000000000000003");
        assert_eq!(added.name, "new_func");
        assert_eq!(added.change_type, ChangeType::Added);
        assert!(added.before.is_none());
        assert!(added.after.is_some());
    }

    #[test]
    fn test_detect_removed_symbol() {
        let before = create_test_snapshot(1000);
        let mut after = create_test_snapshot(2000);

        // Remove a symbol
        after.symbols.remove("0000000000000002");
        after.edges.remove("0000000000000001");
        after.edges.insert("0000000000000001".to_string(), vec![]);
        after.stats.total_symbols = 2;
        after.stats.total_edges = 0;

        let diff = compare_snapshots(&before, &after).unwrap();

        assert_eq!(diff.symbols_added, 0);
        assert_eq!(diff.symbols_removed, 1);
        assert_eq!(diff.symbols_modified, 0);

        // Check the removed symbol details
        let removed = &diff.symbol_details[0];
        assert_eq!(removed.id, "0000000000000002");
        assert_eq!(removed.name, "helper");
        assert_eq!(removed.change_type, ChangeType::Removed);
        assert!(removed.before.is_some());
        assert!(removed.after.is_none());
    }

    #[test]
    fn test_detect_modified_symbol() {
        let before = create_test_snapshot(1000);
        let mut after = create_test_snapshot(2000);

        // Modify a symbol (change its name)
        if let Some(sym) = after.symbols.get_mut("0000000000000002") {
            sym.name = "helper_v2".to_string();
        }

        let diff = compare_snapshots(&before, &after).unwrap();

        assert_eq!(diff.symbols_added, 0);
        assert_eq!(diff.symbols_removed, 0);
        assert_eq!(diff.symbols_modified, 1);

        // Check the modified symbol details
        let modified = &diff.symbol_details[0];
        assert_eq!(modified.id, "0000000000000002");
        assert_eq!(modified.name, "helper_v2");
        assert_eq!(modified.change_type, ChangeType::Modified);
        assert!(modified.before.is_some());
        assert!(modified.after.is_some());
        assert_eq!(modified.before.as_ref().unwrap().name, "helper");
        assert_eq!(modified.after.as_ref().unwrap().name, "helper_v2");
    }

    #[test]
    fn test_detect_added_edge() {
        let before = create_test_snapshot(1000);
        let mut after = create_test_snapshot(2000);

        // Add an edge (make helper call something else)
        // Add a new symbol first
        after.symbols.insert(
            "0000000000000003".to_string(),
            SymbolInfo {
                id: "0000000000000003".to_string(),
                name: "util".to_string(),
                file_path: "src/util.rs".to_string(),
                kind: "fn".to_string(),
                byte_span: (0, 30),
                fan_in: 1,
                fan_out: 0,
            },
        );

        // Add edge from helper to util
        after.edges.insert(
            "0000000000000002".to_string(),
            vec!["0000000000000003".to_string()],
        );
        after.stats.total_symbols = 3;
        after.stats.total_edges = 2;

        let diff = compare_snapshots(&before, &after).unwrap();

        assert_eq!(diff.symbols_added, 1);
        assert_eq!(diff.edges_added, 1);
        assert_eq!(diff.edges_removed, 0);

        // Check the added edge
        let added_edge = &diff.edge_details[0];
        assert_eq!(added_edge.from, "0000000000000002");
        assert_eq!(added_edge.to, "0000000000000003");
        assert_eq!(added_edge.change_type, ChangeType::Added);
    }

    #[test]
    fn test_detect_removed_edge() {
        let before = create_test_snapshot(1000);
        let mut after = create_test_snapshot(2000);

        // Remove the edge from main to helper
        after.edges.insert("0000000000000001".to_string(), vec![]);
        after.stats.total_edges = 0;

        // Update fan_in for helper
        if let Some(sym) = after.symbols.get_mut("0000000000000002") {
            sym.fan_in = 0;
        }

        // Update fan_out for main
        if let Some(sym) = after.symbols.get_mut("0000000000000001") {
            sym.fan_out = 0;
        }

        let diff = compare_snapshots(&before, &after).unwrap();

        assert_eq!(diff.symbols_added, 0);
        assert_eq!(diff.symbols_removed, 0);
        assert_eq!(diff.symbols_modified, 2); // Both symbols modified (fan counts changed)
        assert_eq!(diff.edges_added, 0);
        assert_eq!(diff.edges_removed, 1);

        // Check the removed edge
        let removed_edge = &diff.edge_details[0];
        assert_eq!(removed_edge.from, "0000000000000001");
        assert_eq!(removed_edge.to, "0000000000000002");
        assert_eq!(removed_edge.change_type, ChangeType::Removed);
    }
}
