//! Integration tests for rich span extensions across all 7 languages.
//!
//! Tests verify that rich span fields work correctly for:
//! - Rust, Python, C, C++, Java, JavaScript, TypeScript
//!
//! Fields tested:
//! - context (before, selected, after arrays)
//! - semantic_kind (function, variable, parameter, etc.)
//! - language (detected from file extension)
//! - checksum_before (SHA-256 of span bytes)
//! - file_checksum_before (SHA-256 of entire file)
//! - error_code (SPL-E### format with severity, location, hint)

use serde_json::json;
use splice::checksum::{checksum_file, checksum_span};
use splice::context::{extract_context, extract_context_asymmetric};
use splice::error_codes::{ErrorCode, SpliceErrorCode};
use splice::ingest::{detect_language, detect_semantic_kind, Language};
use splice::output::SpanResult;
use std::io::Write;
use tempfile::NamedTempFile;

fn span_kind(span: &SpanResult) -> Option<String> {
    span.semantics.as_ref().map(|s| s.kind.clone())
}

fn span_language(span: &SpanResult) -> Option<String> {
    span.semantics.as_ref().map(|s| s.language.clone())
}

fn span_checksum_before(span: &SpanResult) -> Option<String> {
    span.checksums
        .as_ref()
        .and_then(|c| c.checksum_before.clone())
}

fn span_file_checksum_before(span: &SpanResult) -> Option<String> {
    span.checksums
        .as_ref()
        .and_then(|c| c.file_checksum_before.clone())
}

///////////////////////////////////////////////////////////////////////////////
// Language-specific test fixtures
///////////////////////////////////////////////////////////////////////////////

/// Rust function fixture with doc comment
const RUST_FUNCTION: &str = r#"
/// Adds two numbers together.
///
/// # Examples
/// ```
/// let result = add(2, 3);
/// assert_eq!(result, 5);
/// ```
pub fn add(a: i32, b: i32) -> i32 {
    a + b
}

/// Multiplies two numbers
pub fn multiply(a: i32, b: i32) -> i32 {
    a * b
}
"#;

/// Python function fixture with docstring
const PYTHON_FUNCTION: &str = r#"
"""Adds two numbers.

This function performs addition of two integers.

Args:
    a: First integer
    b: Second integer

Returns:
    The sum of a and b
"""
def add(a: int, b: int) -> int:
    """Add two integers together."""
    return a + b

def multiply(a: int, b: int) -> int:
    """Multiply two integers."""
    return a * b
"#;

/// C function fixture with block comment
const C_FUNCTION: &str = r#"
/* Adds two numbers.
 *
 * This function performs addition of two integers.
 *
 * @param a First integer
 * @param b Second integer
 * @return The sum of a and b
 */
int add(int a, int b) {
    return a + b;
}

/* Multiplies two numbers */
int multiply(int a, int b) {
    return a * b;
}
"#;

/// C++ function fixture with block comment
const CPP_FUNCTION: &str = r#"
// Adds two numbers.
// This function performs addition of two integers.
//
// @param a First integer
// @param b Second integer
// @return The sum of a and b
int add(int a, int b) {
    return a + b;
}

// Multiplies two numbers
int multiply(int a, int b) {
    return a * b;
}
"#;

/// Java class fixture with Javadoc
const JAVA_CLASS: &str = r#"
/**
 * A simple calculator class.
 *
 * This class provides basic arithmetic operations.
 */
public class Calculator {
    /**
     * Adds two numbers.
     *
     * @param a First number
     * @param b Second number
     * @return The sum
     */
    public int add(int a, int b) {
        return a + b;
    }

    /**
     * Multiplies two numbers.
     */
    public int multiply(int a, int b) {
        return a * b;
    }
}
"#;

/// JavaScript function fixture with JSDoc
const JAVASCRIPT_FUNCTION: &str = r#"
/**
 * Adds two numbers.
 *
 * @param {number} a - First number
 * @param {number} b - Second number
 * @returns {number} The sum
 */
function add(a, b) {
    return a + b;
}

/**
 * Multiplies two numbers.
 *
 * @param {number} a - First number
 * @param {number} b - Second number
 * @returns {number} The product
 */
const multiply = (a, b) => a * b;

// Simple function
function subtract(a, b) {
    return a - b;
}
"#;

/// TypeScript function fixture with typed JSDoc
const TYPESCRIPT_FUNCTION: &str = r#"
/**
 * Adds two numbers.
 *
 * @param a - First number
 * @param b - Second number
 * @returns The sum
 */
function add(a: number, b: number): number {
    return a + b;
}

/**
 * Multiplies two numbers.
 */
const multiply = (a: number, b: number): number => {
    return a * b;
};

// TypeScript interface
interface Calculator {
    add(a: number, b: number): number;
    multiply(a: number, b: number): number;
}

// TypeScript class implementing interface
class BasicCalculator implements Calculator {
    add(a: number, b: number): number {
        return a + b;
    }

    multiply(a: number, b: number): number {
        return a * b;
    }
}
"#;

///////////////////////////////////////////////////////////////////////////////
// Test 1: Language Detection for All 7 Languages
///////////////////////////////////////////////////////////////////////////////

#[test]
fn test_language_detection_all_languages() {
    let test_cases = vec![
        ("test.rs", Language::Rust),
        ("lib.rs", Language::Rust),
        ("main.rs", Language::Rust),
        ("test.py", Language::Python),
        ("script.py", Language::Python),
        ("app.py", Language::Python),
        ("test.c", Language::C),
        ("header.h", Language::C),
        ("main.c", Language::C),
        ("test.cpp", Language::Cpp),
        ("header.hpp", Language::Cpp),
        ("lib.cc", Language::Cpp),
        ("test.java", Language::Java),
        ("Main.java", Language::Java),
        ("test.js", Language::JavaScript),
        ("app.mjs", Language::JavaScript),
        ("module.cjs", Language::JavaScript),
        ("test.ts", Language::TypeScript),
        ("component.tsx", Language::TypeScript),
    ];

    for (filename, expected_lang) in test_cases {
        let detected = detect_language(std::path::Path::new(filename));
        assert_eq!(
            detected,
            Some(expected_lang),
            "Language detection failed for {}: got {:?}, expected {:?}",
            filename,
            detected,
            expected_lang
        );
    }
}

///////////////////////////////////////////////////////////////////////////////
// Test 2: Semantic Kind Detection for All 7 Languages
///////////////////////////////////////////////////////////////////////////////

#[test]
fn test_semantic_kind_detection_rust() {
    let test_cases = vec![
        ("function_item", "function"),
        ("struct_item", "type"),
        ("enum_item", "type"),
        ("trait_item", "trait"),
        ("impl_item", "trait"),
        ("static_item", "constant"),
        ("const_item", "constant"),
        ("type_item", "type_alias"),
        ("mod_item", "module"),
    ];

    for (node_kind, expected_semantic_kind) in test_cases {
        let detected = detect_semantic_kind(node_kind, Language::Rust);
        assert_eq!(
            detected.as_str(),
            expected_semantic_kind,
            "Semantic kind mismatch for Rust node kind '{}': got '{}', expected '{}'",
            node_kind,
            detected.as_str(),
            expected_semantic_kind
        );
    }
}

