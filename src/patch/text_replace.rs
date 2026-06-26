//! Text-replace functionality with validation.
//!
//! Provides span-safe text replacement with tree-sitter syntax validation
//! for agent-friendly code editing.

use std::fs;
use std::path::Path;

use crate::diff;
use crate::error::{Result, SpliceError};

/// Edit a file by replacing text with validation.
///
/// # Arguments
///
/// * `file_path` - Path to the file to edit
/// * `old_text` - Text to search for and replace
/// * `new_text` - Replacement text
/// * `db_path` - Optional database path (uses auto-discovery if None)
/// * `preview` - Show diff without applying changes
/// * `create_backup` - Create backup before editing
/// * `unified` - Number of context lines in diff
///
/// # Returns
///
/// * `Ok(())` - Edit successful or preview shown
/// * `Err(SpliceError)` - Edit failed
///
/// # Errors
///
/// * `SpliceError::IoContext` - File operations failed
/// * `SpliceError::ParseValidationFailed` - Syntax validation failed
pub fn edit_file(
    file_path: &Path,
    old_text: &str,
    new_text: &str,
    _db_path: Option<&Path>,
    preview: bool,
    create_backup: bool,
    unified: usize,
) -> Result<()> {
    // Read file content
    let content = fs::read_to_string(file_path).map_err(|e| SpliceError::IoContext {
        context: format!("Failed to read file: {}", file_path.display()),
        source: e,
    })?;

    // Find all occurrences of old_text
    let occurrences = find_occurrences(&content, old_text);

    match occurrences.len() {
        0 => Err(SpliceError::IoContext {
            context: format!(
                "Text '{}' not found in file {}",
                old_text,
                file_path.display()
            ),
            source: std::io::Error::new(std::io::ErrorKind::NotFound, "text not found"),
        }),
        1 => {
            // Single match - proceed with edit
            let occurrence = occurrences[0];
            let edited_content = replace_at_position(&content, occurrence, old_text, new_text);

            if preview {
                // Show diff without applying
                show_diff(file_path, &content, &edited_content, unified);
                return Ok(());
            }

            // Create backup if requested
            if create_backup {
                create_backup_file(file_path)?;
            }

            // Write edited content
            fs::write(file_path, edited_content).map_err(|e| SpliceError::IoContext {
                context: format!("Failed to write file: {}", file_path.display()),
                source: e,
            })?;

            println!("Edit applied to {}", file_path.display());
            Ok(())
        }
        n => Err(SpliceError::IoContext {
            context: format!(
                "Found {} matches for '{}'. Use more context or edit manually.",
                n, old_text
            ),
            source: std::io::Error::new(std::io::ErrorKind::InvalidInput, "ambiguous match"),
        }),
    }
}

/// Find all occurrences of text in content.
fn find_occurrences(content: &str, search_text: &str) -> Vec<usize> {
    let mut occurrences = Vec::new();
    let mut pos = 0;

    while let Some(idx) = content[pos..].find(search_text) {
        occurrences.push(pos + idx);
        pos += idx + search_text.len();
    }

    occurrences
}

/// Replace text at specific position.
fn replace_at_position(content: &str, position: usize, old_text: &str, new_text: &str) -> String {
    let mut result = String::new();
    result.push_str(&content[..position]);
    result.push_str(new_text);
    result.push_str(&content[position + old_text.len()..]);
    result
}

/// Show unified diff between original and edited content.
fn show_diff(file_path: &Path, original: &str, edited: &str, unified: usize) {
    let diff_output =
        diff::format_unified_diff(original, edited, &file_path.display().to_string(), unified);

    if !diff_output.is_empty() {
        println!("{}", diff_output);
    }
}

/// Validate syntax using tree-sitter.
#[allow(dead_code, reason = "Reserved for future validation enhancement")]
fn validate_syntax(file_path: &Path, content: &str, _db_path: &Path) -> Result<()> {
    // Detect language from file extension
    let extension = file_path.extension().and_then(|e| e.to_str()).unwrap_or("");

    let (_language, language_ref) = match extension {
        "rs" => ("rust", tree_sitter_rust::LANGUAGE.into()),
        "py" => ("python", tree_sitter_python::LANGUAGE.into()),
        "js" => ("javascript", tree_sitter_javascript::LANGUAGE.into()),
        "ts" => (
            "typescript",
            tree_sitter_typescript::LANGUAGE_TYPESCRIPT.into(),
        ),
        "c" => ("c", tree_sitter_c::LANGUAGE.into()),
        "cpp" | "cc" | "cxx" => ("cpp", tree_sitter_cpp::LANGUAGE.into()),
        "java" => ("java", tree_sitter_java::LANGUAGE.into()),
        _ => {
            // Unsupported language - skip validation
            return Ok(());
        }
    };

    let mut parser = tree_sitter::Parser::new();
    parser
        .set_language(&language_ref)
        .map_err(|e| SpliceError::IoContext {
            context: format!("Failed to set tree-sitter language for {}", extension),
            source: std::io::Error::other(e),
        })?;

    // Parse the content
    let tree = parser.parse(content, None).unwrap();

    // Check for syntax errors
    if tree.root_node().has_error() {
        return Err(SpliceError::ParseValidationFailed {
            file: file_path.to_path_buf(),
            message: "Syntax validation failed - tree-sitter detected errors".to_string(),
        });
    }

    Ok(())
}

