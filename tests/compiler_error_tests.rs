//! Tests for compiler error code parsing.
//!
//! Verifies that error codes (E0XXX, TSXXXX) are correctly extracted
//! from compiler output for structured diagnostics.

use splice::validate;

#[test]
fn test_parse_typescript_error_code() {
    let output = r#"
test.ts(2,5): error TS1002: Unterminated string literal
another.ts(10,12): error TS2304: Cannot find name 'foo'
"#;

    let errors = validate::parse_typescript_output(output);

    assert_eq!(errors.len(), 2, "Should parse 2 TypeScript errors");

    // First error
    assert_eq!(errors[0].file, "test.ts");
    assert_eq!(errors[0].line, 2);
    assert_eq!(errors[0].column, 5);
    assert_eq!(errors[0].code, Some("TS1002".to_string()));
    assert!(errors[0].message.contains("Unterminated"));

    // Second error
    assert_eq!(errors[1].file, "another.ts");
    assert_eq!(errors[1].line, 10);
    assert_eq!(errors[1].column, 12);
    assert_eq!(errors[1].code, Some("TS2304".to_string()));
}

#[test]
fn test_parse_typescript_warning() {
    let output = r#"test.ts(5,1): warning TS7006: Parameter 'x' implicitly has an 'any' type."#;

    let errors = validate::parse_typescript_output(output);

    assert_eq!(errors.len(), 1);
    assert_eq!(errors[0].level, validate::ErrorLevel::Warning);
    assert_eq!(errors[0].code, Some("TS7006".to_string()));
}

#[test]
fn test_parse_rust_error_code_existing() {
    // Verify existing Rust error parsing still works
    let output = r#"
error[E0425]: cannot find function `missing_helper` in this scope
 --> src/lib.rs:2:5
  |
2 |     missing_helper(name)
  |     ^^^^^^^^^^^^^^ not found in this scope
"#;

    let errors = validate::parse_rust_analyzer_output(output);

    assert_eq!(errors.len(), 1);
    assert_eq!(errors[0].code.as_deref(), Some("E0425"));
}

#[test]
fn test_remediation_link_typescript() {
    let link = validate::remediation_link_for_code("TS1002");
    assert_eq!(
        link,
        Some("https://www.typescriptlang.org/errors/TS1002".to_string())
    );
}

#[test]
fn test_remediation_link_rust() {
    let link = validate::remediation_link_for_code("E0425");
    assert_eq!(
        link,
        Some("https://doc.rust-lang.org/error-index.html#E0425".to_string())
    );
}