#[test]
fn test_semantic_kind_detection_python() {
    let test_cases = vec![
        ("function_definition", "function"),
        ("class_definition", "type"),
        ("assignment", "variable"),
        ("annotated_assignment", "variable"),
        ("type_alias_statement", "type_alias"),
        ("import_statement", "module"),
        ("import_from_statement", "module"),
    ];

    for (node_kind, expected_semantic_kind) in test_cases {
        let detected = detect_semantic_kind(node_kind, Language::Python);
        assert_eq!(
            detected.as_str(),
            expected_semantic_kind,
            "Semantic kind mismatch for Python node kind '{}': got '{}', expected '{}'",
            node_kind,
            detected.as_str(),
            expected_semantic_kind
        );
    }
}

#[test]
fn test_semantic_kind_detection_javascript() {
    let test_cases = vec![
        ("function_declaration", "function"),
        ("function_expression", "function"),
        ("arrow_function", "function"),
        ("class_declaration", "type"),
        ("class_expression", "type"),
        ("variable_declaration", "variable"),
        ("lexical_declaration", "variable"),
        ("enum_declaration", "enum"),
        ("import_statement", "module"),
        ("export_statement", "module"),
    ];

    for (node_kind, expected_semantic_kind) in test_cases {
        let detected = detect_semantic_kind(node_kind, Language::JavaScript);
        assert_eq!(
            detected.as_str(),
            expected_semantic_kind,
            "Semantic kind mismatch for JavaScript node kind '{}': got '{}', expected '{}'",
            node_kind,
            detected.as_str(),
            expected_semantic_kind
        );
    }
}

#[test]
fn test_semantic_kind_detection_typescript() {
    let test_cases = vec![
        ("function_declaration", "function"),
        ("arrow_function", "function"),
        ("function_expression", "function"),
        ("class_declaration", "type"),
        ("class_expression", "type"),
        ("interface_declaration", "trait"),
        ("type_alias_declaration", "type_alias"),
        ("variable_declaration", "variable"),
        ("lexical_declaration", "variable"),
        ("enum_declaration", "enum"),
        ("method_definition", "function"),
        ("namespace_declaration", "module"),
        ("constructor_parameters", "constructor"),
    ];

    for (node_kind, expected_semantic_kind) in test_cases {
        let detected = detect_semantic_kind(node_kind, Language::TypeScript);
        assert_eq!(
            detected.as_str(),
            expected_semantic_kind,
            "Semantic kind mismatch for TypeScript node kind '{}': got '{}', expected '{}'",
            node_kind,
            detected.as_str(),
            expected_semantic_kind
        );
    }
}

#[test]
fn test_semantic_kind_detection_java() {
    let test_cases = vec![
        ("method_declaration", "function"),
        ("constructor_declaration", "constructor"),
        ("class_declaration", "type"),
        ("interface_declaration", "type"),
        ("enum_declaration", "enum"),
        ("field_declaration", "variable"),
    ];

    for (node_kind, expected_semantic_kind) in test_cases {
        let detected = detect_semantic_kind(node_kind, Language::Java);
        assert_eq!(
            detected.as_str(),
            expected_semantic_kind,
            "Semantic kind mismatch for Java node kind '{}': got '{}', expected '{}'",
            node_kind,
            detected.as_str(),
            expected_semantic_kind
        );
    }
}

#[test]
fn test_semantic_kind_detection_c() {
    let test_cases = vec![
        ("function_definition", "function"),
        ("declaration", "variable"),
        ("struct_specifier", "type"),
        ("enum_specifier", "enum"),
        ("type_definition", "type_alias"),
    ];

    for (node_kind, expected_semantic_kind) in test_cases {
        let detected = detect_semantic_kind(node_kind, Language::C);
        assert_eq!(
            detected.as_str(),
            expected_semantic_kind,
            "Semantic kind mismatch for C node kind '{}': got '{}', expected '{}'",
            node_kind,
            detected.as_str(),
            expected_semantic_kind
        );
    }
}

#[test]
fn test_semantic_kind_detection_cpp() {
    let test_cases = vec![
        ("function_definition", "function"),
        ("declaration", "variable"),
        ("class_specifier", "type"),
        ("struct_specifier", "type"),
        ("enum_specifier", "enum"),
        ("type_definition", "type_alias"),
        ("constructor_definition", "constructor"),
    ];

    for (node_kind, expected_semantic_kind) in test_cases {
        let detected = detect_semantic_kind(node_kind, Language::Cpp);
        assert_eq!(
            detected.as_str(),
            expected_semantic_kind,
            "Semantic kind mismatch for C++ node kind '{}': got '{}', expected '{}'",
            node_kind,
            detected.as_str(),
            expected_semantic_kind
        );
    }
}

///////////////////////////////////////////////////////////////////////////////
// Test 3: Context Extraction for All 7 Languages
///////////////////////////////////////////////////////////////////////////////

#[test]
fn test_context_extraction_rust() {
    let mut file = NamedTempFile::new().unwrap();
    file.write_all(RUST_FUNCTION.as_bytes()).unwrap();
    let file_path = file.path();

    // Find offset of the first "add" function
    let source = std::fs::read_to_string(file_path).unwrap();
    let offset = source.find("pub fn add").unwrap();
    let offset_end = offset + 50; // Include part of the function

    let ctx = extract_context(file_path, offset, offset_end, 2).unwrap();

    // Verify context arrays exist
    assert!(!ctx.selected.is_empty(), "Selected should not be empty");
    // Verify context contains meaningful content
    let has_content = ctx.before.iter().any(|l| l.contains("///"))
        || ctx.selected.iter().any(|l| l.contains("add"))
        || ctx.after.iter().any(|l| l.contains("multiply"));
    assert!(has_content, "Context should contain meaningful content");
}

#[test]
fn test_context_extraction_python() {
    let mut file = NamedTempFile::new().unwrap();
    file.write_all(PYTHON_FUNCTION.as_bytes()).unwrap();
    let file_path = file.path();

    let source = std::fs::read_to_string(file_path).unwrap();
    let offset = source.find("def add").unwrap();
    let offset_end = offset + 50;

    let ctx = extract_context(file_path, offset, offset_end, 2).unwrap();

    assert!(!ctx.selected.is_empty(), "Selected should not be empty");
    let has_content = ctx.before.iter().any(|l| l.contains("Adds"))
        || ctx.selected.iter().any(|l| l.contains("def add"))
        || ctx.after.iter().any(|l| l.contains("multiply"));
    assert!(has_content, "Context should contain meaningful content");
}

#[test]
fn test_context_extraction_c() {
    let mut file = NamedTempFile::new().unwrap();
    file.write_all(C_FUNCTION.as_bytes()).unwrap();
    let file_path = file.path();

    let source = std::fs::read_to_string(file_path).unwrap();
    let offset = source.find("int add").unwrap();
    let offset_end = offset + 50;

    let ctx = extract_context(file_path, offset, offset_end, 1).unwrap();

    assert!(!ctx.selected.is_empty(), "Selected should not be empty");
    let has_content = ctx.before.iter().any(|l| l.contains("/*"))
        || ctx.selected.iter().any(|l| l.contains("int add"));
    assert!(has_content, "Context should contain meaningful content");
}

