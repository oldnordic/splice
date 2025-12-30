//! Graph persistence and symbol resolution tests.

use splice::graph::CodeGraph;
use splice::ingest::rust::{extract_rust_symbols, RustSymbolKind};
use std::io::Write;
use tempfile::NamedTempFile;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_store_and_retrieve_symbol_spans() {
        // Create a temporary Rust file
        let source = r#"
fn hello_world() {
    println!("Hello, world!");
}

fn goodbye() {
    println!("Goodbye!");
}
"#;

        let mut temp_file = NamedTempFile::new().expect("Failed to create temp file");
        temp_file
            .write_all(source.as_bytes())
            .expect("Failed to write to temp file");
        let temp_path = temp_file.path();

        // Create temporary graph database
        let graph_db = NamedTempFile::new().expect("Failed to create temp db");
        let graph_path = graph_db.path();

        // Open graph
        let mut code_graph = CodeGraph::open(graph_path).expect("Failed to open graph database");

        // Ingest symbols from source
        let symbols =
            extract_rust_symbols(temp_path, source.as_bytes()).expect("Failed to parse Rust file");

        // Assert we found 2 functions
        assert_eq!(symbols.len(), 2, "Expected 2 functions");

        // Store symbols in graph
        let mut node_ids = Vec::new();
        for symbol in &symbols {
            let node_id = code_graph
                .store_symbol(
                    &symbol.name,
                    symbol.kind,
                    symbol.byte_start,
                    symbol.byte_end,
                )
                .expect("Failed to store symbol");
            node_ids.push(node_id);
        }

        // Verify we can retrieve spans
        for (i, symbol) in symbols.iter().enumerate() {
            let retrieved_span = code_graph
                .get_span(node_ids[i])
                .expect("Failed to retrieve span");

            // Assert exact equality with ingest output
            assert_eq!(
                retrieved_span,
                (symbol.byte_start, symbol.byte_end),
                "Span mismatch for {}",
                symbol.name
            );
        }

        // Verify hello_world specific values
        let hello_world = &symbols[0];
        assert_eq!(hello_world.name, "hello_world");
        assert_eq!(hello_world.kind, RustSymbolKind::Function);
        assert_eq!(hello_world.byte_start, 1);
        assert!(hello_world.byte_end > 40);
    }

    #[test]
    fn test_resolve_nonexistent_symbol() {
        // Create temporary graph database
        let graph_db = NamedTempFile::new().expect("Failed to create temp db");
        let graph_path = graph_db.path();

        // Open graph
        let code_graph = CodeGraph::open(graph_path).expect("Failed to open graph database");

        // Try to resolve a symbol that doesn't exist
        let result = code_graph.resolve_symbol("nonexistent_function");

        // Should return an error
        assert!(result.is_err(), "Expected error for nonexistent symbol");
    }
}
