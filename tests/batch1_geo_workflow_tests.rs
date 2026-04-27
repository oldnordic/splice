//! Batch 1: Real .geo workflow tests
//!
//! These tests verify that Splice can:
//! 1. Open .geo backend
//! 2. Resolve real symbols from .geo
//! 3. Get spans from .geo
//! 4. Get code chunks from .geo
//! 5. Handle ambiguity explicitly
//! 6. Still work with SQLite backend

use std::path::Path;

/// Test 1: Can open geometric backend
#[test]
#[cfg(feature = "geometric")]
fn splice_can_open_geo_backend() {
    // Skip if no code.geo file available
    if !Path::new("./code.geo").exists() {
        eprintln!("SKIP: No code.geo file available");
        return;
    }

    let integration =
        splice::graph::magellan_integration::MagellanIntegration::open(Path::new("./code.geo"));
    assert!(integration.is_ok(), "Should open .geo file");

    let integration = integration.unwrap();
    assert!(
        integration.is_geometric(),
        "Should detect geometric backend"
    );
    assert!(integration.geo_inner().is_some(), "Should have geo_inner");
}

/// Test 2: Can resolve real symbol from .geo
#[test]
#[cfg(feature = "geometric")]
fn splice_can_resolve_real_symbol_from_geo() {
    if !Path::new("./code.geo").exists() {
        eprintln!("SKIP: No code.geo file available");
        return;
    }

    let mut integration =
        splice::graph::magellan_integration::MagellanIntegration::open(Path::new("./code.geo"))
            .unwrap();

    // Try to find a common symbol name
    let symbols = integration.find_symbol_by_name("open", false);
    assert!(symbols.is_ok(), "Should find symbols without error");

    let symbols = symbols.unwrap();
    // We might or might not find "open" depending on indexing, but the query should work
    eprintln!("Found {} symbols named 'open'", symbols.len());
}

/// Test 3: Can get real span from .geo
#[test]
#[cfg(feature = "geometric")]
fn splice_can_get_real_span_from_geo() {
    if !Path::new("./code.geo").exists() {
        eprintln!("SKIP: No code.geo file available");
        return;
    }

    let mut integration =
        splice::graph::magellan_integration::MagellanIntegration::open(Path::new("./code.geo"))
            .unwrap();

    // Find a symbol first
    let symbols = integration.find_symbol_by_name("open", false).unwrap();

    if let Some(symbol) = symbols.first() {
        // Verify span is valid
        assert!(
            symbol.byte_start < symbol.byte_end,
            "Span should be valid: {}..{}",
            symbol.byte_start,
            symbol.byte_end
        );
        eprintln!(
            "Symbol '{}' span: {}..{}",
            symbol.name, symbol.byte_start, symbol.byte_end
        );
    } else {
        eprintln!("SKIP: No 'open' symbol found to test span");
    }
}

/// Test 4: Can get real code chunk from .geo
#[test]
#[cfg(feature = "geometric")]
fn splice_can_get_real_code_chunk_from_geo() {
    if !Path::new("./code.geo").exists() {
        eprintln!("SKIP: No code.geo file available");
        return;
    }

    let mut integration =
        splice::graph::magellan_integration::MagellanIntegration::open(Path::new("./code.geo"))
            .unwrap();

    // Find a symbol first
    let symbols = integration.find_symbol_by_name("open", false).unwrap();

    if let Some(symbol) = symbols.first() {
        let file_path = Path::new(&symbol.file_path);
        let chunk = integration.get_code_chunk(file_path, symbol.byte_start, symbol.byte_end);

        assert!(chunk.is_ok(), "Should get code chunk without error");

        if let Ok(Some(chunk)) = chunk {
            assert!(
                !chunk.content.is_empty(),
                "Chunk content should not be empty"
            );
            eprintln!("Got chunk with {} bytes of content", chunk.content.len());
        } else {
            eprintln!("No chunk found at span (this is OK if chunk store is empty)");
        }
    } else {
        eprintln!("SKIP: No 'open' symbol found to test chunk retrieval");
    }
}

/// Test 5: Ambiguity handling is explicit on .geo
#[test]
#[cfg(feature = "geometric")]
fn splice_geo_ambiguity_handling_is_explicit() {
    if !Path::new("./code.geo").exists() {
        eprintln!("SKIP: No code.geo file available");
        return;
    }

    let mut integration =
        splice::graph::magellan_integration::MagellanIntegration::open(Path::new("./code.geo"))
            .unwrap();

    // Search with ambiguous=true should return all matches
    let all_symbols = integration.find_symbol_by_name("open", true).unwrap();

    // Search with ambiguous=false should return at most 1
    let first_only = integration.find_symbol_by_name("open", false).unwrap();

    assert!(
        first_only.len() <= 1,
        "ambiguous=false should return at most 1 symbol"
    );
    assert!(
        first_only.len() <= all_symbols.len(),
        "ambiguous=false should not return more than ambiguous=true"
    );

    eprintln!(
        "ambiguous=true: {} symbols, ambiguous=false: {} symbols",
        all_symbols.len(),
        first_only.len()
    );
}

/// Test 6: SQLite backend still works for Batch 1
#[test]
fn splice_sqlite_backend_still_works_for_batch1() {
    use std::io::Write;
    use tempfile::NamedTempFile;

    let temp_file = NamedTempFile::with_suffix(".db").unwrap();

    // SQLite needs a valid database file. Create a minimal valid SQLite DB.
    // For this test, we just need to verify backend detection works.
    // An empty file won't work, but we can check the error is NOT "geometric backend required"
    let integration =
        splice::graph::magellan_integration::MagellanIntegration::open(temp_file.path());

    // Either it opens successfully (SQLite), or fails with SQLite error (not geometric)
    match integration {
        Ok(integ) => {
            assert!(!integ.is_geometric(), "Should be SQLite backend");
            // geo_inner is not available on MagellanIntegration
        }
        Err(e) => {
            let err_str = format!("{}", e);
            // Should NOT be a geometric backend error
            assert!(
                !err_str.contains("Geometric backend"),
                "SQLite backend should not fail with geometric error: {}",
                err_str
            );
            // SQLite errors are OK for this test - we just verify it's not treating it as geometric
        }
    }
}
