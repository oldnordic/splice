//! File and span checksum computation for validation hooks.
//!
//! Provides deterministic checksums for:
//! - Entire files (SHA-256 for compatibility)
//! - Byte spans within files (for verification)
//! - Line ranges (for pre/post validation)

use sha2::{Digest, Sha256};
use std::path::Path;
use crate::error::{Result, SpliceError};

/// Checksum algorithm types.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChecksumAlgorithm {
    /// SHA-256 (default, cryptographically secure)
    Sha256,
}

/// Computed checksum with metadata.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Checksum {
    /// Hex-encoded checksum value
    pub value: String,
    /// Algorithm used
    pub algorithm: ChecksumAlgorithm,
    /// Byte count of checksummed content
    pub size: usize,
}

impl Checksum {
    /// Create a new checksum.
    pub fn new(value: String, algorithm: ChecksumAlgorithm, size: usize) -> Self {
        Self {
            value,
            algorithm,
            size,
        }
    }

    /// Get the hex-encoded checksum value.
    pub fn as_hex(&self) -> &str {
        &self.value
    }
}

/// Compute checksum of entire file.
pub fn checksum_file(path: &Path) -> Result<Checksum> {
    let contents = std::fs::read(path)
        .map_err(|e| SpliceError::IoContext {
            context: format!("Failed to read file for checksum: {}", path.display()),
            source: e,
        })?;

    let size = contents.len();
    let mut hasher = Sha256::new();
    hasher.update(&contents);
    let result = hasher.finalize();
    let value = format!("{:x}", result);

    Ok(Checksum::new(value, ChecksumAlgorithm::Sha256, size))
}

/// Compute checksum of byte span within file.
pub fn checksum_span(path: &Path, start: usize, end: usize) -> Result<Checksum> {
    let contents = std::fs::read(path)
        .map_err(|e| SpliceError::IoContext {
            context: format!("Failed to read file for span checksum: {}", path.display()),
            source: e,
        })?;

    if start > end || end > contents.len() {
        return Err(SpliceError::InvalidSpan {
            file: path.to_path_buf(),
            start,
            end,
            file_size: contents.len(),
        });
    }

    let span = &contents[start..end];
    let size = span.len();
    let mut hasher = Sha256::new();
    hasher.update(span);
    let result = hasher.finalize();
    let value = format!("{:x}", result);

    Ok(Checksum::new(value, ChecksumAlgorithm::Sha256, size))
}

/// Compute checksum of line range within file.
pub fn checksum_line_range(path: &Path, line_start: usize, line_end: usize) -> Result<Checksum> {
    let contents = std::fs::read(path)
        .map_err(|e| SpliceError::IoContext {
            context: format!("Failed to read file for line range checksum: {}", path.display()),
            source: e,
        })?;

    // Convert to string for line-based operations
    let text = std::str::from_utf8(&contents)
        .map_err(|e| SpliceError::InvalidUtf8 {
            file: path.to_path_buf(),
            source: e,
        })?;

    let lines: Vec<&str> = text.lines().collect();

    if line_start < 1 || line_start > line_end || line_end > lines.len() {
        return Err(SpliceError::InvalidLineRange {
            file: path.to_path_buf(),
            line_start,
            line_end,
            total_lines: lines.len(),
        });
    }

    // Extract the specified line range (1-based indexing)
    let range_text = lines[line_start - 1..line_end].join("\n");
    let size = range_text.len();
    let mut hasher = Sha256::new();
    hasher.update(range_text.as_bytes());
    let result = hasher.finalize();
    let value = format!("{:x}", result);

    Ok(Checksum::new(value, ChecksumAlgorithm::Sha256, size))
}

/// Verify file matches expected checksum.
pub fn verify_file(path: &Path, expected: &Checksum) -> Result<bool> {
    let actual = checksum_file(path)?;
    Ok(actual.value == expected.value)
}

/// Detect if file content has changed by comparing checksums.
pub fn has_file_changed(path: &Path, expected_checksum: &str) -> Result<bool> {
    let actual = checksum_file(path)?;
    Ok(actual.as_hex() != expected_checksum)
}

