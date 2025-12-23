//! Integration tests for span-safe patching with validation gates.
//!
//! These tests validate the full pipeline:
//! resolve → patch-by-span → tree-sitter reparse gate → cargo check gate → optional rust-analyzer

use splice::graph::CodeGraph;
use splice::ingest::rust::{extract_rust_symbols, RustSymbolKind};
use splice::resolve::resolve_symbol;
use splice::patch::apply_patch_with_validation;
use splice::validate::AnalyzerMode;
use std::io::Write;
use std::path::Path;
use tempfile::{NamedTempFile, TempDir};

#[cfg(test)]
mod tests {
    use super::*;

    /// Test A: Patch succeeds with all gates passing.
    ///
    /// This test creates a temporary Rust workspace, indexes symbols, resolves a function,
    /// applies a valid patch, and verifies:
    /// 1) File content changed exactly in the resolved byte span
    /// 2) Tree-sitter reparse succeeds
    /// 3) cargo check succeeds
    #[test]
    fn test_patch_succeeds_with_all_gates() {
        // Create temporary workspace directory
        let workspace_dir = TempDir::new().expect("Failed to create temp workspace");
        let workspace_path = workspace_dir.path();

        // Create Cargo.toml
        let cargo_toml_path = workspace_path.join("Cargo.toml");
        let mut cargo_toml = NamedTempFile::new().expect("Failed to create Cargo.toml");
        write!(
            cargo_toml,
            r#"[package]
name = "temp-test"
version = "0.1.0"
edition = "2021"

[lib]
name = "temp_test"
path = "src/lib.rs"
"#
        ).expect("Failed to write Cargo.toml");
        std::fs::rename(cargo_toml.path(), &cargo_toml_path)
            .expect("Failed to move Cargo.toml");

        // Create src directory
        let src_dir = workspace_path.join("src");
        std::fs::create_dir(&src_dir).expect("Failed to create src directory");

        // Create lib.rs with function to patch
        let lib_rs_path = src_dir.join("lib.rs");
        let source = r#"
pub fn greet(name: &str) -> String {
    format!("Hello, {}!", name)
}

pub fn farewell(name: &str) -> String {
    format!("Goodbye, {}!", name)
}
"#;

        std::fs::write(&lib_rs_path, source).expect("Failed to write lib.rs");

        // Create temporary graph database
        let graph_db_path = workspace_path.join("graph.db");
        let mut code_graph = CodeGraph::open(&graph_db_path)
            .expect("Failed to open graph database");

        // Ingest symbols from lib.rs
        let symbols = extract_rust_symbols(&lib_rs_path, source.as_bytes())
            .expect("Failed to parse lib.rs");

        assert_eq!(symbols.len(), 2, "Expected 2 functions");

        // Store symbols with file association
        for symbol in &symbols {
            code_graph
                .store_symbol_with_file(
                    &lib_rs_path,
                    &symbol.name,
                    symbol.kind,
                    symbol.byte_start,
                    symbol.byte_end,
                )
                .expect("Failed to store symbol");
        }

        // Resolve the "greet" function
        let resolved = resolve_symbol(
            &code_graph,
            Some(&lib_rs_path),
            Some(RustSymbolKind::Function),
            "greet",
        ).expect("Failed to resolve greet function");

        // Verify we got the right span
        let greet_symbol = &symbols[0];
        assert_eq!(resolved.name, "greet");
        assert_eq!(resolved.byte_start, greet_symbol.byte_start);
        assert_eq!(resolved.byte_end, greet_symbol.byte_end);

        // Apply patch: replace function body
        let new_body = r#"
pub fn greet(name: &str) -> String {
    format!("Greetings, {}!", name)
}
"#;

        let result = apply_patch_with_validation(
            &lib_rs_path,
            resolved.byte_start,
            resolved.byte_end,
            new_body.trim(),
            workspace_path, // For cargo check
            AnalyzerMode::Off, // rust-analyzer OFF for this test
        );

        // Should succeed
        assert!(result.is_ok(), "Patch should succeed: {:?}", result);

        // Verify file content changed exactly in the span
        let new_content = std::fs::read_to_string(&lib_rs_path)
            .expect("Failed to read patched file");

        assert!(new_content.contains("Greetings, "), "Patched content should be present");
        assert!(!new_content.contains("Hello, "), "Old content should be gone");

        // Verify the other function is unchanged
        assert!(new_content.contains("Goodbye,"), "Other function should be unchanged");
    }

