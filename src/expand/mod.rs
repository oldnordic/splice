//! AST-aware symbol expansion for retrieving full symbol bodies.
//!
//! This module provides symbol expansion capabilities that allow retrieving
//! full symbol definitions (not just name identifiers) by walking tree-sitter
//! parent chains. This is the foundation for progressive expansion features.
//!
//! # Overview
//!
//! Symbol expansion works by:
//! 1. Parsing a file with tree-sitter to get an AST
//! 2. Finding the node at a given byte offset
//! 3. Walking up the parent chain to find the containing symbol body
//! 4. Optionally expanding to containing blocks (level 2 expansion)
//!
//! # Expansion Levels
//!
//! - `ExpansionLevel::None` (0): No expansion, returns the original node
//! - `ExpansionLevel::Body` (1): Expands to the symbol's full definition body
//! - `ExpansionLevel::ContainingBlock` (2): Expands to the containing block/module
//!
//! # Example
//!
//! ```no_run
//! use splice::expand::{expand_symbol, ExpansionLevel};
//! use splice::symbol::Language;
//! use std::path::Path;
//!
//! let path = Path::new("src/main.rs");
//! let byte_offset = 42; // Offset within a function name
//! let language = Language::Rust;
//!
//! // Expand to get the full function body
//! let (start, end) = expand_symbol(path, byte_offset, language, ExpansionLevel::Body)?;
//! # Ok::<(), splice::SpliceError>(())
//! ```

use crate::error::{Result, SpliceError};
use crate::expand::tree_walker::find_parent_symbol_node;
use crate::symbol::Language;
use std::path::Path;

pub mod tree_walker;

/// Expansion level for symbol expansion.
///
/// Defines how far up the parent chain to walk when expanding a symbol.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExpansionLevel {
    /// No expansion - return the original node (level 0).
    None = 0,

    /// Expand to symbol body - includes full definition (level 1).
    Body = 1,

    /// Expand to containing block - includes parent module/class (level 2).
    ContainingBlock = 2,
}

impl ExpansionLevel {
    /// Get the numeric value of the expansion level.
    pub fn as_u8(self) -> u8 {
        self as u8
    }

    /// Create from numeric value.
    pub fn from_u8(value: u8) -> Option<Self> {
        match value {
            0 => Some(ExpansionLevel::None),
            1 => Some(ExpansionLevel::Body),
            2 => Some(ExpansionLevel::ContainingBlock),
            _ => None,
        }
    }
}

/// Language-specific symbol expander.
///
/// This trait defines how to expand symbols in different programming languages.
/// Each language implementation knows which tree-sitter node kinds represent
/// symbol boundaries.
pub trait SymbolExpander {
    /// Expand a node to its containing symbol body.
    ///
    /// Returns `Some((start_byte, end_byte))` if a symbol body is found,
    /// or `None` if the parent chain doesn't contain a symbol definition.
    ///
    /// # Arguments
    ///
    /// * `node` - The tree-sitter node to expand from
    /// * `source` - The source code bytes (for utf8 text extraction if needed)
    fn expand_to_body(&self, node: tree_sitter::Node, source: &[u8]) -> Option<(usize, usize)>;

    /// Check if a node kind represents a symbol definition.
    ///
    /// This is used to identify when we've reached a symbol boundary while
    /// walking the parent chain.
    fn is_symbol_kind(&self, node_kind: &str) -> bool;

    /// Check if a node kind represents a block/module.
    ///
    /// This is used for level 2 expansion (containing block).
    fn is_block_kind(&self, node_kind: &str) -> bool;
}

/// Rust symbol expander.
#[derive(Debug, Clone, Copy)]
pub struct RustExpander;

impl SymbolExpander for RustExpander {
    fn expand_to_body(&self, node: tree_sitter::Node, source: &[u8]) -> Option<(usize, usize)> {
        find_parent_symbol_node(node, source, |kind| self.is_symbol_kind(kind))
            .map(|n| (n.start_byte() as usize, n.end_byte() as usize))
    }

