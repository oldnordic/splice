//! Magellan field translation utilities.
//!
//! This module provides bidirectional conversion between Magellan and Splice field naming conventions.
//!
//! ## Field Name Differences
//!
//! Magellan uses `start_line`/`end_line` and `start_col`/`end_col`, while Splice uses
//! `line_start`/`line_end` and `col_start`/`col_end`. This module translates between these
//! conventions to enable seamless interoperability.
//!
//! ## Translation Direction
//!
//! - **from_magellan**: Convert MagellanSpan (Magellan format) to SpliceSpan (Splice format)
//! - **to_magellan**: Convert SpliceSpan (Splice format) to MagellanSpan (Magellan format)
//!
//! ## Field Mapping
//!
//! | Magellan   | Splice     |
//! |------------|------------|
//! | start_line | line_start |
//! | end_line   | line_end   |
//! | start_col  | col_start  |
//! | end_col    | col_end    |

use serde::{Deserialize, Serialize};

/// Type alias for Splice's SpanResult from the output module.
pub type SpliceSpan = crate::output::SpanResult;

/// Span representation using Magellan field naming conventions.
///
/// This struct mirrors the Span structure but uses Magellan's naming:
/// - `start_line` instead of `line_start`
/// - `end_line` instead of `line_end`
/// - `start_col` instead of `col_start`
/// - `end_col` instead of `col_end`
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MagellanSpan {
    /// Stable span identifier (SHA-256 hash)
    pub span_id: String,
    /// File path (absolute or root-relative)
    pub file_path: String,
    /// Byte range start (inclusive)
    pub byte_start: usize,
    /// Byte range end (exclusive)
    pub byte_end: usize,
    /// Start line (1-indexed) - Magellan convention
    pub start_line: usize,
    /// End line (1-indexed) - Magellan convention
    pub end_line: usize,
    /// Start column (0-indexed, byte-based) - Magellan convention
    pub start_col: usize,
    /// End column (0-indexed, byte-based) - Magellan convention
    pub end_col: usize,
    /// Context lines around the span
    #[serde(skip_serializing_if = "Option::is_none")]
    pub context: Option<crate::output::SpanContext>,
    /// Semantic information (kind, language)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub semantics: Option<crate::output::SpanSemantics>,
    /// Checksums for content verification
    #[serde(skip_serializing_if = "Option::is_none")]
    pub checksums: Option<crate::output::SpanChecksums>,
}

impl MagellanSpan {
    /// Create a new MagellanSpan from individual fields.
    pub fn new(
        span_id: String,
        file_path: String,
        byte_start: usize,
        byte_end: usize,
        start_line: usize,
        end_line: usize,
        start_col: usize,
        end_col: usize,
    ) -> Self {
        Self {
            span_id,
            file_path,
            byte_start,
            byte_end,
            start_line,
            end_line,
            start_col,
            end_col,
            context: None,
            semantics: None,
            checksums: None,
        }
    }

    /// Add context to the span.
    pub fn with_context(mut self, context: crate::output::SpanContext) -> Self {
        self.context = Some(context);
        self
    }

    /// Add semantic info to the span.
    pub fn with_semantics(mut self, semantics: crate::output::SpanSemantics) -> Self {
        self.semantics = Some(semantics);
        self
    }

    /// Add checksums to the span.
    pub fn with_checksums(mut self, checksums: crate::output::SpanChecksums) -> Self {
        self.checksums = Some(checksums);
        self
    }
}

/// Convert MagellanSpan to SpliceSpan (from_magellan).
///
/// Maps Magellan field names to Splice field names:
/// - `start_line` -> `line_start`
/// - `end_line` -> `line_end`
/// - `start_col` -> `col_start`
/// - `end_col` -> `col_end`
///
/// All optional fields (context, semantics, checksums) are preserved.
///
/// # Example
///
/// ```no_run
/// use splice::format::magellan::{MagellanSpan, from_magellan};
///
/// let magellan_span = MagellanSpan::new(
///     "abc123".to_string(),
///     "src/main.rs".to_string(),
///     100,
///     200,
///     5,
///     10,
///     0,
///     4,
/// );
///
/// let splice_span = from_magellan(magellan_span);
/// assert_eq!(splice_span.start_line, 5);  // Magellan's start_line
/// assert_eq!(splice_span.end_line, 10);   // Magellan's end_line
/// ```
pub fn from_magellan(span: MagellanSpan) -> SpliceSpan {
    let mut result = SpliceSpan::from_byte_span(span.file_path, span.byte_start, span.byte_end);

    // Map Magellan fields to Splice fields
    result = result.with_line_col(span.start_line, span.end_line, span.start_col, span.end_col);

    // Preserve span_id
    result.span_id = span.span_id;

    // Preserve optional fields
    if let Some(context) = span.context {
        result = result.with_context(context);
    }
    if let Some(semantics) = span.semantics {
        result.semantics = Some(semantics);
    }
    if let Some(checksums) = span.checksums {
        result.checksums = Some(checksums);
    }

    result
}

