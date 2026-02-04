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
pub mod magellan;
pub mod python;
pub mod rust;
pub mod semantic_kind;
pub mod typescript;

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
pub use magellan::{ingest_file_with_magellan, MagellanIngestor};
pub use python::{extract_python_symbols, PythonSymbol, PythonSymbolKind};
pub use rust::{extract_rust_symbols, RustSymbol, RustSymbolKind, Visibility};
pub use semantic_kind::{detect_semantic_kind, SemanticKind};
pub use typescript::{extract_typescript_symbols, TypeScriptSymbol, TypeScriptSymbolKind};
