//! Invariant validation for refactoring proofs.
//!
//! This module provides functions to validate that graph invariants
//! are preserved during refactoring operations.

use crate::error::Result;
use crate::proof::data_structures::{
    GraphSnapshot, InvariantCheck, InvariantViolation, ViolationSeverity,
};
use std::collections::{HashMap, HashSet};

/// Validate that refactoring invariants are preserved.
///
/// Checks:
/// - Reference counts are preserved (same number of incoming/outgoing edges)
/// - No orphaned symbols (all symbols remain reachable from entry points)
/// - Symbol IDs are stable (no new IDs generated, only name changes)
/// - Entry points are preserved (no loss of public API)
///
/// # Arguments
/// * `before` - Graph snapshot before refactoring
/// * `after` - Graph snapshot after refactoring
///
/// # Returns
/// A vector of invariant check results, one for each invariant validated.
pub fn validate_invariants(
    before: &GraphSnapshot,
    after: &GraphSnapshot,
) -> Result<Vec<InvariantCheck>> {
    let checks = vec![
        // Check 1: Reference counts are preserved
        check_reference_counts(before, after),
        // Check 2: No orphaned symbols
        check_orphaned_symbols(after),
        // Check 3: Symbol IDs are stable
        check_symbol_id_stability(before, after),
        // Check 4: Entry points are preserved
        check_entry_points(before, after),
    ];

    Ok(checks)
}

/// Check that reference counts (fan-in and fan-out) are preserved.
///
/// For a correct refactoring, each symbol should maintain the same
/// number of incoming and outgoing references, only the target names
/// should change.
fn check_reference_counts(before: &GraphSnapshot, after: &GraphSnapshot) -> InvariantCheck {
    let invariant_name = "Reference Counts Preserved";
    let mut violations = Vec::new();

    // Build a map of name -> (fan_in, fan_out) for before and after
    let before_refs: HashMap<String, (usize, usize)> = before
        .symbols
        .values()
        .map(|s| (s.name.clone(), (s.fan_in, s.fan_out)))
        .collect();

    let after_refs: HashMap<String, (usize, usize)> = after
        .symbols
        .values()
        .map(|s| (s.name.clone(), (s.fan_in, s.fan_out)))
        .collect();

    // Check that all symbols from before still exist with same reference counts
    for (name, (before_fan_in, before_fan_out)) in &before_refs {
        if let Some((after_fan_in, after_fan_out)) = after_refs.get(name) {
            // Symbol still exists, check if counts match
            if before_fan_in != after_fan_in || before_fan_out != after_fan_out {
                violations.push(InvariantViolation {
                    severity: ViolationSeverity::Error,
                    subject: name.clone(),
                    message: format!(
                        "Reference counts changed: fan-in {} -> {}, fan-out {} -> {}",
                        before_fan_in, after_fan_in, before_fan_out, after_fan_out
                    ),
                    suggestion: Some(
                        "Ensure all references to the renamed symbol were updated".to_string(),
                    ),
                });
            }
        }
        // Note: If symbol doesn't exist in after, it was renamed
        // This is OK as long as a new symbol appeared with different name
    }

    InvariantCheck {
        invariant_name: invariant_name.to_string(),
        passed: violations.is_empty(),
        violations,
    }
}

