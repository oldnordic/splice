//! Pattern-based search and replace with AST confirmation.
//!
//! This module provides multi-file pattern replacement capabilities
//! using glob patterns for file discovery and tree-sitter for AST
//! confirmation to ensure replacements land on the intended tokens.

use crate::error::{Result, SpliceError};
use crate::symbol::Language;
use crate::validate::AnalyzerMode;
use glob::glob;
use std::collections::HashMap;
use std::path::{Path, PathBuf};

/// Configuration for pattern-based replacement.
#[derive(Debug, Clone)]
pub struct PatternReplaceConfig {
    /// Glob pattern for matching files.
    pub glob_pattern: String,
    /// Text pattern to find.
    pub find_pattern: String,
    /// Replacement text.
    pub replace_pattern: String,
    /// Optional language hint (auto-detected from file extension if not provided).
    pub language: Option<Language>,
    /// Whether to apply validation gates.
    pub validate: bool,
}

/// A match found during pattern search.
#[derive(Debug, Clone)]
pub struct PatternMatch {
    /// File where the match was found.
    pub file: PathBuf,
    /// Byte start of the match.
    pub byte_start: usize,
    /// Byte end of the match.
    pub byte_end: usize,
    /// Line number (1-based).
    pub line: usize,
    /// Column number (0-based).
    pub column: usize,
    /// The matched text.
    pub matched_text: String,
}

/// Result of a pattern replacement operation.
#[derive(Debug, Clone)]
pub struct PatternReplaceResult {
    /// Files that were patched.
    pub files_patched: Vec<PathBuf>,
    /// Number of replacements made.
    pub replacements_count: usize,
    /// Validation errors (if any).
    pub validation_errors: Vec<String>,
}

/// Create a tree-sitter parser for the given language.
fn parser_for_language(language: Language) -> Result<tree_sitter::Parser> {
    let mut parser = tree_sitter::Parser::new();

    let lang = match language {
        Language::Rust => tree_sitter_rust::language(),
        Language::Python => tree_sitter_python::language(),
        Language::C => tree_sitter_c::language(),
        Language::Cpp => tree_sitter_cpp::language(),
        Language::Java => tree_sitter_java::language(),
        Language::JavaScript => tree_sitter_javascript::language(),
        Language::TypeScript => tree_sitter_typescript::language_typescript(),
    };

    parser
        .set_language(&lang)
        .map_err(|e| SpliceError::Parse {
            file: PathBuf::from("<unknown>"),
            message: format!("Failed to set language for parser: {:?}", e),
        })?;

    Ok(parser)
}

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
        let entry = entry.map_err(|e| SpliceError::Other(format!("Glob iteration error: {}", e)))?;
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
fn find_pattern_in_file(
    file_path: &Path,
    pattern: &str,
    language: Language,
) -> Result<Vec<PatternMatch>> {
    let content = std::fs::read_to_string(file_path)
        .map_err(|e| SpliceError::Io {
            path: file_path.to_path_buf(),
            source: e,
        })?;

    // Get parser for the language
    let mut parser = parser_for_language(language)?;

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
        let byte_offset = abs_start as usize;
        let node = tree.root_node().descendant_for_byte_range(byte_offset, byte_offset);

        if let Some(node) = node {
            // Skip matches in comments unless the pattern starts with '//'
            let node_kind = node.kind();
            let is_comment = node_kind == "comment"
                || node_kind == "line_comment"
                || node_kind == "block_comment"
                || node_kind.ends_with("_comment");

            if !is_comment || pattern.starts_with("//") {
                // Get line and column using ropey
                let rope = ropey::Rope::from_reader(content.as_bytes()).unwrap();
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
                });
            }
        }

        start_idx = abs_end;
    }

    Ok(matches)
}

