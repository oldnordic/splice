//! JSON schema compatibility tests for Magellan alignment.
//!
//! This module tests format compatibility between Splice and Magellan:
//!
//! - Field translation between Magellan and Splice conventions
//! - JSON serialization uses correct field names
//! - Roundtrip conversion preserves data
//! - Optional fields are preserved through translation

use splice::format::magellan::{from_magellan, to_magellan, translate_field_name, MagellanSpan};
use splice::output::{Span, SpanContext, SpanResult, SpanSemantics};

///////////////////////////////////////////////////////////////////////////////
// Field Translation Tests
///////////////////////////////////////////////////////////////////////////////

#[test]
fn test_from_magellan_translation() {
    // Create a MagellanSpan with known field values
    let magellan = MagellanSpan::new(
        "test_span_id".to_string(),
        "/path/to/file.rs".to_string(),
        100,
        200,
        5,  // start_line (Magellan convention)
        10, // end_line (Magellan convention)
        0,  // start_col (Magellan convention)
        4,  // end_col (Magellan convention)
    );

    // Convert to SpliceSpan
    let splice = from_magellan(magellan.clone());

    // Verify field mapping: Magellan's start_line -> Splice's start_line
    assert_eq!(splice.start_line, 5, "start_line should map correctly");
    assert_eq!(splice.end_line, 10, "end_line should map correctly");
    assert_eq!(splice.start_col, 0, "start_col should map correctly");
    assert_eq!(splice.end_col, 4, "end_col should map correctly");
    assert_eq!(splice.byte_start, 100, "byte_start should be preserved");
    assert_eq!(splice.byte_end, 200, "byte_end should be preserved");
    assert_eq!(
        splice.file_path, "/path/to/file.rs",
        "file_path should be preserved"
    );
    assert_eq!(
        splice.span_id, "test_span_id",
        "span_id should be preserved"
    );
}

#[test]
fn test_to_magellan_translation() {
    // Create a SpliceSpan with known field values
    let splice = SpanResult::from_byte_span("/path/to/file.rs".to_string(), 100, 200)
        .with_line_col(5, 10, 0, 4);

    // Convert to MagellanSpan
    let magellan = to_magellan(splice.clone());

    // Verify field mapping
    assert_eq!(magellan.start_line, 5, "start_line should be set correctly");
    assert_eq!(magellan.end_line, 10, "end_line should be set correctly");
    assert_eq!(magellan.start_col, 0, "start_col should be set correctly");
    assert_eq!(magellan.end_col, 4, "end_col should be set correctly");
    assert_eq!(magellan.byte_start, 100, "byte_start should be preserved");
    assert_eq!(magellan.byte_end, 200, "byte_end should be preserved");
    assert_eq!(
        magellan.file_path, "/path/to/file.rs",
        "file_path should be preserved"
    );
}

#[test]
fn test_roundtrip_preserves_data() {
    // Create an original SpliceSpan
    let original = SpanResult::from_byte_span("/path/to/file.rs".to_string(), 100, 200)
        .with_line_col(5, 10, 0, 4);

    // Splice -> Magellan -> Splice
    let magellan = to_magellan(original.clone());
    let roundtrip = from_magellan(magellan);

    // Verify all translatable fields are preserved
    assert_eq!(roundtrip.file_path, original.file_path);
    assert_eq!(roundtrip.byte_start, original.byte_start);
    assert_eq!(roundtrip.byte_end, original.byte_end);
    assert_eq!(roundtrip.start_line, original.start_line);
    assert_eq!(roundtrip.end_line, original.end_line);
    assert_eq!(roundtrip.start_col, original.start_col);
    assert_eq!(roundtrip.end_col, original.end_col);
    assert_eq!(roundtrip.span_id, original.span_id);
}

#[test]
fn test_translate_field_name_mapping() {
    // Test all four field name translations
    assert_eq!(translate_field_name("line_start"), Some("start_line"));
    assert_eq!(translate_field_name("line_end"), Some("end_line"));
    assert_eq!(translate_field_name("col_start"), Some("start_col"));
    assert_eq!(translate_field_name("col_end"), Some("end_col"));

    // Test fields that don't change
    assert_eq!(translate_field_name("file_path"), None);
    assert_eq!(translate_field_name("span_id"), None);
    assert_eq!(translate_field_name("byte_start"), None);
    assert_eq!(translate_field_name("byte_end"), None);
    assert_eq!(translate_field_name("unknown_field"), None);
}

