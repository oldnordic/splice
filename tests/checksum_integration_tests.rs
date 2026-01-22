//! Integration tests for checksum fields in SpanResult.

use splice::checksum::{checksum_file, checksum_span};
use splice::output::SpanResult;
use std::io::Write;
use tempfile::NamedTempFile;

#[test]
fn test_span_result_with_checksums() {
    let mut file = NamedTempFile::new().unwrap();
    writeln!(file, "line 1").unwrap();
    writeln!(file, "line 2").unwrap();
    writeln!(file, "line 3").unwrap();

    let byte_start = 8; // Start of "line 2"
    let byte_end = 14;   // End of "line 2"

    // Create span with checksums
    let span = SpanResult::from_byte_span(file.path().to_string_lossy().to_string(), byte_start, byte_end);

    // Compute checksums
    let span_checksum = checksum_span(file.path(), byte_start, byte_end).unwrap();
    let file_checksum = checksum_file(file.path()).unwrap();

    // Add checksums to span
    let span = span.with_both_checksums(span_checksum.as_hex(), file_checksum.as_hex());

    // Verify checksums are set
    assert_eq!(span.checksum_before, Some(span_checksum.as_hex().to_string()));
    assert_eq!(span.file_checksum_before, Some(file_checksum.as_hex().to_string()));
}

#[test]
fn test_span_result_checksums_serialize_to_json() {
    let mut file = NamedTempFile::new().unwrap();
    writeln!(file, "line 1").unwrap();
    writeln!(file, "line 2").unwrap();

    let byte_start = 0;
    let byte_end = 14;

    let mut span = SpanResult::from_byte_span(file.path().to_string_lossy().to_string(), byte_start, byte_end);

    let span_checksum = checksum_span(file.path(), byte_start, byte_end).unwrap();
    let file_checksum = checksum_file(file.path()).unwrap();

    span = span.with_both_checksums(span_checksum.as_hex(), file_checksum.as_hex());

    // Serialize to JSON
    let json = serde_json::to_string(&span).unwrap();

    // Verify checksum fields are in JSON
    assert!(json.contains("\"checksum_before\""));
    assert!(json.contains("\"file_checksum_before\""));
    assert!(json.contains(&span_checksum.as_hex().to_string()));
}

#[test]
fn test_span_result_without_checksums_omits_fields() {
    let span = SpanResult::from_byte_span("test.rs".to_string(), 0, 10);

    // Serialize to JSON
    let json = serde_json::to_string(&span).unwrap();

    // Verify checksum fields are NOT in JSON when None (due to skip_serializing_if)
    assert!(!json.contains("\"checksum_before\""));
    assert!(!json.contains("\"file_checksum_before\""));
}

#[test]
fn test_checksum_fields_are_independent() {
    // Verify that checksum_before and span_checksum_before are independent fields
    let mut file = NamedTempFile::new().unwrap();
    writeln!(file, "content").unwrap();

    let byte_start = 0;
    let byte_end = 7;

    let span_checksum = checksum_span(file.path(), byte_start, byte_end).unwrap();
    let file_checksum = checksum_file(file.path()).unwrap();

    let mut span = SpanResult::from_byte_span(file.path().to_string_lossy().to_string(), byte_start, byte_end);

    // Use the new with_both_checksums method
    span = span.with_both_checksums(span_checksum.as_hex(), file_checksum.as_hex());

    // Also use the older with_span_checksums method
    span = span.with_span_checksums("old_span_checksum".to_string(), "old_span_after".to_string());

    // All fields should be set independently
    assert_eq!(span.checksum_before, Some(span_checksum.as_hex().to_string()));
    assert_eq!(span.file_checksum_before, Some(file_checksum.as_hex().to_string()));
    assert_eq!(span.span_checksum_before, Some("old_span_checksum".to_string()));
    assert_eq!(span.span_checksum_after, Some("old_span_after".to_string()));
}
