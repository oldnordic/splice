//! Integration tests for rich span extensions (Phase 11).
//!
//! Tests verify that all rich span fields work together correctly
//! and backward compatibility is maintained.

use splice::context::extract_context;
use splice::ingest::{detect_language, detect_semantic_kind, Language};
use splice::checksum::{checksum_file, checksum_span};
use splice::error_codes::{ErrorCode, SpliceErrorCode};
use splice::output::SpanResult;
use std::io::Write;
use tempfile::NamedTempFile;

// Test all rich span fields together
#[test]
fn test_rich_span_complete() {
    let mut file = NamedTempFile::new().unwrap();
    writeln!(file, "/// Documentation").unwrap();
    writeln!(file, "fn greet(name: &str) -> String {{").unwrap();
    writeln!(file, "    format!(\"Hello, {{}}\", name)").unwrap();
    writeln!(file, "}}").unwrap();

    let file_path = file.path();
    let byte_start = 23; // Start of "fn greet..."
    let byte_end = 65;   // End of function body

    // Create base span
    let mut span = SpanResult::from_byte_span(
        file_path.to_string_lossy().to_string(),
        byte_start,
        byte_end,
    );

    // Add context
    let context = extract_context(file_path, byte_start, byte_end, 1).unwrap();
    span = span.with_context(context);

    // Add semantic info (don't rely on detect_language for temp file)
    span = span.with_semantic_info("function", "rust");

    // Add checksums
    let span_checksum = checksum_span(file_path, byte_start, byte_end).unwrap();
    let file_checksum = checksum_file(file_path).unwrap();
    span = span.with_both_checksums(span_checksum.as_hex(), file_checksum.as_hex());

    // Add error code
    let error_code = ErrorCode::new(
        "SPL-E001",
        "error",
        format!("{}:2:1", file_path.display()),
        "Example hint",
    );
    span = span.with_error_code(error_code);

    // Verify all fields are populated
    assert!(span.context.is_some());
    assert_eq!(span.semantic_kind, Some("function".to_string()));
    assert_eq!(span.language, Some("rust".to_string()));
    assert!(span.checksum_before.is_some());
    assert!(span.file_checksum_before.is_some());
    assert!(span.error_code.is_some());
}

// Test backward compatibility: old JSON should parse
#[test]
fn test_backward_compatibility_old_json() {
    let old_json = r#"{
        "file_path": "src/main.rs",
        "symbol": null,
        "kind": null,
        "byte_start": 0,
        "byte_end": 10,
        "line_start": 0,
        "line_end": 0,
        "col_start": 0,
        "col_end": 0,
        "span_id": "test-id",
        "match_id": null,
        "before_hash": null,
        "after_hash": null,
        "span_checksum_before": null,
        "span_checksum_after": null
    }"#;

    let span: SpanResult = serde_json::from_str(old_json).unwrap();
    assert_eq!(span.file_path, "src/main.rs");
    assert_eq!(span.byte_start, 0);
    assert_eq!(span.byte_end, 10);

    // New fields should be None
    assert!(span.context.is_none());
    assert!(span.semantic_kind.is_none());
    assert!(span.language.is_none());
    assert!(span.checksum_before.is_none());
    assert!(span.file_checksum_before.is_none());
    assert!(span.error_code.is_none());
}

