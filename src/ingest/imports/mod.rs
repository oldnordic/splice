//! Import statement extraction from source code ASTs.
//!
//! This module provides import extraction for multiple languages:
//! - Rust: `use` statements
//! - Python: `import` and `from ... import` statements
//! - C/C++: `#include` directives
//! - JavaScript/TypeScript: `import` and `require`
//! - Java: `import` statements

use crate::error::{Result, SpliceError};
use crate::symbol::Language;
use std::path::Path;

pub mod cpp;
pub mod java;
pub mod javascript;
pub mod python;
pub mod rust;
pub mod typescript;

/// Trait for language-specific import extraction.
///
/// Each language implements this trait to provide its import extraction logic.
/// The trait defines the common interface and default implementations for
/// shared functionality.
pub trait ImportExtractor {
    /// The tree-sitter language for this extractor.
    fn language() -> tree_sitter::Language;

    /// The Language enum variant for this extractor.
    fn language_enum() -> Language;

    /// Extract imports from a parsed AST node.
    ///
    /// This is where language-specific logic lives - each language
    /// knows its own AST node types and import structures.
    fn extract_from_node(
        node: tree_sitter::Node,
        source: &[u8],
        imports: &mut Vec<ImportFact>,
    );

    /// Extract imports from source code.
    ///
    /// This is the main entry point, providing the common flow:
    /// 1. Create parser
    /// 2. Parse source
    /// 3. Walk AST and extract imports
    fn extract(path: &Path, source: &[u8]) -> Result<Vec<ImportFact>> {
        // Create tree-sitter parser
        let mut parser = tree_sitter::Parser::new();
        parser
            .set_language(&Self::language())
            .map_err(|e| SpliceError::Parse {
                file: path.to_path_buf(),
                message: format!("Failed to set language: {:?}", e),
            })?;

        // Parse the source code
        let tree = parser
            .parse(source, None)
            .ok_or_else(|| SpliceError::Parse {
                file: path.to_path_buf(),
                message: "Parse failed - no tree returned".to_string(),
            })?;

        // Extract imports from the AST
        let mut imports = Vec::new();
        Self::extract_from_node(tree.root_node(), source, &mut imports);

        Ok(imports)
    }
}

/// Represents a single import statement in source code.
#[derive(Debug, Clone, PartialEq)]
pub struct ImportFact {
    /// File path containing the import.
    pub file_path: std::path::PathBuf,

    /// Kind of import (determines resolution strategy).
    pub import_kind: ImportKind,

    /// Import path segments (e.g., ["crate", "b", "foo"]).
    pub path: Vec<String>,

    /// Names imported from the path (e.g., ["foo", "bar"]).
    pub imported_names: Vec<String>,

    /// Whether this is a glob import (e.g., `use crate::module::*`).
    pub is_glob: bool,

    /// Whether this is a re-export (e.g., `pub use` in Rust).
    /// Re-exports make the imported symbol available to other modules.
    pub is_reexport: bool,

    /// Byte span of the import statement in source.
    pub byte_span: (usize, usize),
}

/// Kind of import statement (language-specific).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ImportKind {
    /// `use crate::module::symbol` — explicit crate-local path
    UseCrate,

    /// `use super::symbol` — parent module
    UseSuper,

    /// `use self::symbol` — current module
    UseSelf,

    /// `use extern_crate::symbol` — external crate dependency
    ExternCrate,

    /// `use module::symbol` — plain path (no leading keyword)
    PlainUse,

    /// Python: `import foo` — module import
    PythonImport,

    /// Python: `from foo import bar` — from import
    PythonFrom,

    /// Python: `from . import foo` — relative import
    PythonFromRelative,

    /// Python: `from .. import foo` — parent relative import
    PythonFromParent,

    /// Python: `from ... import foo` — multi-level parent relative
    PythonFromAncestor,

    /// C/C++: `#include <header.h>` — system header
    CppSystemInclude,

    /// C/C++: `#include "header.h"` — local header
    CppLocalInclude,

    /// JavaScript: `import { foo } from 'bar'` — named imports
    JsImport,

    /// JavaScript: `import foo from 'bar'` — default import
    JsDefaultImport,

    /// JavaScript: `import * as foo from 'bar'` — namespace import
    JsNamespaceImport,

    /// JavaScript: `import 'bar'` — side-effect import
    JsSideEffectImport,

    /// JavaScript: `const foo = require('bar')` — CommonJS require
    JsRequire,

    /// Java: `import foo.Bar` — regular import
    JavaImport,

    /// Java: `import static foo.Bar` — static import
    JavaStaticImport,

    /// TypeScript: `import type { Foo } from 'bar'` — type-only named import
    TsTypeImport,

    /// TypeScript: `import type Foo from 'bar'` — type-only default import
    TsTypeDefaultImport,
}

impl ImportKind {
    /// Convert to string for storage.
    pub fn as_str(&self) -> &'static str {
        match self {
            ImportKind::UseCrate => "UseCrate",
            ImportKind::UseSuper => "UseSuper",
            ImportKind::UseSelf => "UseSelf",
            ImportKind::ExternCrate => "ExternCrate",
            ImportKind::PlainUse => "PlainUse",
            ImportKind::PythonImport => "PythonImport",
            ImportKind::PythonFrom => "PythonFrom",
            ImportKind::PythonFromRelative => "PythonFromRelative",
            ImportKind::PythonFromParent => "PythonFromParent",
            ImportKind::PythonFromAncestor => "PythonFromAncestor",
            ImportKind::CppSystemInclude => "CppSystemInclude",
            ImportKind::CppLocalInclude => "CppLocalInclude",
            ImportKind::JsImport => "JsImport",
            ImportKind::JsDefaultImport => "JsDefaultImport",
            ImportKind::JsNamespaceImport => "JsNamespaceImport",
            ImportKind::JsSideEffectImport => "JsSideEffectImport",
            ImportKind::JsRequire => "JsRequire",
            ImportKind::JavaImport => "JavaImport",
            ImportKind::JavaStaticImport => "JavaStaticImport",
            ImportKind::TsTypeImport => "TsTypeImport",
            ImportKind::TsTypeDefaultImport => "TsTypeDefaultImport",
        }
    }
}

// Re-exports for convenience
pub use cpp::extract_cpp_imports;
pub use java::extract_java_imports;
pub use javascript::extract_javascript_imports;
pub use python::extract_python_imports;
pub use rust::extract_rust_imports;
pub use typescript::extract_typescript_imports;