/// Compute diff checksum for verification (checksum of only changed bytes).
///
/// This function computes a checksum of the concatenated changes, which can be used
/// to verify that a set of replacements were applied correctly.
///
/// # Arguments
///
/// * `path` - Path to the file
/// * `changes` - Slice of (start, end, replacement_content) tuples
///
/// # Returns
///
/// * `Ok(Checksum)` - Checksum of all replacement content concatenated
/// * `Err(SpliceError)` - If file cannot be read or changes are out of bounds
///
/// # Example
///
/// ```no_run
/// use splice::checksum;
///
/// // Compute checksum of changes to verify later
/// let changes = vec![
///     (10, 20, "new content 1"),
///     (50, 60, "new content 2"),
/// ];
/// let checksum = checksum::checksum_diff(path, &changes)?;
/// ```
pub fn checksum_diff(path: &Path, changes: &[(usize, usize, &str)]) -> Result<Checksum> {
    // Read file to validate bounds
    let contents = std::fs::read(path)
        .map_err(|e| SpliceError::IoContext {
            context: format!("Failed to read file for diff checksum: {}", path.display()),
            source: e,
        })?;

    let file_size = contents.len();

    // Validate all spans are within bounds
    for (start, end, _) in changes {
        if *start > *end || *end > file_size {
            return Err(SpliceError::InvalidSpan {
                file: path.to_path_buf(),
                start: *start,
                end: *end,
                file_size,
            });
        }
    }

    // Concatenate all replacement content and compute checksum
    let diff_content: String = changes
        .iter()
        .map(|(_, _, replacement)| *replacement)
        .collect();

    let size = diff_content.len();
    let mut hasher = Sha256::new();
    hasher.update(diff_content.as_bytes());
    let result = hasher.finalize();
    let value = format!("{:x}", result);

    Ok(Checksum::new(value, ChecksumAlgorithm::Sha256, size))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::NamedTempFile;

    #[test]
    fn test_checksum_file_consistent() {
        let mut file = NamedTempFile::new().unwrap();
        file.write_all(b"Hello, World!").unwrap();

        let checksum1 = checksum_file(file.path()).unwrap();
        let checksum2 = checksum_file(file.path()).unwrap();

        assert_eq!(checksum1.value, checksum2.value);
        assert_eq!(checksum1.size, 13);
    }

    #[test]
    fn test_checksum_span() {
        let mut file = NamedTempFile::new().unwrap();
        file.write_all(b"0123456789").unwrap();

        let checksum = checksum_span(file.path(), 2, 7).unwrap();
        assert_eq!(checksum.size, 5); // bytes at positions 2,3,4,5,6

        // Verify span is different from full file
        let full_checksum = checksum_file(file.path()).unwrap();
        assert_ne!(checksum.value, full_checksum.value);
    }

    #[test]
    fn test_checksum_line_range() {
        let mut file = NamedTempFile::new().unwrap();
        writeln!(file, "line1").unwrap();
        writeln!(file, "line2").unwrap();
        writeln!(file, "line3").unwrap();

        let checksum = checksum_line_range(file.path(), 1, 2).unwrap();
        assert_eq!(checksum.size, 11); // "line1\nline2" = 5+1+5 = 11

        // Different ranges should produce different checksums
        let checksum2 = checksum_line_range(file.path(), 2, 3).unwrap();
        assert_ne!(checksum.value, checksum2.value);
    }

    #[test]
    fn test_verify_file_mismatch() {
        let mut file = NamedTempFile::new().unwrap();
        file.write_all(b"replaced content").unwrap();

        let checksum = checksum_file(file.path()).unwrap();

        // Tamper with file
        file.write_all(b"modified content").unwrap();

        let matches = verify_file(file.path(), &checksum).unwrap();
        assert!(!matches);
    }

    #[test]
    fn test_verify_file_match() {
        let mut file = NamedTempFile::new().unwrap();
        file.write_all(b"unchanged content").unwrap();

        let checksum = checksum_file(file.path()).unwrap();

        let matches = verify_file(file.path(), &checksum).unwrap();
        assert!(matches);
    }

    #[test]
    fn test_has_file_changed() {
        let mut file = NamedTempFile::new().unwrap();
        file.write_all(b"initial content").unwrap();

        let checksum = checksum_file(file.path()).unwrap();

        // File unchanged
        let changed = has_file_changed(file.path(), checksum.as_hex()).unwrap();
        assert!(!changed);

        // Modify file
        file.write_all(b"new content").unwrap();
        let changed = has_file_changed(file.path(), checksum.as_hex()).unwrap();
        assert!(changed);
    }

    #[test]
    fn test_checksum_span_invalid_bounds() {
        let mut file = NamedTempFile::new().unwrap();
        file.write_all(b"0123456789").unwrap();

        // Start > end
        assert!(checksum_span(file.path(), 7, 2).is_err());

        // End beyond file size
        assert!(checksum_span(file.path(), 5, 20).is_err());
    }

    #[test]
    fn test_checksum_line_range_invalid_bounds() {
        let mut file = NamedTempFile::new().unwrap();
        writeln!(file, "line1").unwrap();
        writeln!(file, "line2").unwrap();

        // line_start < 1
        assert!(checksum_line_range(file.path(), 0, 1).is_err());

        // line_end > total_lines
        assert!(checksum_line_range(file.path(), 1, 5).is_err());

        // line_start > line_end
        assert!(checksum_line_range(file.path(), 2, 1).is_err());
    }

    #[test]
    fn test_checksum_diff_single_change() {
        let mut file = NamedTempFile::new().unwrap();
        file.write_all(b"0123456789").unwrap();

        let changes = vec![(2, 5, "abc")];
        let checksum = checksum_diff(file.path(), &changes).unwrap();

        assert_eq!(checksum.size, 3); // "abc" length
        assert_eq!(checksum.algorithm, ChecksumAlgorithm::Sha256);
    }

    #[test]
    fn test_checksum_diff_multiple_changes() {
        let mut file = NamedTempFile::new().unwrap();
        file.write_all(b"0123456789abcdefghijklmn").unwrap();

        let changes = vec![
            (2, 5, "abc"),
            (10, 15, "xyz"),
        ];
        let checksum = checksum_diff(file.path(), &changes).unwrap();

        assert_eq!(checksum.size, 6); // "abc" + "xyz" = 3 + 3
    }

    #[test]
    fn test_checksum_diff_same_content_same_checksum() {
        let mut file = NamedTempFile::new().unwrap();
        file.write_all(b"0123456789").unwrap();

        let changes1 = vec![(2, 5, "abc")];
        let changes2 = vec![(2, 5, "abc")];

        let checksum1 = checksum_diff(file.path(), &changes1).unwrap();
        let checksum2 = checksum_diff(file.path(), &changes2).unwrap();

        assert_eq!(checksum1.value, checksum2.value);
    }

    #[test]
    fn test_checksum_diff_different_content_different_checksum() {
        let mut file = NamedTempFile::new().unwrap();
        file.write_all(b"0123456789").unwrap();

        let changes1 = vec![(2, 5, "abc")];
        let changes2 = vec![(2, 5, "def")];

        let checksum1 = checksum_diff(file.path(), &changes1).unwrap();
        let checksum2 = checksum_diff(file.path(), &changes2).unwrap();

        assert_ne!(checksum1.value, checksum2.value);
    }

    #[test]
    fn test_checksum_diff_invalid_bounds() {
        let mut file = NamedTempFile::new().unwrap();
        file.write_all(b"0123456789").unwrap();

        // start > end
        let changes = vec![(5, 2, "abc")];
        assert!(checksum_diff(file.path(), &changes).is_err());

        // end beyond file size
        let changes = vec![(5, 20, "abc")];
        assert!(checksum_diff(file.path(), &changes).is_err());
    }
}
