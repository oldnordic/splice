//! Context extraction for span surroundings.
//!
//! Provides line-based context extraction using ropey for efficient
//! UTF-8 aware line/column calculations.

use crate::error::{Result, SpliceError};
use std::path::Path;

use crate::output::SpanContext;

/// Extract context lines for a byte span.
///
/// Given a file path and byte range, extracts lines before, within, and after
/// the span. Uses UTF-8 byte offsets consistent with span coordinates.
///
/// # Arguments
///
/// * `path` - File path to read
/// * `byte_start` - Start byte offset (must be <= byte_end)
/// * `byte_end` - End byte offset (must be <= file size)
/// * `context_lines` - Number of context lines before/after (default: 3)
///
/// # Returns
///
/// * `Ok(SpanContext)` - Extracted context with before/selected/after arrays
/// * `Err(SpliceError)` - If file cannot be read or span is invalid
///
/// # Examples
///
/// ```no_run
/// use splice::context::extract_context;
/// use std::path::Path;
///
/// let context = extract_context(Path::new("src/main.rs"), 100, 200, 3)?;
/// println!("Before: {} lines", context.before.len());
/// # Ok::<(), splice::error::SpliceError>(())
/// ```
pub fn extract_context(
    path: &Path,
    byte_start: usize,
    byte_end: usize,
    context_lines: usize,
) -> Result<SpanContext> {
    use ropey::Rope;

    // Validate byte range
    if byte_start > byte_end {
        return Err(SpliceError::InvalidSpan {
            file: path.to_path_buf(),
            start: byte_start,
            end: byte_end,
            file_size: 0, // Will be updated after read
        });
    }

    // Read file
    let contents = std::fs::read(path).map_err(|e| SpliceError::IoContext {
        context: format!("Failed to read file for context extraction: {}", path.display()),
        source: e,
    })?;

    let file_size = contents.len();

    // Validate end is within file
    if byte_end > file_size {
        return Err(SpliceError::InvalidSpan {
            file: path.to_path_buf(),
            start: byte_start,
            end: byte_end,
            file_size,
        });
    }

    // Create Rope for efficient line operations (UTF-8 aware)
    let rope = Rope::from_str(std::str::from_utf8(&contents).map_err(|e| {
        SpliceError::InvalidUtf8 {
            file: path.to_path_buf(),
            source: e,
        }
    })?);

    // Convert byte offsets to line numbers (0-based)
    let start_line = rope.byte_to_line(byte_start);
    let end_line = rope.byte_to_line(byte_end);

    // Calculate context boundaries
    let context_start = start_line.saturating_sub(context_lines);
    let context_end = (end_line + context_lines + 1).min(rope.len_lines());

    // Extract before lines
    let before: Vec<String> = (context_start..start_line)
        .map(|i| rope.line(i).to_string())
        .collect();

    // Extract selected lines (the span itself)
    let selected: Vec<String> = (start_line..=end_line)
        .map(|i| rope.line(i).to_string())
        .collect();

    // Extract after lines
    let after: Vec<String> = (end_line + 1..context_end)
        .map(|i| rope.line(i).to_string())
        .collect();

    Ok(SpanContext {
        before,
        selected,
        after,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::NamedTempFile;

    #[test]
    fn test_extract_context_basic() {
        let mut file = NamedTempFile::new().unwrap();
        writeln!(file, "line 1").unwrap();
        writeln!(file, "line 2").unwrap();
        writeln!(file, "line 3").unwrap();
        writeln!(file, "line 4").unwrap();
        writeln!(file, "line 5").unwrap();

        // Context for lines 2-3 (bytes 8-24 approx)
        let context = extract_context(file.path(), 8, 24, 1).unwrap();

        assert_eq!(context.before.len(), 1); // "line 1"
        assert_eq!(context.selected.len(), 2); // "line 2", "line 3"
        assert_eq!(context.after.len(), 1); // "line 4"
    }

    #[test]
    fn test_extract_context_zero_context() {
        let mut file = NamedTempFile::new().unwrap();
        writeln!(file, "line 1").unwrap();
        writeln!(file, "line 2").unwrap();
        writeln!(file, "line 3").unwrap();

        let contents = std::fs::read(file.path()).unwrap();
        let line2_start = contents.iter().position(|&b| b == b'2').unwrap();
        let line2_end = line2_start + 6; // "line 2\n"

        let context = extract_context(file.path(), line2_start, line2_end, 0).unwrap();

        assert_eq!(context.before.len(), 0);
        assert_eq!(context.selected.len(), 1);
        assert_eq!(context.after.len(), 0);
    }

    #[test]
    fn test_extract_context_start_of_file() {
        let mut file = NamedTempFile::new().unwrap();
        writeln!(file, "line 1").unwrap();
        writeln!(file, "line 2").unwrap();
        writeln!(file, "line 3").unwrap();

        let contents = std::fs::read(file.path()).unwrap();
        let context = extract_context(file.path(), 0, 12, 2).unwrap(); // "line 1\nline 2\n"

        assert_eq!(context.before.len(), 0); // No lines before start
        assert_eq!(context.selected.len(), 2);
        assert_eq!(context.after.len(), 1); // "line 3"
    }

    #[test]
    fn test_extract_context_end_of_file() {
        let mut file = NamedTempFile::new().unwrap();
        writeln!(file, "line 1").unwrap();
        writeln!(file, "line 2").unwrap();
        writeln!(file, "line 3").unwrap();

        let contents = std::fs::read(file.path()).unwrap();
        let line3_start = contents.iter().position(|&b| b == b'3').unwrap();

        let context = extract_context(file.path(), line3_start, contents.len(), 2).unwrap();

        assert_eq!(context.before.len(), 2); // "line 1", "line 2"
        assert_eq!(context.selected.len(), 1); // "line 3"
        assert_eq!(context.after.len(), 0); // No lines after end
    }

    #[test]
    fn test_extract_context_utf8_multibyte() {
        let mut file = NamedTempFile::new().unwrap();
        // Use emoji (multi-byte UTF-8)
        writeln!(file, "line 🦀 1").unwrap();
        writeln!(file, "line 🚀 2").unwrap();
        writeln!(file, "line ⭐ 3").unwrap();

        let contents = std::fs::read(file.path()).unwrap();
        let rocket_start = contents.iter().position(|&b| b == 0xF0).unwrap(); // UTF-8 start byte for emoji

        // Context should still work with multi-byte characters
        let context = extract_context(file.path(), rocket_start + 6, rocket_start + 10, 1).unwrap();

        assert_eq!(context.selected.len(), 1);
        assert!(context.selected[0].contains("🚀"));
    }

    #[test]
    fn test_extract_context_invalid_span() {
        let mut file = NamedTempFile::new().unwrap();
        writeln!(file, "line 1").unwrap();

        // start > end
        let result = extract_context(file.path(), 10, 5, 1);
        assert!(result.is_err());

        // end beyond file size
        let result = extract_context(file.path(), 0, 1000, 1);
        assert!(result.is_err());
    }

    #[test]
    fn test_extract_context_empty_file() {
        let file = NamedTempFile::new().unwrap();
        let result = extract_context(file.path(), 0, 0, 1);
        assert!(result.is_ok());
        // Empty file should return empty context
        let context = result.unwrap();
        assert_eq!(context.before.len(), 0);
        assert_eq!(context.selected.len(), 0);
        assert_eq!(context.after.len(), 0);
    }

    #[test]
    fn test_extract_context_large_context_request() {
        let mut file = NamedTempFile::new().unwrap();
        writeln!(file, "line 1").unwrap();
        writeln!(file, "line 2").unwrap();
        writeln!(file, "line 3").unwrap();

        // Request more context lines than exist
        let context = extract_context(file.path(), 7, 13, 100).unwrap();

        // Should saturate at file boundaries
        assert_eq!(context.before.len(), 1); // Only "line 1"
        assert_eq!(context.selected.len(), 1);
        assert_eq!(context.after.len(), 1); // Only "line 3"
    }
}