/// Check that there are no orphaned symbols.
///
/// An orphaned symbol is one that is not reachable from any entry point.
/// This typically indicates that a symbol was renamed but its callers
/// were not updated.
fn check_orphaned_symbols(after: &GraphSnapshot) -> InvariantCheck {
    let invariant_name = "No Orphaned Symbols";
    let mut violations = Vec::new();

    // Perform reachability analysis from entry points
    let mut reachable: HashSet<String> = HashSet::new();
    let mut to_visit: Vec<String> = after.entry_points.clone();

    while let Some(symbol_id) = to_visit.pop() {
        if reachable.contains(&symbol_id) {
            continue;
        }

        reachable.insert(symbol_id.clone());

        // Add all callees to the visit queue
        if let Some(callees) = after.edges.get(&symbol_id) {
            for callee_id in callees {
                if !reachable.contains(callee_id) {
                    to_visit.push(callee_id.clone());
                }
            }
        }
    }

    // Find symbols that are not reachable
    for symbol in after.symbols.values() {
        if !reachable.contains(&symbol.id) {
            // Check if this symbol has any incoming references
            // If it has fan-in > 0 but is not reachable, it's an orphan
            if symbol.fan_in > 0 {
                violations.push(InvariantViolation {
                    severity: ViolationSeverity::Warning,
                    subject: symbol.name.clone(),
                    message: format!(
                        "Symbol '{}' ({}) is not reachable from entry points but has {} incoming references",
                        symbol.name, symbol.id, symbol.fan_in
                    ),
                    suggestion: Some(
                        "Ensure all callers of this symbol were updated after rename".to_string(),
                    ),
                });
            }
        }
    }

    InvariantCheck {
        invariant_name: invariant_name.to_string(),
        passed: violations.is_empty(),
        violations,
    }
}

/// Check that symbol IDs are stable.
///
/// Symbol IDs should remain stable across refactoring - no new IDs
/// should be generated, only names should change. This ensures that
/// the refactoring was a pure renaming operation.
fn check_symbol_id_stability(before: &GraphSnapshot, after: &GraphSnapshot) -> InvariantCheck {
    let invariant_name = "Symbol IDs Stable";
    let mut violations = Vec::new();

    // The total number of symbols should remain the same
    // (or decrease if symbols were deleted, but that's a different operation)
    if after.symbols.len() != before.symbols.len() {
        violations.push(InvariantViolation {
            severity: ViolationSeverity::Warning,
            subject: "symbol_count".to_string(),
            message: format!(
                "Symbol count changed: {} -> {}",
                before.symbols.len(),
                after.symbols.len()
            ),
            suggestion: Some(
                "Symbol count should remain stable during rename operations".to_string(),
            ),
        });
    }

    // Check that no IDs were added or removed (only names should change)
    let before_ids: HashSet<&String> = before.symbols.keys().collect();
    let after_ids: HashSet<&String> = after.symbols.keys().collect();

    let added_ids: Vec<_> = after_ids.difference(&before_ids).cloned().collect();
    let removed_ids: Vec<_> = before_ids.difference(&after_ids).cloned().collect();

    if !added_ids.is_empty() || !removed_ids.is_empty() {
        violations.push(InvariantViolation {
            severity: ViolationSeverity::Error,
            subject: "symbol_ids".to_string(),
            message: format!(
                "Symbol IDs changed: added {:?}, removed {:?}",
                added_ids, removed_ids
            ),
            suggestion: Some(
                "Symbol IDs should remain stable - only names should change".to_string(),
            ),
        });
    }

    InvariantCheck {
        invariant_name: invariant_name.to_string(),
        passed: violations.is_empty(),
        violations,
    }
}