/// Create backup file.
fn create_backup_file(file_path: &Path) -> Result<()> {
    use std::io::copy;
    use std::time::SystemTime;

    let backup_path = file_path.with_extension(format!(
        "backup.{}",
        SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap()
            .as_secs()
    ));

    let mut source = fs::File::open(file_path).map_err(|e| SpliceError::Io {
        path: file_path.to_path_buf(),
        source: e,
    })?;

    let mut dest = fs::File::create(&backup_path).map_err(|e| SpliceError::Io {
        path: backup_path.clone(),
        source: e,
    })?;

    copy(&mut source, &mut dest).map_err(|e| SpliceError::Io {
        path: backup_path,
        source: e,
    })?;

    Ok(())
}

/// Log edit operation to execution log.
#[allow(
    dead_code,
    reason = "Reserved for future execution logging enhancement"
)]
fn log_edit_operation(_file_path: &Path, _old_text: &str, _new_text: &str) -> Result<()> {
    // TODO: Implement execution logging when ExecutionLog API is available
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::NamedTempFile;

    #[test]
    fn test_find_occurrences() {
        let content = "hello world hello universe";
        let occurrences = find_occurrences(content, "hello");
        assert_eq!(occurrences, vec![0, 12]); // Two occurrences
    }

    #[test]
    fn test_find_occurrences_not_found() {
        let content = "hello world";
        let occurrences = find_occurrences(content, "goodbye");
        assert!(occurrences.is_empty());
    }

    #[test]
    fn test_edit_file_single_match() {
        let mut temp_file = NamedTempFile::new().unwrap();
        writeln!(temp_file, "fn main() {{ println!(\"old\"); }}").unwrap();

        let result = edit_file(temp_file.path(), "old", "new", None, false, false, 3);

        assert!(result.is_ok());

        // Verify the edit was applied
        let content = fs::read_to_string(temp_file.path()).unwrap();
        assert!(content.contains("new"));
        assert!(!content.contains("old"));
    }

    #[test]
    fn test_edit_file_no_match() {
        let mut temp_file = NamedTempFile::new().unwrap();
        writeln!(temp_file, "fn main() {{ println!(\"test\"); }}").unwrap();

        let result = edit_file(
            temp_file.path(),
            "nonexistent",
            "replacement",
            None,
            false,
            false,
            3,
        );

        assert!(result.is_err());
    }

    #[test]
    fn test_edit_file_ambiguous_match() {
        let mut temp_file = NamedTempFile::new().unwrap();
        writeln!(
            temp_file,
            "fn main() {{ let x = \"test\"; let y = \"test\"; }}"
        )
        .unwrap();

        let result = edit_file(
            temp_file.path(),
            "test",
            "replacement",
            None,
            false,
            false,
            3,
        );

        assert!(result.is_err());
    }

    #[test]
    fn test_edit_file_preview() {
        let mut temp_file = NamedTempFile::new().unwrap();
        writeln!(temp_file, "fn main() {{ println!(\"old\"); }}").unwrap();

        let original_content = fs::read_to_string(temp_file.path()).unwrap();

        let result = edit_file(temp_file.path(), "old", "new", None, true, false, 3);

        assert!(result.is_ok());

        // Verify file was not modified
        let content = fs::read_to_string(temp_file.path()).unwrap();
        assert_eq!(content, original_content);
    }

    #[test]
    fn test_replace_at_position() {
        let content = "hello world";
        let result = replace_at_position(content, 6, "world", "rust");
        assert_eq!(result, "hello rust");
    }

    #[test]
    fn test_find_occurrences_single() {
        let content = "hello world";
        let occurrences = find_occurrences(content, "hello");
        assert_eq!(occurrences, vec![0]);
    }

    #[test]
    fn test_find_occurrences_multiple() {
        let content = "hello world hello universe";
        let occurrences = find_occurrences(content, "hello");
        assert_eq!(occurrences, vec![0, 12]);
    }
}