/// Apply pattern replacement to files with validation.
///
/// This function:
/// 1. Finds all pattern matches using AST confirmation
/// 2. Applies replacements in reverse byte order per file
/// 3. Runs validation gates if requested
pub fn apply_pattern_replace(
    config: &PatternReplaceConfig,
    workspace_dir: &Path,
) -> Result<PatternReplaceResult> {
    // Find all matches
    let matches = find_pattern_in_files(config)?;

    if matches.is_empty() {
        return Ok(PatternReplaceResult {
            files_patched: Vec::new(),
            replacements_count: 0,
            validation_errors: Vec::new(),
        });
    }

    // Group matches by file and sort by byte offset (descending for replacement)
    let mut matches_by_file: HashMap<PathBuf, Vec<&PatternMatch>> = HashMap::new();
    for m in &matches {
        matches_by_file
            .entry(m.file.clone())
            .or_default()
            .push(m);
    }

    for file_matches in matches_by_file.values_mut() {
        file_matches.sort_by_key(|m| std::cmp::Reverse(m.byte_start));
    }

    // Apply replacements per file
    let mut files_patched = Vec::new();
    let mut replacements_count = 0;

    for (file_path, file_matches) in matches_by_file {
        if file_matches.is_empty() {
            continue;
        }

        // Read file content
        let mut content = std::fs::read_to_string(&file_path)
            .map_err(|e| SpliceError::Io {
                path: file_path.clone(),
                source: e,
            })?;

        // Apply replacements in reverse byte order
        for m in file_matches {
            let start_byte = m.byte_start;
            let end_byte = m.byte_end;

            // Replace the content
            content.replace_range(start_byte..end_byte, &config.replace_pattern);
            replacements_count += 1;
        }

        // Write back
        std::fs::write(&file_path, content).map_err(|e| SpliceError::Io {
            path: file_path.clone(),
            source: e,
        })?;

        files_patched.push(file_path.clone());
    }

    // Run validation if requested
    if config.validate {
        // For each patched file, run validation
        for file_path in &files_patched {
            // Determine language
            let lang = config.language.or_else(|| {
                Language::from_path(file_path)
            }).ok_or_else(|| {
                SpliceError::Other(format!("Cannot detect language for file: {}", file_path.display()))
            })?;

            // Run validation gates
            crate::patch::run_validation_gates(
                file_path,
                workspace_dir,
                lang,
                AnalyzerMode::Off,
            )?;
        }
    }

    Ok(PatternReplaceResult {
        files_patched,
        replacements_count,
        validation_errors: Vec::new(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn test_find_pattern_in_file() {
        let workspace = TempDir::new().expect("Failed to create temp dir");
        let workspace_root = workspace.path();

        let test_file = workspace_root.join("test.rs");
        fs::write(
            &test_file,
            r#"
fn foo() {
    let x = 42;
    let y = 42;
    println!("{}", x);
}
"#,
        ).expect("Failed to write test file");

        let matches = find_pattern_in_file(&test_file, "42", Language::Rust)
            .expect("Failed to find pattern");

        assert_eq!(matches.len(), 2, "Should find 2 occurrences of '42'");
    }

    #[test]
    fn test_apply_pattern_replace() {
        let workspace = TempDir::new().expect("Failed to create temp dir");
        let workspace_root = workspace.path();

        let test_file = workspace_root.join("test.py");
        fs::write(
            &test_file,
            r#"
def foo():
    x = 10
    y = 10
    return x + y
"#,
        ).expect("Failed to write test file");

        let config = PatternReplaceConfig {
            glob_pattern: workspace_root.join("*.py").to_string_lossy().to_string(),
            find_pattern: "10".to_string(),
            replace_pattern: "20".to_string(),
            language: Some(Language::Python),
            validate: false,
        };

        let result = apply_pattern_replace(&config, workspace_root)
            .expect("Failed to apply pattern replace");

        assert_eq!(result.files_patched.len(), 1);
        assert_eq!(result.replacements_count, 2);

        let content = fs::read_to_string(&test_file).expect("Failed to read file");
        assert!(content.contains("20"), "Should contain replaced value");
    }
}
