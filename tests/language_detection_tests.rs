//! Language detection tests.
//!
//! TDD for table-driven language detection from file extensions.

use splice::ingest::detect::{detect_language, Language};
use std::path::Path;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_detect_rust_file() {
        let path = Path::new("main.rs");
        assert_eq!(detect_language(path), Some(Language::Rust));
    }

    #[test]
    fn test_detect_python_file() {
        let path = Path::new("script.py");
        assert_eq!(detect_language(path), Some(Language::Python));
    }

    #[test]
    fn test_detect_c_file() {
        let path = Path::new("header.h");
        assert_eq!(detect_language(path), Some(Language::C));

        let path2 = Path::new("source.c");
        assert_eq!(detect_language(path2), Some(Language::C));
    }

    #[test]
    fn test_detect_cpp_file() {
        let path = Path::new("header.hpp");
        assert_eq!(detect_language(path), Some(Language::Cpp));

        let path2 = Path::new("source.cpp");
        assert_eq!(detect_language(path2), Some(Language::Cpp));

        let path3 = Path::new("source.cc");
        assert_eq!(detect_language(path3), Some(Language::Cpp));

        let path4 = Path::new("source.cxx");
        assert_eq!(detect_language(path4), Some(Language::Cpp));
    }

    #[test]
    fn test_detect_java_file() {
        let path = Path::new("Main.java");
        assert_eq!(detect_language(path), Some(Language::Java));
    }

    #[test]
    fn test_detect_javascript_file() {
        let path = Path::new("script.js");
        assert_eq!(detect_language(path), Some(Language::JavaScript));

        let path2 = Path::new("module.mjs");
        assert_eq!(detect_language(path2), Some(Language::JavaScript));

        let path3 = Path::new("module.cjs");
        assert_eq!(detect_language(path3), Some(Language::JavaScript));
    }

    #[test]
    fn test_detect_typescript_file() {
        let path = Path::new("component.ts");
        assert_eq!(detect_language(path), Some(Language::TypeScript));

        let path2 = Path::new("component.tsx");
        assert_eq!(detect_language(path2), Some(Language::TypeScript));
    }

    #[test]
    fn test_unknown_extension_returns_none() {
        let path = Path::new("file.unknown");
        assert_eq!(detect_language(path), None);
    }

    #[test]
    fn test_text_file_returns_none() {
        let path = Path::new("README.txt");
        assert_eq!(detect_language(path), None);
    }

    #[test]
    fn test_no_extension_returns_none() {
        let path = Path::new("Makefile");
        assert_eq!(detect_language(path), None);
    }

    #[test]
    fn test_path_with_directory_components() {
        let path = Path::new("src/module/main.rs");
        assert_eq!(detect_language(path), Some(Language::Rust));
    }

    #[test]
    fn test_absolute_path() {
        let path = Path::new("/usr/local/bin/script.py");
        assert_eq!(detect_language(path), Some(Language::Python));
    }

    #[test]
    fn test_case_sensitive_extensions() {
        // Extensions should be case-sensitive on Unix
        let path = Path::new("file.RS");
        assert_eq!(detect_language(path), None); // .RS is not .rs
    }

    #[test]
    fn test_empty_filename() {
        let path = Path::new("");
        assert_eq!(detect_language(path), None);
    }

    #[test]
    fn test_dotfile_returns_none() {
        let path = Path::new(".gitignore");
        assert_eq!(detect_language(path), None);
    }

    #[test]
    fn test_language_as_str() {
        assert_eq!(Language::Rust.as_str(), "rust");
        assert_eq!(Language::Python.as_str(), "python");
        assert_eq!(Language::C.as_str(), "c");
        assert_eq!(Language::Cpp.as_str(), "cpp");
        assert_eq!(Language::Java.as_str(), "java");
        assert_eq!(Language::JavaScript.as_str(), "javascript");
        assert_eq!(Language::TypeScript.as_str(), "typescript");
    }
}
