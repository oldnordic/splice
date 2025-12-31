//! Integration tests for span-safe patching with validation gates.
//!
//! These tests validate the full pipeline:
//! resolve → patch-by-span → tree-sitter reparse gate → cargo check gate → optional rust-analyzer

use splice::graph::CodeGraph;
use splice::ingest::rust::extract_rust_symbols;
use splice::patch::apply_patch_with_validation;
use splice::patch::{apply_batch_with_validation, SpanBatch, SpanReplacement};
use splice::resolve::resolve_symbol;
use splice::symbol::Language;
use splice::validate::AnalyzerMode;
use std::io::Write;
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
        )
        .expect("Failed to write Cargo.toml");
        std::fs::rename(cargo_toml.path(), &cargo_toml_path).expect("Failed to move Cargo.toml");

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
        let mut code_graph =
            CodeGraph::open(&graph_db_path).expect("Failed to open graph database");

        // Ingest symbols from lib.rs
        let symbols =
            extract_rust_symbols(&lib_rs_path, source.as_bytes()).expect("Failed to parse lib.rs");

        assert_eq!(symbols.len(), 2, "Expected 2 functions");

        // Store symbols with file association and language
        for symbol in &symbols {
            code_graph
                .store_symbol_with_file_and_language(
                    &lib_rs_path,
                    &symbol.name,
                    symbol.kind.as_str(),
                    Language::Rust,
                    symbol.byte_start,
                    symbol.byte_end,
                )
                .expect("Failed to store symbol");
        }

        // Resolve the "greet" function
        let resolved = resolve_symbol(&code_graph, Some(&lib_rs_path), Some("function"), "greet")
            .expect("Failed to resolve greet function");

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
            workspace_path,    // For compiler check
            Language::Rust,    // Rust file
            AnalyzerMode::Off, // rust-analyzer OFF for this test
        );

        // Should succeed
        assert!(result.is_ok(), "Patch should succeed: {:?}", result);

        // Verify file content changed exactly in the span
        let new_content =
            std::fs::read_to_string(&lib_rs_path).expect("Failed to read patched file");

        assert!(
            new_content.contains("Greetings, "),
            "Patched content should be present"
        );
        assert!(
            !new_content.contains("Hello, "),
            "Old content should be gone"
        );

        // Verify the other function is unchanged
        assert!(
            new_content.contains("Goodbye,"),
            "Other function should be unchanged"
        );
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
        )
        .expect("Failed to write Cargo.toml");
        std::fs::rename(cargo_toml.path(), &cargo_toml_path).expect("Failed to move Cargo.toml");

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
        let mut code_graph =
            CodeGraph::open(&graph_db_path).expect("Failed to open graph database");

        // Ingest and store symbols
        let symbols =
            extract_rust_symbols(&lib_rs_path, source.as_bytes()).expect("Failed to parse lib.rs");

        let symbol = &symbols[0];
        code_graph
            .store_symbol_with_file_and_language(
                &lib_rs_path,
                &symbol.name,
                symbol.kind.as_str(),
                Language::Rust,
                symbol.byte_start,
                symbol.byte_end,
            )
            .expect("Failed to store symbol");

        // Resolve function
        let resolved = resolve_symbol(
            &code_graph,
            Some(&lib_rs_path),
            Some("function"),
            "valid_function",
        )
        .expect("Failed to resolve function");

        // Read original content for comparison
        let original_content =
            std::fs::read_to_string(&lib_rs_path).expect("Failed to read original file");

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
            Language::Rust,    // Rust file
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
        let current_content =
            std::fs::read_to_string(&lib_rs_path).expect("Failed to read current file");

        assert_eq!(
            original_content, current_content,
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
        )
        .expect("Failed to write Cargo.toml");
        std::fs::rename(cargo_toml.path(), &cargo_toml_path).expect("Failed to move Cargo.toml");

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
        let mut code_graph =
            CodeGraph::open(&graph_db_path).expect("Failed to open graph database");

        // Ingest and store symbols
        let symbols =
            extract_rust_symbols(&lib_rs_path, source.as_bytes()).expect("Failed to parse lib.rs");

        let symbol = &symbols[0];
        code_graph
            .store_symbol_with_file_and_language(
                &lib_rs_path,
                &symbol.name,
                symbol.kind.as_str(),
                Language::Rust,
                symbol.byte_start,
                symbol.byte_end,
            )
            .expect("Failed to store symbol");

        // Resolve function
        let resolved = resolve_symbol(
            &code_graph,
            Some(&lib_rs_path),
            Some("function"),
            "get_number",
        )
        .expect("Failed to resolve function");

        // Read original content for comparison
        let original_content =
            std::fs::read_to_string(&lib_rs_path).expect("Failed to read original file");

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
            Language::Rust,    // Rust file
            AnalyzerMode::Off, // rust-analyzer OFF for this test
        );

        // Should fail with compiler validation error
        assert!(result.is_err(), "Patch should fail on type error");

        match result {
            Err(splice::SpliceError::CargoCheckFailed { .. }) => {
                // Expected error type (Rust-specific cargo check)
            }
            Err(other) => {
                panic!("Expected CargoCheckFailed, got: {:?}", other);
            }
            Ok(_) => {
                panic!("Expected error for type mismatch, but patch succeeded");
            }
        }

        // Verify original file is unchanged (atomic rollback)
        let current_content =
            std::fs::read_to_string(&lib_rs_path).expect("Failed to read current file");

        assert_eq!(
            original_content, current_content,
            "File should be unchanged after failed patch (atomic rollback)"
        );
    }

    /// Test D: Batch patch rolls back when a later replacement fails validation.
    ///
    /// This test sets up two files. The first replacement is valid, the second introduces
    /// a type error. The entire batch must fail atomically with both files untouched.
    #[test]
    fn test_apply_batch_rolls_back_on_failure() {
        let workspace_dir = TempDir::new().expect("Failed to create temp workspace");
        let workspace_path = workspace_dir.path();

        // Create Cargo manifest
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
        )
        .expect("Failed to write Cargo.toml");
        std::fs::rename(cargo_toml.path(), &cargo_toml_path).expect("Failed to move Cargo.toml");

        let src_dir = workspace_path.join("src");
        std::fs::create_dir(&src_dir).expect("Failed to create src directory");

        // lib.rs contains a helper invoked by module a and b
        let lib_rs_path = src_dir.join("lib.rs");
        std::fs::write(
            &lib_rs_path,
            r#"
pub fn helper(x: i32) -> i32 {
    x + 1
}

pub mod a;
pub mod b;
"#,
        )
        .expect("Failed to write lib.rs");

        let file_a = src_dir.join("a.rs");
        std::fs::write(
            &file_a,
            r#"
pub fn value() -> i32 {
    helper(10)
}
"#,
        )
        .expect("Failed to write a.rs");

        let file_b = src_dir.join("b.rs");
        std::fs::write(
            &file_b,
            r#"
pub fn broken() -> i32 {
    helper(5)
}
"#,
        )
        .expect("Failed to write b.rs");

        // Compute spans for helper usage
        let mut replacements = Vec::new();

        let symbols =
            extract_rust_symbols(&file_a, std::fs::read(&file_a).unwrap().as_slice()).unwrap();
        let target = symbols.iter().find(|s| s.name == "value").unwrap();
        replacements.push(SpanReplacement {
            file: file_a.clone(),
            start: target.byte_start,
            end: target.byte_end,
            content: r#"
pub fn value() -> i32 {
    helper(42)
}
"#
            .trim()
            .to_string(),
        });

        let symbols_b =
            extract_rust_symbols(&file_b, std::fs::read(&file_b).unwrap().as_slice()).unwrap();
        let target_b = symbols_b.iter().find(|s| s.name == "broken").unwrap();
        replacements.push(SpanReplacement {
            file: file_b.clone(),
            start: target_b.byte_start,
            end: target_b.byte_end,
            content: r#"
pub fn broken() -> i32 {
    helper("oops")
}
"#
            .trim()
            .to_string(),
        });

        let batches = vec![SpanBatch::new(replacements)];
        let original_a = std::fs::read_to_string(&file_a).unwrap();
        let original_b = std::fs::read_to_string(&file_b).unwrap();

        let result = apply_batch_with_validation(
            &batches,
            workspace_path,
            Language::Rust,
            AnalyzerMode::Off,
        );

        assert!(
            result.is_err(),
            "Batch should fail due to invalid second patch"
        );
        let err = result.err().unwrap();
        assert!(
            matches!(err, splice::SpliceError::CargoCheckFailed { .. }),
            "Expected CargoCheckFailed, got {:?}",
            err
        );

        assert_eq!(
            original_a,
            std::fs::read_to_string(&file_a).unwrap(),
            "File a.rs should remain unchanged after batch failure"
        );
        assert_eq!(
            original_b,
            std::fs::read_to_string(&file_b).unwrap(),
            "File b.rs should remain unchanged after batch failure"
        );
    }
}
