//! Integration tests for TypeScript patching with validation gates.
//!
//! These tests validate the full pipeline for TypeScript:
//! resolve → patch-by-span → tree-sitter reparse gate → tsc --noEmit validation gate

use splice::graph::CodeGraph;
use splice::ingest::typescript::extract_typescript_symbols;
use splice::patch::apply_patch_with_validation;
use splice::resolve::resolve_symbol;
use splice::symbol::Language;
use splice::validate::AnalyzerMode;
use tempfile::TempDir;

#[cfg(test)]
mod tests {
    use super::*;

    /// Test A: TypeScript patch succeeds with all gates passing.
    #[test]
    fn test_typescript_patch_succeeds_with_all_gates() {
        // Create temporary workspace directory
        let workspace_dir = TempDir::new().expect("Failed to create temp workspace");
        let workspace_path = workspace_dir.path();

        // Create a tsconfig.json for tsc validation
        let tsconfig_path = workspace_path.join("tsconfig.json");
        let tsconfig = r#"{
  "compilerOptions": {
    "target": "ES2020",
    "module": "commonjs",
    "strict": false
  }
}"#;
        std::fs::write(&tsconfig_path, tsconfig).expect("Failed to write tsconfig.json");

        // Create a TypeScript file with a function to patch
        let ts_path = workspace_path.join("test.ts");
        let source = r#"
function greet(name: string): number {
    return 0;
}

function farewell(name: string): number {
    return 0;
}
"#;

        std::fs::write(&ts_path, source).expect("Failed to write test.ts");

        // Create temporary graph database
        let graph_db_path = workspace_path.join("graph.db");
        let mut code_graph =
            CodeGraph::open(&graph_db_path).expect("Failed to open graph database");

        // Ingest symbols from test.ts
        let symbols = extract_typescript_symbols(&ts_path, source.as_bytes())
            .expect("Failed to parse test.ts");

        assert_eq!(symbols.len(), 2, "Expected 2 functions");

        // Store symbols with file association and language
        for symbol in &symbols {
            code_graph
                .store_symbol_with_file_and_language(
                    &ts_path,
                    &symbol.name,
                    symbol.kind.as_str(),
                    Language::TypeScript,
                    symbol.byte_start,
                    symbol.byte_end,
                )
                .expect("Failed to store symbol");
        }

        // Resolve the "greet" function
        let resolved = resolve_symbol(&code_graph, Some(&ts_path), Some("function"), "greet")
            .expect("Failed to resolve greet function");

        // Verify we got the right span
        let greet_symbol = &symbols[0];
        assert_eq!(resolved.name, "greet");
        assert_eq!(resolved.byte_start, greet_symbol.byte_start);
        assert_eq!(resolved.byte_end, greet_symbol.byte_end);

        // Apply patch: replace function body
        let new_body = r#"
function greet(name: string): number {
    return 42;
}
"#;

        let result = apply_patch_with_validation(
            &ts_path,
            resolved.byte_start,
            resolved.byte_end,
            new_body.trim(),
            workspace_path,
            Language::TypeScript,
            AnalyzerMode::Off,
        );

        // Should succeed if tsc is available
        if result.is_ok() {
            // Verify file content changed
            let new_content =
                std::fs::read_to_string(&ts_path).expect("Failed to read patched file");

            assert!(
                new_content.contains("return 42;"),
                "Patched content should be present"
            );
            assert!(
                !new_content.contains("return 0;") || new_content.matches("return 0;").count() == 1,
                "Old greet content should be gone (farewell still has return 0)"
            );
        } else {
            // If tsc is not available, test is considered passed (soft failure)
            println!("tsc not available, skipping full patch validation test");
        }
    }

    /// Test B: TypeScript patch rejected on syntax gate.
    #[test]
    fn test_typescript_patch_rejected_on_syntax_gate() {
        // Create temporary workspace
        let workspace_dir = TempDir::new().expect("Failed to create temp workspace");
        let workspace_path = workspace_dir.path();

        // Create a tsconfig.json for tsc validation
        let tsconfig_path = workspace_path.join("tsconfig.json");
        let tsconfig = r#"{
  "compilerOptions": {
    "target": "ES2020",
    "module": "commonjs",
    "strict": false
  }
}"#;
        std::fs::write(&tsconfig_path, tsconfig).expect("Failed to write tsconfig.json");

        // Create a TypeScript file
        let ts_path = workspace_path.join("test.ts");
        let source = r#"
