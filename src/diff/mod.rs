//! Unified diff generation with color support.
//!
//! This module provides utilities for generating standard unified diff format
//! output, detecting when to use colors (respecting NO_COLOR and TTY detection),
//! and applying colors to deletions/additions following git's convention
//! (red for removed, green for added).

use similar::{ChangeTag, TextDiff};
use std::io::IsTerminal;

/// Determines whether colored output should be used.
///
/// This function checks the NO_COLOR environment variable first (accessibility standard),
/// then falls back to TTY detection via std::io::stdout().is_terminal().
///
/// # Returns
/// * `true` - if colors should be used (TTY detected and NO_COLOR not set)
/// * `false` - if NO_COLOR is set or not writing to a TTY
///
/// # Examples
/// ```ignore
/// // When NO_COLOR is set
/// std::env::set_var("NO_COLOR", "1");
/// assert!(!should_use_color());
///
/// // When writing to a TTY and NO_COLOR is not set
/// std::env::remove_var("NO_COLOR");
/// // (result depends on whether stdout is a TTY)
/// let use_color = should_use_color();
/// ```
pub fn should_use_color() -> bool {
    // Check NO_COLOR environment variable first (accessibility standard)
    // See: https://no-color.org/
    match std::env::var("NO_COLOR") {
        Ok(_) => false, // NO_COLOR is set, disable colors
        Err(_) => std::io::stdout().is_terminal(),
    }
}

/// Generates a unified diff between two strings.
///
/// This function uses the similar crate to generate a standard unified diff format
/// compatible with the `patch` command. The diff includes headers with file paths
/// and context lines to show surrounding code.
///
/// # Arguments
/// * `old` - The original content before changes
/// * `new` - The new content after changes
/// * `path` - The file path to display in the diff header
/// * `context_lines` - Number of context lines to include (default: 3 for git compatibility)
///
/// # Returns
/// A String containing the unified diff in standard format. Returns an empty string
/// if old == new (no changes).
///
/// # Examples
/// ```
/// use splice::format_unified_diff;
///
/// let old = "fn hello() {\n    println!(\"Hello\");\n}\n";
/// let new = "fn hello() {\n    println!(\"Hello, World!\");\n}\n";
/// let diff = format_unified_diff(old, new, "src/main.rs", 3);
/// assert!(diff.contains("--- a/src/main.rs"));
/// assert!(diff.contains("+++ b/src/main.rs"));
/// ```
pub fn format_unified_diff(old: &str, new: &str, path: &str, context_lines: usize) -> String {
    // If old and new are identical, return empty string
    if old == new {
        return String::new();
    }

    let diff = TextDiff::from_lines(old, new);
    let header_old = format!("a/{}", path);
    let header_new = format!("b/{}", path);

    diff.unified_diff()
        .context_radius(context_lines)
        .header(&header_old, &header_new)
        .to_string()
}

/// Formats a colored diff between two strings.
///
/// This function applies ANSI colors to diff output following git's convention:
/// - Red for deletions (lines starting with `-`)
/// - Green for additions (lines starting with `+`)
/// - No color for context lines (lines starting with ` `)
///
/// # Arguments
/// * `old` - The original content before changes
/// * `new` - The new content after changes
/// * `use_color` - Whether to apply ANSI colors (if false, returns plain text)
///
/// # Returns
/// A String with colored diff output or plain text depending on use_color parameter.
///
/// # Examples
/// ```
/// use splice::format_colored_diff;
///
/// let old = "line 1\nline 2\nline 3\n";
/// let new = "line 1\nline 2 modified\nline 3\n";
/// let colored = format_colored_diff(old, new, false); // Plain text
/// let colored_with_ansi = format_colored_diff(old, new, true); // With ANSI codes
/// ```
pub fn format_colored_diff(old: &str, new: &str, use_color: bool) -> String {
    let diff = TextDiff::from_lines(old, new);
    let mut result = String::new();

    for change in diff.iter_all_changes() {
        let tag = change.tag();
        let line = change.value();

        let formatted = if use_color {
            match tag {
                ChangeTag::Delete => {
                    // Red for deletions (following git convention)
                    format!("{}{}", nu_ansi_term::Color::Red.paint("-"), line)
                }
                ChangeTag::Insert => {
                    // Green for additions (following git convention)
                    format!("{}{}", nu_ansi_term::Color::Green.paint("+"), line)
                }
                ChangeTag::Equal => {
                    // No color for context lines
                    format!(" {}", line)
                }
            }
        } else {
            // Plain text output
            match tag {
                ChangeTag::Delete => format!("-{}", line),
                ChangeTag::Insert => format!("+{}", line),
                ChangeTag::Equal => format!(" {}", line),
            }
        };

        result.push_str(&formatted);
    }

    result
}

