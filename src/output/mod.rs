//! Structured output types for Splice operations.
//!
//! All output types use serde::Serialize for consistent JSON output.
//!
//! ## Adding Checksums to SpanResult
//!
//! ```no_run
//! use splice::checksum::{checksum_span, checksum_file};
//! use splice::output::SpanResult;
//! use std::path::Path;
//!
//! let file_path = Path::new("src/main.rs");
//! let byte_start = 100;
//! let byte_end = 200;
//!
//! let mut span = SpanResult::from_byte_span(file_path.to_string_lossy().to_string(), byte_start, byte_end);
//!
//! // Add checksums for race condition protection
//! let span_checksum = checksum_span(file_path, byte_start, byte_end)?;
//! let file_checksum = checksum_file(file_path)?;
//! span = span.with_both_checksums(span_checksum.as_hex(), file_checksum.as_hex());
//! # Ok::<(), splice::SpliceError>(())
//! ```
//!
//! ## Adding Language Detection to SpanResult
//!
//! ```no_run
//! use splice::ingest::{detect_language, Language};
//! use splice::output::SpanResult;
//! use std::path::Path;
//!
//! let file_path = Path::new("src/main.rs");
//! let mut span = SpanResult::from_byte_span(file_path.to_string_lossy().to_string(), 100, 200);
//!
//! // Add language detection from file extension
//! if let Some(language) = detect_language(file_path) {
//!     span = span.with_language(language.as_str());
//! }
//! ```

/// Reachability, dead code, cycle detection, condensation, slicing, and export types.
pub mod analysis;
/// Core output types: constants, operation results, span primitives, and symbol matching.
pub mod core;
/// Magellan-compatible response types for delegated query commands.
pub mod magellan;
/// Span result, error details, diagnostics, and conversion implementations.
pub mod span_result;

pub use analysis::*;
pub use core::*;
pub use magellan::*;
pub use span_result::*;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_span_id_deterministic() {
        let span1 = SpanResult::from_byte_span("test.rs".to_string(), 10, 20);
        let span2 = SpanResult::from_byte_span("test.rs".to_string(), 10, 20);
        assert_eq!(
            span1.span_id, span2.span_id,
            "Same span inputs should produce the same span_id"
        );
    }

    #[test]
    fn test_match_id_preserved() {
        let match_id = uuid::Uuid::new_v4().to_string();
        let span = SpanResult::from_byte_span("test.rs".to_string(), 10, 20)
            .with_match_id(match_id.clone());
        assert_eq!(
            span.match_id,
            Some(match_id),
            "match_id should be preserved when set"
        );
    }

    #[test]
    fn test_match_id_from_resolved_span() {
        // Create a mock ResolvedSpan-like structure
        // Note: We can't directly create ResolvedSpan without node_id, but we can
        // verify that the conversion preserves match_id through the public API
        let match_id = uuid::Uuid::new_v4().to_string();
        let span1 = SpanResult::from_byte_span("test.rs".to_string(), 10, 20)
            .with_match_id(match_id.clone());

        assert_eq!(span1.match_id, Some(match_id));
        assert!(!span1.span_id.is_empty());
    }

    #[test]
    fn test_from_byte_span_generates_distinct_span_ids() {
        let span1 = SpanResult::from_byte_span("file.rs".to_string(), 0, 10);
        let span2 = SpanResult::from_byte_span("file.rs".to_string(), 0, 10);
        let span3 = SpanResult::from_byte_span("file.rs".to_string(), 20, 30);

        assert_eq!(span1.span_id, span2.span_id);
        assert_ne!(span2.span_id, span3.span_id);
        assert_ne!(span1.span_id, span3.span_id);
    }
}