#[test]
fn test_context_extraction_cpp() {
    let mut file = NamedTempFile::new().unwrap();
    file.write_all(CPP_FUNCTION.as_bytes()).unwrap();
    let file_path = file.path();

    let source = std::fs::read_to_string(file_path).unwrap();
    let offset = source.find("int add").unwrap();
    let offset_end = offset + 50;

    let ctx = extract_context(file_path, offset, offset_end, 1).unwrap();

    assert!(!ctx.selected.is_empty(), "Selected should not be empty");
    let has_content = ctx.before.iter().any(|l| l.contains("//"))
        || ctx.selected.iter().any(|l| l.contains("int add"));
    assert!(has_content, "Context should contain meaningful content");
}

#[test]
fn test_context_extraction_java() {
    let mut file = NamedTempFile::new().unwrap();
    file.write_all(JAVA_CLASS.as_bytes()).unwrap();
    let file_path = file.path();

    let source = std::fs::read_to_string(file_path).unwrap();
    let offset = source.find("public int add").unwrap();
    let offset_end = offset + 50;

    let ctx = extract_context(file_path, offset, offset_end, 2).unwrap();

    assert!(!ctx.selected.is_empty(), "Selected should not be empty");
    let has_content = ctx.before.iter().any(|l| l.contains("*"))
        || ctx.selected.iter().any(|l| l.contains("public int add"));
    assert!(has_content, "Context should contain meaningful content");
}

#[test]
fn test_context_extraction_javascript() {
    let mut file = NamedTempFile::new().unwrap();
    file.write_all(JAVASCRIPT_FUNCTION.as_bytes()).unwrap();
    let file_path = file.path();

    let source = std::fs::read_to_string(file_path).unwrap();
    let offset = source.find("function add").unwrap();
    let offset_end = offset + 50;

    let ctx = extract_context(file_path, offset, offset_end, 1).unwrap();

    assert!(!ctx.selected.is_empty(), "Selected should not be empty");
    let has_content = ctx.before.iter().any(|l| l.contains("*"))
        || ctx.selected.iter().any(|l| l.contains("function add"));
    assert!(has_content, "Context should contain meaningful content");
}

#[test]
fn test_context_extraction_typescript() {
    let mut file = NamedTempFile::new().unwrap();
    file.write_all(TYPESCRIPT_FUNCTION.as_bytes()).unwrap();
    let file_path = file.path();

    let source = std::fs::read_to_string(file_path).unwrap();
    let offset = source.find("function add").unwrap();
    let offset_end = offset + 50;

    let ctx = extract_context(file_path, offset, offset_end, 1).unwrap();

    assert!(!ctx.selected.is_empty(), "Selected should not be empty");
    let has_content = ctx.before.iter().any(|l| l.contains("*"))
        || ctx.selected.iter().any(|l| l.contains("function add"));
    assert!(has_content, "Context should contain meaningful content");
}

///////////////////////////////////////////////////////////////////////////////
// Test 4: Checksum Calculation for All 7 Languages
///////////////////////////////////////////////////////////////////////////////

#[test]
fn test_checksum_calculation_rust() {
    let mut file = NamedTempFile::new().unwrap();
    file.write_all(RUST_FUNCTION.as_bytes()).unwrap();
    let file_path = file.path();

    let byte_start = 10;
    let byte_end = 100;

    let span_checksum = checksum_span(file_path, byte_start, byte_end).unwrap();
    let file_checksum = checksum_file(file_path).unwrap();

    // Verify checksums are valid hex strings (64 chars for SHA-256)
    assert_eq!(
        span_checksum.as_hex().len(),
        64,
        "Span checksum should be 64 hex chars for Rust"
    );
    assert_eq!(
        file_checksum.as_hex().len(),
        64,
        "File checksum should be 64 hex chars for Rust"
    );

    // Verify checksums are deterministic
    let span_checksum2 = checksum_span(file_path, byte_start, byte_end).unwrap();
    assert_eq!(
        span_checksum.as_hex(),
        span_checksum2.as_hex(),
        "Checksums should be deterministic for Rust"
    );
}

#[test]
fn test_checksum_calculation_python() {
    let mut file = NamedTempFile::new().unwrap();
    file.write_all(PYTHON_FUNCTION.as_bytes()).unwrap();
    let file_path = file.path();

    let byte_start = 10;
    let byte_end = 100;

    let span_checksum = checksum_span(file_path, byte_start, byte_end).unwrap();
    let file_checksum = checksum_file(file_path).unwrap();

    assert_eq!(span_checksum.as_hex().len(), 64);
    assert_eq!(file_checksum.as_hex().len(), 64);

    // Different spans should have different checksums
    let span_checksum2 = checksum_span(file_path, 50, 150).unwrap();
    assert_ne!(
        span_checksum.as_hex(),
        span_checksum2.as_hex(),
        "Different spans should have different checksums"
    );
}

#[test]
fn test_checksum_calculation_c() {
    let mut file = NamedTempFile::new().unwrap();
    file.write_all(C_FUNCTION.as_bytes()).unwrap();
    let file_path = file.path();

    let span_checksum = checksum_span(file_path, 20, 100).unwrap();
    let file_checksum = checksum_file(file_path).unwrap();

    assert_eq!(span_checksum.as_hex().len(), 64);
    assert_eq!(file_checksum.as_hex().len(), 64);
}

#[test]
fn test_checksum_calculation_cpp() {
    let mut file = NamedTempFile::new().unwrap();
    file.write_all(CPP_FUNCTION.as_bytes()).unwrap();
    let file_path = file.path();

    let span_checksum = checksum_span(file_path, 20, 100).unwrap();
    let file_checksum = checksum_file(file_path).unwrap();

    assert_eq!(span_checksum.as_hex().len(), 64);
    assert_eq!(file_checksum.as_hex().len(), 64);
}

#[test]
fn test_checksum_calculation_java() {
    let mut file = NamedTempFile::new().unwrap();
    file.write_all(JAVA_CLASS.as_bytes()).unwrap();
    let file_path = file.path();

    let span_checksum = checksum_span(file_path, 20, 100).unwrap();
    let file_checksum = checksum_file(file_path).unwrap();

    assert_eq!(span_checksum.as_hex().len(), 64);
    assert_eq!(file_checksum.as_hex().len(), 64);
}

#[test]
fn test_checksum_calculation_javascript() {
    let mut file = NamedTempFile::new().unwrap();
    file.write_all(JAVASCRIPT_FUNCTION.as_bytes()).unwrap();
    let file_path = file.path();

    let span_checksum = checksum_span(file_path, 20, 100).unwrap();
    let file_checksum = checksum_file(file_path).unwrap();

    assert_eq!(span_checksum.as_hex().len(), 64);
    assert_eq!(file_checksum.as_hex().len(), 64);
}

