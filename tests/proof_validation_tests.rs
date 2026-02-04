//! Integration tests for proof-based invariant validation.
//!
//! These tests verify that invariant validation works correctly
//! for real refactoring operations.

use std::collections::HashMap;
use std::path::PathBuf;
use splice::proof::data_structures::{
    GraphSnapshot, InvariantCheck, InvariantViolation, ViolationSeverity,
    SymbolInfo, GraphStats,
};
use splice::proof::validation::validate_invariants;

/// Helper to create a minimal graph snapshot for testing.
fn create_minimal_snapshot() -> GraphSnapshot {
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
    edges.insert("0000000000000001".to_string(), vec!["0000000000000002".to_string()]);
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
fn test_validate_invariants_no_change() {
    let before = create_minimal_snapshot();
    let after = create_minimal_snapshot();

    let checks = validate_invariants(&before, &after).unwrap();

    // Should have 4 invariant checks
    assert_eq!(checks.len(), 4);

    // All should pass when there's no change
    for check in &checks {
        assert!(
            check.passed,
            "{} check failed: {:?}",
            check.invariant_name, check.violations
        );
    }
}

#[test]
fn test_validate_invariants_detects_reference_count_change() {
    let mut before = create_minimal_snapshot();
    let mut after = create_minimal_snapshot();

    // Simulate a broken refactoring: fan-in changed
    if let Some(sym) = after.symbols.get_mut("0000000000000002") {
        sym.fan_in = 2; // Changed from 1
    }

    let checks = validate_invariants(&before, &after).unwrap();

    // Find the reference count check
    let ref_check = checks
        .iter()
        .find(|c| c.invariant_name == "Reference Counts Preserved")
        .expect("Reference count check should exist");

    assert!(!ref_check.passed);
    assert_eq!(ref_check.violations.len(), 1);
    assert_eq!(ref_check.violations[0].severity, ViolationSeverity::Error);
}

#[test]
fn test_validate_invariants_detects_orphaned_symbols() {
    let mut snapshot = create_minimal_snapshot();

    // Remove entry point to create orphans
    snapshot.entry_points.clear();

    let checks = validate_invariants(&snapshot, &snapshot).unwrap();

    // Find the orphaned symbols check
    let orphan_check = checks
        .iter()
        .find(|c| c.invariant_name == "No Orphaned Symbols")
        .expect("Orphaned symbols check should exist");

    // Should fail because symbols are orphaned
    assert!(!orphan_check.passed);
}

#[test]
fn test_validate_invariants_detects_id_instability() {
    let before = create_minimal_snapshot();
    let mut after = create_minimal_snapshot();

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

    let checks = validate_invariants(&before, &after).unwrap();

    // Find the symbol ID stability check
    let id_check = checks
        .iter()
        .find(|c| c.invariant_name == "Symbol IDs Stable")
        .expect("Symbol ID stability check should exist");

    assert!(!id_check.passed);
}

#[test]
fn test_validate_invariants_detects_lost_entry_points() {
    let before = create_minimal_snapshot();
    let mut after = create_minimal_snapshot();

    // Remove entry point
    after.entry_points.clear();

    let checks = validate_invariants(&before, &after).unwrap();

    // Find the entry points check
    let entry_check = checks
        .iter()
        .find(|c| c.invariant_name == "Entry Points Preserved")
        .expect("Entry points check should exist");

    assert!(!entry_check.passed);
    assert_eq!(entry_check.violations.len(), 1);
    assert_eq!(
        entry_check.violations[0].severity,
        ViolationSeverity::Critical
    );
    assert_eq!(entry_check.violations[0].subject, "main");
}

#[test]
fn test_validate_invariants_reports_new_entry_points() {
    let before = create_minimal_snapshot();
    let mut after = create_minimal_snapshot();

    // Add a new entry point
    after.entry_points.push("0000000000000002".to_string());

    let checks = validate_invariants(&before, &after).unwrap();

    // Find the entry points check
    let entry_check = checks
        .iter()
        .find(|c| c.invariant_name == "Entry Points Preserved")
        .expect("Entry points check should exist");

    // Should have an info-level warning about new entry point
    assert!(!entry_check.passed);
    let info_violations: Vec<_> = entry_check
        .violations
        .iter()
        .filter(|v| v.severity == ViolationSeverity::Info)
        .collect();

    assert!(!info_violations.is_empty());
}

#[test]
fn test_validate_invariants_multiple_violations() {
    let mut before = create_minimal_snapshot();
    let mut after = create_minimal_snapshot();

    // Make multiple changes to trigger multiple violations
    after.entry_points.clear(); // Lost entry point
    if let Some(sym) = after.symbols.get_mut("0000000000000002") {
        sym.fan_in = 2; // Changed reference count
    }

    let checks = validate_invariants(&before, &after).unwrap();

    // Count failed checks
    let failed: Vec<_> = checks.iter().filter(|c| !c.passed).collect();

    // Should have at least 2 failed checks
    assert!(failed.len() >= 2);
}

#[test]
fn test_validate_invariants_realistic_rename() {
    // Simulate a realistic rename operation
    let mut before = create_minimal_snapshot();

    // Create "after" state where helper was renamed to helper_v2
    let mut after_symbols = HashMap::new();

    // main stays the same
    after_symbols.insert(
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

    // helper renamed to helper_v2 (same ID!)
    after_symbols.insert(
        "0000000000000002".to_string(),
        SymbolInfo {
            id: "0000000000000002".to_string(),
            name: "helper_v2".to_string(),
            file_path: "src/helper.rs".to_string(),
            kind: "fn".to_string(),
            byte_span: (0, 50),
            fan_in: 1,
            fan_out: 0,
        },
    );

    let mut edges = HashMap::new();
    edges.insert("0000000000000001".to_string(), vec!["0000000000000002".to_string()]);
    edges.insert("0000000000000002".to_string(), vec![]);

    let after = GraphSnapshot {
        timestamp: 1,
        symbols: after_symbols,
        edges,
        entry_points: vec!["0000000000000001".to_string()],
        stats: GraphStats {
            total_symbols: 2,
            total_edges: 1,
            entry_point_count: 1,
            max_complexity: None,
        },
    };

    let checks = validate_invariants(&before, &after).unwrap();

    // All checks should pass for a correct rename
    for check in &checks {
        assert!(
            check.passed,
            "{} check failed for valid rename: {:?}",
            check.invariant_name, check.violations
        );
    }
}

#[test]
fn test_validate_invariants_complex_graph() {
    // Create a more complex graph with multiple entry points and layers
    let mut symbols = HashMap::new();

    // Entry points
    for i in 1..=3 {
        let id = format!("{:016x}", i);
        symbols.insert(
            id.clone(),
            SymbolInfo {
                id: id.clone(),
                name: format!("api_{}", i),
                file_path: "src/api.rs".to_string(),
                kind: "fn".to_string(),
                byte_span: (0, 100),
                fan_in: 0,
                fan_out: 2,
            },
        );
    }

    // Middle layer
    for i in 4..=6 {
        let id = format!("{:016x}", i);
        symbols.insert(
            id.clone(),
            SymbolInfo {
                id: id.clone(),
                name: format!("service_{}", i),
                file_path: "src/service.rs".to_string(),
                kind: "fn".to_string(),
                byte_span: (0, 50),
                fan_in: 3,
                fan_out: 1,
            },
        );
    }

    // Bottom layer
    symbols.insert(
        "0000000000000007".to_string(),
        SymbolInfo {
            id: "0000000000000007".to_string(),
            name: "db".to_string(),
            file_path: "src/db.rs".to_string(),
            kind: "fn".to_string(),
            byte_span: (0, 50),
            fan_in: 3,
            fan_out: 0,
        },
    );

    let mut edges = HashMap::new();
    // Entry points call all services
    for i in 1..=3 {
        let entry_id = format!("{:016x}", i);
        edges.insert(
            entry_id,
            vec![
                "0000000000000004".to_string(),
                "0000000000000005".to_string(),
                "0000000000000006".to_string(),
            ],
        );
    }
    // Services call db
    for i in 4..=6 {
        let service_id = format!("{:016x}", i);
        edges.insert(service_id, vec!["0000000000000007".to_string()]);
    }
    edges.insert("0000000000000007".to_string(), vec![]);

    let snapshot = GraphSnapshot {
        timestamp: 0,
        symbols,
        edges,
        entry_points: vec![
            "0000000000000001".to_string(),
            "0000000000000002".to_string(),
            "0000000000000003".to_string(),
        ],
        stats: GraphStats {
            total_symbols: 7,
            total_edges: 12,
            entry_point_count: 3,
            max_complexity: None,
        },
    };

    let checks = validate_invariants(&snapshot, &snapshot).unwrap();

    // All checks should pass for a consistent graph
    assert_eq!(checks.len(), 4);
    for check in &checks {
        assert!(
            check.passed,
            "{} check failed: {:?}",
            check.invariant_name, check.violations
        );
    }
}
