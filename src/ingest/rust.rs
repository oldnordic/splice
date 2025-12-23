//! Rust-specific tree-sitter parsing logic.
//!
//! This module contains tree-sitter-rust integration for extracting
//! functions, impls, structs, and other Rust constructs with byte spans.

use crate::error::{Result, SpliceError};
use ropey::Rope;
use std::path::Path;

/// Extract symbols and spans from a Rust source file.
///
/// Uses tree-sitter-rust to parse the file and extract:
/// - Functions with signatures and bodies
/// - Struct and enum definitions
/// - Impl blocks with methods
/// - Module declarations
///
/// Returns a list of symbol entries ready for graph insertion.
pub fn extract_rust_symbols(path: &Path, source: &[u8]) -> Result<Vec<RustSymbol>> {
    // Create tree-sitter parser for Rust
    let mut parser = tree_sitter::Parser::new();
    parser
        .set_language(&tree_sitter_rust::language())
        .map_err(|e| SpliceError::Parse {
            file: path.to_path_buf(),
            message: format!("Failed to set Rust language: {:?}", e),
        })?;

    // Parse the source code
    let tree = parser
        .parse(source, None)
        .ok_or_else(|| SpliceError::Parse {
            file: path.to_path_buf(),
            message: "Parse failed - no tree returned".to_string(),
        })?;

    // Create Rope for line/col conversion
    let rope = Rope::from_str(std::str::from_utf8(source)?);

    // Extract symbols from the AST
    let mut symbols = Vec::new();
    extract_functions(tree.root_node(), source, &rope, &mut symbols);

    Ok(symbols)
}

/// Extract function symbols from AST nodes.
fn extract_functions(
    node: tree_sitter::Node,
    source: &[u8],
    rope: &Rope,
    symbols: &mut Vec<RustSymbol>,
) {
    // Process current node if it's a function
    if node.kind() == "function_item" {
        if let Some(symbol) = extract_function_symbol(node, source, rope) {
            symbols.push(symbol);
        }
    }

    // Recursively process children
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        extract_functions(child, source, rope, symbols);
    }
}

/// Extract a single function symbol from a function_item node.
fn extract_function_symbol(
    node: tree_sitter::Node,
    source: &[u8],
    rope: &Rope,
) -> Option<RustSymbol> {
    // Get function name from declaration
    let name_node = node.child_by_field_name("name")?;
    let name = name_node.utf8_text(source).ok()?.to_string();

    // Compute byte spans
    let byte_start = node.start_byte();
    let byte_end = node.end_byte();

    // Convert bytes to line/col using ropey (deterministic)
    let start_char = rope.byte_to_char(byte_start);
    let end_char = rope.byte_to_char(byte_end);

    let line_start = rope.char_to_line(start_char);
    let line_end = rope.char_to_line(end_char);

    // Column is byte offset within the line (0-based)
    let line_start_byte = rope.line_to_byte(line_start);
    let line_end_byte = rope.line_to_byte(line_end);

    let col_start = byte_start - line_start_byte;
    let col_end = byte_end - line_end_byte;

    Some(RustSymbol {
        name,
        kind: RustSymbolKind::Function,
        byte_start,
        byte_end,
        line_start: line_start + 1, // Convert to 1-based
        line_end: line_end + 1,     // Convert to 1-based
        col_start,
        col_end,
        children: Vec::new(), // Nested functions not implemented yet
    })
}

/// Represents a Rust symbol with its byte and line/col spans.
#[derive(Debug, Clone, PartialEq)]
pub struct RustSymbol {
    /// Fully qualified symbol name (e.g., `crate::module::function`).
    pub name: String,

    /// Symbol kind (function, struct, impl, etc.).
    pub kind: RustSymbolKind,

    /// Start byte offset.
    pub byte_start: usize,

    /// End byte offset.
    pub byte_end: usize,

    /// Start line (1-based).
    pub line_start: usize,

    /// End line (1-based).
    pub line_end: usize,

    /// Start column (0-based, in bytes).
    pub col_start: usize,

    /// End column (0-based, in bytes).
    pub col_end: usize,

    /// Nested symbols (e.g., methods inside impl).
    pub children: Vec<RustSymbol>,
}

/// Kinds of Rust symbols.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RustSymbolKind {
    Function,
    Struct,
    Enum,
    Impl,
    Module,
    Trait,
    TypeAlias,
    Const,
    Static,
}

impl RustSymbolKind {
    /// Convert to string for storage.
    pub fn as_str(&self) -> &'static str {
        match self {
            RustSymbolKind::Function => "function",
            RustSymbolKind::Struct => "struct",
            RustSymbolKind::Enum => "enum",
            RustSymbolKind::Impl => "impl",
            RustSymbolKind::Module => "module",
            RustSymbolKind::Trait => "trait",
            RustSymbolKind::TypeAlias => "type_alias",
            RustSymbolKind::Const => "const",
            RustSymbolKind::Static => "static",
        }
    }
}