/// Generates a git-style summary header for a diff.
///
/// This function creates a summary header following git's diffstat format exactly:
/// - Shows file count with proper singular/plural ("1 file changed" vs "N files changed")
/// - Shows insertions with proper singular/plural ("1 insertion(+)" vs "N insertions(+)")
/// - Shows deletions with proper singular/plural ("1 deletion(-)" vs "N deletions(-)")
///
/// # Arguments
/// * `files` - Number of files changed
/// * `insertions` - Number of lines added
/// * `deletions` - Number of lines removed
///
/// # Returns
/// A String containing the formatted summary header, or empty string if files == 0.
///
/// # Examples
/// ```
/// use splice::format_diff_summary;
///
/// let summary = format_diff_summary(1, 5, 2);
/// // Output: " 1 file changed, 5 insertions(+), 2 deletions(-)"
///
/// let summary = format_diff_summary(2, 10, 3);
/// // Output: " 2 files changed, 10 insertions(+), 3 deletions(-)"
///
/// let summary = format_diff_summary(1, 1, 0);
/// // Output: " 1 file changed, 1 insertion(+)"
///
/// let summary = format_diff_summary(1, 0, 1);
/// // Output: " 1 file changed, 1 deletion(-)"
/// ```
pub fn format_diff_summary(files: usize, insertions: usize, deletions: usize) -> String {
    // Return empty string if no files changed
    if files == 0 {
        return String::new();
    }

    let mut parts = Vec::new();

    // File count with singular/plural
    if files == 1 {
        parts.push("1 file changed".to_string());
    } else {
        parts.push(format!("{} files changed", files));
    }

    // Insertions with singular/plural
    if insertions > 0 {
        if insertions == 1 {
            parts.push("1 insertion(+)".to_string());
        } else {
            parts.push(format!("{} insertions(+)", insertions));
        }
    }

    // Deletions with singular/plural
    if deletions > 0 {
        if deletions == 1 {
            parts.push("1 deletion(-)".to_string());
        } else {
            parts.push(format!("{} deletions(-)", deletions));
        }
    }

    // Edge case: no actual changes (0 insertions, 0 deletions)
    if insertions == 0 && deletions == 0 {
        parts.push("0 insertions(+)".to_string());
        parts.push("0 deletions(-)".to_string());
    }

    format!(" {}", parts.join(", "))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::env;

    #[test]
    fn test_should_use_color_with_no_color_set() {
        // Set NO_COLOR environment variable
        env::set_var("NO_COLOR", "1");
        let result = should_use_color();
        env::remove_var("NO_COLOR");

        assert!(!result, "should_use_color should return false when NO_COLOR is set");
    }

    #[test]
    fn test_should_use_color_without_no_color() {
        // Ensure NO_COLOR is not set
        env::remove_var("NO_COLOR");

        // Result depends on whether stdout is a TTY
        // We can't test TTY detection in unit tests, but we can verify it doesn't panic
        let result = should_use_color();

        // In CI/non-TTY environments, this should be false
        // In a terminal, it would be true
        // Just verify we get a boolean without panicking
        let _ = result;
    }

    #[test]
    fn test_format_unified_diff_basic() {
        let old = "line 1\nline 2\nline 3\n";
        let new = "line 1\nline 2 modified\nline 3\n";

        let diff = format_unified_diff(old, new, "test.txt", 3);

        assert!(diff.contains("--- a/test.txt"));
        assert!(diff.contains("+++ b/test.txt"));
        assert!(diff.contains("-line 2"));
        assert!(diff.contains("+line 2 modified"));
    }

    #[test]
    fn test_format_unified_diff_no_changes() {
        let old = "line 1\nline 2\nline 3\n";
        let new = "line 1\nline 2\nline 3\n";

        let diff = format_unified_diff(old, new, "test.txt", 3);

        assert!(diff.is_empty(), "Diff should be empty when old == new");
    }

    #[test]
    fn test_format_unified_diff_context_lines() {
        let old = "line 1\nline 2\nline 3\nline 4\nline 5\n";
        let new = "line 1\nline 2\nline 3 modified\nline 4\nline 5\n";

        let diff = format_unified_diff(old, new, "test.txt", 2);

        // Should include context lines
        assert!(diff.contains("line 1")); // Context before
        assert!(diff.contains("line 4")); // Context after
        assert!(diff.contains("-line 3"));
        assert!(diff.contains("+line 3 modified"));
    }

    #[test]
    fn test_format_colored_diff_plain_text() {
        let old = "line 1\nline 2\nline 3\n";
        let new = "line 1\nline 2 modified\nline 3\n";

        let diff = format_colored_diff(old, new, false);

        // Should NOT contain ANSI codes when use_color = false
        assert!(!diff.contains("\x1b[")); // ANSI escape sequence
        assert!(diff.contains("-line 2"));
        assert!(diff.contains("+line 2 modified"));
        assert!(diff.contains(" line 1")); // Context line has space prefix
    }

    #[test]
    fn test_format_colored_diff_with_color() {
        let old = "line 1\nline 2\nline 3\n";
        let new = "line 1\nline 2 modified\nline 3\n";

        let diff = format_colored_diff(old, new, true);

        // Should contain ANSI codes when use_color = true
        // Note: The actual ANSI codes depend on nu_ansi_term implementation
        // We just verify the diff is non-empty and contains our content
        assert!(!diff.is_empty());
        assert!(diff.contains("line 2 modified"));
    }

    #[test]
    fn test_format_colored_diff_tags() {
        let old = "keep\nremove\nkeep2\n";
        let new = "keep\nadd\nkeep2\n";

        // Test without color first
        let diff_plain = format_colored_diff(old, new, false);

        // Check that tags are correct
        assert!(diff_plain.contains(" keep\n")); // Context line
        assert!(diff_plain.contains("-remove\n")); // Deletion
        assert!(diff_plain.contains("+add\n")); // Addition
        assert!(diff_plain.contains(" keep2\n")); // Context line

        // Test with color
        let diff_colored = format_colored_diff(old, new, true);
        assert!(!diff_colored.is_empty());
    }

    #[test]
    fn test_format_unified_diff_with_rust_code() {
        let old = "fn hello() {\n    println!(\"Hello\");\n}\n";
        let new = "fn hello() {\n    println!(\"Hello, World!\");\n}\n";

        let diff = format_unified_diff(old, new, "src/main.rs", 3);

        assert!(diff.contains("--- a/src/main.rs"));
        assert!(diff.contains("+++ b/src/main.rs"));
        assert!(diff.contains("-    println!(\"Hello\");"));
        assert!(diff.contains("+    println!(\"Hello, World!\");"));
    }

    #[test]
    fn test_format_diff_summary_single_file_with_insertions_and_deletions() {
        let summary = format_diff_summary(1, 5, 2);
        assert_eq!(summary, " 1 file changed, 5 insertions(+), 2 deletions(-)");
    }

    #[test]
    fn test_format_diff_summary_multiple_files() {
        let summary = format_diff_summary(2, 10, 3);
        assert_eq!(summary, " 2 files changed, 10 insertions(+), 3 deletions(-)");
    }

    #[test]
    fn test_format_diff_summary_only_insertions() {
        let summary = format_diff_summary(1, 3, 0);
        assert_eq!(summary, " 1 file changed, 3 insertions(+)");
    }

    #[test]
    fn test_format_diff_summary_only_deletions() {
        let summary = format_diff_summary(1, 0, 3);
        assert_eq!(summary, " 1 file changed, 3 deletions(-)");
    }

    #[test]
    fn test_format_diff_summary_singular_insertion() {
        let summary = format_diff_summary(1, 1, 0);
        assert_eq!(summary, " 1 file changed, 1 insertion(+)");
    }

    #[test]
    fn test_format_diff_summary_singular_deletion() {
        let summary = format_diff_summary(1, 0, 1);
        assert_eq!(summary, " 1 file changed, 1 deletion(-)");
    }

    #[test]
    fn test_format_diff_summary_no_changes() {
        let summary = format_diff_summary(1, 0, 0);
        assert_eq!(summary, " 1 file changed, 0 insertions(+), 0 deletions(-)");
    }

    #[test]
    fn test_format_diff_summary_no_files() {
        let summary = format_diff_summary(0, 0, 0);
        assert!(summary.is_empty());
    }
}