    /// Test B: Patch rejected on syntax gate.
    ///
    /// This test introduces a syntax error and verifies:
    /// 1) SpliceError::ParseValidationFailed is returned
    /// 2) Original file is unchanged (atomic rollback)
    #[test]
    fn test_patch_rejected_on_syntax_gate() {
        // Create temporary workspace
        let workspace_dir = TempDir::new().expect("Failed to create temp workspace");
        let workspace_path = workspace_dir.path();

        // Create Cargo.toml
        let cargo_toml_path = workspace_path.join("Cargo.toml");
        let mut cargo_toml = NamedTempFile::new().expect("Failed to create Cargo.toml");
        write!(
            cargo_toml,
            r#"[package]
name = "temp-test"
version = "0.1.0"
edition = "2021"

[lib]
name = "temp_test"
path = "src/lib.rs"
"#
        ).expect("Failed to write Cargo.toml");
        std::fs::rename(cargo_toml.path(), &cargo_toml_path)
            .expect("Failed to move Cargo.toml");

        // Create src directory
        let src_dir = workspace_path.join("src");
        std::fs::create_dir(&src_dir).expect("Failed to create src directory");

        // Create lib.rs
        let lib_rs_path = src_dir.join("lib.rs");
        let source = r#"
pub fn valid_function() -> i32 {
    42
}
"#;

        std::fs::write(&lib_rs_path, source).expect("Failed to write lib.rs");

        // Create temporary graph database
        let graph_db_path = workspace_path.join("graph.db");
        let mut code_graph = CodeGraph::open(&graph_db_path)
            .expect("Failed to open graph database");

        // Ingest and store symbols
        let symbols = extract_rust_symbols(&lib_rs_path, source.as_bytes())
            .expect("Failed to parse lib.rs");

        let symbol = &symbols[0];
        code_graph
            .store_symbol_with_file(
                &lib_rs_path,
                &symbol.name,
                symbol.kind,
                symbol.byte_start,
                symbol.byte_end,
            )
            .expect("Failed to store symbol");

        // Resolve function
        let resolved = resolve_symbol(
            &code_graph,
            Some(&lib_rs_path),
            Some(RustSymbolKind::Function),
            "valid_function",
        ).expect("Failed to resolve function");

        // Read original content for comparison
        let original_content = std::fs::read_to_string(&lib_rs_path)
            .expect("Failed to read original file");

        // Apply patch with syntax error (unclosed brace)
        let invalid_patch = r#"
pub fn valid_function() -> i32 {
    42
"#;

        let result = apply_patch_with_validation(
            &lib_rs_path,
            resolved.byte_start,
            resolved.byte_end,
            invalid_patch.trim(),
            workspace_path,
            AnalyzerMode::Off, // rust-analyzer OFF for this test
        );

        // Should fail with parse validation error
        assert!(result.is_err(), "Patch should fail on syntax error");

        match result {
            Err(splice::SpliceError::ParseValidationFailed { .. }) => {
                // Expected error type
            }
            Err(other) => {
                panic!("Expected ParseValidationFailed, got: {:?}", other);
            }
            Ok(_) => {
                panic!("Expected error for syntax error, but patch succeeded");
            }
        }

        // Verify original file is unchanged (atomic rollback)
        let current_content = std::fs::read_to_string(&lib_rs_path)
            .expect("Failed to read current file");

        assert_eq!(
            original_content,
            current_content,
            "File should be unchanged after failed patch (atomic rollback)"
        );
    }

    /// Test C: Patch rejected on compiler gate.
    ///
    /// This test introduces a type error and verifies:
    /// 1) SpliceError::CargoCheckFailed is returned
    /// 2) Original file is unchanged (atomic rollback)
    #[test]
    fn test_patch_rejected_on_compiler_gate() {
        // Create temporary workspace
        let workspace_dir = TempDir::new().expect("Failed to create temp workspace");
        let workspace_path = workspace_dir.path();

        // Create Cargo.toml
        let cargo_toml_path = workspace_path.join("Cargo.toml");
        let mut cargo_toml = NamedTempFile::new().expect("Failed to create Cargo.toml");
        write!(
            cargo_toml,
            r#"[package]
name = "temp-test"
version = "0.1.0"
edition = "2021"

[lib]
name = "temp_test"
path = "src/lib.rs"
"#
        ).expect("Failed to write Cargo.toml");
        std::fs::rename(cargo_toml.path(), &cargo_toml_path)
            .expect("Failed to move Cargo.toml");

        // Create src directory
        let src_dir = workspace_path.join("src");
        std::fs::create_dir(&src_dir).expect("Failed to create src directory");

        // Create lib.rs with function returning i32
        let lib_rs_path = src_dir.join("lib.rs");
        let source = r#"
pub fn get_number() -> i32 {
    42
}
"#;

        std::fs::write(&lib_rs_path, source).expect("Failed to write lib.rs");

        // Create temporary graph database
        let graph_db_path = workspace_path.join("graph.db");
        let mut code_graph = CodeGraph::open(&graph_db_path)
            .expect("Failed to open graph database");

        // Ingest and store symbols
        let symbols = extract_rust_symbols(&lib_rs_path, source.as_bytes())
            .expect("Failed to parse lib.rs");

        let symbol = &symbols[0];
        code_graph
            .store_symbol_with_file(
                &lib_rs_path,
                &symbol.name,
                symbol.kind,
                symbol.byte_start,
                symbol.byte_end,
            )
            .expect("Failed to store symbol");

        // Resolve function
        let resolved = resolve_symbol(
            &code_graph,
            Some(&lib_rs_path),
            Some(RustSymbolKind::Function),
            "get_number",
        ).expect("Failed to resolve function");

        // Read original content for comparison
        let original_content = std::fs::read_to_string(&lib_rs_path)
            .expect("Failed to read original file");

        // Apply patch that breaks the type signature (returns String instead of i32)
        let type_error_patch = r#"
pub fn get_number() -> i32 {
    "this is a string not an i32"
}
"#;

        let result = apply_patch_with_validation(
            &lib_rs_path,
            resolved.byte_start,
            resolved.byte_end,
            type_error_patch.trim(),
            workspace_path,
            AnalyzerMode::Off, // rust-analyzer OFF for this test
        );

        // Should fail with cargo check error
        assert!(result.is_err(), "Patch should fail on type error");

        match result {
            Err(splice::SpliceError::CargoCheckFailed { .. }) => {
                // Expected error type
            }
            Err(other) => {
                panic!("Expected CargoCheckFailed, got: {:?}", other);
            }
            Ok(_) => {
                panic!("Expected error for type mismatch, but patch succeeded");
            }
        }

        // Verify original file is unchanged (atomic rollback)
        let current_content = std::fs::read_to_string(&lib_rs_path)
            .expect("Failed to read current file");

        assert_eq!(
            original_content,
            current_content,
            "File should be unchanged after failed patch (atomic rollback)"
        );
    }
}