    fn is_symbol_kind(&self, node_kind: &str) -> bool {
        matches!(
            node_kind,
            "function_item"
                | "struct_item"
                | "enum_item"
                | "trait_item"
                | "impl_item"
                | "mod_item"
                | "const_item"
                | "static_item"
                | "type_item"
        )
    }

    fn is_block_kind(&self, node_kind: &str) -> bool {
        matches!(node_kind, "impl_item" | "mod_item" | "source_file")
    }
}

/// Python symbol expander.
#[derive(Debug, Clone, Copy)]
pub struct PythonExpander;

impl SymbolExpander for PythonExpander {
    fn expand_to_body(&self, node: tree_sitter::Node, source: &[u8]) -> Option<(usize, usize)> {
        find_parent_symbol_node(node, source, |kind| self.is_symbol_kind(kind))
            .map(|n| (n.start_byte() as usize, n.end_byte() as usize))
    }

    fn is_symbol_kind(&self, node_kind: &str) -> bool {
        matches!(node_kind, "function_definition" | "class_definition")
    }

    fn is_block_kind(&self, node_kind: &str) -> bool {
        matches!(node_kind, "module" | "source_file")
    }
}

/// C/C++ symbol expander.
#[derive(Debug, Clone, Copy)]
pub struct CppExpander;

impl SymbolExpander for CppExpander {
    fn expand_to_body(&self, node: tree_sitter::Node, source: &[u8]) -> Option<(usize, usize)> {
        find_parent_symbol_node(node, source, |kind| self.is_symbol_kind(kind))
            .map(|n| (n.start_byte() as usize, n.end_byte() as usize))
    }

    fn is_symbol_kind(&self, node_kind: &str) -> bool {
        matches!(
            node_kind,
            "function_definition"
                | "class_specifier"
                | "struct_specifier"
                | "enum_specifier"
                | "union_specifier"
                | "namespace_definition"
        )
    }

    fn is_block_kind(&self, node_kind: &str) -> bool {
        matches!(node_kind, "namespace_definition" | "translation_unit")
    }
}

/// Java symbol expander.
#[derive(Debug, Clone, Copy)]
pub struct JavaExpander;

impl SymbolExpander for JavaExpander {
    fn expand_to_body(&self, node: tree_sitter::Node, source: &[u8]) -> Option<(usize, usize)> {
        find_parent_symbol_node(node, source, |kind| self.is_symbol_kind(kind))
            .map(|n| (n.start_byte() as usize, n.end_byte() as usize))
    }

    fn is_symbol_kind(&self, node_kind: &str) -> bool {
        matches!(
            node_kind,
            "class_declaration"
                | "interface_declaration"
                | "method_declaration"
                | "constructor_declaration"
                | "field_declaration"
                | "enum_declaration"
        )
    }

    fn is_block_kind(&self, node_kind: &str) -> bool {
        matches!(node_kind, "class_declaration" | "interface_declaration")
    }
}

/// JavaScript symbol expander.
#[derive(Debug, Clone, Copy)]
pub struct JavaScriptExpander;

impl SymbolExpander for JavaScriptExpander {
    fn expand_to_body(&self, node: tree_sitter::Node, source: &[u8]) -> Option<(usize, usize)> {
        find_parent_symbol_node(node, source, |kind| self.is_symbol_kind(kind))
            .map(|n| (n.start_byte() as usize, n.end_byte() as usize))
    }

    fn is_symbol_kind(&self, node_kind: &str) -> bool {
        matches!(
            node_kind,
            "function_declaration"
                | "class_declaration"
                | "method_definition"
                | "generator_function_declaration"
                | "arrow_function"
        )
    }

    fn is_block_kind(&self, node_kind: &str) -> bool {
        matches!(node_kind, "class_declaration" | "program")
    }
}

/// TypeScript symbol expander.
#[derive(Debug, Clone, Copy)]
pub struct TypeScriptExpander;

impl SymbolExpander for TypeScriptExpander {
    fn expand_to_body(&self, node: tree_sitter::Node, source: &[u8]) -> Option<(usize, usize)> {
        find_parent_symbol_node(node, source, |kind| self.is_symbol_kind(kind))
            .map(|n| (n.start_byte() as usize, n.end_byte() as usize))
    }

