//! Token extraction for current cursor context

use std::fs;
use std::path::PathBuf;

/// Extract the token being typed at cursor position
pub fn extract_current_token(file_path: &PathBuf, line: usize, column: usize) -> Option<String> {
    // Read file content
    let content = fs::read_to_string(file_path).ok()?;

    // Get the specific line (1-based to 0-based)
    let lines: Vec<&str> = content.lines().collect();
    let line_content = lines.get(line.saturating_sub(1))?;

    // Get column position (1-based to 0-based)
    let col_index = column.saturating_sub(1);

    // Find the start of the token being typed
    let token_start = line_content[..col_index]
        .rfind(|c: char| !c.is_alphanumeric() && c != '_')
        .map(|i| i + 1)
        .unwrap_or(0);

    // Extract token
    let token = &line_content[token_start..col_index];

    if token.is_empty() {
        None
    } else {
        Some(token.to_string())
    }
}

/// Determine if cursor is in a position that accepts completions
pub fn is_completion_position(file_path: &PathBuf, line: usize, column: usize) -> bool {
    let content = match fs::read_to_string(file_path) {
        Ok(c) => c,
        Err(_) => return false,
    };

    let lines: Vec<&str> = content.lines().collect();
    let line_content = match lines.get(line.saturating_sub(1)) {
        Some(l) => l,
        None => return false,
    };

    if line_content.is_empty() {
        return true;
    }

    let col_index = column.saturating_sub(1);
    if col_index >= line_content.len() {
        return true;
    }

    let char_at_cursor = match line_content.chars().nth(col_index) {
        Some(c) => c,
        None => return true,
    };

    // Complete after: whitespace, operators, opening brackets
    matches!(
        char_at_cursor,
        ' ' | '\t'
            | '('
            | '['
            | '{'
            | '='
            | '+'
            | '-'
            | '*'
            | '/'
            | '%'
            | '&'
            | '|'
            | '^'
            | '!'
            | '<'
            | '>'
            | ':'
            | ';'
            | ','
            | '.'
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    #[test]
    fn test_extract_token() {
        let mut file = tempfile::NamedTempFile::new().unwrap();
        writeln!(file, "fn test() {{").unwrap();
        writeln!(file, "    let x = foobar").unwrap();
        writeln!(file, "}}").unwrap();

        let token = extract_current_token(&file.path().to_path_buf(), 2, 19);
        assert_eq!(token.as_deref(), Some("foobar"));
    }
}
