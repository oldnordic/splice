//! Language-aware symbol extraction dispatcher.
//!
//! This module provides a unified interface for extracting symbols from source files
//! in any supported language. It automatically detects the language from the file
//! extension and routes to the appropriate parser.

use crate::error::{Result, SpliceError};
use crate::ingest::{
    detect::detect_language,
    detect::Language as DetectLanguage,
    {extract_cpp_symbols, extract_java_symbols, extract_javascript_symbols,
     extract_python_symbols, extract_rust_symbols, extract_typescript_symbols},
};
use crate::symbol::{AnySymbol, Language};
use std::path::Path;

/// Extract symbols from a source file, auto-detecting the language from extension.
///
/// This function automatically determines the language based on the file extension
/// and uses the appropriate parser. Returns language-specific symbols wrapped in
/// the `AnySymbol` enum.
///
/// # Errors
///
/// Returns `SpliceError::Parse` if:
/// - The file extension is not recognized
/// - Parsing fails
///
/// # Example
///
/// ```no_run
/// use splice::ingest::dispatch::extract_symbols;
/// use std::path::Path;
///
/// let path = Path::new("main.rs");
/// let source = b"fn main() {}";
/// let symbols = extract_symbols(path, source)?;
/// # Ok::<(), splice::SpliceError>(())
/// ```
pub fn extract_symbols(path: &Path, source: &[u8]) -> Result<Vec<AnySymbol>> {
    // Detect language from file extension
    let detect_lang = detect_language(path).ok_or_else(|| {
        SpliceError::Parse {
            file: path.to_path_buf(),
            message: "Unknown file extension - cannot detect language".to_string(),
        }
    })?;

    // Route to appropriate parser
    match detect_lang {
        DetectLanguage::Rust => {
            let symbols = extract_rust_symbols(path, source)?;
            Ok(symbols.into_iter().map(AnySymbol::Rust).collect())
        }
        DetectLanguage::Python => {
            let symbols = extract_python_symbols(path, source)?;
            Ok(symbols.into_iter().map(AnySymbol::Python).collect())
        }
        DetectLanguage::C | DetectLanguage::Cpp => {
            let symbols = extract_cpp_symbols(path, source)?;
            Ok(symbols.into_iter().map(AnySymbol::Cpp).collect())
        }
        DetectLanguage::Java => {
            let symbols = extract_java_symbols(path, source)?;
            Ok(symbols.into_iter().map(AnySymbol::Java).collect())
        }
        DetectLanguage::JavaScript => {
            let symbols = extract_javascript_symbols(path, source)?;
            Ok(symbols.into_iter().map(AnySymbol::JavaScript).collect())
        }
        DetectLanguage::TypeScript => {
            let symbols = extract_typescript_symbols(path, source)?;
            Ok(symbols.into_iter().map(AnySymbol::TypeScript).collect())
        }
    }
}

