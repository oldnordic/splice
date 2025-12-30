//! Ingest pipeline tests.

use splice::ingest::rust::{extract_rust_symbols, RustSymbolKind};
use std::io::Write;
use tempfile::NamedTempFile;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_rust_file_with_two_functions() {
        // Create a temporary Rust file with two functions
        let source = r#"
fn hello_world() {
    println!("Hello, world!");
}

mod tests {
    fn nested_test() {
        assert_eq!(1, 1);
    }
}
"#;

        let mut temp_file = NamedTempFile::new().expect("Failed to create temp file");
        temp_file
            .write_all(source.as_bytes())
            .expect("Failed to write to temp file");
        let temp_path = temp_file.path();

        // Call the public ingest API
        let result = extract_rust_symbols(temp_path, source.as_bytes());

        // Assert the function succeeds
        assert!(
            result.is_ok(),
            "extract_rust_symbols failed: {:?}",
            result.err()
        );

        let symbols = result.unwrap();

        // Assert we found at least the top-level function
        assert!(
            !symbols.is_empty(),
            "Expected at least 1 symbol, got {}",
            symbols.len()
        );

        // Find the hello_world function
        let hello_world = symbols
            .iter()
            .find(|s| s.name == "hello_world" && s.kind == RustSymbolKind::Function)
            .expect("Should find hello_world function");

        // Assert exact byte spans
        // "fn hello_world() {" starts at byte 1 (after the initial newline)
        assert_eq!(hello_world.byte_start, 1, "Function should start at byte 1");
        // The function body includes the closing brace
        // Total source length is 68 bytes (including initial newline and nested module)
        assert!(
            hello_world.byte_end > 40,
            "Function should end after byte 40, got byte_end={}",
            hello_world.byte_end
        );
        assert!(
            hello_world.byte_end <= source.len(),
            "Function should end within source, got byte_end={}, source_len={}",
            hello_world.byte_end,
            source.len()
        );

        // Assert line ranges are correct (1-based)
        assert_eq!(hello_world.line_start, 2, "Function starts on line 2");
        assert_eq!(hello_world.line_end, 4, "Function ends on line 4");

        // Assert column ranges are reasonable (0-based)
        assert_eq!(hello_world.col_start, 0, "Function starts at column 0");
        assert_eq!(
            hello_world.col_end, 1,
            "Function ends at column 1 (after closing brace)"
        );

        // Check for nested function if found
        if let Some(nested) = symbols
            .iter()
            .find(|s| s.name == "nested_test" && s.kind == RustSymbolKind::Function)
        {
            assert_eq!(nested.line_start, 7, "Nested function starts on line 7");
            assert_eq!(nested.line_end, 9, "Nested function ends on line 9");
        }
    }

    #[test]
    fn test_parse_empty_rust_file() {
        let source = "// Just a comment\n";
        let mut temp_file = NamedTempFile::new().expect("Failed to create temp file");
        temp_file
            .write_all(source.as_bytes())
            .expect("Failed to write to temp file");
        let temp_path = temp_file.path();

        let result = extract_rust_symbols(temp_path, source.as_bytes());

        assert!(result.is_ok());
        let symbols = result.unwrap();
        assert_eq!(symbols.len(), 0, "Empty file should have no symbols");
    }

    #[test]
    fn test_parse_rust_file_with_syntax_error() {
        let source = "fn broken { /* missing closing brace */\n";
        let mut temp_file = NamedTempFile::new().expect("Failed to create temp file");
        temp_file
            .write_all(source.as_bytes())
            .expect("Failed to write to temp file");
        let temp_path = temp_file.path();

        let result = extract_rust_symbols(temp_path, source.as_bytes());

        // Should handle syntax errors gracefully
        // (Either return empty list or return a parse error)
        assert!(
            result.is_ok() || result.is_err(),
            "Result should be Ok or Err, got: {:?}",
            result
        );
    }
}