///////////////////////////////////////////////////////////////////////////////
// JSON Schema Compatibility Tests
///////////////////////////////////////////////////////////////////////////////

#[test]
fn test_magellan_span_serializable() {
    let magellan = MagellanSpan::new(
        "abc123".to_string(),
        "src/main.rs".to_string(),
        100,
        200,
        5,
        10,
        0,
        4,
    );

    // Should serialize to valid JSON
    let json = serde_json::to_string(&magellan);
    assert!(json.is_ok(), "MagellanSpan should serialize to valid JSON");

    let json_str = json.unwrap();
    assert!(
        json_str.contains("start_line"),
        "JSON should contain start_line field"
    );
    assert!(
        json_str.contains("end_line"),
        "JSON should contain end_line field"
    );
}

#[test]
fn test_magellan_span_field_names() {
    let magellan = MagellanSpan::new(
        "test_id".to_string(),
        "file.rs".to_string(),
        0,
        10,
        1,
        2,
        0,
        5,
    );

    let json = serde_json::to_value(&magellan).unwrap();

    // Verify Magellan field names in JSON
    assert!(
        json.get("start_line").is_some(),
        "Should have start_line field (Magellan naming)"
    );
    assert!(
        json.get("end_line").is_some(),
        "Should have end_line field (Magellan naming)"
    );
    assert!(
        json.get("start_col").is_some(),
        "Should have start_col field (Magellan naming)"
    );
    assert!(
        json.get("end_col").is_some(),
        "Should have end_col field (Magellan naming)"
    );

    // These should NOT be present in MagellanSpan JSON
    assert!(
        json.get("line_start").is_none(),
        "Should NOT have line_start (Splice naming)"
    );
    assert!(
        json.get("line_end").is_none(),
        "Should NOT have line_end (Splice naming)"
    );
}

#[test]
fn test_splice_span_field_names() {
    let splice = SpanResult::from_byte_span("file.rs".to_string(), 0, 10).with_line_col(1, 2, 0, 5);

    let json = serde_json::to_value(&splice).unwrap();

    // SpliceSpan uses start_line, end_line (same as Magellan for these fields)
    assert!(json.get("start_line").is_some(), "Should have start_line");
    assert!(json.get("end_line").is_some(), "Should have end_line");
    assert!(json.get("start_col").is_some(), "Should have start_col");
    assert!(json.get("end_col").is_some(), "Should have end_col");
}

///////////////////////////////////////////////////////////////////////////////
// Integration with Existing Types
///////////////////////////////////////////////////////////////////////////////

#[test]
fn test_spanresult_to_magellan() {
    let span_result = SpanResult::from_byte_span("/absolute/path.rs".to_string(), 100, 200)
        .with_line_col(5, 10, 0, 4)
        .with_symbol("my_function".to_string(), "function".to_string());

    let magellan = to_magellan(span_result);

    assert_eq!(magellan.file_path, "/absolute/path.rs");
    assert_eq!(magellan.byte_start, 100);
    assert_eq!(magellan.byte_end, 200);
    assert_eq!(magellan.start_line, 5);
    assert_eq!(magellan.end_line, 10);
    assert_eq!(magellan.start_col, 0);
    assert_eq!(magellan.end_col, 4);
}

#[test]
fn test_spanresult_field_ordering() {
    let span_result =
        SpanResult::from_byte_span("file.rs".to_string(), 0, 10).with_line_col(1, 2, 0, 5);

    let json = serde_json::to_string(&span_result).unwrap();

    // Verify the JSON contains all expected fields in some order
    assert!(json.contains("\"file_path\""));
    assert!(json.contains("\"span_id\""));
    assert!(json.contains("\"byte_start\""));
    assert!(json.contains("\"byte_end\""));
    assert!(json.contains("\"start_line\""));
    assert!(json.contains("\"end_line\""));
}

///////////////////////////////////////////////////////////////////////////////
// Optional Fields Handling
///////////////////////////////////////////////////////////////////////////////

