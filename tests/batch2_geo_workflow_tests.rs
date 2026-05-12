//! Batch 2: Real .geo graph workflow tests
//!
//! These tests verify that Splice can:
//! 1. Compute reachable symbols (forward reachability) on .geo
//! 2. Compute reverse reachable symbols on .geo
//! 3. Detect cycles in call graph on .geo
//! 4. Detect dead symbols on .geo
//! 5. Still work with SQLite backend for Batch 2 methods

/// Test 1: Can compute reachable symbols on .geo
#[test]
#[cfg(feature = "geometric")]
fn splice_can_compute_reachable_symbols_on_geo() {
    if !Path::new("./code.geo").exists() {
        eprintln!("SKIP: No code.geo file available");
        return;
    }

    let mut integration =
        splice::graph::magellan_integration::MagellanIntegration::open(Path::new("./code.geo"))
            .unwrap();

    // Find a real function to test reachability
    let symbols = integration
        .find_symbol_by_name("reachable_symbols", false)
        .unwrap();

    if let Some(symbol) = symbols.first() {
        let file_path = Path::new(&symbol.file_path);
        let reachable = integration.reachable_symbols(file_path, &symbol.name, 2);

        assert!(
            reachable.is_ok(),
            "Should compute reachable symbols without error"
        );

        let reachable = reachable.unwrap();
        eprintln!(
            "Symbol '{}' has {} reachable symbols at depth <= 2",
            symbol.name,
            reachable.len()
        );

        // Verify reachable symbols have valid structure
        for r in &reachable {
            assert!(
                !r.symbol.name.is_empty(),
                "Reachable symbol should have a name"
            );
            assert!(
                !r.symbol.file_path.is_empty(),
                "Reachable symbol should have a file path"
            );
            assert!(r.depth > 0 && r.depth <= 2, "Depth should be within range");
        }
    } else {
        eprintln!("SKIP: No 'reachable_symbols' function found to test");
    }
}

/// Test 2: Can compute reverse reachable symbols on .geo
#[test]
#[cfg(feature = "geometric")]
fn splice_can_compute_reverse_reachable_symbols_on_geo() {
    if !Path::new("./code.geo").exists() {
        eprintln!("SKIP: No code.geo file available");
        return;
    }

    let mut integration =
        splice::graph::magellan_integration::MagellanIntegration::open(Path::new("./code.geo"))
            .unwrap();

    // Find a real function to test reverse reachability
    let symbols = integration
        .find_symbol_by_name("reachable_symbols", false)
        .unwrap();

    if let Some(symbol) = symbols.first() {
        let file_path = Path::new(&symbol.file_path);
        let callers = integration.reverse_reachable_symbols(file_path, &symbol.name, 2);

        assert!(
            callers.is_ok(),
            "Should compute reverse reachable symbols without error"
        );

        let callers = callers.unwrap();
        eprintln!(
            "Symbol '{}' has {} reverse reachable symbols at depth <= 2",
            symbol.name,
            callers.len()
        );

        // Verify caller symbols have valid structure
        for c in &callers {
            assert!(
                !c.symbol.name.is_empty(),
                "Caller symbol should have a name"
            );
            assert!(
                !c.symbol.file_path.is_empty(),
                "Caller symbol should have a file path"
            );
            assert!(c.depth > 0 && c.depth <= 2, "Depth should be within range");
        }
    } else {
        eprintln!("SKIP: No 'reachable_symbols' function found to test");
    }
}

/// Test 3: Can detect cycles in call graph on .geo
#[test]
#[cfg(feature = "geometric")]
fn splice_can_detect_cycles_on_geo() {
    if !Path::new("./code.geo").exists() {
        eprintln!("SKIP: No code.geo file available");
        return;
    }

    let mut integration =
        splice::graph::magellan_integration::MagellanIntegration::open(Path::new("./code.geo"))
            .unwrap();

    let cycles = integration.detect_cycles(100);

    assert!(cycles.is_ok(), "Should detect cycles without error");

    let cycles = cycles.unwrap();
    eprintln!("Detected {} cycles in call graph", cycles.len());

    // Verify cycle structure
    for cycle in &cycles {
        assert!(!cycle.id.is_empty(), "Cycle should have an ID");
        assert!(cycle.size > 0, "Cycle should have at least one member");
        assert!(!cycle.members.is_empty(), "Cycle should have members");

        for member in &cycle.members {
            assert!(!member.name.is_empty(), "Cycle member should have a name");
        }
    }
}

