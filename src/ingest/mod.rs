//! Filesystem → AST → SQLiteGraph ingestion pipeline.
//!
//! This module handles reading Rust/Python/C/C++/Java/JavaScript/TypeScript source files, parsing them with
//! tree-sitter, and storing symbols and spans in the code graph.

pub mod cpp;
pub mod detect;
pub mod dispatch;
pub mod imports;
pub mod java;
pub mod javascript;
pub mod python;
pub mod rust;
pub mod typescript;

use crate::error::Result;
use crate::graph::CodeGraph;
use std::path::Path;

/// Re-export common types for convenience.
pub use cpp::{extract_cpp_symbols, CppSymbol, CppSymbolKind};
pub use detect::{detect_language, Language};
pub use dispatch::{extract_symbols, extract_symbols_with_language};
pub use imports::{
    extract_cpp_imports, extract_java_imports, extract_javascript_imports, extract_python_imports,
    extract_rust_imports, ImportFact, ImportKind,
};
pub use java::{extract_java_symbols, JavaSymbol, JavaSymbolKind};
pub use javascript::{extract_javascript_symbols, JavaScriptSymbol, JavaScriptSymbolKind};
pub use python::{extract_python_symbols, PythonSymbol, PythonSymbolKind};
pub use rust::{extract_rust_symbols, RustSymbol, RustSymbolKind, Visibility};
pub use typescript::{extract_typescript_symbols, TypeScriptSymbol, TypeScriptSymbolKind};

/// Main ingest orchestrator.
///
/// Reads Rust files from the filesystem, parses them with tree-sitter,
/// and stores symbols/spans in the SQLiteGraph database.
pub struct Ingestor {
    /// Graph database handle (not yet used, pending implementation).
    _graph: CodeGraph,
}

impl Ingestor {
    /// Create a new ingestor with the given graph database.
    pub fn new(graph: CodeGraph) -> Self {
        Self { _graph: graph }
    }

    /// Ingest a single Rust source file.
    pub fn ingest_file(&mut self, _path: &Path) -> Result<()> {
        // TODO: Implement in Task 1
        Err(crate::error::SpliceError::Other(
            "Not implemented yet".to_string(),
        ))
    }

    /// Ingest a directory of Rust source files recursively.
    pub fn ingest_dir(&mut self, _path: &Path) -> Result<()> {
        // TODO: Implement in Task 1
        Err(crate::error::SpliceError::Other(
            "Not implemented yet".to_string(),
        ))
    }
}