#[test]
fn test_optional_fields_preserved_translation() {
    // Create context
    let context = SpanContext {
        before: vec!["line 1".to_string()],
        selected: vec!["line 2".to_string()],
        after: vec!["line 3".to_string()],
    };

    // Create semantics
    let semantics = SpanSemantics {
        kind: "function".to_string(),
        language: "rust".to_string(),
    };

    // Create MagellanSpan with optional fields
    let magellan = MagellanSpan::new(
        "test_span_id".to_string(),
        "/path/to/file.rs".to_string(),
        100,
        200,
        5,
        10,
        0,
        4,
    )
    .with_context(context.clone())
    .with_semantics(semantics.clone());

    // Convert to Splice and back
    let splice = from_magellan(magellan);
    let roundtrip = to_magellan(splice);

    // Verify optional fields are preserved
    assert!(roundtrip.context.is_some());
    assert!(roundtrip.semantics.is_some());

    let roundtrip_context = roundtrip.context.unwrap();
    assert_eq!(roundtrip_context.before, context.before);
    assert_eq!(roundtrip_context.selected, context.selected);
    assert_eq!(roundtrip_context.after, context.after);

    let roundtrip_semantics = roundtrip.semantics.unwrap();
    assert_eq!(roundtrip_semantics.kind, semantics.kind);
    assert_eq!(roundtrip_semantics.language, semantics.language);
}

#[test]
fn test_checksums_preserved() {
    // Create a MagellanSpan with checksums
    let checksums = splice::output::SpanChecksums {
        checksum_before: Some("abc123".to_string()),
        checksum_after: Some("def456".to_string()),
        file_checksum_before: Some("file789".to_string()),
    };

    let magellan = MagellanSpan::new(
        "test_span_id".to_string(),
        "/path/to/file.rs".to_string(),
        100,
        200,
        5,
        10,
        0,
        4,
    )
    .with_checksums(checksums.clone());

    // Convert to Splice and back
    let splice = from_magellan(magellan);
    let roundtrip = to_magellan(splice);

    // Verify checksums are preserved
    assert!(roundtrip.checksums.is_some());

    let roundtrip_checksums = roundtrip.checksums.unwrap();
    assert_eq!(
        roundtrip_checksums.checksum_before,
        checksums.checksum_before
    );
    assert_eq!(roundtrip_checksums.checksum_after, checksums.checksum_after);
    assert_eq!(
        roundtrip_checksums.file_checksum_before,
        checksums.file_checksum_before
    );
}

///////////////////////////////////////////////////////////////////////////////
// Span Type Tests
///////////////////////////////////////////////////////////////////////////////

#[test]
fn test_span_uses_magellan_field_names() {
    // The Span type from output.rs uses Magellan-compatible field names
    let span = Span::new(
        "file.rs".to_string(),
        100,
        200,
        5,  // start_line
        0,  // start_col
        10, // end_line
        4,  // end_col
    );

    let json = serde_json::to_string(&span).unwrap();

    // Span uses Magellan-compatible field names
    assert!(json.contains("start_line"), "Span should use start_line");
    assert!(json.contains("end_line"), "Span should use end_line");
    assert!(json.contains("start_col"), "Span should use start_col");
    assert!(json.contains("end_col"), "Span should use end_col");
}

#[test]
fn test_magellan_span_from_span_conversion() {
    // Create a Span with all fields
    let context = SpanContext {
        before: vec!["// before".to_string()],
        selected: vec!["fn main() {}".to_string()],
        after: vec!["// after".to_string()],
    };

    let semantics = SpanSemantics {
        kind: "function".to_string(),
        language: "rust".to_string(),
    };

    let span = Span::new("src/main.rs".to_string(), 100, 200, 5, 0, 10, 4)
        .with_context(context.clone())
        .with_semantics(semantics.clone());

    // Convert to MagellanSpan via SpliceSpan
    let span_result =
        SpanResult::from_byte_span(span.file_path.clone(), span.byte_start, span.byte_end)
            .with_line_col(span.start_line, span.end_line, span.start_col, span.end_col)
            .with_context(span.context.unwrap())
            .with_semantic_info(
                span.semantics.as_ref().unwrap().kind.clone(),
                span.semantics.as_ref().unwrap().language.clone(),
            );

    let magellan = to_magellan(span_result);

    // Verify all fields match
    assert_eq!(magellan.file_path, span.file_path);
    assert_eq!(magellan.byte_start, span.byte_start);
    assert_eq!(magellan.byte_end, span.byte_end);
    assert_eq!(magellan.start_line, span.start_line);
    assert_eq!(magellan.end_line, span.end_line);
    assert_eq!(magellan.start_col, span.start_col);
    assert_eq!(magellan.end_col, span.end_col);

    // Verify optional fields
    assert!(magellan.context.is_some());
    assert!(magellan.semantics.is_some());
}