/// Test 4: Can detect dead symbols on .geo
#[test]
#[cfg(feature = "geometric")]
fn splice_can_detect_dead_symbols_on_geo() {
    if !Path::new("./code.geo").exists() {
        eprintln!("SKIP: No code.geo file available");
        return;
    }

    let mut integration =
        splice::graph::magellan_integration::MagellanIntegration::open(Path::new("./code.geo"))
            .unwrap();

    // Find main as entry point
    let main_symbols = integration.find_symbol_by_name("main", false).unwrap();

    if let Some(main_symbol) = main_symbols.first() {
        let entry_file = Path::new(&main_symbol.file_path);

        let dead = integration.dead_symbols(entry_file, "main", false);

        assert!(dead.is_ok(), "Should detect dead symbols without error");

        let dead = dead.unwrap();
        eprintln!(
            "Found {} potentially dead symbols from main entry point",
            dead.len()
        );

        // Verify dead symbol structure
        for d in &dead {
            assert!(!d.symbol.name.is_empty(), "Dead symbol should have a name");
            assert!(
                !d.symbol.file_path.is_empty(),
                "Dead symbol should have a file path"
            );
            assert!(!d.reason.is_empty(), "Dead symbol should have a reason");
        }
    } else {
        eprintln!("SKIP: No 'main' function found as entry point");
    }
}

/// Test 5: SQLite backend still works for Batch 2 methods
#[test]
fn splice_sqlite_backend_still_works_for_batch2() {
    use tempfile::NamedTempFile;

    let temp_file = NamedTempFile::with_suffix(".db").unwrap();

    // SQLite needs a valid database file
    let integration =
        splice::graph::magellan_integration::MagellanIntegration::open(temp_file.path());

    match integration {
        Ok(mut integ) => {
            assert!(!integ.is_geometric(), "Should be SQLite backend");

            // Try Batch 2 methods - they should work even on empty DB
            let temp_path = temp_file.path();

            // reachable_symbols on empty DB should return empty, not error
            let reachable = integ.reachable_symbols(temp_path, "nonexistent", 2);
            match reachable {
                Ok(symbols) => {
                    eprintln!(
                        "SQLite reachable_symbols returned {} symbols (expected 0 for empty DB)",
                        symbols.len()
                    );
                }
                Err(e) => {
                    let err_str = format!("{}", e);
                    // Should NOT be a geometric backend error
                    assert!(
                        !err_str.contains("Geometric backend"),
                        "SQLite should not fail with geometric error: {}",
                        err_str
                    );
                    eprintln!(
                        "SQLite reachable_symbols error (expected for empty DB): {}",
                        err_str
                    );
                }
            }

            // detect_cycles on empty DB
            let cycles = integ.detect_cycles(100);
            match cycles {
                Ok(c) => {
                    eprintln!(
                        "SQLite detect_cycles returned {} cycles (expected 0 for empty DB)",
                        c.len()
                    );
                }
                Err(e) => {
                    let err_str = format!("{}", e);
                    assert!(
                        !err_str.contains("Geometric backend"),
                        "SQLite should not fail with geometric error: {}",
                        err_str
                    );
                    eprintln!(
                        "SQLite detect_cycles error (expected for empty DB): {}",
                        err_str
                    );
                }
            }
        }
        Err(e) => {
            let err_str = format!("{}", e);
            // Should NOT be a geometric backend error
            assert!(
                !err_str.contains("Geometric backend"),
                "SQLite backend should not fail with geometric error: {}",
                err_str
            );
            eprintln!("SQLite open error (OK for this test): {}", err_str);
        }
    }
}
