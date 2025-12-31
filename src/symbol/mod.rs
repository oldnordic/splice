//! Common symbol abstraction for multi-language support.
//!
//! This module provides language-agnostic interfaces for symbols extracted
//! from various programming languages. Each language has specific symbol types,
//! but they all share common properties accessible through the `Symbol` trait.

use crate::ingest::{
    cpp::CppSymbol, java::JavaSymbol, javascript::JavaScriptSymbol, python::PythonSymbol,
    rust::RustSymbol, typescript::TypeScriptSymbol,
};
use std::path::Path;

/// Common trait that all language-specific symbols implement.
///
/// This trait provides access to the common properties shared across all languages:
/// - Name (local and fully qualified)
/// - Byte and line/column spans
/// - Language identification
pub trait Symbol {
    /// Get the local symbol name (e.g., `foo`).
    fn name(&self) -> &str;

    /// Get the symbol kind as a string (e.g., "function", "class").
    fn kind(&self) -> &str;

    /// Get the start byte offset.
    fn byte_start(&self) -> usize;

    /// Get the end byte offset.
    fn byte_end(&self) -> usize;

    /// Get the start line (1-based).
    fn line_start(&self) -> usize;

    /// Get the end line (1-based).
    fn line_end(&self) -> usize;

    /// Get the start column (0-based, in bytes).
    fn col_start(&self) -> usize;

    /// Get the end column (0-based, in bytes).
    fn col_end(&self) -> usize;

    /// Get the fully qualified name (e.g., `module::foo`).
    fn fully_qualified(&self) -> &str;

    /// Get the programming language this symbol belongs to.
    fn language(&self) -> Language;
}

/// Programming languages supported by Splice.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Language {
    /// Rust (.rs)
    Rust,
    /// Python (.py)
    Python,
    /// C (.c, .h)
    C,
    /// C++ (.cpp, .hpp, .cc, .cxx)
    Cpp,
    /// Java (.java)
    Java,
    /// JavaScript (.js, .mjs, .cjs)
    JavaScript,
    /// TypeScript (.ts, .tsx)
    TypeScript,
}

impl Language {
    /// Convert language to string identifier.
    pub fn as_str(&self) -> &'static str {
        match self {
            Language::Rust => "rust",
            Language::Python => "python",
            Language::C => "c",
            Language::Cpp => "cpp",
            Language::Java => "java",
            Language::JavaScript => "javascript",
            Language::TypeScript => "typescript",
        }
    }

    /// Detect language from file path extension.
    pub fn from_path(path: &Path) -> Option<Self> {
        use crate::ingest::detect::detect_language;
        detect_language(path).map(|lang| match lang {
            crate::ingest::detect::Language::Rust => Language::Rust,
            crate::ingest::detect::Language::Python => Language::Python,
            crate::ingest::detect::Language::C => Language::C,
            crate::ingest::detect::Language::Cpp => Language::Cpp,
            crate::ingest::detect::Language::Java => Language::Java,
            crate::ingest::detect::Language::JavaScript => Language::JavaScript,
            crate::ingest::detect::Language::TypeScript => Language::TypeScript,
        })
    }
}

/// Wrapper enum for all language-specific symbols.
///
/// This allows storing symbols from different languages in a homogeneous collection
/// while preserving the language-specific data.
#[derive(Debug, Clone)]
pub enum AnySymbol {
    /// Rust symbol.
    Rust(RustSymbol),
    /// Python symbol.
    Python(PythonSymbol),
    /// C/C++ symbol.
    Cpp(CppSymbol),
    /// Java symbol.
    Java(JavaSymbol),
    /// JavaScript symbol.
    JavaScript(JavaScriptSymbol),
    /// TypeScript symbol.
    TypeScript(TypeScriptSymbol),
}

impl Symbol for AnySymbol {
    fn name(&self) -> &str {
        match self {
            AnySymbol::Rust(s) => s.name.as_str(),
            AnySymbol::Python(s) => s.name.as_str(),
            AnySymbol::Cpp(s) => s.name.as_str(),
            AnySymbol::Java(s) => s.name.as_str(),
            AnySymbol::JavaScript(s) => s.name.as_str(),
            AnySymbol::TypeScript(s) => s.name.as_str(),
        }
    }

    fn kind(&self) -> &str {
        match self {
            AnySymbol::Rust(s) => s.kind.as_str(),
            AnySymbol::Python(s) => s.kind.as_str(),
            AnySymbol::Cpp(s) => s.kind.as_str(),
            AnySymbol::Java(s) => s.kind.as_str(),
            AnySymbol::JavaScript(s) => s.kind.as_str(),
            AnySymbol::TypeScript(s) => s.kind.as_str(),
        }
    }