#[test]
fn test_checksum_calculation_typescript() {
    let mut file = NamedTempFile::new().unwrap();
    file.write_all(TYPESCRIPT_FUNCTION.as_bytes()).unwrap();
    let file_path = file.path();

    let span_checksum = checksum_span(file_path, 20, 100).unwrap();
    let file_checksum = checksum_file(file_path).unwrap();

    assert_eq!(span_checksum.as_hex().len(), 64);
    assert_eq!(file_checksum.as_hex().len(), 64);
}

///////////////////////////////////////////////////////////////////////////////
// Test 5: Error Code Formatting for All Languages
///////////////////////////////////////////////////////////////////////////////

#[test]
fn test_error_code_formatting_all() {
    let test_cases = vec![
        (
            "SPL-E001",
            "error",
            "/path/to/file.rs:10:5",
            "Symbol not found",
        ),
        (
            "SPL-E002",
            "warning",
            "/path/to/file.py:20:10",
            "Ambiguous symbol",
        ),
        (
            "SPL-E003",
            "error",
            "/path/to/file.ts:30:15",
            "Reference failed",
        ),
        ("SPL-E011", "error", "file.c:1:1", "Parse error"),
        ("SPL-E021", "error", "main.cpp:5:3", "Invalid span"),
        (
            "SPL-E043",
            "error",
            "Calculator.java:10:1",
            "Compiler validation failed",
        ),
        (
            "SPL-W001",
            "warning",
            "app.js:42:7",
            "Ambiguous symbol as warning",
        ),
    ];

    for (code, severity, location, hint) in test_cases {
        let error_code = ErrorCode::new(code, severity, location.to_string(), hint.to_string());

        assert_eq!(error_code.code, code);
        assert_eq!(error_code.severity, severity);
        assert_eq!(error_code.location, Some(location.to_string()));
        assert_eq!(error_code.hint, hint);
    }
}

#[test]
fn test_error_code_from_splice_code_all_severities() {
    // Error severity
    let error_code = ErrorCode::from_splice_code(
        SpliceErrorCode::SymbolNotFound,
        Some("src/main.rs"),
        Some(42),
        Some(10),
    );
    assert_eq!(error_code.code, "SPL-E001");
    assert_eq!(error_code.severity, "error");
    assert_eq!(error_code.location, Some("src/main.rs:42:10".to_string()));

    // Warning severity
    let warning_code = ErrorCode::from_splice_code(
        SpliceErrorCode::AmbiguousSymbol,
        Some("test.py"),
        Some(10),
        Some(5),
    );
    assert_eq!(warning_code.code, "SPL-E002");
    assert_eq!(warning_code.severity, "warning");
    assert_eq!(warning_code.location, Some("test.py:10:5".to_string()));

    // Parse error
    let parse_error = ErrorCode::from_splice_code(
        SpliceErrorCode::ParseError,
        Some("file.ts"),
        Some(1),
        Some(1),
    );
    assert_eq!(parse_error.code, "SPL-E011");
    assert_eq!(parse_error.severity, "error");
}

///////////////////////////////////////////////////////////////////////////////
// Test 6: Rich Span JSON Serialization for All Languages
///////////////////////////////////////////////////////////////////////////////

#[test]
fn test_rich_span_json_serialization_rust() {
    let mut file = NamedTempFile::new().unwrap();
    file.write_all(RUST_FUNCTION.as_bytes()).unwrap();
    let file_path = file.path();

    let byte_start = 10;
    let byte_end = 100;

    let mut span = SpanResult::from_byte_span(
        file_path.to_string_lossy().to_string(),
        byte_start,
        byte_end,
    );

    // Add all rich span fields
    let ctx = extract_context(file_path, byte_start, byte_end, 1).unwrap();
    span = span.with_context(ctx);
    span = span.with_semantic_info("function", "rust");

    let span_checksum = checksum_span(file_path, byte_start, byte_end).unwrap();
    let file_checksum = checksum_file(file_path).unwrap();
    span = span.with_both_checksums(span_checksum.as_hex(), file_checksum.as_hex());

    let error_code = ErrorCode::new(
        "SPL-E001",
        "error",
        format!("{}.rs:1:10", file_path.display()),
        "Example hint",
    );
    span = span.with_error_code(error_code);

    // Serialize to JSON
    let json_value = serde_json::to_value(&span).unwrap();

    // Verify all fields present
    assert!(json_value["context"].is_object());
    assert_eq!(json_value["semantics"]["kind"], "function");
    assert_eq!(json_value["semantics"]["language"], "rust");
    assert!(json_value["checksums"]["checksum_before"].is_string());
    assert!(json_value["checksums"]["file_checksum_before"].is_string());
    assert!(json_value["error_code"].is_object());
    assert_eq!(json_value["error_code"]["code"], "SPL-E001");
}

#[test]
fn test_rich_span_json_serialization_python() {
    let mut file = NamedTempFile::new().unwrap();
    file.write_all(PYTHON_FUNCTION.as_bytes()).unwrap();
    let file_path = file.path();

    let byte_start = 10;
    let byte_end = 100;

    let mut span = SpanResult::from_byte_span(
        file_path.to_string_lossy().to_string(),
        byte_start,
        byte_end,
    );

    let ctx = extract_context(file_path, byte_start, byte_end, 1).unwrap();
    span = span.with_context(ctx);
    span = span.with_semantic_info("function", "python");

    let span_checksum = checksum_span(file_path, byte_start, byte_end).unwrap();
    let file_checksum = checksum_file(file_path).unwrap();
    span = span.with_both_checksums(span_checksum.as_hex(), file_checksum.as_hex());

    let json_value = serde_json::to_value(&span).unwrap();

    assert_eq!(json_value["semantics"]["kind"], "function");
    assert_eq!(json_value["semantics"]["language"], "python");
    assert!(json_value["checksums"]["checksum_before"].is_string());
    assert!(json_value["checksums"]["file_checksum_before"].is_string());
}

#[test]
fn test_rich_span_json_serialization_typescript() {
    let mut file = NamedTempFile::new().unwrap();
    file.write_all(TYPESCRIPT_FUNCTION.as_bytes()).unwrap();
    let file_path = file.path();

    let byte_start = 10;
    let byte_end = 100;

    let mut span = SpanResult::from_byte_span(
        file_path.to_string_lossy().to_string(),
        byte_start,
        byte_end,
    );

    let ctx = extract_context(file_path, byte_start, byte_end, 1).unwrap();
    span = span.with_context(ctx);
    span = span.with_semantic_info("function", "typescript");

    let span_checksum = checksum_span(file_path, byte_start, byte_end).unwrap();
    let file_checksum = checksum_file(file_path).unwrap();
    span = span.with_both_checksums(span_checksum.as_hex(), file_checksum.as_hex());

    let json_value = serde_json::to_value(&span).unwrap();

    assert_eq!(json_value["semantics"]["kind"], "function");
    assert_eq!(json_value["semantics"]["language"], "typescript");
    assert!(json_value["checksums"]["checksum_before"].is_string());
}

