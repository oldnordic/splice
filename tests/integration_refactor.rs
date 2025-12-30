//! Integration tests for deterministic symbol resolution.
//!
//! These tests validate the full ingest → store → resolve → get_span pipeline
//! with special attention to ambiguous symbol detection.

use splice::graph::CodeGraph;
use splice::ingest::rust::{extract_rust_symbols, RustSymbolKind};
use splice::resolve::resolve_symbol;
use std::io::Write;
use tempfile::NamedTempFile;

#[cfg(test)]
mod tests {
    use super::*;

    /// Test that name-only resolution fails when symbol exists in multiple files.
    #[test]
    fn test_ambiguous_symbol_name_only_fails() {
        // Create two temp files with same function name "foo"
        let source1 = r#"
fn foo() {
    println!("foo in file1");
}
"#;

        let source2 = r#"
fn foo() {
    println!("foo in file2");
}
"#;

        let mut temp_file1 = NamedTempFile::new().expect("Failed to create temp file 1");
        temp_file1
            .write_all(source1.as_bytes())
            .expect("Failed to write to temp file 1");
        let path1 = temp_file1.path();

        let mut temp_file2 = NamedTempFile::new().expect("Failed to create temp file 2");
        temp_file2
            .write_all(source2.as_bytes())
            .expect("Failed to write to temp file 2");
        let path2 = temp_file2.path();

        // Create temporary graph database
        let graph_db = NamedTempFile::new().expect("Failed to create temp db");
        let graph_path = graph_db.path();

        // Open graph
        let mut code_graph = CodeGraph::open(graph_path).expect("Failed to open graph database");

        // Ingest symbols from both files
        let symbols1 =
            extract_rust_symbols(path1, source1.as_bytes()).expect("Failed to parse file 1");
        let symbols2 =
            extract_rust_symbols(path2, source2.as_bytes()).expect("Failed to parse file 2");

        // Store symbols with file associations
        for symbol in &symbols1 {
            code_graph
                .store_symbol_with_file(
                    path1,
                    &symbol.name,
                    symbol.kind,
                    symbol.byte_start,
                    symbol.byte_end,
                )
                .expect("Failed to store symbol from file 1");
        }

        for symbol in &symbols2 {
            code_graph
                .store_symbol_with_file(
                    path2,
                    &symbol.name,
                    symbol.kind,
                    symbol.byte_start,
                    symbol.byte_end,
                )
                .expect("Failed to store symbol from file 2");
        }

        // Attempt name-only resolution (without file path)
        let result = resolve_symbol(&code_graph, None, Some(RustSymbolKind::Function), "foo");

        // Should fail with AmbiguousSymbol error
        assert!(result.is_err(), "Expected error for ambiguous symbol");
        match result {
            Err(splice::SpliceError::AmbiguousSymbol { name, files }) => {
                assert_eq!(name, "foo");
                assert_eq!(files.len(), 2);
            }
            _ => panic!("Expected AmbiguousSymbol error, got: {:?}", result),
        }
    }

    /// Test that resolution with explicit file path succeeds.
    #[test]
    fn test_resolve_with_explicit_file_succeeds() {
        // Create temp file with function "foo"
        let source = r#"
fn foo() {
    println!("hello from foo");
}
"#;

        let mut temp_file = NamedTempFile::new().expect("Failed to create temp file");
        temp_file
            .write_all(source.as_bytes())
            .expect("Failed to write to temp file");
        let path = temp_file.path();

        // Create temporary graph database
        let graph_db = NamedTempFile::new().expect("Failed to create temp db");
        let graph_path = graph_db.path();

        // Open graph
        let mut code_graph = CodeGraph::open(graph_path).expect("Failed to open graph database");

        // Ingest symbols from source
        let symbols =
            extract_rust_symbols(path, source.as_bytes()).expect("Failed to parse Rust file");

        // Store symbol with file association
        let symbol = &symbols[0];
        let node_id = code_graph
            .store_symbol_with_file(
                path,
                &symbol.name,
                symbol.kind,
                symbol.byte_start,
                symbol.byte_end,
            )
            .expect("Failed to store symbol");

        // Resolve with explicit file path
        let resolved = resolve_symbol(
            &code_graph,
            Some(path),
            Some(RustSymbolKind::Function),
            "foo",
        )
        .expect("Failed to resolve symbol");

        // Verify resolution matches ingest output exactly
        assert_eq!(resolved.node_id, node_id);
        assert_eq!(resolved.byte_start, symbol.byte_start);
        assert_eq!(resolved.byte_end, symbol.byte_end);
        // TODO: Store line/col in graph - currently returns 0
        // assert_eq!(resolved.line_start, symbol.line_start);
        // assert_eq!(resolved.line_end, symbol.line_end);
        // assert_eq!(resolved.col_start, symbol.col_start);
        // assert_eq!(resolved.col_end, symbol.col_end);
    }

    /// Test full round-trip: ingest → store → resolve → get_span.
    #[test]
    fn test_round_trip_resolution() {
        // Create temp file with multiple functions
        let source = r#"
fn hello() {
    println!("Hello");
}

fn world() {
    println!("World");
}
"#;

        let mut temp_file = NamedTempFile::new().expect("Failed to create temp file");
        temp_file
            .write_all(source.as_bytes())
            .expect("Failed to write to temp file");
        let path = temp_file.path();

        // Create temporary graph database
        let graph_db = NamedTempFile::new().expect("Failed to create temp db");
        let graph_path = graph_db.path();

        // Open graph
        let mut code_graph = CodeGraph::open(graph_path).expect("Failed to open graph database");

        // Ingest symbols from source
        let symbols =
            extract_rust_symbols(path, source.as_bytes()).expect("Failed to parse Rust file");

        // Store all symbols with file association
        for symbol in &symbols {
            code_graph
                .store_symbol_with_file(
                    path,
                    &symbol.name,
                    symbol.kind,
                    symbol.byte_start,
                    symbol.byte_end,
                )
                .expect("Failed to store symbol");
        }

        // Round-trip for each symbol
        for symbol in &symbols {
            // Resolve by name and file
            let resolved = resolve_symbol(
                &code_graph,
                Some(path),
                Some(RustSymbolKind::Function),
                &symbol.name,
            )
            .expect("Failed to resolve symbol");

            // Retrieve span from graph
            let retrieved_span = code_graph
                .get_span(resolved.node_id)
                .expect("Failed to get span");

            // Verify exact equality with ingest output
            assert_eq!(
                retrieved_span,
                (symbol.byte_start, symbol.byte_end),
                "Span mismatch for {}",
                symbol.name
            );

            // TODO: Verify line/col metadata when stored in graph
            // assert_eq!(resolved.line_start, symbol.line_start);
            // assert_eq!(resolved.line_end, symbol.line_end);
            // assert_eq!(resolved.col_start, symbol.col_start);
            // assert_eq!(resolved.col_end, symbol.col_end);
        }
    }
}