/// Convert SpliceSpan to MagellanSpan (to_magellan).
///
/// Maps Splice field names to Magellan field names:
/// - `line_start` -> `start_line`
/// - `line_end` -> `end_line`
/// - `col_start` -> `start_col`
/// - `col_end` -> `end_col`
///
/// All optional fields (context, semantics, checksums) are preserved.
///
/// # Example
///
/// ```no_run
/// use splice::format::magellan::{to_magellan, SpliceSpan};
/// use splice::output::SpanResult;
///
/// let splice_span = SpanResult::from_byte_span("src/main.rs".to_string(), 100, 200)
///     .with_line_col(5, 10, 0, 4);
///
/// let magellan_span = to_magellan(splice_span);
/// assert_eq!(magellan_span.start_line, 5);  // From Splice's start_line
/// assert_eq!(magellan_span.end_line, 10);   // From Splice's end_line
/// ```
pub fn to_magellan(span: SpliceSpan) -> MagellanSpan {
    MagellanSpan {
        span_id: span.span_id,
        file_path: span.file_path,
        byte_start: span.byte_start,
        byte_end: span.byte_end,
        // Map Splice fields to Magellan fields
        start_line: span.start_line,
        end_line: span.end_line,
        start_col: span.start_col,
        end_col: span.end_col,
        context: span.context,
        semantics: span.semantics,
        checksums: span.checksums,
    }
}

/// Translate a Splice field name to its Magellan equivalent.
///
/// Returns `None` if the field name doesn't have a Magellan equivalent
/// (i.e., it's the same in both conventions).
///
/// # Example
///
/// ```
/// use splice::format::magellan::translate_field_name;
///
/// assert_eq!(translate_field_name("line_start"), Some("start_line"));
/// assert_eq!(translate_field_name("line_end"), Some("end_line"));
/// assert_eq!(translate_field_name("col_start"), Some("start_col"));
/// assert_eq!(translate_field_name("col_end"), Some("end_col"));
/// assert_eq!(translate_field_name("file_path"), None);  // Same in both
/// ```
pub fn translate_field_name(splice_field: &str) -> Option<&'static str> {
    match splice_field {
        "line_start" => Some("start_line"),
        "line_end" => Some("end_line"),
        "col_start" => Some("start_col"),
        "col_end" => Some("end_col"),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_from_magellan_translation() {
        // Create a MagellanSpan with known field values
        let magellan = MagellanSpan::new(
            "test_span_id".to_string(),
            "/path/to/file.rs".to_string(),
            100,
            200,
            5,
            10,
            0,
            4,
        );

        // Convert to SpliceSpan
        let splice = from_magellan(magellan.clone());

        // Verify field mapping: start_line -> start_line (same in SpliceSpan)
        assert_eq!(splice.start_line, 5, "start_line should map to start_line");
        assert_eq!(splice.end_line, 10, "end_line should map to end_line");
        assert_eq!(splice.start_col, 0, "start_col should map to start_col");
        assert_eq!(splice.end_col, 4, "end_col should map to end_col");
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
        let splice = SpliceSpan::from_byte_span("/path/to/file.rs".to_string(), 100, 200)
            .with_line_col(5, 10, 0, 4);

        // Convert to MagellanSpan
        let magellan = to_magellan(splice.clone());

        // Verify field mapping
        assert_eq!(
            magellan.start_line, 5,
            "start_line should come from start_line"
        );
        assert_eq!(magellan.end_line, 10, "end_line should come from end_line");
        assert_eq!(
            magellan.start_col, 0,
            "start_col should come from start_col"
        );
        assert_eq!(magellan.end_col, 4, "end_col should come from end_col");
        assert_eq!(magellan.byte_start, 100, "byte_start should be preserved");
        assert_eq!(magellan.byte_end, 200, "byte_end should be preserved");
        assert_eq!(
            magellan.file_path, "/path/to/file.rs",
            "file_path should be preserved"
        );
    }

    #[test]
    fn test_roundtrip_translation() {
        // Create an original SpliceSpan
        let original = SpliceSpan::from_byte_span("/path/to/file.rs".to_string(), 100, 200)
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
        // Note: symbol/kind are SpliceSpan fields not present in MagellanSpan,
        // so they are NOT preserved in the roundtrip (expected limitation)
    }

    #[test]
    fn test_translate_field_name() {
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

    #[test]
    fn test_optional_fields_preserved() {
        // Create MagellanSpan with optional fields
        let context = crate::output::SpanContext {
            before: vec!["line 1".to_string()],
            selected: vec!["line 2".to_string()],
            after: vec!["line 3".to_string()],
        };

        let semantics = crate::output::SpanSemantics {
            kind: "function".to_string(),
            language: "rust".to_string(),
        };

        let checksums = crate::output::SpanChecksums {
            checksum_before: Some("abc123".to_string()),
            checksum_after: Some("def456".to_string()),
            file_checksum_before: Some("file789".to_string()),
        };

        // Create MagellanSpan with all optional fields
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
        .with_semantics(semantics.clone())
        .with_checksums(checksums.clone());

        // Convert to Splice and back
        let splice = from_magellan(magellan);
        let roundtrip = to_magellan(splice);

        // Verify optional fields are preserved
        assert!(roundtrip.context.is_some());
        assert!(roundtrip.semantics.is_some());
        assert!(roundtrip.checksums.is_some());

        let roundtrip_context = roundtrip.context.unwrap();
        assert_eq!(roundtrip_context.before, context.before);
        assert_eq!(roundtrip_context.selected, context.selected);
        assert_eq!(roundtrip_context.after, context.after);

        let roundtrip_semantics = roundtrip.semantics.unwrap();
        assert_eq!(roundtrip_semantics.kind, semantics.kind);
        assert_eq!(roundtrip_semantics.language, semantics.language);

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

    #[test]
    fn test_span_id_preserved() {
        // Test that span_id is preserved through translation
        let magellan = MagellanSpan::new(
            "unique_span_id_12345".to_string(),
            "/path/to/file.rs".to_string(),
            100,
            200,
            5,
            10,
            0,
            4,
        );

        let splice = from_magellan(magellan.clone());
        assert_eq!(splice.span_id, "unique_span_id_12345");

        // And roundtrip
        let magellan2 = to_magellan(splice);
        assert_eq!(magellan2.span_id, "unique_span_id_12345");
    }
}