///////////////////////////////////////////////////////////////////////////////
// Test 7: Backward Compatibility
///////////////////////////////////////////////////////////////////////////////

#[test]
fn test_backward_compatibility_with_new_fields() {
    let old_json = json!({
        "file_path": "src/lib.rs",
        "symbol": null,
        "kind": null,
        "byte_start": 0,
        "byte_end": 100,
        "line_start": 1,
        "line_end": 5,
        "col_start": 0,
        "col_end": 10,
        "span_id": "test-id",
        "match_id": null,
        "before_hash": null,
        "after_hash": null,
        "span_checksum_before": null,
        "span_checksum_after": null
    });

    let parsed = serde_json::from_value::<SpanResult>(old_json);
    assert!(parsed.is_err(), "old schema should not deserialize");
}

///////////////////////////////////////////////////////////////////////////////
// Test 8: Context Extraction with Asymmetric Before/After
///////////////////////////////////////////////////////////////////////////////

#[test]
fn test_context_asymmetric_rust() {
    let mut file = NamedTempFile::new().unwrap();
    file.write_all(RUST_FUNCTION.as_bytes()).unwrap();
    let file_path = file.path();

    let source = std::fs::read_to_string(file_path).unwrap();
    let offset = source.find("pub fn add").unwrap();
    let offset_end = offset + 50;

    // Test asymmetric context (3 before, 1 after)
    let ctx = extract_context_asymmetric(file_path, offset, offset_end, 3, 1).unwrap();

    // Should have requested context lines
    assert!(ctx.before.len() <= 3, "Should have at most 3 lines before");
    assert!(ctx.after.len() <= 1, "Should have at most 1 line after");
}

#[test]
fn test_context_asymmetric_python() {
    let mut file = NamedTempFile::new().unwrap();
    file.write_all(PYTHON_FUNCTION.as_bytes()).unwrap();
    let file_path = file.path();

    let source = std::fs::read_to_string(file_path).unwrap();
    let offset = source.find("def add").unwrap();
    let offset_end = offset + 50;

    let ctx = extract_context_asymmetric(file_path, offset, offset_end, 2, 2).unwrap();

    // Should have requested context lines
    assert!(ctx.before.len() <= 2, "Should have at most 2 lines before");
    assert!(ctx.after.len() <= 2, "Should have at most 2 lines after");
}

#[test]
fn test_context_asymmetric_java() {
    let mut file = NamedTempFile::new().unwrap();
    file.write_all(JAVA_CLASS.as_bytes()).unwrap();
    let file_path = file.path();

    let source = std::fs::read_to_string(file_path).unwrap();
    let offset = source.find("public int add").unwrap();
    let offset_end = offset + 50;

    let ctx = extract_context_asymmetric(file_path, offset, offset_end, 1, 3).unwrap();

    // Asymmetric context: 1 before, 3 after
    assert!(ctx.before.len() <= 1, "Should have at most 1 line before");
    assert!(ctx.after.len() <= 3, "Should have at most 3 lines after");
}

///////////////////////////////////////////////////////////////////////////////
// Test 9: All Semantic Kind Categories Across Languages
///////////////////////////////////////////////////////////////////////////////

#[test]
fn test_semantic_kind_function_all_languages() {
    // Test function detection across all languages
    let test_cases = vec![
        ("function_item", Language::Rust),
        ("function_definition", Language::Python),
        ("function_definition", Language::C),
        ("function_definition", Language::Cpp),
        ("method_declaration", Language::Java),
        ("function_declaration", Language::JavaScript),
        ("function_declaration", Language::TypeScript),
    ];

    for (node_kind, lang) in test_cases {
        let detected = detect_semantic_kind(node_kind, lang);
        assert_eq!(
            detected.as_str(),
            "function",
            "Function kind mismatch for {:?} node '{}'",
            lang,
            node_kind
        );
    }
}

#[test]
fn test_semantic_kind_type_all_languages() {
    // Test type/class detection across all languages
    let test_cases = vec![
        ("struct_item", Language::Rust),
        ("class_definition", Language::Python),
        ("struct_specifier", Language::C),
        ("class_specifier", Language::Cpp),
        ("class_declaration", Language::Java),
        ("class_declaration", Language::JavaScript),
        ("class_declaration", Language::TypeScript),
    ];

    for (node_kind, lang) in test_cases {
        let detected = detect_semantic_kind(node_kind, lang);
        assert_eq!(
            detected.as_str(),
            "type",
            "Type kind mismatch for {:?} node '{}'",
            lang,
            node_kind
        );
    }
}

#[test]
fn test_semantic_kind_variable_all_languages() {
    // Test variable/field detection across all languages
    let test_cases = vec![
        // Rust doesn't have let_declaration in tree-sitter mapping, use Unknown check
        ("const_item", Language::Rust, "constant"),
        ("assignment", Language::Python, "variable"),
        ("declaration", Language::C, "variable"),
        ("declaration", Language::Cpp, "variable"),
        ("field_declaration", Language::Java, "variable"),
        ("variable_declaration", Language::JavaScript, "variable"),
        ("variable_declaration", Language::TypeScript, "variable"),
    ];

    for (node_kind, lang, expected) in test_cases {
        let detected = detect_semantic_kind(node_kind, lang);
        assert_eq!(
            detected.as_str(),
            expected,
            "Kind mismatch for {:?} node '{}': got '{}', expected '{}'",
            lang,
            node_kind,
            detected.as_str(),
            expected
        );
    }
}

///////////////////////////////////////////////////////////////////////////////
// Test 10: Rich Span with All Fields for All Languages
///////////////////////////////////////////////////////////////////////////////

#[test]
fn test_complete_rich_span_rust() {
    let mut file = NamedTempFile::new().unwrap();
    writeln!(file, "/// Docs").unwrap();
    writeln!(file, "fn test() {{}}").unwrap();
    let file_path = file.path();
    let content = std::fs::read_to_string(file_path).unwrap();
    let file_size = content.len();

    // Use valid byte offsets within the file
    let byte_start = 10;
    let byte_end = file_size.min(25);

    let mut span = SpanResult::from_byte_span(
        file_path.to_string_lossy().to_string(),
        byte_start,
        byte_end,
    );

    // Populate all rich span fields
    let ctx = extract_context(file_path, byte_start, byte_end, 0).unwrap();
    span = span.with_context(ctx);
    span = span.with_semantic_info("function", "rust");

    let span_checksum = checksum_span(file_path, byte_start, byte_end).unwrap();
    let file_checksum = checksum_file(file_path).unwrap();
    span = span.with_both_checksums(span_checksum.as_hex(), file_checksum.as_hex());

    let ec = ErrorCode::new("SPL-E001", "error", "test:1:1", "hint");
    span = span.with_error_code(ec);

    // Verify all fields are populated
    assert!(span.context.is_some());
    assert_eq!(span_kind(&span), Some("function".to_string()));
    assert_eq!(span_language(&span), Some("rust".to_string()));
    assert!(span_checksum_before(&span).is_some());
    assert!(span_file_checksum_before(&span).is_some());
    assert!(span.error_code.is_some());
}