    fn is_symbol_kind(&self, node_kind: &str) -> bool {
        matches!(
            node_kind,
            "function_declaration"
                | "class_declaration"
                | "interface_declaration"
                | "type_alias_declaration"
                | "method_definition"
                | "generator_function_declaration"
                | "arrow_function"
                | "enum_declaration"
        )
    }

    fn is_block_kind(&self, node_kind: &str) -> bool {
        matches!(
            node_kind,
            "class_declaration" | "interface_declaration" | "module"
        )
    }
}

/// Get the expander for a given language.
fn get_expander(language: Language) -> Box<dyn SymbolExpander> {
    match language {
        Language::Rust => Box::new(RustExpander),
        Language::Python => Box::new(PythonExpander),
        Language::C | Language::Cpp => Box::new(CppExpander),
        Language::Java => Box::new(JavaExpander),
        Language::JavaScript => Box::new(JavaScriptExpander),
        Language::TypeScript => Box::new(TypeScriptExpander),
    }
}

/// Create a tree-sitter parser for the given language.
///
/// This follows the same pattern as `patch::pattern::parser_for_language`.
fn parser_for_language(language: Language) -> Result<tree_sitter::Parser> {
    let mut parser = tree_sitter::Parser::new();

    let lang = match language {
        Language::Rust => tree_sitter_rust::language(),
        Language::Python => tree_sitter_python::language(),
        Language::C => tree_sitter_c::language(),
        Language::Cpp => tree_sitter_cpp::language(),
        Language::Java => tree_sitter_java::language(),
        Language::JavaScript => tree_sitter_javascript::language(),
        Language::TypeScript => tree_sitter_typescript::language_typescript(),
    };

    parser.set_language(&lang).map_err(|e| SpliceError::Parse {
        file: std::path::PathBuf::from("<unknown>"),
        message: format!("Failed to set language for parser: {:?}", e),
    })?;

    Ok(parser)
}

/// Expand a symbol at a given location to its full definition body.
///
/// This function parses the file, finds the node at the given byte offset,
/// and walks up the parent chain to find the containing symbol definition.
///
/// # Arguments
///
/// * `path` - Path to the source file
/// * `byte_offset` - Byte offset within the file (typically pointing to a symbol name)
/// * `language` - Programming language
/// * `level` - Expansion level (None, Body, or ContainingBlock)
///
/// # Returns
///
/// Returns `Ok((byte_start, byte_end))` with the expanded span, or an error if:
/// - The file cannot be read
/// - The file cannot be parsed
/// - No node exists at the byte offset
///
/// # Example
///
/// ```no_run
/// use splice::expand::{expand_symbol, ExpansionLevel};
/// use splice::symbol::Language;
/// use std::path::Path;
///
/// // Get the full function body when given an offset within the function name
/// let (start, end) = expand_symbol(Path::new("src/lib.rs"), 100, Language::Rust, ExpansionLevel::Body)?;
/// # Ok::<(), splice::SpliceError>(())
/// ```
pub fn expand_symbol(
    path: &Path,
    byte_offset: usize,
    language: Language,
    level: ExpansionLevel,
) -> Result<(usize, usize)> {
    expand_symbol_impl(path, byte_offset, language, level)
}

/// Expand a symbol using a numeric level (convenience function for CLI).
///
/// This is a convenience wrapper that converts usize to ExpansionLevel.
/// - 0 = None (no expansion)
/// - 1 = Body (expand to symbol definition)
/// - 2 = ContainingBlock (expand to containing block)
/// - >= 3 defaults to Body
pub fn expand_symbol_with_level(
    path: &Path,
    byte_offset: usize,
    language: Language,
    level: usize,
) -> Result<(usize, usize)> {
    let expansion_level = match level {
        0 => ExpansionLevel::None,
        1 => ExpansionLevel::Body,
        2 => ExpansionLevel::ContainingBlock,
        _ => ExpansionLevel::Body, // Default to Body for higher values
    };
    expand_symbol_impl(path, byte_offset, language, expansion_level)
}