// Test new JSON includes all fields when populated
#[test]
fn test_new_json_includes_rich_fields() {
    let mut file = NamedTempFile::new().unwrap();
    writeln!(file, "line 1").unwrap();
    writeln!(file, "line 2").unwrap();

    let file_path = file.path();
    let mut span = SpanResult::from_byte_span(
        file_path.to_string_lossy().to_string(),
        0,
        14,
    );

    // Populate all new fields
    let context = extract_context(file_path, 0, 14, 1).unwrap();
    span = span.with_context(context);
    span = span.with_semantic_info("function", "rust");
    span = span.with_checksum_before("abc123");
    span = span.with_file_checksum_before("def456");
    let error_code = ErrorCode::new("SPL-E001", "error", "test.rs:1:1", "hint");
    span = span.with_error_code(error_code);

    // Serialize to JSON
    let json = serde_json::to_string_pretty(&span).unwrap();

    // Verify all new fields are present
    assert!(json.contains("\"context\""));
    assert!(json.contains("\"semantic_kind\""));
    assert!(json.contains("\"language\""));
    assert!(json.contains("\"checksum_before\""));
    assert!(json.contains("\"file_checksum_before\""));
    assert!(json.contains("\"error_code\""));
}

// Test new fields are omitted when None (skip_serializing_if)
#[test]
fn test_none_fields_omitted_from_json() {
    let span = SpanResult::from_byte_span("test.rs".to_string(), 0, 10);
    let json = serde_json::to_string(&span).unwrap();

    // New fields should NOT be in JSON when None
    assert!(!json.contains("\"context\""));
    assert!(!json.contains("\"semantic_kind\""));
    assert!(!json.contains("\"language\""));
    assert!(!json.contains("\"checksum_before\""));
    assert!(!json.contains("\"file_checksum_before\""));
    assert!(!json.contains("\"error_code\""));
}

// Test context extraction integration
#[test]
fn test_context_extraction_with_span() {
    let mut file = NamedTempFile::new().unwrap();
    writeln!(file, "// before").unwrap();
    writeln!(file, "fn foo() {{").unwrap();
    writeln!(file, "}}").unwrap();
    writeln!(file, "// after").unwrap();

    let context = extract_context(file.path(), 12, 22, 1).unwrap();

    assert_eq!(context.before.len(), 1);
    assert!(context.before[0].contains("before"));
    assert_eq!(context.selected.len(), 2);
    assert_eq!(context.after.len(), 1);
    assert!(context.after[0].contains("after"));
}

// Test semantic kind detection integration
#[test]
fn test_semantic_kind_detection() {
    // Rust function
    let kind = detect_semantic_kind("function_item", Language::Rust);
    assert_eq!(kind.as_str(), "function");

    // Python class
    let kind = detect_semantic_kind("class_definition", Language::Python);
    assert_eq!(kind.as_str(), "type");

    // TypeScript interface
    let kind = detect_semantic_kind("interface_declaration", Language::TypeScript);
    assert_eq!(kind.as_str(), "trait");
}

// Test language detection integration
#[test]
fn test_language_detection_with_span() {
    let file_paths = vec![
        ("main.rs", "rust"),
        ("script.py", "python"),
        ("code.c", "c"),
        ("class.cpp", "cpp"),
        ("Main.java", "java"),
        ("app.js", "javascript"),
        ("lib.ts", "typescript"),
    ];

    for (file_name, expected_lang) in file_paths {
        let mut span = SpanResult::from_byte_span(file_name.to_string(), 0, 10);
        let path = std::path::Path::new(file_name);

        if let Some(detected) = detect_language(path) {
            span = span.with_language(detected.as_str());
            assert_eq!(span.language, Some(expected_lang.to_string()), "Failed for {}", file_name);
        }
    }
}

// Test checksum integration
#[test]
fn test_checksum_with_span() {
    let mut file = NamedTempFile::new().unwrap();
    writeln!(file, "content").unwrap();

    let byte_start = 0;
    let byte_end = 7;

    let span_checksum = checksum_span(file.path(), byte_start, byte_end).unwrap();
    let file_checksum = checksum_file(file.path()).unwrap();

    let span = SpanResult::from_byte_span(
        file.path().to_string_lossy().to_string(),
        byte_start,
        byte_end,
    )
    .with_both_checksums(span_checksum.as_hex(), file_checksum.as_hex());

    assert_eq!(span.checksum_before, Some(span_checksum.as_hex().to_string()));
    assert_eq!(span.file_checksum_before, Some(file_checksum.as_hex().to_string()));
}