#[test]
fn test_complete_rich_span_python() {
    let mut file = NamedTempFile::new().unwrap();
    writeln!(file, "\"\"\"Docs\"\"\"").unwrap();
    writeln!(file, "def test():").unwrap();
    writeln!(file, "    pass").unwrap();
    let file_path = file.path();

    let mut span = SpanResult::from_byte_span(file_path.to_string_lossy().to_string(), 12, 30);

    let ctx = extract_context(file_path, 12, 30, 0).unwrap();
    span = span.with_context(ctx);
    span = span.with_semantic_info("function", "python");

    let span_checksum = checksum_span(file_path, 12, 30).unwrap();
    let file_checksum = checksum_file(file_path).unwrap();
    span = span.with_both_checksums(span_checksum.as_hex(), file_checksum.as_hex());

    assert!(span.context.is_some());
    assert_eq!(span_kind(&span), Some("function".to_string()));
    assert_eq!(span_language(&span), Some("python".to_string()));
    assert!(span_checksum_before(&span).is_some());
    assert!(span_file_checksum_before(&span).is_some());
}

#[test]
fn test_complete_rich_span_c() {
    let mut file = NamedTempFile::new().unwrap();
    writeln!(file, "/* Docs */").unwrap();
    writeln!(file, "int test() {{").unwrap();
    writeln!(file, "    return 0;").unwrap();
    writeln!(file, "}}").unwrap();
    let file_path = file.path();

    let mut span = SpanResult::from_byte_span(file_path.to_string_lossy().to_string(), 12, 35);

    let ctx = extract_context(file_path, 12, 35, 0).unwrap();
    span = span.with_context(ctx);
    span = span.with_semantic_info("function", "c");

    let span_checksum = checksum_span(file_path, 12, 35).unwrap();
    let file_checksum = checksum_file(file_path).unwrap();
    span = span.with_both_checksums(span_checksum.as_hex(), file_checksum.as_hex());

    assert!(span.context.is_some());
    assert_eq!(span_kind(&span), Some("function".to_string()));
    assert_eq!(span_language(&span), Some("c".to_string()));
    assert!(span_checksum_before(&span).is_some());
    assert!(span_file_checksum_before(&span).is_some());
}

#[test]
fn test_complete_rich_span_cpp() {
    let mut file = NamedTempFile::new().unwrap();
    writeln!(file, "// Docs").unwrap();
    writeln!(file, "int test() {{").unwrap();
    writeln!(file, "    return 0;").unwrap();
    writeln!(file, "}}").unwrap();
    let file_path = file.path();

    let mut span = SpanResult::from_byte_span(file_path.to_string_lossy().to_string(), 8, 35);

    let ctx = extract_context(file_path, 8, 35, 0).unwrap();
    span = span.with_context(ctx);
    span = span.with_semantic_info("function", "cpp");

    let span_checksum = checksum_span(file_path, 8, 35).unwrap();
    let file_checksum = checksum_file(file_path).unwrap();
    span = span.with_both_checksums(span_checksum.as_hex(), file_checksum.as_hex());

    assert!(span.context.is_some());
    assert_eq!(span_kind(&span), Some("function".to_string()));
    assert_eq!(span_language(&span), Some("cpp".to_string()));
}

#[test]
fn test_complete_rich_span_java() {
    let mut file = NamedTempFile::new().unwrap();
    writeln!(file, "/** Docs */").unwrap();
    writeln!(file, "int test() {{").unwrap();
    writeln!(file, "    return 0;").unwrap();
    writeln!(file, "}}").unwrap();
    let file_path = file.path();

    let mut span = SpanResult::from_byte_span(file_path.to_string_lossy().to_string(), 14, 40);

    let ctx = extract_context(file_path, 14, 40, 0).unwrap();
    span = span.with_context(ctx);
    span = span.with_semantic_info("function", "java");

    let span_checksum = checksum_span(file_path, 14, 40).unwrap();
    let file_checksum = checksum_file(file_path).unwrap();
    span = span.with_both_checksums(span_checksum.as_hex(), file_checksum.as_hex());

    assert!(span.context.is_some());
    assert_eq!(span_kind(&span), Some("function".to_string()));
    assert_eq!(span_language(&span), Some("java".to_string()));
}

#[test]
fn test_complete_rich_span_javascript() {
    let mut file = NamedTempFile::new().unwrap();
    writeln!(file, "/** Docs */").unwrap();
    writeln!(file, "function test() {{").unwrap();
    writeln!(file, "    return 0;").unwrap();
    writeln!(file, "}}").unwrap();
    let file_path = file.path();

    let mut span = SpanResult::from_byte_span(file_path.to_string_lossy().to_string(), 14, 45);

    let ctx = extract_context(file_path, 14, 45, 0).unwrap();
    span = span.with_context(ctx);
    span = span.with_semantic_info("function", "javascript");

    let span_checksum = checksum_span(file_path, 14, 45).unwrap();
    let file_checksum = checksum_file(file_path).unwrap();
    span = span.with_both_checksums(span_checksum.as_hex(), file_checksum.as_hex());

    assert!(span.context.is_some());
    assert_eq!(span_kind(&span), Some("function".to_string()));
    assert_eq!(span_language(&span), Some("javascript".to_string()));
}

#[test]
fn test_complete_rich_span_typescript() {
    let mut file = NamedTempFile::new().unwrap();
    writeln!(file, "/** Docs */").unwrap();
    writeln!(file, "function test(): number {{").unwrap();
    writeln!(file, "    return 0;").unwrap();
    writeln!(file, "}}").unwrap();
    let file_path = file.path();
    let content = std::fs::read_to_string(file_path).unwrap();
    let file_size = content.len();

    // Use valid byte offsets within the file
    let byte_start = 14;
    let byte_end = file_size.min(55);

    let mut span = SpanResult::from_byte_span(
        file_path.to_string_lossy().to_string(),
        byte_start,
        byte_end,
    );

    let ctx = extract_context(file_path, byte_start, byte_end, 0).unwrap();
    span = span.with_context(ctx);
    span = span.with_semantic_info("function", "typescript");

    let span_checksum = checksum_span(file_path, byte_start, byte_end).unwrap();
    let file_checksum = checksum_file(file_path).unwrap();
    span = span.with_both_checksums(span_checksum.as_hex(), file_checksum.as_hex());

    assert!(span.context.is_some());
    assert_eq!(span_kind(&span), Some("function".to_string()));
    assert_eq!(span_language(&span), Some("typescript".to_string()));
}

///////////////////////////////////////////////////////////////////////////////
// Test 11: Error Code Severity Levels
///////////////////////////////////////////////////////////////////////////////

#[test]
fn test_error_code_severity_levels() {
    // Error severity
    let error = ErrorCode::new("SPL-E001", "error", "file.rs:1:1", "Error hint");
    assert_eq!(error.severity, "error");

    // Warning severity
    let warning = ErrorCode::new("SPL-W001", "warning", "file.py:2:1", "Warning hint");
    assert_eq!(warning.severity, "warning");

    // Note severity
    let note = ErrorCode::new("SPL-N001", "note", "file.ts:3:1", "Note hint");
    assert_eq!(note.severity, "note");
}