/// Extract symbols with an explicit language override.
///
/// Like `extract_symbols`, but allows specifying the language explicitly.
/// This is useful when you want to force a specific parser regardless of
/// file extension.
///
/// # Errors
///
/// Returns `SpliceError::Parse` if parsing fails.
///
/// # Example
///
/// ```no_run
/// use splice::ingest::dispatch::extract_symbols_with_language;
/// use splice::symbol::Language;
/// use std::path::Path;
///
/// let path = Path::new("main.txt");
/// let source = b"fn main() {}";
/// let symbols = extract_symbols_with_language(path, source, Language::Rust)?;
/// # Ok::<(), splice::SpliceError>(())
/// ```
pub fn extract_symbols_with_language(
    path: &Path,
    source: &[u8],
    language: Language,
) -> Result<Vec<AnySymbol>> {
    match language {
        Language::Rust => {
            let symbols = extract_rust_symbols(path, source)?;
            Ok(symbols.into_iter().map(AnySymbol::Rust).collect())
        }
        Language::Python => {
            let symbols = extract_python_symbols(path, source)?;
            Ok(symbols.into_iter().map(AnySymbol::Python).collect())
        }
        Language::C | Language::Cpp => {
            let symbols = extract_cpp_symbols(path, source)?;
            Ok(symbols.into_iter().map(AnySymbol::Cpp).collect())
        }
        Language::Java => {
            let symbols = extract_java_symbols(path, source)?;
            Ok(symbols.into_iter().map(AnySymbol::Java).collect())
        }
        Language::JavaScript => {
            let symbols = extract_javascript_symbols(path, source)?;
            Ok(symbols.into_iter().map(AnySymbol::JavaScript).collect())
        }
        Language::TypeScript => {
            let symbols = extract_typescript_symbols(path, source)?;
            Ok(symbols.into_iter().map(AnySymbol::TypeScript).collect())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::symbol::Symbol;

    #[test]
    fn test_extract_rust_symbols() {
        let source = b"fn main() {}\nfn foo() {}\n";
        let path = Path::new("test.rs");
        let result = extract_symbols(path, source);
        assert!(result.is_ok());
        let symbols = result.unwrap();
        assert_eq!(symbols.len(), 2);
        assert_eq!(symbols[0].name(), "main");
        assert_eq!(symbols[0].kind(), "function");
        assert_eq!(symbols[0].language(), Language::Rust);
    }

    #[test]
    fn test_extract_python_symbols() {
        let source = b"def main():\n    pass\ndef foo():\n    pass\n";
        let path = Path::new("test.py");
        let result = extract_symbols(path, source);
        assert!(result.is_ok());
        let symbols = result.unwrap();
        assert_eq!(symbols.len(), 2);
        assert_eq!(symbols[0].name(), "main");
        assert_eq!(symbols[0].kind(), "function");
        assert_eq!(symbols[0].language(), Language::Python);
    }

    #[test]
    fn test_extract_cpp_symbols() {
        let source = b"int main() { return 0; }\nint foo() { return 1; }\n";
        let path = Path::new("test.cpp");
        let result = extract_symbols(path, source);
        assert!(result.is_ok());
        let symbols = result.unwrap();
        assert_eq!(symbols.len(), 2);
        assert_eq!(symbols[0].name(), "main");
        assert_eq!(symbols[0].kind(), "function");
        assert_eq!(symbols[0].language(), Language::Cpp);
    }

    #[test]
    fn test_extract_java_symbols() {
        let source = b"class Main { public static void main(String[] args) {} }\n";
        let path = Path::new("test.java");
        let result = extract_symbols(path, source);
        assert!(result.is_ok());
        let symbols = result.unwrap();
        assert!(!symbols.is_empty());
        assert_eq!(symbols[0].name(), "Main");
        assert_eq!(symbols[0].kind(), "class");
        assert_eq!(symbols[0].language(), Language::Java);
    }

    #[test]
    fn test_extract_javascript_symbols() {
        let source = b"function main() { return 0; }\nfunction foo() { return 1; }\n";
        let path = Path::new("test.js");
        let result = extract_symbols(path, source);
        assert!(result.is_ok());
        let symbols = result.unwrap();
        assert_eq!(symbols.len(), 2);
        assert_eq!(symbols[0].name(), "main");
        assert_eq!(symbols[0].kind(), "function");
        assert_eq!(symbols[0].language(), Language::JavaScript);
    }

    #[test]
    fn test_extract_typescript_symbols() {
        let source = b"interface User { name: string; }\n";
        let path = Path::new("test.ts");
        let result = extract_symbols(path, source);
        assert!(result.is_ok());
        let symbols = result.unwrap();
        assert!(!symbols.is_empty());
        assert_eq!(symbols[0].name(), "User");
        assert_eq!(symbols[0].kind(), "interface");
        assert_eq!(symbols[0].language(), Language::TypeScript);
    }

    #[test]
    fn test_unknown_extension_returns_error() {
        let source = b"fn main() {}\n";
        let path = Path::new("test.unknown");
        let result = extract_symbols(path, source);
        assert!(result.is_err());
    }

    #[test]
    fn test_extract_with_language_override() {
        let source = b"fn main() {}\n";
        let path = Path::new("test.txt");
        let result = extract_symbols_with_language(path, source, Language::Rust);
        assert!(result.is_ok());
        let symbols = result.unwrap();
        assert_eq!(symbols.len(), 1);
        assert_eq!(symbols[0].name(), "main");
        assert_eq!(symbols[0].language(), Language::Rust);
    }

    #[test]
    fn test_symbol_trait_common_fields() {
        let source = b"fn test_function() { return 42; }\n";
        let path = Path::new("test.rs");
        let result = extract_symbols(path, source);
        assert!(result.is_ok());
        let symbols = result.unwrap();
        let symbol = &symbols[0];

        // Test all Symbol trait methods
        assert_eq!(symbol.name(), "test_function");
        assert_eq!(symbol.kind(), "function");
        assert!(symbol.byte_start() < symbol.byte_end());
        assert_eq!(symbol.line_start(), 1);
        assert_eq!(symbol.line_end(), 1);
        assert_eq!(symbol.col_start(), 0);
        assert_eq!(symbol.fully_qualified(), "crate::test_function");
        assert_eq!(symbol.language(), Language::Rust);
    }
}