    fn byte_start(&self) -> usize {
        match self {
            AnySymbol::Rust(s) => s.byte_start,
            AnySymbol::Python(s) => s.byte_start,
            AnySymbol::Cpp(s) => s.byte_start,
            AnySymbol::Java(s) => s.byte_start,
            AnySymbol::JavaScript(s) => s.byte_start,
            AnySymbol::TypeScript(s) => s.byte_start,
        }
    }

    fn byte_end(&self) -> usize {
        match self {
            AnySymbol::Rust(s) => s.byte_end,
            AnySymbol::Python(s) => s.byte_end,
            AnySymbol::Cpp(s) => s.byte_end,
            AnySymbol::Java(s) => s.byte_end,
            AnySymbol::JavaScript(s) => s.byte_end,
            AnySymbol::TypeScript(s) => s.byte_end,
        }
    }

    fn line_start(&self) -> usize {
        match self {
            AnySymbol::Rust(s) => s.line_start,
            AnySymbol::Python(s) => s.line_start,
            AnySymbol::Cpp(s) => s.line_start,
            AnySymbol::Java(s) => s.line_start,
            AnySymbol::JavaScript(s) => s.line_start,
            AnySymbol::TypeScript(s) => s.line_start,
        }
    }

    fn line_end(&self) -> usize {
        match self {
            AnySymbol::Rust(s) => s.line_end,
            AnySymbol::Python(s) => s.line_end,
            AnySymbol::Cpp(s) => s.line_end,
            AnySymbol::Java(s) => s.line_end,
            AnySymbol::JavaScript(s) => s.line_end,
            AnySymbol::TypeScript(s) => s.line_end,
        }
    }

    fn col_start(&self) -> usize {
        match self {
            AnySymbol::Rust(s) => s.col_start,
            AnySymbol::Python(s) => s.col_start,
            AnySymbol::Cpp(s) => s.col_start,
            AnySymbol::Java(s) => s.col_start,
            AnySymbol::JavaScript(s) => s.col_start,
            AnySymbol::TypeScript(s) => s.col_end,
        }
    }

    fn col_end(&self) -> usize {
        match self {
            AnySymbol::Rust(s) => s.col_end,
            AnySymbol::Python(s) => s.col_end,
            AnySymbol::Cpp(s) => s.col_end,
            AnySymbol::Java(s) => s.col_end,
            AnySymbol::JavaScript(s) => s.col_end,
            AnySymbol::TypeScript(s) => s.col_end,
        }
    }

    fn fully_qualified(&self) -> &str {
        match self {
            AnySymbol::Rust(s) => s.fully_qualified.as_str(),
            AnySymbol::Python(s) => s.fully_qualified.as_str(),
            AnySymbol::Cpp(s) => s.fully_qualified.as_str(),
            AnySymbol::Java(s) => s.fully_qualified.as_str(),
            AnySymbol::JavaScript(s) => s.fully_qualified.as_str(),
            AnySymbol::TypeScript(s) => s.fully_qualified.as_str(),
        }
    }

    fn language(&self) -> Language {
        match self {
            AnySymbol::Rust(_) => Language::Rust,
            AnySymbol::Python(_) => Language::Python,
            AnySymbol::Cpp(_) => Language::Cpp,
            AnySymbol::Java(_) => Language::Java,
            AnySymbol::JavaScript(_) => Language::JavaScript,
            AnySymbol::TypeScript(_) => Language::TypeScript,
        }
    }
}

// Implement Symbol for all language-specific symbols

impl Symbol for RustSymbol {
    fn name(&self) -> &str {
        self.name.as_str()
    }

    fn kind(&self) -> &str {
        self.kind.as_str()
    }

    fn byte_start(&self) -> usize {
        self.byte_start
    }

    fn byte_end(&self) -> usize {
        self.byte_end
    }

    fn line_start(&self) -> usize {
        self.line_start
    }

    fn line_end(&self) -> usize {
        self.line_end
    }

    fn col_start(&self) -> usize {
        self.col_start
    }

    fn col_end(&self) -> usize {
        self.col_end
    }

    fn fully_qualified(&self) -> &str {
        self.fully_qualified.as_str()
    }

    fn language(&self) -> Language {
        Language::Rust
    }
}

impl Symbol for PythonSymbol {
    fn name(&self) -> &str {
        self.name.as_str()
    }

    fn kind(&self) -> &str {
        self.kind.as_str()
    }

    fn byte_start(&self) -> usize {
        self.byte_start
    }

    fn byte_end(&self) -> usize {
        self.byte_end
    }