///////////////////////////////////////////////////////////////////////////////
// Test 12: Checksums with Different Span Sizes
///////////////////////////////////////////////////////////////////////////////

#[test]
fn test_checksum_different_span_sizes_rust() {
    let content = b"fn test() { return 42; }";
    let mut file = NamedTempFile::new().unwrap();
    file.write_all(content).unwrap();
    let file_path = file.path();

    let file_size = content.len();

    // Small span (0-10)
    let checksum1 = checksum_span(file_path, 0, 10).unwrap();
    // Medium span (0-15)
    let checksum2 = checksum_span(file_path, 0, 15).unwrap();
    // Large span (0 to file_size)
    let checksum3 = checksum_span(file_path, 0, file_size).unwrap();

    // All checksums should be different
    assert_ne!(checksum1.as_hex(), checksum2.as_hex());
    assert_ne!(checksum2.as_hex(), checksum3.as_hex());
    assert_ne!(checksum1.as_hex(), checksum3.as_hex());
}

///////////////////////////////////////////////////////////////////////////////
// Test 13: Context Extraction at File Boundaries
///////////////////////////////////////////////////////////////////////////////

#[test]
fn test_context_at_file_boundaries_all_languages() {
    let test_cases = vec![
        ("test.rs", "fn test() {}\n"),
        ("test.py", "def test(): pass\n"),
        ("test.c", "int test() { return 0; }\n"),
        ("test.cpp", "int test() { return 0; }\n"),
        ("test.java", "int test() { return 0; }\n"),
        ("test.js", "function test() { return 0; }\n"),
        ("test.ts", "function test(): number { return 0; }\n"),
    ];

    for (filename, content) in test_cases {
        let mut file = NamedTempFile::new().unwrap();
        file.write_all(content.as_bytes()).unwrap();
        let file_path = file.path();

        // Context at start of file
        let ctx = extract_context(file_path, 0, 5, 5).unwrap();
        assert_eq!(
            ctx.before.len(),
            0,
            "Should have 0 lines before for {}",
            filename
        );

        // Context at end of file
        let content_len = content.len();
        let ctx = extract_context(file_path, content_len - 5, content_len, 5).unwrap();
        assert_eq!(
            ctx.after.len(),
            0,
            "Should have 0 lines after for {}",
            filename
        );
    }
}

///////////////////////////////////////////////////////////////////////////////
// Test 14: Language Detection Edge Cases
///////////////////////////////////////////////////////////////////////////////

#[test]
fn test_language_detection_edge_cases() {
    let test_cases = vec![
        // Multiple dots in filename
        ("lib.test.rs", Some(Language::Rust)),
        ("file.test.py", Some(Language::Python)),
        // Mixed case (should fail - extensions are case-sensitive)
        ("test.RS", None),
        ("test.PY", None),
        // Unknown extensions
        ("test.txt", None),
        ("test.md", None),
        // No extension
        ("Makefile", None),
        ("Dockerfile", None),
    ];

    for (filename, expected) in test_cases {
        let detected = detect_language(std::path::Path::new(filename));
        assert_eq!(
            detected, expected,
            "Language detection edge case failed for {}: got {:?}, expected {:?}",
            filename, detected, expected
        );
    }
}

///////////////////////////////////////////////////////////////////////////////
// Test 15: Semantic Kind Unknown Fallback
///////////////////////////////////////////////////////////////////////////////

#[test]
fn test_semantic_kind_unknown_fallback() {
    // Unknown node types should return Unknown kind for all languages
    let languages = vec![
        Language::Rust,
        Language::Python,
        Language::C,
        Language::Cpp,
        Language::Java,
        Language::JavaScript,
        Language::TypeScript,
    ];

    for lang in languages {
        let detected = detect_semantic_kind("completely_unknown_node_type", lang);
        assert_eq!(
            detected.as_str(),
            "unknown",
            "Unknown node type should return 'unknown' for {:?}",
            lang
        );
    }
}

///////////////////////////////////////////////////////////////////////////////
// Test 16: JSON Serialization Round-Trip
///////////////////////////////////////////////////////////////////////////////

#[test]
fn test_json_serialization_roundtrip_all_fields() {
    let mut file = NamedTempFile::new().unwrap();
    writeln!(file, "fn test() {{}}").unwrap();
    let file_path = file.path();

    let mut span = SpanResult::from_byte_span(file_path.to_string_lossy().to_string(), 0, 13);

    // Populate all optional fields
    let ctx = extract_context(file_path, 0, 13, 0).unwrap();
    span = span.with_context(ctx);
    span = span.with_semantic_info("function", "rust");
    span = span.with_checksum_before("span_hash");
    span = span.with_file_checksum_before("file_hash");
    let ec = ErrorCode::new("SPL-E001", "error", "test:1:1", "hint");
    span = span.with_error_code(ec);

    // Serialize and deserialize
    let json = serde_json::to_string(&span).unwrap();
    let deserialized: SpanResult = serde_json::from_str(&json).unwrap();

    // Verify all fields preserved
    assert_eq!(deserialized.file_path, span.file_path);
    assert!(deserialized.context.is_some());
    assert_eq!(span_kind(&deserialized), span_kind(&span));
    assert_eq!(span_language(&deserialized), span_language(&span));
    assert_eq!(
        span_checksum_before(&deserialized),
        span_checksum_before(&span)
    );
    assert_eq!(
        span_file_checksum_before(&deserialized),
        span_file_checksum_before(&span)
    );
    assert!(deserialized.error_code.is_some());
}

///////////////////////////////////////////////////////////////////////////////
// Test 17: Checksum Algorithm Consistency
///////////////////////////////////////////////////////////////////////////////

#[test]
fn test_checksum_algorithm_consistency() {
    let test_cases = vec![
        ("test.rs", RUST_FUNCTION),
        ("test.py", PYTHON_FUNCTION),
        ("test.c", C_FUNCTION),
        ("test.cpp", CPP_FUNCTION),
        ("test.java", JAVA_CLASS),
        ("test.js", JAVASCRIPT_FUNCTION),
        ("test.ts", TYPESCRIPT_FUNCTION),
    ];

    for (filename, content) in test_cases {
        let mut file = NamedTempFile::new().unwrap();
        file.write_all(content.as_bytes()).unwrap();
        let file_path = file.path();

        // Same file should produce same checksum
        let checksum1 = checksum_file(file_path).unwrap();
        let checksum2 = checksum_file(file_path).unwrap();

        assert_eq!(
            checksum1.as_hex(),
            checksum2.as_hex(),
            "Checksum should be consistent for {}",
            filename
        );
        assert_eq!(
            checksum1.algorithm,
            splice::checksum::ChecksumAlgorithm::Sha256
        );
    }
}

///////////////////////////////////////////////////////////////////////////////
// Test 18: Context with UTF-8 Multi-Byte Characters
///////////////////////////////////////////////////////////////////////////////

