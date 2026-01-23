//! Integration tests for enhanced error output.
//!
//! Verifies that errors include:
//! - Severity levels (error/warning/note)
//! - Precise location (file:line:column)
//! - Stable error codes (SPL-E001)
//! - Actionable hints
//! - Similar symbol suggestions (when applicable)

use splice::SpliceError;

#[test]
fn test_symbol_not_found_has_severity() {
    let error = SpliceError::symbol_not_found("foo", None);
    let code = splice::error_codes::SpliceErrorCode::from_splice_error(&error);

    assert!(code.is_some(), "SymbolNotFound should map to error code");
    let splice_code = code.unwrap();
    assert_eq!(splice_code.severity(), "error");
}

#[test]
fn test_symbol_not_found_with_suggestions() {
    let candidates = vec!["foobar".to_string(), "baz".to_string()];

    let error = SpliceError::symbol_not_found_with_suggestions("foobaz", None, &candidates);

    let hint = error.hint().unwrap_or("");
    assert!(
        hint.contains("Did you mean"),
        "Hint should include suggestions"
    );
}

#[test]
fn test_error_code_format() {
    let error = SpliceError::symbol_not_found("test", None);
    let code = splice::error_codes::SpliceErrorCode::from_splice_error(&error);

    assert!(code.is_some());
    let code_str = code.unwrap().code();
    eprintln!("Error code: '{}' (len={})", code_str, code_str.len());
    assert!(
        code_str.starts_with("SPL-E"),
        "Error code should have SPL-E### format"
    );
    assert!(
        code_str.len() >= 7,
        "Error code should be at least 7 characters (SPL-E###)"
    );
}

#[test]
fn test_ambiguous_symbol_has_warning_severity() {
    let error = SpliceError::AmbiguousSymbol {
        name: "foo".to_string(),
        files: vec!["a.rs".to_string(), "b.rs".to_string()],
    };

    let code = splice::error_codes::SpliceErrorCode::from_splice_error(&error);
    assert!(code.is_some());

    // AmbiguousSymbol should map to warning severity (from plan 15-01)
    let splice_code = code.unwrap();
    assert_eq!(splice_code.severity(), "warning");
}

#[test]
fn test_error_includes_location() {
    let error = SpliceError::AmbiguousReference {
        name: "foo".to_string(),
        file: "test.rs".to_string(),
        line: 10,
        col: 5,
        candidates: vec!["a::foo".to_string()],
    };

    let (file, line, column) = error.location();
    assert_eq!(file, Some("test.rs"));
    assert_eq!(line, Some(10));
    assert_eq!(column, Some(5));
}

#[test]
fn test_error_code_from_symbol_not_found() {
    let error = SpliceError::SymbolNotFound {
        message: "Symbol 'my_func' not found".to_string(),
        symbol: "my_func".to_string(),
        file: None,
        hint: "Run `splice ingest` to index the codebase.".to_string(),
    };

    let code = splice::error_codes::SpliceErrorCode::from_splice_error(&error);
    assert!(code.is_some());

    let splice_code = code.unwrap();
    assert_eq!(splice_code.code(), "SPL-E001");
    assert_eq!(splice_code.severity(), "error");
    assert!(!splice_code.hint().is_empty());
}