    fn line_start(&self) -> usize {
        self.line_start
    }

    fn line_end(&self) -> usize {
        self.line_end
    }

    fn col_start(&self) -> usize {
        self.col_start
    }

    fn col_end(&self) -> usize {
        self.col_end
    }

    fn fully_qualified(&self) -> &str {
        self.fully_qualified.as_str()
    }

    fn language(&self) -> Language {
        Language::Python
    }
}

impl Symbol for CppSymbol {
    fn name(&self) -> &str {
        self.name.as_str()
    }

    fn kind(&self) -> &str {
        self.kind.as_str()
    }

    fn byte_start(&self) -> usize {
        self.byte_start
    }

    fn byte_end(&self) -> usize {
        self.byte_end
    }

    fn line_start(&self) -> usize {
        self.line_start
    }

    fn line_end(&self) -> usize {
        self.line_end
    }

    fn col_start(&self) -> usize {
        self.col_start
    }

    fn col_end(&self) -> usize {
        self.col_end
    }

    fn fully_qualified(&self) -> &str {
        self.fully_qualified.as_str()
    }

    fn language(&self) -> Language {
        Language::Cpp
    }
}

impl Symbol for JavaSymbol {
    fn name(&self) -> &str {
        self.name.as_str()
    }

    fn kind(&self) -> &str {
        self.kind.as_str()
    }

    fn byte_start(&self) -> usize {
        self.byte_start
    }

    fn byte_end(&self) -> usize {
        self.byte_end
    }

    fn line_start(&self) -> usize {
        self.line_start
    }

    fn line_end(&self) -> usize {
        self.line_end
    }

    fn col_start(&self) -> usize {
        self.col_start
    }

    fn col_end(&self) -> usize {
        self.col_end
    }

    fn fully_qualified(&self) -> &str {
        self.fully_qualified.as_str()
    }

    fn language(&self) -> Language {
        Language::Java
    }
}

impl Symbol for JavaScriptSymbol {
    fn name(&self) -> &str {
        self.name.as_str()
    }

    fn kind(&self) -> &str {
        self.kind.as_str()
    }

    fn byte_start(&self) -> usize {
        self.byte_start
    }

    fn byte_end(&self) -> usize {
        self.byte_end
    }

    fn line_start(&self) -> usize {
        self.line_start
    }

    fn line_end(&self) -> usize {
        self.line_end
    }

    fn col_start(&self) -> usize {
        self.col_start
    }

    fn col_end(&self) -> usize {
        self.col_end
    }

    fn fully_qualified(&self) -> &str {
        self.fully_qualified.as_str()
    }

    fn language(&self) -> Language {
        Language::JavaScript
    }
}

impl Symbol for TypeScriptSymbol {
    fn name(&self) -> &str {
        self.name.as_str()
    }

    fn kind(&self) -> &str {
        self.kind.as_str()
    }

    fn byte_start(&self) -> usize {
        self.byte_start
    }

    fn byte_end(&self) -> usize {
        self.byte_end
    }

    fn line_start(&self) -> usize {
        self.line_start
    }

    fn line_end(&self) -> usize {
        self.line_end
    }

    fn col_start(&self) -> usize {
        self.col_start
    }

    fn col_end(&self) -> usize {
        self.col_end
    }

    fn fully_qualified(&self) -> &str {
        self.fully_qualified.as_str()
    }

    fn language(&self) -> Language {
        Language::TypeScript
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_language_as_str() {
        assert_eq!(Language::Rust.as_str(), "rust");
        assert_eq!(Language::Python.as_str(), "python");
        assert_eq!(Language::Cpp.as_str(), "cpp");
        assert_eq!(Language::Java.as_str(), "java");
        assert_eq!(Language::JavaScript.as_str(), "javascript");
        assert_eq!(Language::TypeScript.as_str(), "typescript");
    }

    #[test]
    fn test_language_from_path() {
        use std::path::Path;
        assert_eq!(
            Language::from_path(Path::new("main.rs")),
            Some(Language::Rust)
        );
        assert_eq!(
            Language::from_path(Path::new("script.py")),
            Some(Language::Python)
        );
        assert_eq!(
            Language::from_path(Path::new("main.cpp")),
            Some(Language::Cpp)
        );
        assert_eq!(
            Language::from_path(Path::new("Main.java")),
            Some(Language::Java)
        );
        assert_eq!(
            Language::from_path(Path::new("test.js")),
            Some(Language::JavaScript)
        );
        assert_eq!(
            Language::from_path(Path::new("test.ts")),
            Some(Language::TypeScript)
        );
        assert_eq!(Language::from_path(Path::new("file.txt")), None);
    }
}