/// Check that entry points are preserved.
///
/// Public API entry points should not be lost during refactoring.
/// This ensures that external code depending on these symbols
/// will not break.
fn check_entry_points(before: &GraphSnapshot, after: &GraphSnapshot) -> InvariantCheck {
    let invariant_name = "Entry Points Preserved";
    let mut violations = Vec::new();

    // Check that all entry points still exist
    let before_entry_set: HashSet<&String> = before.entry_points.iter().collect();
    let after_entry_set: HashSet<&String> = after.entry_points.iter().collect();

    let lost_entries: Vec<_> = before_entry_set
        .difference(&after_entry_set)
        .cloned()
        .collect();

    if !lost_entries.is_empty() {
        for lost_id in lost_entries {
            if let Some(symbol) = before.symbols.get(lost_id) {
                violations.push(InvariantViolation {
                    severity: ViolationSeverity::Critical,
                    subject: symbol.name.clone(),
                    message: format!(
                        "Entry point '{}' ({}) was lost during refactoring",
                        symbol.name, symbol.id
                    ),
                    suggestion: Some(
                        "Entry points (public API) should not be removed during rename".to_string(),
                    ),
                });
            }
        }
    }

    // Warn if new entry points appeared (could indicate accidental public API)
    let new_entries: Vec<_> = after_entry_set
        .difference(&before_entry_set)
        .cloned()
        .collect();

    if !new_entries.is_empty() {
        for new_id in new_entries {
            if let Some(symbol) = after.symbols.get(new_id) {
                violations.push(InvariantViolation {
                    severity: ViolationSeverity::Info,
                    subject: symbol.name.clone(),
                    message: format!(
                        "New entry point '{}' ({}) appeared during refactoring",
                        symbol.name, symbol.id
                    ),
                    suggestion: Some(
                        "Verify this is intended - may indicate new public API".to_string(),
                    ),
                });
            }
        }
    }

    InvariantCheck {
        invariant_name: invariant_name.to_string(),
        passed: violations.is_empty(),
        violations,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::proof::data_structures::{GraphStats, SymbolInfo};

    fn create_test_snapshot() -> GraphSnapshot {
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
            timestamp: 0,
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
    fn test_validate_invariants_success() {
        let before = create_test_snapshot();
        let after = create_test_snapshot();

        let checks = validate_invariants(&before, &after).unwrap();

        assert_eq!(checks.len(), 4);
        for check in &checks {
            assert!(
                check.passed,
                "{} check failed: {:?}",
                check.invariant_name, check.violations
            );
        }
    }

    #[test]
    fn test_check_reference_counts_failure() {
        let before = create_test_snapshot();
        let mut after = create_test_snapshot();

        // Change fan-in in after snapshot
        if let Some(sym) = after.symbols.get_mut("0000000000000002") {
            sym.fan_in = 2; // Changed from 1
        }

        let check = check_reference_counts(&before, &after);

        assert!(!check.passed);
        assert_eq!(check.violations.len(), 1);
        assert_eq!(check.violations[0].severity, ViolationSeverity::Error);
    }

    #[test]
    fn test_check_orphaned_symbols() {
        let mut snapshot = create_test_snapshot();

        // Remove entry point to create orphaned symbols
        snapshot.entry_points.clear();

        let check = check_orphaned_symbols(&snapshot);

        assert!(!check.passed);
        // Should have at least one warning about orphaned symbols
        assert!(
            check
                .violations
                .iter()
                .any(|v| v.subject == "helper" || v.subject == "main"),
            "Expected violation for orphaned symbols"
        );
    }

    #[test]
    fn test_check_symbol_id_stability_failure() {
        let before = create_test_snapshot();
        let mut after = create_test_snapshot();

        // Add a new symbol ID
        after.symbols.insert(
            "0000000000000003".to_string(),
            SymbolInfo {
                id: "0000000000000003".to_string(),
                name: "new_func".to_string(),
                file_path: "src/new.rs".to_string(),
                kind: "fn".to_string(),
                byte_span: (0, 50),
                fan_in: 0,
                fan_out: 0,
            },
        );

        let check = check_symbol_id_stability(&before, &after);

        assert!(!check.passed);
        assert_eq!(check.violations.len(), 2); // Count change and ID change
    }

    #[test]
    fn test_check_entry_points_lost() {
        let before = create_test_snapshot();
        let mut after = create_test_snapshot();

        // Remove entry point
        after.entry_points.clear();

        let check = check_entry_points(&before, &after);

        assert!(!check.passed);
        assert_eq!(check.violations.len(), 1);
        assert_eq!(check.violations[0].severity, ViolationSeverity::Critical);
        assert_eq!(check.violations[0].subject, "main");
    }
}