// Test error code integration
#[test]
fn test_error_code_with_span() {
    let error_code = ErrorCode::from_splice_code(
        SpliceErrorCode::SymbolNotFound,
        Some("src/main.rs"),
        Some(42),
        Some(10),
    );

    let span = SpanResult::from_byte_span("src/main.rs".to_string(), 0, 10)
        .with_error_code(error_code);

    assert!(span.error_code.is_some());
    let ec = span.error_code.as_ref().unwrap();
    assert_eq!(ec.code, "SPL-E001");
    assert_eq!(ec.severity, "error");
    assert_eq!(ec.location, "src/main.rs:42:10");
    assert!(!ec.hint.is_empty());
}

// Test all fields serialize correctly to JSON
#[test]
fn test_full_span_serialization() {
    let mut file = NamedTempFile::new().unwrap();
    writeln!(file, "fn test() {{}}").unwrap();

    let file_path = file.path();
    let mut span = SpanResult::from_byte_span(
        file_path.to_string_lossy().to_string(),
        0,
        13,
    );

    // Populate all optional fields
    let context = extract_context(file_path, 0, 13, 0).unwrap();
    span = span.with_context(context);
    span = span.with_semantic_kind("function");
    span = span.with_language("rust");
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
    assert_eq!(deserialized.semantic_kind, span.semantic_kind);
    assert_eq!(deserialized.language, span.language);
    assert_eq!(deserialized.checksum_before, span.checksum_before);
    assert_eq!(deserialized.file_checksum_before, span.file_checksum_before);
    assert!(deserialized.error_code.is_some());
}

// Test UTF-8 context with multi-byte characters
#[test]
fn test_context_utf8_multibyte() {
    let mut file = NamedTempFile::new().unwrap();
    writeln!(file, "// 🦀 before").unwrap();
    writeln!(file, "fn foo() {{}}").unwrap();
    writeln!(file, "// 🚀 after").unwrap();

    let context = extract_context(file.path(), 15, 25, 1).unwrap();

    assert!(context.before[0].contains("🦀") || context.before[0].contains("before"));
    assert!(context.after[0].contains("🚀") || context.after[0].contains("after"));
}

// Test empty context at file boundaries
#[test]
fn test_context_at_file_boundaries() {
    let mut file = NamedTempFile::new().unwrap();
    writeln!(file, "fn foo() {{}}").unwrap();

    // Context at start of file
    let context = extract_context(file.path(), 0, 11, 5).unwrap();
    assert_eq!(context.before.len(), 0);
    assert_eq!(context.selected.len(), 1);

    // Context at end of file
    let context = extract_context(file.path(), 10, 11, 5).unwrap();
    assert_eq!(context.after.len(), 0);
}

// Test semantic kind for all languages
#[test]
fn test_semantic_kind_all_languages() {
    // Test each language has at least one mapping
    let tests = vec![
        ("function_item", Language::Rust, "function"),
        ("struct_item", Language::Rust, "type"),
        ("function_definition", Language::Python, "function"),
        ("class_definition", Language::Python, "type"),
        ("method_declaration", Language::Java, "function"),
        ("class_declaration", Language::Java, "type"),
        ("function_definition", Language::C, "function"),
        ("struct_specifier", Language::C, "type"),
        ("function_definition", Language::Cpp, "function"),
        ("class_specifier", Language::Cpp, "type"),
        ("function_declaration", Language::JavaScript, "function"),
        ("class_declaration", Language::JavaScript, "type"),
        ("function_declaration", Language::TypeScript, "function"),
        ("interface_declaration", Language::TypeScript, "trait"),
    ];

    for (node_type, lang, expected) in tests {
        let kind = detect_semantic_kind(node_type, lang);
        assert_eq!(kind.as_str(), expected, "Failed for {} {:?}", node_type, lang);
    }
}
