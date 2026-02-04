//! Cross-file rename with byte-accurate replacement.
//!
//! This module implements safe symbol renaming using ReferenceFact
//! spans from Magellan. Replacements are applied at exact byte offsets
//! with proper offset recalculation for variable-length names.

use crate::error::{Result, SpliceError};
use crate::graph::MagellanIntegration;
use magellan::references::ReferenceFact;
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

/// Perform byte-accurate replacement at a specific span.
///
/// This function replaces the content between byte_start and byte_end
/// with new_name, preserving all other content exactly as-is.
///
/// # Arguments
/// * `content` - File content as bytes
/// * `span` - ReferenceFact with byte_start/byte_end
/// * `new_name` - Replacement name (as bytes)
///
/// # Returns
/// Modified content with replacement applied
///
/// # Errors
/// Returns InvalidSpan if byte_start/byte_end are out of bounds
/// or if the span crosses UTF-8 character boundaries.
pub fn replace_at_span(
    content: &[u8],
    span: &ReferenceFact,
    new_name: &[u8],
) -> Result<Vec<u8>> {
    // Validate span boundaries
    if span.byte_start >= content.len() || span.byte_end > content.len() {
        return Err(SpliceError::InvalidSpan {
            file: span.file_path.clone(),
            start: span.byte_start,
            end: span.byte_end,
            file_size: content.len(),
        });
    }

    // Validate UTF-8 boundaries
    MagellanIntegration::validate_utf8_span(
        content,
        span.byte_start,
        span.byte_end,
        &span.file_path,
    )?;

    // Build new content: before + new_name + after
    let mut result = Vec::with_capacity(content.len() + new_name.len());
    result.extend_from_slice(&content[..span.byte_start]);
    result.extend_from_slice(new_name);
    result.extend_from_slice(&content[span.byte_end..]);

    Ok(result)
}

/// Apply replacements to a single file with offset recalculation.
///
/// This function reads a file, applies multiple replacements at specific
/// byte spans, and writes the result back. Replacements are applied from
/// end to start (descending byte_start) to ensure that earlier replacements
/// don't affect the byte offsets of later ones.
///
/// # Arguments
/// * `file_path` - Path to file
/// * `_old_name` - Original symbol name (for validation, currently unused)
/// * `new_name` - Replacement symbol name
/// * `references` - ReferenceFact entries sorted by byte_start DESCENDING
///
/// # Returns
/// Number of replacements applied
///
/// # Errors
/// Returns Io error if file cannot be read or written.
/// Returns InvalidSpan if any reference span is invalid.
pub fn apply_replacements_in_file(
    file_path: &Path,
    _old_name: &str,
    new_name: &str,
    references: &[ReferenceFact],
) -> Result<usize> {
    // Read file as bytes
    let content = fs::read(file_path).map_err(|e| SpliceError::Io {
        path: file_path.to_path_buf(),
        source: e,
    })?;

    let new_name_bytes = new_name.as_bytes();
    let mut current_content = content;
    let mut replacements = 0;

    // Apply replacements from end to start (descending byte_start)
    // This ensures earlier replacements don't affect later offsets
    for reference in references {
        match replace_at_span(&current_content, reference, new_name_bytes) {
            Ok(new_content) => {
                current_content = new_content;
                replacements += 1;
            }
            Err(e) => {
                return Err(SpliceError::Other(format!(
                    "Failed to replace in {} at {}..{}: {}",
                    file_path.display(),
                    reference.byte_start,
                    reference.byte_end,
                    e
                )));
            }
        }
    }

    // Write modified content back
    fs::write(file_path, current_content).map_err(|e| SpliceError::Io {
        path: file_path.to_path_buf(),
        source: e,
    })?;

    Ok(replacements)
}

/// Group references by file path.
///
/// This function takes a flat list of references and groups them by file.
/// Within each file, references are sorted by byte_start in descending order
/// to enable safe sequential replacement without offset recalculation issues.
///
/// # Arguments
/// * `references` - Flat list of ReferenceFact entries
///
/// # Returns
/// HashMap mapping file path to vector of ReferenceFact entries sorted
/// by byte_start DESCENDING for safe replacement.
pub fn group_references_by_file(
    references: &[ReferenceFact],
) -> HashMap<PathBuf, Vec<ReferenceFact>> {
    let mut grouped: HashMap<PathBuf, Vec<ReferenceFact>> = HashMap::new();

    for reference in references {
        grouped
            .entry(reference.file_path.clone())
            .or_insert_with(Vec::new)
            .push(reference.clone());
    }

    // Sort each file's references by byte_start DESCENDING
    // This ensures replacements are applied from end to start
    for refs in grouped.values_mut() {
        refs.sort_by(|a, b| b.byte_start.cmp(&a.byte_start));
    }

    grouped
}

