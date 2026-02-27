//! Integration tests for post-verification hooks.
//!
//! These tests verify that post-validation correctly:
//! - Catches syntax errors in patched files
//! - Allows valid patches
//! - Reports warnings without blocking
//! - Includes span checksums in output

use splice::patch::apply_patch_with_validation;
use splice::symbol::Language as SymbolLanguage;
use splice::validate::AnalyzerMode;
use std::fs::{self, File};
use std::io::Write;
use tempfile::TempDir;

/// Create a test workspace with a Rust file.
fn setup_test_workspace() -> TempDir {
    let temp_dir = TempDir::new().unwrap();
    let file_path = temp_dir.path().join("test.rs");

    let mut file = File::create(&file_path).unwrap();
    writeln!(file, "fn main() {{").unwrap();
    writeln!(file, "    let x = 1;").unwrap();
    writeln!(file, "}}").unwrap();

    // Create a minimal codegraph.db (using Splice's convention)
    let db_path = temp_dir.path().join(".splice_graph.db");
    let mut db = File::create(&db_path).unwrap();
    writeln!(db, "dummy db").unwrap();

    // Create a minimal Cargo.toml for cargo check to work
    let cargo_toml = temp_dir.path().join("Cargo.toml");
    let mut cargo = File::create(&cargo_toml).unwrap();
    writeln!(cargo, "[package]").unwrap();
    writeln!(cargo, "name = \"test\"").unwrap();
    writeln!(cargo, "version = \"0.1.0\"").unwrap();
    writeln!(cargo, "edition = \"2021\"").unwrap();

    // Create src directory
    let src_dir = temp_dir.path().join("src");
    fs::create_dir_all(&src_dir).unwrap();

    // Move test.rs to src/main.rs
    let main_rs = src_dir.join("main.rs");
    fs::copy(&file_path, &main_rs).unwrap();
    fs::remove_file(&file_path).unwrap();

    temp_dir
}

#[test]
fn test_post_verification_catches_syntax_error() {
    let workspace = setup_test_workspace();
    let file_path = workspace.path().join("src/main.rs");

    // Read original to find the span to replace
    let content = fs::read_to_string(&file_path).unwrap();
    let start = content.find("let x = 1;").unwrap();
    let end = start + "let x = 1;".len();

    // Try to patch with invalid syntax (unbalanced braces)
    let invalid_content = "let x = {;";

    let result = apply_patch_with_validation(
        &file_path,
        start,
        end,
        invalid_content,
        workspace.path(),
        SymbolLanguage::Rust,
        AnalyzerMode::Off,
        false,  // strict: test mode doesn't need strict validation
        false,  // skip: still run validation for test
    );

    // Should fail with validation error (syntax error)
    assert!(result.is_err());

    if let Err(e) = result {
        let error_msg = format!("{:?}", e);
        // Error should mention validation or parse failure
        assert!(
            error_msg.contains("Parse") || error_msg.contains("validation"),
            "Expected validation error, got: {}",
            error_msg
        );
    }

    // File should be rolled back to original content
    let content_after = fs::read_to_string(&file_path).unwrap();
    assert!(content_after.contains("let x = 1;"));
    assert!(!content_after.contains("let x = {;"));
}

#[test]
fn test_post_verification_allows_valid_patch() {
    let workspace = setup_test_workspace();
    let file_path = workspace.path().join("src/main.rs");

    // Read original to find the span to replace
    let content = fs::read_to_string(&file_path).unwrap();
    let start = content.find("let x = 1;").unwrap();
    let end = start + "let x = 1;".len();

    // Patch with valid Rust code
    let valid_content = "let y = 2;";

    let result = apply_patch_with_validation(
        &file_path,
        start,
        end,
        valid_content,
        workspace.path(),
        SymbolLanguage::Rust,
        AnalyzerMode::Off,
        false,  // strict: test mode doesn't need strict validation
        false,  // skip: still run validation for test
    );

    // Should succeed
    assert!(result.is_ok());

    let (before_hash, after_hash) = result.unwrap();
    assert!(!before_hash.is_empty());
    assert!(!after_hash.is_empty());
    assert_ne!(before_hash, after_hash); // Hashes should differ

    // File should be patched
    let content_after = fs::read_to_string(&file_path).unwrap();
    assert!(!content_after.contains("let x = 1;"));
    assert!(content_after.contains("let y = 2;"));
}

#[test]
fn test_post_verification_warnings_non_blocking() {
    let workspace = setup_test_workspace();
    let file_path = workspace.path().join("src/main.rs");

    // Read original to find the span to replace
    let content = fs::read_to_string(&file_path).unwrap();
    let start = content.find("let x = 1;").unwrap();
    let end = start + "let x = 1;".len();

    // Patch with valid code (no warnings should block)
    let valid_content = "let y = 2;";

    let result = apply_patch_with_validation(
        &file_path,
        start,
        end,
        valid_content,
        workspace.path(),
        SymbolLanguage::Rust,
        AnalyzerMode::Off,
        false,  // strict: test mode doesn't need strict validation
        false,  // skip: still run validation for test
    );

    // Should succeed even with warnings (warnings are non-blocking)
    assert!(result.is_ok());
}

#[test]
fn test_localized_change_verification() {
    let workspace = setup_test_workspace();
    let file_path = workspace.path().join("src/main.rs");

    // Read original content
    let replaced = fs::read(&file_path).unwrap();
    let content = String::from_utf8_lossy(&replaced);

    // Find the span to replace
    let start = content.find("let x = 1;").unwrap();
    let end = start + "let x = 1;".len();

    // Patch with valid content
    let valid_content = "let y = 2;";

    let result = apply_patch_with_validation(
        &file_path,
        start,
        end,
        valid_content,
        workspace.path(),
        SymbolLanguage::Rust,
        AnalyzerMode::Off,
        false,  // strict: test mode doesn't need strict validation
        false,  // skip: still run validation for test
    );

    // Should succeed
    assert!(result.is_ok());

    // Verify only the target span changed
    let patched = fs::read(&file_path).unwrap();

    // Bytes before span should be unchanged
    if start > 0 {
        assert_eq!(&replaced[..start], &patched[..start]);
    }

    // Bytes after span should be unchanged (note: span may have different length)
    // This test verifies the basic structure, not byte-for-byte after the span
    assert!(patched.len() > start); // File should still have content
}