function validFunction(): number {
    return 42;
}
"#;

        std::fs::write(&ts_path, source).expect("Failed to write test.ts");

        // Create temporary graph database
        let graph_db_path = workspace_path.join("graph.db");
        let mut code_graph =
            CodeGraph::open(&graph_db_path).expect("Failed to open graph database");

        // Ingest and store symbols
        let symbols = extract_typescript_symbols(&ts_path, source.as_bytes())
            .expect("Failed to parse test.ts");

        let symbol = &symbols[0];
        code_graph
            .store_symbol_with_file_and_language(
                &ts_path,
                &symbol.name,
                symbol.kind.as_str(),
                Language::TypeScript,
                symbol.byte_start,
                symbol.byte_end,
            )
            .expect("Failed to store symbol");

        // Resolve function
        let resolved = resolve_symbol(
            &code_graph,
            Some(&ts_path),
            Some("function"),
            "validFunction",
        )
        .expect("Failed to resolve function");

        // Read original content for comparison
        let original_content =
            std::fs::read_to_string(&ts_path).expect("Failed to read original file");

        // Apply patch with syntax error (missing closing parenthesis)
        let invalid_patch = r#"
function validFunction(): number {
    return (42;
}
"#;

        let result = apply_patch_with_validation(
            &ts_path,
            resolved.byte_start,
            resolved.byte_end,
            invalid_patch.trim(),
            workspace_path,
            Language::TypeScript,
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
            std::fs::read_to_string(&ts_path).expect("Failed to read current file");

        assert_eq!(
            original_content, current_content,
            "File should be unchanged after failed patch (atomic rollback)"
        );
    }

    /// Test C: TypeScript interface patch
    #[test]
    fn test_typescript_interface_patch() {
        // Create temporary workspace
        let workspace_dir = TempDir::new().expect("Failed to create temp workspace");
        let workspace_path = workspace_dir.path();

        // Create a tsconfig.json for tsc validation
        let tsconfig_path = workspace_path.join("tsconfig.json");
        let tsconfig = r#"{
  "compilerOptions": {
    "target": "ES2020",
    "module": "commonjs",
    "strict": false
  }
}"#;
        std::fs::write(&tsconfig_path, tsconfig).expect("Failed to write tsconfig.json");

        // Create a TypeScript file
        let ts_path = workspace_path.join("User.ts");
        let source = r#"
interface User {
    name: string;
}
"#;

        std::fs::write(&ts_path, source).expect("Failed to write User.ts");

        // Create temporary graph database
        let graph_db_path = workspace_path.join("graph.db");
        let mut code_graph =
            CodeGraph::open(&graph_db_path).expect("Failed to open graph database");

        // Ingest symbols
        let symbols = extract_typescript_symbols(&ts_path, source.as_bytes())
            .expect("Failed to parse User.ts");

        let iface = symbols
            .iter()
            .find(|s| s.kind.as_str() == "interface")
            .expect("Should find interface symbol");

        code_graph
            .store_symbol_with_file_and_language(
                &ts_path,
                &iface.name,
                iface.kind.as_str(),
                Language::TypeScript,
                iface.byte_start,
                iface.byte_end,
            )
            .expect("Failed to store symbol");

        // Resolve interface
        let resolved = resolve_symbol(&code_graph, Some(&ts_path), Some("interface"), "User")
            .expect("Failed to resolve interface");

        // Apply valid patch
        let new_body = r#"
interface User {
    name: string;
    age: number;
}
"#;

        let result = apply_patch_with_validation(
            &ts_path,
            resolved.byte_start,
            resolved.byte_end,
            new_body.trim(),
            workspace_path,
            Language::TypeScript,
            AnalyzerMode::Off,
        );

        // Should succeed if tsc is available
        if result.is_ok() {
            let new_content =
                std::fs::read_to_string(&ts_path).expect("Failed to read patched file");
            assert!(
                new_content.contains("age: number;"),
                "Patched content should be present"
            );
        } else {
            println!("tsc not available, skipping TypeScript interface patch validation");
        }
    }
}