/// Expand a symbol to its full body including leading doc comments.
///
/// This is the preferred expansion function for user-facing output since
/// documentation provides essential context for understanding symbols.
///
/// # Arguments
///
/// * `path` - Path to the source file
/// * `byte_offset` - Byte offset within the file (typically pointing to a symbol name)
/// * `language` - Programming language
///
/// # Returns
///
/// Returns `Ok((byte_start, byte_end))` with the expanded span including docs,
/// or an error if the file cannot be read or parsed.
///
/// # Example
///
/// ```no_run
/// use splice::expand::expand_to_body_with_docs;
/// use splice::symbol::Language;
/// use std::path::Path;
///
/// // Get the full function body including preceding /// docs
/// let (start, end) = expand_to_body_with_docs(Path::new("src/lib.rs"), 100, Language::Rust)?;
/// # Ok::<(), splice::SpliceError>(())
/// ```
pub fn expand_to_body_with_docs(
    path: &Path,
    byte_offset: usize,
    language: Language,
) -> Result<(usize, usize)> {
    // Read the file
    let source = std::fs::read(path).map_err(|e| SpliceError::Io {
        path: path.to_path_buf(),
        source: e,
    })?;

    // Create parser
    let mut parser = parser_for_language(language)?;

    // Parse the file
    let tree = parser
        .parse(&source, None)
        .ok_or_else(|| SpliceError::Parse {
            file: path.to_path_buf(),
            message: "Parse failed - no tree returned".to_string(),
        })?;

    // Find the node at the byte offset
    let root_node = tree.root_node();
    let node = root_node
        .descendant_for_byte_range(byte_offset, byte_offset)
        .ok_or_else(|| SpliceError::InvalidSpan {
            file: path.to_path_buf(),
            start: byte_offset,
            end: byte_offset,
            file_size: source.len(),
        })?;

    // Get the expander for this language
    let expander = get_expander(language);

    // Expand to symbol body
    let (body_start, body_end) = expander.expand_to_body(node, &source).ok_or_else(|| {
        SpliceError::Other(format!(
            "Could not expand symbol at offset {} in {}",
            byte_offset,
            path.display()
        ))
    })?;

    // Find the body node for doc extraction
    let body_node = root_node
        .descendant_for_byte_range(body_start, body_end)
        .ok_or_else(|| {
            SpliceError::Other(format!(
                "Could not find expanded body node in {}",
                path.display()
            ))
        })?;

    // Extend to include leading docs
    let doc_start = tree_walker::extract_leading_docs(&body_node, &source);

    Ok((doc_start, body_end))
}

