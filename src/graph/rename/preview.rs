use super::*;
use crate::diff::{format_colored_diff, format_unified_diff, should_use_color};

/// Generate preview diff for rename operation.
///
/// Creates a unified diff showing what changes would be made to a file
/// when renaming a symbol. This is used in preview mode to show the
/// user what will change without actually modifying files.
///
/// # Arguments
/// * `file_path` - Path to file
/// * `original_content` - Original file content
/// * `modified_content` - Content after rename
///
/// # Returns
/// Unified diff string in git-compatible format
pub fn generate_preview_diff(
    file_path: &Path,
    original_content: &str,
    modified_content: &str,
) -> String {
    format_unified_diff(
        original_content,
        modified_content,
        &file_path.display().to_string(),
        3, // context lines
    )
}

/// Generate colored preview if TTY detected.
///
/// This function automatically detects whether to use colored output
/// based on terminal capabilities and the NO_COLOR environment variable.
/// When colors are enabled, deletions are shown in red and additions
/// in green following git's convention.
///
/// # Arguments
/// * `file_path` - Path to file
/// * `original_content` - Original file content
/// * `modified_content` - Content after rename
///
/// # Returns
/// Colored diff string or plain text diff depending on terminal support
pub fn generate_colored_preview(
    file_path: &Path,
    original_content: &str,
    modified_content: &str,
) -> String {
    let use_color = should_use_color();
    if use_color {
        // Apply ANSI colors (red for deletions, green for additions)
        format_colored_diff(original_content, modified_content, use_color)
    } else {
        // Plain unified diff
        format_unified_diff(
            original_content,
            modified_content,
            &file_path.display().to_string(),
            3,
        )
    }
}

/// Simulate replacements on a file's content without modifying it.
///
/// This function applies the same replacement logic that would be used
/// during an actual rename, but returns the modified content instead of
/// writing it to disk. This is used for preview mode.
///
/// # Arguments
/// * `content` - Original file content
/// * `references` - ReferenceFact entries sorted by byte_start DESCENDING
/// * `_old_name` - Original symbol name (for validation, currently unused)
/// * `new_name` - New symbol name
///
/// # Returns
/// Modified content with all replacements applied
pub fn simulate_replacements_content(
    content: &str,
    references: &[ReferenceFact],
    _old_name: &str,
    new_name: &str,
) -> Result<String> {
    let content_bytes = content.as_bytes();
    let new_name_bytes = new_name.as_bytes();
    let mut current_content = content_bytes.to_vec();

    // Apply replacements from end to start (descending byte_start)
    for reference in references {
        match replace_at_span(&current_content, reference, new_name_bytes) {
            Ok(new_content) => {
                current_content = new_content;
            }
            Err(e) => {
                return Err(SpliceError::Other(format!(
                    "Failed to simulate replacement at {}..{}: {}",
                    reference.byte_start, reference.byte_end, e
                )));
            }
        }
    }

    // Convert back to string, validating UTF-8
    String::from_utf8(current_content).map_err(|e| SpliceError::InvalidUtf8 {
        file: PathBuf::from("<preview>"),
        source: e.utf8_error(),
    })
}
