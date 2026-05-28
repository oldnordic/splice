//! Pattern search with AST confirmation.

use super::*;
use crate::error::{Result, SpliceError};
use crate::symbol::{parser_for_language, Language};
use glob::glob;
use std::path::Path;

/// Find all occurrences of a text pattern in files matching a glob pattern.
///
/// This function performs text-based search and uses tree-sitter to confirm
/// that matches are in valid AST nodes (not in comments or strings, unless
/// the pattern specifically targets those).
pub fn find_pattern_in_files(config: &PatternReplaceConfig) -> Result<Vec<PatternMatch>> {
    let mut matches = Vec::new();

    // Use glob to find matching files
    let glob_paths = glob(&config.glob_pattern)
        .map_err(|e| SpliceError::Other(format!("Invalid glob pattern: {}", e)))?;

    for entry in glob_paths {
        let entry =
            entry.map_err(|e| SpliceError::Other(format!("Glob iteration error: {}", e)))?;
        let path = entry;

        // Skip directories
        if path.is_dir() {
            continue;
        }

        // Determine language for this file
        let language = if let Some(lang) = config.language {
            lang
        } else {
            Language::from_path(&path).ok_or_else(|| {
                SpliceError::Other(format!(
                    "Cannot detect language for file: {}",
                    path.display()
                ))
            })?
        };

        // Find matches in this file
        let file_matches = find_pattern_in_file(&path, &config.find_pattern, language)?;
        matches.extend(file_matches);
    }

    Ok(matches)
}

/// Find all occurrences of a text pattern in a single file with AST confirmation.
///
/// Uses tree-sitter to ensure matches are in valid code locations.
pub(crate) fn find_pattern_in_file(
    file_path: &Path,
    pattern: &str,
    language: Language,
) -> Result<Vec<PatternMatch>> {
    let content = std::fs::read_to_string(file_path).map_err(|e| SpliceError::Io {
        path: file_path.to_path_buf(),
        source: e,
    })?;

    // Get parser for the language
    let mut parser = parser_for_language(file_path, language)?;

    let tree = parser
        .parse(&content, None)
        .ok_or_else(|| SpliceError::Other("Failed to parse file".to_string()))?;

    let mut matches = Vec::new();

    // Find all text occurrences of the pattern
    let mut start_idx = 0;
    while let Some(idx) = content[start_idx..].find(pattern) {
        let abs_start = start_idx + idx;
        let abs_end = abs_start + pattern.len();

        // Check if this location is in a valid AST node
        let byte_offset = abs_start;
        let node = tree
            .root_node()
            .descendant_for_byte_range(byte_offset, byte_offset);

        if let Some(node) = node {
            // Skip matches in comments unless the pattern starts with '//'
            let node_kind = node.kind();
            let is_comment = node_kind == "comment"
                || node_kind == "line_comment"
                || node_kind == "block_comment"
                || node_kind.ends_with("_comment");

            if !is_comment || pattern.starts_with("//") {
                // Get line and column using ropey
                let rope = ropey::Rope::from_reader(content.as_bytes()).map_err(|e| {
                    crate::SpliceError::Other(format!("Failed to create rope: {}", e))
                })?;
                let line = rope.byte_to_line(abs_start) + 1;
                let line_start_byte = rope.line_to_byte(line - 1);
                let column = abs_start - line_start_byte;

                matches.push(PatternMatch {
                    file: file_path.to_path_buf(),
                    byte_start: abs_start,
                    byte_end: abs_end,
                    line,
                    column,
                    matched_text: pattern.to_string(),
                    context_before: None,
                    context_after: None,
                });
            }
        }

        start_idx = abs_end;
    }

    Ok(matches)
}
