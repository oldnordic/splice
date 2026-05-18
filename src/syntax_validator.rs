//! Syntax validation using tree-sitter
//!
//! Provides tree-sitter parsing validation for multiple languages.

use crate::error::{Result, SpliceError};
use std::path::Path;
use tree_sitter::Parser;

/// Validate syntax by parsing with tree-sitter
///
/// # Arguments
/// * `file_path` - Path to the file (used for language detection)
/// * `source` - Source code to validate
///
/// # Returns
/// * `Ok(bool)` - true if syntax is valid, false if parse failed
/// * `Err(SpliceError)` - Validation process failed
pub fn validate_syntax(file_path: &Path, source: &[u8]) -> Result<bool> {
    // Detect language from file extension
    let language = detect_language(file_path)?;

    if language.is_none() {
        // Unknown language - skip validation (don't block)
        log::debug!(
            "Unknown language for {:?}, skipping syntax validation",
            file_path
        );
        return Ok(true);
    }

    let language = language.unwrap();

    // Create parser
    let mut parser = Parser::new();
    parser
        .set_language(&language)
        .map_err(|e| SpliceError::Parse {
            file: file_path.to_path_buf(),
            message: format!("Failed to set language: {:?}", e),
        })?;

    // Parse the source
    let tree = parser.parse(source, None);

    // Check if parse succeeded
    match tree {
        Some(tree) => {
            // Check for parse errors in the tree
            let has_errors = has_parse_errors(tree.root_node());
            if has_errors {
                log::warn!("Syntax errors detected in {:?}", file_path);
            }
            Ok(!has_errors)
        }
        None => {
            log::warn!("Parse failed for {:?}", file_path);
            Ok(false)
        }
    }
}

/// Detect tree-sitter language from file extension
fn detect_language(file_path: &Path) -> Result<Option<tree_sitter::Language>> {
    let extension = file_path
        .extension()
        .and_then(|ext| ext.to_str())
        .unwrap_or("");

    match extension {
        "rs" => Ok(Some(tree_sitter_rust::LANGUAGE.into())),
        "py" => Ok(Some(tree_sitter_python::LANGUAGE.into())),
        "c" | "h" => Ok(Some(tree_sitter_c::LANGUAGE.into())),
        "cpp" | "cc" | "cxx" | "hpp" => Ok(Some(tree_sitter_cpp::LANGUAGE.into())),
        "js" | "mjs" => Ok(Some(tree_sitter_javascript::LANGUAGE.into())),
        "ts" => Ok(Some(tree_sitter_typescript::LANGUAGE_TSX.into())),
        "tsx" => Ok(Some(tree_sitter_typescript::LANGUAGE_TSX.into())),
        "java" => Ok(Some(tree_sitter_java::LANGUAGE.into())),
        _ => Ok(None),
    }
}

/// Check if the parse tree contains ERROR nodes
fn has_parse_errors(node: tree_sitter::Node) -> bool {
    if node.kind() == "ERROR" || node.is_missing() {
        return true;
    }

    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        if has_parse_errors(child) {
            return true;
        }
    }

    false
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs::File;
    use std::io::Write;
    use tempfile::TempDir;

    #[test]
    fn test_valid_rust_syntax() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("test.rs");
        let mut file = File::create(&file_path).unwrap();
        writeln!(file, "fn main() {{ }}").unwrap();

        let source = b"fn main() {}";
        let result = validate_syntax(&file_path, source).unwrap();
        assert!(result, "Valid Rust syntax should pass validation");
    }

    #[test]
    fn test_invalid_rust_syntax() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("test.rs");

        // Missing closing brace - should fail
        let source = b"fn main() {";
        let result = validate_syntax(&file_path, source).unwrap();
        assert!(!result, "Invalid Rust syntax should fail validation");
    }

    #[test]
    fn test_unknown_language() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("test.xyz");

        let source = b"some code";
        let result = validate_syntax(&file_path, source).unwrap();
        assert!(result, "Unknown language should pass (skip validation)");
    }
}
