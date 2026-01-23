//! Tests for symbol suggestions module.

use splice::SpliceError;

#[test]
fn test_symbol_not_found_with_suggestions_found() {
    let candidates = vec![
        "foo".to_string(),
        "foobar".to_string(),
        "bar".to_string(),
        "baz".to_string(),
    ];

    let error = SpliceError::symbol_not_found_with_suggestions("fooo", None, &candidates);

    // Verify hint contains suggestion
    let hint = error.hint().unwrap_or("");
    assert!(
        hint.contains("Did you mean"),
        "Hint should mention suggestions"
    );
    assert!(hint.contains("foo"), "Hint should suggest similar symbol");
}

#[test]
fn test_symbol_not_found_with_suggestions_no_match() {
    let candidates = vec![
        "apple".to_string(),
        "banana".to_string(),
        "cherry".to_string(),
    ];

    let error = SpliceError::symbol_not_found_with_suggestions("xyz", None, &candidates);

    // Verify hint doesn't contain "Did you mean" when no similar symbols
    let hint = error.hint().unwrap_or("");
    assert!(
        !hint.contains("Did you mean"),
        "Hint should not suggest when no matches"
    );
    assert!(
        hint.contains("ingest") || hint.contains("not found"),
        "Hint should have generic message"
    );
}

#[test]
fn test_symbol_not_found_with_suggestions_with_file() {
    let candidates = vec!["my_function".to_string()];

    let error = SpliceError::symbol_not_found_with_suggestions(
        "my_functionn",
        Some(std::path::Path::new("src/main.rs")),
        &candidates,
    );

    let hint = error.hint().unwrap_or("");
    assert!(
        hint.contains("Did you mean"),
        "Should suggest similar symbol"
    );
}

#[test]
fn test_symbol_not_found_empty_candidates() {
    let candidates: Vec<String> = vec![];

    let error = SpliceError::symbol_not_found_with_suggestions("foo", None, &candidates);

    let hint = error.hint().unwrap_or("");
    assert!(
        hint.contains("ingest") || hint.contains("not found"),
        "Should give generic hint"
    );
}