/// Simulate replacements to generate preview data.
///
/// This function generates a preview of what changes would be made
/// without actually modifying any files. It returns a mapping of
/// file paths to the number of replacements that would be applied.
///
/// # Arguments
/// * `references` - ReferenceFact entries to simulate
///
/// # Returns
/// HashMap mapping file path to count of replacements
pub fn simulate_replacements(
    references: &[ReferenceFact],
) -> HashMap<PathBuf, usize> {
    let mut simulation: HashMap<PathBuf, usize> = HashMap::new();

    for reference in references {
        *simulation
            .entry(reference.file_path.clone())
            .or_insert(0) += 1;
    }

    simulation
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_reference(
        file_path: &str,
        byte_start: usize,
        byte_end: usize,
    ) -> ReferenceFact {
        ReferenceFact {
            file_path: PathBuf::from(file_path),
            referenced_symbol: "old_name".to_string(),
            byte_start,
            byte_end,
            start_line: 1,
            start_col: byte_start,
            end_line: 1,
            end_col: byte_end,
        }
    }

    #[test]
    fn test_replace_at_span_basic() {
        let content = b"fn old_name() { old_name(); }";
        let span = create_test_reference("test.rs", 3, 11);
        let new_name = b"new_name";

        let result = replace_at_span(content, &span, new_name).unwrap();
        assert_eq!(result, b"fn new_name() { old_name(); }");
    }

    #[test]
    fn test_replace_at_span_different_length() {
        let content = b"fn foo() {}";
        let span = create_test_reference("test.rs", 3, 6);
        let new_name = b"bar";

        let result = replace_at_span(content, &span, new_name).unwrap();
        assert_eq!(result, b"fn bar() {}");
    }

    #[test]
    fn test_replace_at_span_longer_name() {
        let content = b"fn foo() {}";
        let span = create_test_reference("test.rs", 3, 6);
        let new_name = b"baz_qux";

        let result = replace_at_span(content, &span, new_name).unwrap();
        assert_eq!(result, b"fn baz_qux() {}");
    }

    #[test]
    fn test_replace_at_span_shorter_name() {
        let content = b"function foo() {}";
        let span = create_test_reference("test.rs", 9, 12);
        let new_name = b"x";

        let result = replace_at_span(content, &span, new_name).unwrap();
        assert_eq!(result, b"function x() {}");
    }

    #[test]
    fn test_replace_at_span_invalid_start() {
        let content = b"fn foo() {}";
        let span = create_test_reference("test.rs", 100, 105);
        let new_name = b"bar";

        let result = replace_at_span(content, &span, new_name);
        assert!(result.is_err());
        match result.unwrap_err() {
            SpliceError::InvalidSpan { start, .. } => assert_eq!(start, 100),
            _ => panic!("Expected InvalidSpan error"),
        }
    }

    #[test]
    fn test_replace_at_span_invalid_end() {
        let content = b"fn foo() {}";
        let span = create_test_reference("test.rs", 0, 100);
        let new_name = b"bar";

        let result = replace_at_span(content, &span, new_name);
        assert!(result.is_err());
        match result.unwrap_err() {
            SpliceError::InvalidSpan { end, .. } => assert_eq!(end, 100),
            _ => panic!("Expected InvalidSpan error"),
        }
    }

    #[test]
    fn test_replace_at_span_empty_replacement() {
        let content = b"fn foo() {}";
        let span = create_test_reference("test.rs", 3, 6);
        let new_name = b"";

        let result = replace_at_span(content, &span, new_name).unwrap();
        assert_eq!(result, b"fn () {}");
    }

    #[test]
    fn test_group_references_by_file() {
        let references = vec![
            create_test_reference("/src/a.rs", 100, 103),
            create_test_reference("/src/b.rs", 50, 53),
            create_test_reference("/src/a.rs", 20, 23),
            create_test_reference("/src/b.rs", 10, 13),
        ];

        let grouped = group_references_by_file(&references);

        assert_eq!(grouped.len(), 2);
        assert!(grouped.contains_key(PathBuf::from("/src/a.rs").as_path()));
        assert!(grouped.contains_key(PathBuf::from("/src/b.rs").as_path()));

        // Check that each file's refs are sorted descending by byte_start
        let a_refs = grouped.get(&PathBuf::from("/src/a.rs")).unwrap();
        assert_eq!(a_refs[0].byte_start, 100);
        assert_eq!(a_refs[1].byte_start, 20);

        let b_refs = grouped.get(&PathBuf::from("/src/b.rs")).unwrap();
        assert_eq!(b_refs[0].byte_start, 50);
        assert_eq!(b_refs[1].byte_start, 10);
    }

    #[test]
    fn test_simulate_replacements() {
        let references = vec![
            create_test_reference("/src/a.rs", 100, 103),
            create_test_reference("/src/b.rs", 50, 53),
            create_test_reference("/src/a.rs", 20, 23),
        ];

        let simulation = simulate_replacements(&references);

        assert_eq!(simulation.len(), 2);
        assert_eq!(simulation.get(&PathBuf::from("/src/a.rs")), Some(&2));
        assert_eq!(simulation.get(&PathBuf::from("/src/b.rs")), Some(&1));
    }

    #[test]
    fn test_utf8_multibyte_character_replacement() {
        // Content with multibyte UTF-8 characters
        // "世界" is 6 bytes (3 bytes per character)
        let content = "fn foo() { // 世界 }".as_bytes();
        let span = create_test_reference("test.rs", 3, 6);
        let new_name = b"bar";

        let result = replace_at_span(content, &span, new_name).unwrap();
        assert_eq!(result, b"fn bar() { // \xe4\xb8\x96\xe7\x95\x8c }");
    }

    #[test]
    fn test_apply_replacements_in_file_integration() {
        use tempfile::NamedTempFile;

        let temp_file = NamedTempFile::new().unwrap();
        let file_path = temp_file.path();

        // Write initial content
        let initial_content = b"fn old_name() {\n    old_name();\n    old_name();\n}\n";
        fs::write(file_path, initial_content).unwrap();

        // Create references (in descending byte_start order)
        // "old_name" appears at: 3-11, 20-28, 36-44
        // We skip the definition (3-11) and replace the usages
        let references = vec![
            create_test_reference(file_path.to_str().unwrap(), 36, 44),
            create_test_reference(file_path.to_str().unwrap(), 20, 28),
        ];

        // Apply replacements
        let count = apply_replacements_in_file(file_path, "old_name", "new_name", &references).unwrap();

        assert_eq!(count, 2);

        // Verify the result - definition is NOT changed
        let result_content = fs::read_to_string(file_path).unwrap();
        assert_eq!(result_content, "fn old_name() {\n    new_name();\n    new_name();\n}\n");
    }

    #[test]
    fn test_apply_replacements_with_multibyte_utf8() {
        use tempfile::NamedTempFile;

        let temp_file = NamedTempFile::new().unwrap();
        let file_path = temp_file.path();

        // Content with multibyte UTF-8 comments
        let initial_content = "fn foo() { // 世界\n    foo(); // 世界\n}".as_bytes();
        fs::write(file_path, initial_content).unwrap();

        let references = vec![create_test_reference(file_path.to_str().unwrap(), 3, 6)];

        let count = apply_replacements_in_file(file_path, "foo", "bar", &references).unwrap();

        assert_eq!(count, 1);

        // Verify multibyte characters are preserved
        let result_content = fs::read_to_string(file_path).unwrap();
        assert!(result_content.contains("世界"));
        assert!(result_content.contains("bar()"));
    }

    #[test]
    fn test_multiple_replacements_same_file() {
        use tempfile::NamedTempFile;

        let temp_file = NamedTempFile::new().unwrap();
        let file_path = temp_file.path();

        // Multiple occurrences of the same symbol
        let initial_content = b"a = foo + foo * foo;";
        fs::write(file_path, initial_content).unwrap();

        // "foo" appears at byte 4, 10, 16 (each is 3 bytes long)
        // References must be in descending order
        let references = vec![
            create_test_reference(file_path.to_str().unwrap(), 16, 19),
            create_test_reference(file_path.to_str().unwrap(), 10, 13),
            create_test_reference(file_path.to_str().unwrap(), 4, 7),
        ];

        let count = apply_replacements_in_file(file_path, "foo", "bar", &references).unwrap();

        assert_eq!(count, 3);

        let result_content = fs::read_to_string(file_path).unwrap();
        assert_eq!(result_content, "a = bar + bar * bar;");
    }

    #[test]
    fn test_apply_replacements_empty_list() {
        use tempfile::NamedTempFile;

        let temp_file = NamedTempFile::new().unwrap();
        let file_path = temp_file.path();

        let initial_content = b"fn foo() {}";
        fs::write(file_path, initial_content).unwrap();

        let references: Vec<ReferenceFact> = vec![];

        let count = apply_replacements_in_file(file_path, "foo", "bar", &references).unwrap();

        assert_eq!(count, 0);

        // Content should be unchanged
        let result_content = fs::read_to_string(file_path).unwrap();
        assert_eq!(result_content, "fn foo() {}");
    }
}
