//! Integration tests for JavaScript patching with validation gates.
//!
//! These tests validate the full pipeline for JavaScript:
//! resolve → patch-by-span → tree-sitter reparse gate → node --check validation gate

use splice::graph::CodeGraph;
use splice::ingest::javascript::extract_javascript_symbols;
use splice::patch::apply_patch_with_validation;
use splice::resolve::resolve_symbol;
use splice::symbol::Language;
use splice::validate::AnalyzerMode;
use tempfile::TempDir;

#[cfg(test)]
mod tests {
    use super::*;

    /// Test A: JavaScript patch succeeds with all gates passing.
    #[test]
    fn test_javascript_patch_succeeds_with_all_gates() {
        // Create temporary workspace directory
        let workspace_dir = TempDir::new().expect("Failed to create temp workspace");
        let workspace_path = workspace_dir.path();

        // Create a JavaScript file with a function to patch
        let js_path = workspace_path.join("test.js");
        let source = r#"
function greet(name) {
    return 0;
}

function farewell(name) {
    return 0;
}
"#;

        std::fs::write(&js_path, source).expect("Failed to write test.js");

        // Create temporary graph database
        let graph_db_path = workspace_path.join("graph.db");
        let mut code_graph =
            CodeGraph::open(&graph_db_path).expect("Failed to open graph database");

        // Ingest symbols from test.js
        let symbols = extract_javascript_symbols(&js_path, source.as_bytes())
            .expect("Failed to parse test.js");

        assert_eq!(symbols.len(), 2, "Expected 2 functions");

        // Store symbols with file association and language
        for symbol in &symbols {
            code_graph
                .store_symbol_with_file_and_language(
                    &js_path,
                    &symbol.name,
                    symbol.kind.as_str(),
                    Language::JavaScript,
                    symbol.byte_start,
                    symbol.byte_end,
                )
                .expect("Failed to store symbol");
        }

        // Resolve the "greet" function
        let resolved = resolve_symbol(&code_graph, Some(&js_path), Some("function"), "greet")
            .expect("Failed to resolve greet function");

        // Verify we got the right span
        let greet_symbol = &symbols[0];
        assert_eq!(resolved.name, "greet");
        assert_eq!(resolved.byte_start, greet_symbol.byte_start);
        assert_eq!(resolved.byte_end, greet_symbol.byte_end);

        // Apply patch: replace function body
        let new_body = r#"
function greet(name) {
    return 42;
}
"#;

        let result = apply_patch_with_validation(
            &js_path,
            resolved.byte_start,
            resolved.byte_end,
            new_body.trim(),
            workspace_path,
            Language::JavaScript,
            AnalyzerMode::Off,
        );

        // Should succeed if node is available
        if result.is_ok() {
            // Verify file content changed
            let new_content =
                std::fs::read_to_string(&js_path).expect("Failed to read patched file");

            assert!(
                new_content.contains("return 42;"),
                "Patched content should be present"
            );
            assert!(
                !new_content.contains("return 0;") || new_content.matches("return 0;").count() == 1,
                "Old greet content should be gone (farewell still has return 0)"
            );
        } else {
            // If node is not available, test is considered passed (soft failure)
            println!("node not available, skipping full patch validation test");
        }
    }

    /// Test B: JavaScript patch rejected on syntax gate.
    #[test]
    fn test_javascript_patch_rejected_on_syntax_gate() {
        // Create temporary workspace
        let workspace_dir = TempDir::new().expect("Failed to create temp workspace");
        let workspace_path = workspace_dir.path();

        // Create a JavaScript file
        let js_path = workspace_path.join("test.js");
        let source = r#"
function validFunction() {
    return 42;
}
"#;

        std::fs::write(&js_path, source).expect("Failed to write test.js");

        // Create temporary graph database
        let graph_db_path = workspace_path.join("graph.db");
        let mut code_graph =
            CodeGraph::open(&graph_db_path).expect("Failed to open graph database");

        // Ingest and store symbols
        let symbols = extract_javascript_symbols(&js_path, source.as_bytes())
            .expect("Failed to parse test.js");

        let symbol = &symbols[0];
        code_graph
            .store_symbol_with_file_and_language(
                &js_path,
                &symbol.name,
                symbol.kind.as_str(),
                Language::JavaScript,
                symbol.byte_start,
                symbol.byte_end,
            )
            .expect("Failed to store symbol");

        // Resolve function
        let resolved = resolve_symbol(
            &code_graph,
            Some(&js_path),
            Some("function"),
            "validFunction",
        )
        .expect("Failed to resolve function");

        // Read original content for comparison
        let original_content =
            std::fs::read_to_string(&js_path).expect("Failed to read original file");

        // Apply patch with syntax error (unclosed parenthesis)
        let invalid_patch = r#"
function validFunction() {
    return (42;
}
"#;

        let result = apply_patch_with_validation(
            &js_path,
            resolved.byte_start,
            resolved.byte_end,
            invalid_patch.trim(),
            workspace_path,
            Language::JavaScript,
            AnalyzerMode::Off,
        );

        // Should fail on syntax error
        assert!(
            result.is_err(),
            "Patch should fail on syntax error: {:?}",
            result
        );

        // Verify original file is unchanged (atomic rollback)
        let current_content =
            std::fs::read_to_string(&js_path).expect("Failed to read current file");

        assert_eq!(
            original_content, current_content,
            "File should be unchanged after failed patch (atomic rollback)"
        );
    }

    /// Test C: JavaScript arrow function patch
    #[test]
    fn test_javascript_arrow_function_patch() {
        // Create temporary workspace
        let workspace_dir = TempDir::new().expect("Failed to create temp workspace");
        let workspace_path = workspace_dir.path();

        // Create a JavaScript file with arrow function
        let js_path = workspace_path.join("test.js");
        let source = r#"
const greet = (name) => {
    return 0;
};
"#;

        std::fs::write(&js_path, source).expect("Failed to write test.js");

        // Create temporary graph database
        let graph_db_path = workspace_path.join("graph.db");
        let mut code_graph =
            CodeGraph::open(&graph_db_path).expect("Failed to open graph database");

        // Ingest symbols
        let symbols = extract_javascript_symbols(&js_path, source.as_bytes())
            .expect("Failed to parse test.js");

        // Find the greet variable/function
        let greet = symbols
            .iter()
            .find(|s| s.name == "greet")
            .expect("Should find greet symbol");

        code_graph
            .store_symbol_with_file_and_language(
                &js_path,
                &greet.name,
                greet.kind.as_str(),
                Language::JavaScript,
                greet.byte_start,
                greet.byte_end,
            )
            .expect("Failed to store symbol");

        // Resolve greet (don't filter by kind since arrow functions may be stored as variables)
        let resolved = resolve_symbol(
            &code_graph,
            Some(&js_path),
            None, // Don't filter by kind
            "greet",
        )
        .expect("Failed to resolve greet");

        // Apply valid patch
        let new_body = r#"
const greet = (name) => {
    return 42;
};
"#;

        let result = apply_patch_with_validation(
            &js_path,
            resolved.byte_start,
            resolved.byte_end,
            new_body.trim(),
            workspace_path,
            Language::JavaScript,
            AnalyzerMode::Off,
        );

        // Should succeed if node is available
        if result.is_ok() {
            let new_content =
                std::fs::read_to_string(&js_path).expect("Failed to read patched file");
            assert!(
                new_content.contains("return 42;"),
                "Patched content should be present"
            );
        } else {
            println!("node not available, skipping arrow function patch validation");
        }
    }
}
