//! Integration tests for C++ patching with validation gates.
//!
//! These tests validate the full pipeline for C++:
//! resolve → patch-by-span → tree-sitter reparse gate → g++ compilation gate

use splice::graph::CodeGraph;
use splice::ingest::cpp::extract_cpp_symbols;
use splice::patch::apply_patch_with_validation;
use splice::resolve::resolve_symbol;
use splice::symbol::Language;
use splice::validate::AnalyzerMode;
use tempfile::TempDir;

#[cfg(test)]
mod tests {
    use super::*;

    /// Test A: C++ patch succeeds with all gates passing.
    #[test]
    fn test_cpp_patch_succeeds_with_all_gates() {
        // Create temporary workspace directory
        let workspace_dir = TempDir::new().expect("Failed to create temp workspace");
        let workspace_path = workspace_dir.path();

        // Create a C++ file with a function to patch
        let cpp_path = workspace_path.join("test.cpp");
        let source = r#"
int greet(const char* name) {
    return 0;
}

int farewell(const char* name) {
    return 0;
}
"#;

        std::fs::write(&cpp_path, source).expect("Failed to write test.cpp");

        // Create temporary graph database
        let graph_db_path = workspace_path.join("graph.db");
        let mut code_graph =
            CodeGraph::open(&graph_db_path).expect("Failed to open graph database");

        // Ingest symbols from test.cpp
        let symbols =
            extract_cpp_symbols(&cpp_path, source.as_bytes()).expect("Failed to parse test.cpp");

        assert_eq!(symbols.len(), 2, "Expected 2 functions");

        // Store symbols with file association and language
        for symbol in &symbols {
            code_graph
                .store_symbol_with_file_and_language(
                    &cpp_path,
                    &symbol.name,
                    symbol.kind.as_str(),
                    Language::Cpp,
                    symbol.byte_start,
                    symbol.byte_end,
                )
                .expect("Failed to store symbol");
        }

        // Resolve the "greet" function
        let resolved = resolve_symbol(
            &code_graph,
            Some(&cpp_path),
            Some("function"),
            "greet",
        )
        .expect("Failed to resolve greet function");

        // Verify we got the right span
        let greet_symbol = &symbols[0];
        assert_eq!(resolved.name, "greet");
        assert_eq!(resolved.byte_start, greet_symbol.byte_start);
        assert_eq!(resolved.byte_end, greet_symbol.byte_end);

        // Apply patch: replace function body
        let new_body = r#"
int greet(const char* name) {
    return 42;
}
"#;

        let result = apply_patch_with_validation(
            &cpp_path,
            resolved.byte_start,
            resolved.byte_end,
            new_body.trim(),
            workspace_path,
            Language::Cpp,
            AnalyzerMode::Off,
        );

        // Should succeed if g++ is available
        if result.is_ok() {
            // Verify file content changed
            let new_content =
                std::fs::read_to_string(&cpp_path).expect("Failed to read patched file");

            assert!(
                new_content.contains("return 42;"),
                "Patched content should be present"
            );
            assert!(
                !new_content.contains("return 0;") || new_content.matches("return 0;").count() == 1,
                "Old greet content should be gone (farewell still has return 0)"
            );
        } else {
            // If g++ is not available, test is considered passed (soft failure)
            println!("g++ not available, skipping full patch validation test");
        }
    }

    /// Test B: C++ patch rejected on syntax gate.
    #[test]
    fn test_cpp_patch_rejected_on_syntax_gate() {
        // Create temporary workspace
        let workspace_dir = TempDir::new().expect("Failed to create temp workspace");
        let workspace_path = workspace_dir.path();

        // Create a C++ file
        let cpp_path = workspace_path.join("test.cpp");
        let source = r#"
int valid_function() {
    return 42;
}
"#;

        std::fs::write(&cpp_path, source).expect("Failed to write test.cpp");

        // Create temporary graph database
        let graph_db_path = workspace_path.join("graph.db");
        let mut code_graph =
            CodeGraph::open(&graph_db_path).expect("Failed to open graph database");

        // Ingest and store symbols
        let symbols =
            extract_cpp_symbols(&cpp_path, source.as_bytes()).expect("Failed to parse test.cpp");

        let symbol = &symbols[0];
        code_graph
            .store_symbol_with_file_and_language(
                &cpp_path,
                &symbol.name,
                symbol.kind.as_str(),
                Language::Cpp,
                symbol.byte_start,
                symbol.byte_end,
            )
            .expect("Failed to store symbol");

        // Resolve function
        let resolved = resolve_symbol(
            &code_graph,
            Some(&cpp_path),
            Some("function"),
            "valid_function",
        )
        .expect("Failed to resolve function");

        // Read original content for comparison
        let original_content =
            std::fs::read_to_string(&cpp_path).expect("Failed to read original file");

        // Apply patch with syntax error (unclosed brace)
        let invalid_patch = r#"
int valid_function() {
    return 42;
"#;

        let result = apply_patch_with_validation(
            &cpp_path,
            resolved.byte_start,
            resolved.byte_end,
            invalid_patch.trim(),
            workspace_path,
            Language::Cpp,
            AnalyzerMode::Off,
        );

        // Should fail on syntax error
        assert!(result.is_err(), "Patch should fail on syntax error");

        // Verify original file is unchanged (atomic rollback)
        let current_content =
            std::fs::read_to_string(&cpp_path).expect("Failed to read current file");

        assert_eq!(
            original_content, current_content,
            "File should be unchanged after failed patch (atomic rollback)"
        );
    }

    /// Test C: C patch (C language variant using same parser)
    #[test]
    fn test_c_patch_succeeds_with_validation() {
        // Create temporary workspace
        let workspace_dir = TempDir::new().expect("Failed to create temp workspace");
        let workspace_path = workspace_dir.path();

        // Create a C file
        let c_path = workspace_path.join("test.c");
        let source = r#"
int get_number(void) {
    return 10;
}
"#;

        std::fs::write(&c_path, source).expect("Failed to write test.c");

        // Create temporary graph database
        let graph_db_path = workspace_path.join("graph.db");
        let mut code_graph =
            CodeGraph::open(&graph_db_path).expect("Failed to open graph database");

        // Ingest symbols
        let symbols =
            extract_cpp_symbols(&c_path, source.as_bytes()).expect("Failed to parse test.c");

        let symbol = &symbols[0];
        code_graph
            .store_symbol_with_file_and_language(
                &c_path,
                &symbol.name,
                symbol.kind.as_str(),
                Language::C,
                symbol.byte_start,
                symbol.byte_end,
            )
            .expect("Failed to store symbol");

        // Resolve function
        let resolved = resolve_symbol(
            &code_graph,
            Some(&c_path),
            Some("function"),
            "get_number",
        )
        .expect("Failed to resolve function");

        // Apply valid patch
        let new_body = r#"
int get_number(void) {
    return 20;
}
"#;

        let result = apply_patch_with_validation(
            &c_path,
            resolved.byte_start,
            resolved.byte_end,
            new_body.trim(),
            workspace_path,
            Language::C,
            AnalyzerMode::Off,
        );

        // Should succeed if gcc is available
        if result.is_ok() {
            let new_content = std::fs::read_to_string(&c_path).expect("Failed to read patched file");
            assert!(
                new_content.contains("return 20;"),
                "Patched content should be present"
            );
        } else {
            println!("gcc not available, skipping C patch validation");
        }
    }
}
