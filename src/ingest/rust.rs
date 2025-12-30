//! Rust-specific tree-sitter parsing logic.
//!
//! This module contains tree-sitter-rust integration for extracting
//! functions, impls, structs, and other Rust constructs with byte spans.

use crate::error::{Result, SpliceError};
use ropey::Rope;
use std::path::Path;

/// Visibility modifier for Rust symbols.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Visibility {
    /// Public (`pub`)
    Public,

    /// Restricted visibility (`pub(crate)`, `pub(super)`, `pub(in path)`)
    Restricted(String),

    /// Private (no visibility modifier)
    Private,
}

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
    extract_symbols(tree.root_node(), source, &rope, &mut symbols, "crate");

    Ok(symbols)
}

/// Extract symbols from AST nodes.
fn extract_symbols(
    node: tree_sitter::Node,
    source: &[u8],
    rope: &Rope,
    symbols: &mut Vec<RustSymbol>,
    module_path: &str,
) {
    // Check if this node is a symbol we care about
    let kind = node.kind();
    let symbol_kind = match kind {
        "function_item" => Some(RustSymbolKind::Function),
        "struct_item" => Some(RustSymbolKind::Struct),
        "enum_item" => Some(RustSymbolKind::Enum),
        "trait_item" => Some(RustSymbolKind::Trait),
        "impl_item" => Some(RustSymbolKind::Impl),
        "mod_item" => Some(RustSymbolKind::Module),
        _ => None,
    };

    if let Some(kind) = symbol_kind {
        if let Some(symbol) = extract_symbol(node, source, rope, kind, module_path) {
            symbols.push(symbol);
        }
    }

    // Recursively process children
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        // For mod items with inline bodies, update module path
        let new_module_path = if child.kind() == "mod_item" {
            // Check if this is a module declaration with body (mod name { ... })
            if let Some(name_node) = child.child_by_field_name("name") {
                if let Ok(name) = name_node.utf8_text(source) {
                    Some(format!("{}::{}", module_path, name))
                } else {
                    None
                }
            } else {
                None
            }
        } else {
            None
        };

        let path_for_children = match &new_module_path {
            Some(s) => s.as_str(),
            None => module_path,
        };
        extract_symbols(child, source, rope, symbols, path_for_children);
    }
}

/// Extract a single symbol from a tree-sitter node.
fn extract_symbol(
    node: tree_sitter::Node,
    source: &[u8],
    rope: &Rope,
    kind: RustSymbolKind,
    module_path: &str,
) -> Option<RustSymbol> {
    // Get symbol name from declaration
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

    // Extract visibility modifier
    let visibility = extract_visibility(node, source);

    // Build fully qualified name
    let fully_qualified = format!("{}::{}", module_path, name);

    Some(RustSymbol {
        name,
        kind,
        byte_start,
        byte_end,
        line_start: line_start + 1, // Convert to 1-based
        line_end: line_end + 1,     // Convert to 1-based
        col_start,
        col_end,
        children: Vec::new(), // Nested functions not implemented yet
        module_path: module_path.to_string(),
        fully_qualified,
        visibility,
    })
}

/// Extract visibility modifier from a symbol node.
fn extract_visibility(node: tree_sitter::Node, source: &[u8]) -> Visibility {
    // Get the first non-name child to check for visibility modifiers
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        let kind = child.kind();
        // Skip name field and type parameters
        if kind == "name" || kind == "type_parameters" {
            continue;
        }
        // Stop at body or parameters
        if kind == "declaration_body" || kind == "parameters" {
            break;
        }
        // Check for visibility modifiers
        let text = child.utf8_text(source).unwrap_or("");
        if text == "pub" {
            return Visibility::Public;
        }
        if text.starts_with("pub(") {
            return Visibility::Restricted(text.to_string());
        }
    }
    Visibility::Private
}

/// Represents a Rust symbol with its byte and line/col spans.
#[derive(Debug, Clone, PartialEq)]
pub struct RustSymbol {
    /// Local symbol name (e.g., `foo`).
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

    /// Module path (e.g., `crate::my_module::inner`).
    pub module_path: String,

    /// Fully qualified name (e.g., `crate::my_module::inner::foo`).
    pub fully_qualified: String,

    /// Visibility modifier.
    pub visibility: Visibility,
}

/// Kinds of Rust symbols.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RustSymbolKind {
    /// Function symbol.
    Function,
    /// Struct symbol.
    Struct,
    /// Enum symbol.
    Enum,
    /// Impl block.
    Impl,
    /// Module declaration.
    Module,
    /// Trait declaration.
    Trait,
    /// Type alias.
    TypeAlias,
    /// Constant item.
    Const,
    /// Static item.
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