/// Internal implementation of symbol expansion.
fn expand_symbol_impl(
    path: &Path,
    byte_offset: usize,
    language: Language,
    level: ExpansionLevel,
) -> Result<(usize, usize)> {
    // Read the file
    let source = std::fs::read(path).map_err(|e| SpliceError::Io {
        path: path.to_path_buf(),
        source: e,
    })?;

    // Create parser
    let mut parser = parser_for_language(language)?;

    // Parse the file
    let tree = parser
        .parse(&source, None)
        .ok_or_else(|| SpliceError::Parse {
            file: path.to_path_buf(),
            message: "Parse failed - no tree returned".to_string(),
        })?;

    // Find the node at the byte offset
    let root_node = tree.root_node();
    let node = root_node
        .descendant_for_byte_range(byte_offset, byte_offset)
        .ok_or_else(|| SpliceError::InvalidSpan {
            file: path.to_path_buf(),
            start: byte_offset,
            end: byte_offset,
            file_size: source.len(),
        })?;

    // Get the expander for this language
    let expander = get_expander(language);

    // Apply expansion based on level
    match level {
        ExpansionLevel::None => {
            // No expansion, return the original node's span
            Ok((node.start_byte() as usize, node.end_byte() as usize))
        }
        ExpansionLevel::Body => {
            // Expand to symbol body
            expander.expand_to_body(node, &source).ok_or_else(|| {
                SpliceError::Other(format!(
                    "Could not expand symbol at offset {} in {}",
                    byte_offset,
                    path.display()
                ))
            })
        }
        ExpansionLevel::ContainingBlock => {
            // First expand to body, then to containing block
            let (body_start, body_end) =
                expander.expand_to_body(node, &source).ok_or_else(|| {
                    SpliceError::Other(format!(
                        "Could not expand symbol at offset {} in {}",
                        byte_offset,
                        path.display()
                    ))
                })?;

            // Find the containing block using language-agnostic function
            tree_walker::find_containing_block(&root_node, body_start, body_end, &source)
                .ok_or_else(|| {
                    SpliceError::Other(format!(
                        "Could not expand to containing block in {}",
                        path.display()
                    ))
                })
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_expansion_level_conversions() {
        assert_eq!(ExpansionLevel::None.as_u8(), 0);
        assert_eq!(ExpansionLevel::Body.as_u8(), 1);
        assert_eq!(ExpansionLevel::ContainingBlock.as_u8(), 2);

        assert_eq!(ExpansionLevel::from_u8(0), Some(ExpansionLevel::None));
        assert_eq!(ExpansionLevel::from_u8(1), Some(ExpansionLevel::Body));
        assert_eq!(
            ExpansionLevel::from_u8(2),
            Some(ExpansionLevel::ContainingBlock)
        );
        assert_eq!(ExpansionLevel::from_u8(3), None);
    }

    #[test]
    fn test_rust_expander_symbol_kinds() {
        let expander = RustExpander;

        assert!(expander.is_symbol_kind("function_item"));
        assert!(expander.is_symbol_kind("struct_item"));
        assert!(expander.is_symbol_kind("enum_item"));
        assert!(expander.is_symbol_kind("trait_item"));
        assert!(expander.is_symbol_kind("impl_item"));
        assert!(expander.is_symbol_kind("mod_item"));

        assert!(!expander.is_symbol_kind("identifier"));
        assert!(!expander.is_symbol_kind("string_literal"));
    }

    #[test]
    fn test_python_expander_symbol_kinds() {
        let expander = PythonExpander;

        assert!(expander.is_symbol_kind("function_definition"));
        assert!(expander.is_symbol_kind("class_definition"));

        assert!(!expander.is_symbol_kind("identifier"));
        assert!(!expander.is_symbol_kind("string"));
    }

    #[test]
    fn test_cpp_expander_symbol_kinds() {
        let expander = CppExpander;

        assert!(expander.is_symbol_kind("function_definition"));
        assert!(expander.is_symbol_kind("class_specifier"));
        assert!(expander.is_symbol_kind("struct_specifier"));
        assert!(expander.is_symbol_kind("enum_specifier"));

        assert!(!expander.is_symbol_kind("identifier"));
        assert!(!expander.is_symbol_kind("string_literal"));
    }

    #[test]
    fn test_java_expander_symbol_kinds() {
        let expander = JavaExpander;

        assert!(expander.is_symbol_kind("class_declaration"));
        assert!(expander.is_symbol_kind("interface_declaration"));
        assert!(expander.is_symbol_kind("method_declaration"));

        assert!(!expander.is_symbol_kind("identifier"));
        assert!(!expander.is_symbol_kind("string_literal"));
    }

    #[test]
    fn test_javascript_expander_symbol_kinds() {
        let expander = JavaScriptExpander;

        assert!(expander.is_symbol_kind("function_declaration"));
        assert!(expander.is_symbol_kind("class_declaration"));
        assert!(expander.is_symbol_kind("method_definition"));

        assert!(!expander.is_symbol_kind("identifier"));
        assert!(!expander.is_symbol_kind("string"));
    }

    #[test]
    fn test_typescript_expander_symbol_kinds() {
        let expander = TypeScriptExpander;

        assert!(expander.is_symbol_kind("function_declaration"));
        assert!(expander.is_symbol_kind("class_declaration"));
        assert!(expander.is_symbol_kind("interface_declaration"));
        assert!(expander.is_symbol_kind("type_alias_declaration"));

        assert!(!expander.is_symbol_kind("identifier"));
        assert!(!expander.is_symbol_kind("string"));
    }
}