#[test]
fn test_context_utf8_multibyte_all_languages() {
    let test_cases = vec![
        ("test.rs", "// 🦀 Rust\nfn test() {}\n"),
        ("test.py", "# 🐍 Python\ndef test(): pass\n"),
        ("test.c", "// 🦀 C\nint test() { return 0; }\n"),
        ("test.cpp", "// 🦀 C++\nint test() { return 0; }\n"),
        ("test.java", "// ☕ Java\nint test() { return 0; }\n"),
        (
            "test.js",
            "// 📜 JavaScript\nfunction test() { return 0; }\n",
        ),
        (
            "test.ts",
            "// 📘 TypeScript\nfunction test(): number { return 0; }\n",
        ),
    ];

    for (_filename, content) in test_cases {
        let mut file = NamedTempFile::new().unwrap();
        file.write_all(content.as_bytes()).unwrap();
        let file_path = file.path();

        // Find offset after the emoji comment
        let contents = std::fs::read(file_path).unwrap();
        let newline_offset = contents.iter().position(|&b| b == b'\n').unwrap();

        // Extract context starting after first line
        let ctx = extract_context(file_path, newline_offset + 1, contents.len(), 0).unwrap();

        // Should handle UTF-8 correctly without crashing
        assert!(!ctx.selected.is_empty() || !ctx.before.is_empty() || !ctx.after.is_empty());
    }
}

///////////////////////////////////////////////////////////////////////////////
// Test 19: Empty Context Handling
///////////////////////////////////////////////////////////////////////////////

#[test]
fn test_empty_context_handling() {
    let test_cases = vec![
        ("test.rs", "fn test() {}\n"),
        ("test.py", "def test(): pass\n"),
        ("test.ts", "function test() {}\n"),
    ];

    for (filename, content) in test_cases {
        let mut file = NamedTempFile::new().unwrap();
        file.write_all(content.as_bytes()).unwrap();
        let file_path = file.path();

        // Request 0 context lines
        let ctx = extract_context(file_path, 0, 5, 0).unwrap();

        assert_eq!(
            ctx.before.len(),
            0,
            "Before should be empty for {}",
            filename
        );
        assert_eq!(ctx.after.len(), 0, "After should be empty for {}", filename);
        assert!(!ctx.selected.is_empty(), "Selected should not be empty");
    }
}

///////////////////////////////////////////////////////////////////////////////
// Test 20: All Fields Omitted When None
///////////////////////////////////////////////////////////////////////////////

#[test]
fn test_none_fields_omitted_from_json() {
    // Create span with only required fields (no rich span fields populated)
    let span = SpanResult::from_byte_span("test.rs".to_string(), 0, 10);

    let json = serde_json::to_string(&span).unwrap();

    // New fields should NOT be in JSON when None (skip_serializing_if)
    assert!(!json.contains("\"context\""));
    assert!(!json.contains("\"semantics\""));
    assert!(!json.contains("\"checksums\""));
    assert!(!json.contains("\"error_code\""));
}

///////////////////////////////////////////////////////////////////////////////
// Test 21: SpanResult with_context Method
///////////////////////////////////////////////////////////////////////////////

#[test]
fn test_span_result_with_context_all_languages() {
    let test_cases: Vec<(&str, &str, &str)> = vec![
        (RUST_FUNCTION, "rust", "pub fn add"),
        (PYTHON_FUNCTION, "python", "def add"),
        (C_FUNCTION, "c", "int add"),
        (CPP_FUNCTION, "cpp", "int add"),
        (JAVA_CLASS, "java", "public int add"),
        (JAVASCRIPT_FUNCTION, "javascript", "function add"),
        (TYPESCRIPT_FUNCTION, "typescript", "function add"),
    ];

    for (content, lang, search_term) in test_cases {
        let mut file = NamedTempFile::new().unwrap();
        file.write_all(content.as_bytes()).unwrap();
        let file_path = file.path();

        let source = std::fs::read_to_string(file_path).unwrap();
        let offset = source.find(search_term).unwrap_or(0);
        let offset_end = offset + 50;

        let mut span =
            SpanResult::from_byte_span(file_path.to_string_lossy().to_string(), offset, offset_end);

        let ctx = extract_context(file_path, offset, offset_end, 1).unwrap();
        span = span.with_context(ctx);

        assert!(
            span.context.is_some(),
            "Context should be Some for {}",
            lang
        );
        let span_ctx = span.context.as_ref().unwrap();
        assert!(
            !span_ctx.selected.is_empty(),
            "Selected should not be empty for {}",
            lang
        );
    }
}

///////////////////////////////////////////////////////////////////////////////
// Test 22: All SpliceErrorCode Values Have Valid Format
///////////////////////////////////////////////////////////////////////////////

#[test]
fn test_all_splice_error_codes_valid_format() {
    let error_codes = vec![
        SpliceErrorCode::SymbolNotFound,
        SpliceErrorCode::AmbiguousSymbol,
        SpliceErrorCode::ReferenceFailed,
        SpliceErrorCode::AmbiguousReference,
        SpliceErrorCode::ParseError,
        SpliceErrorCode::InvalidUtf8,
        SpliceErrorCode::InvalidSyntax,
        SpliceErrorCode::InvalidSpan,
        SpliceErrorCode::InvalidLineRange,
        SpliceErrorCode::SpanOutOfBounds,
        SpliceErrorCode::FileReadError,
        SpliceErrorCode::FileWriteError,
        SpliceErrorCode::FileNotFound,
        SpliceErrorCode::FileExternallyModified,
        SpliceErrorCode::AmbiguousSymbolAsWarning,
        SpliceErrorCode::FileSkipped,
        SpliceErrorCode::FileExternallyModifiedWarning,
        SpliceErrorCode::PreVerificationFailed,
        SpliceErrorCode::ParseValidationFailed,
        SpliceErrorCode::CompilerValidationFailed,
        SpliceErrorCode::InvalidPlanSchema,
        SpliceErrorCode::PlanExecutionFailed,
        SpliceErrorCode::InvalidBatchSchema,
        SpliceErrorCode::GraphError,
        SpliceErrorCode::DatabaseError,
        SpliceErrorCode::ExecutionLogError,
        SpliceErrorCode::ExecutionNotFound,
        SpliceErrorCode::AnalyzerNotAvailable,
        SpliceErrorCode::AnalyzerFailed,
    ];

    for code in error_codes {
        let code_str = code.code();
        let severity = code.severity();
        let hint = code.hint();

        // Verify code format
        assert!(
            code_str.starts_with("SPL-"),
            "Error code should start with 'SPL-': {}",
            code_str
        );
        assert!(
            code_str.len() >= 7,
            "Error code should be at least 7 chars: {}",
            code_str
        );

        // Verify severity is valid
        assert!(
            severity == "error" || severity == "warning" || severity == "note",
            "Severity should be error/warning/note: {}",
            severity
        );

        // Verify hint is not empty
        assert!(
            !hint.is_empty(),
            "Hint should not be empty for {}",
            code_str
        );
    }
}
