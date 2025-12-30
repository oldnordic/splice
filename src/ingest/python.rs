//! Python-specific tree-sitter parsing logic.
//!
//! This module contains tree-sitter-python integration for extracting
//! functions, classes, and other Python constructs with byte spans.

use crate::error::{Result, SpliceError};
use ropey::Rope;
use std::path::Path;

/// Represents a Python symbol with its byte and line/col spans.
#[derive(Debug, Clone, PartialEq)]
pub struct PythonSymbol {
    /// Local symbol name (e.g., `foo`).
    pub name: String,

    /// Symbol kind (function, class, etc.).
    pub kind: PythonSymbolKind,

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

    /// Function parameters (for functions and methods).
    pub parameters: Vec<String>,

    /// Module path (e.g., `module::Outer::Inner`).
    pub module_path: String,

    /// Fully qualified name (e.g., `module::Outer::Inner::foo`).
    pub fully_qualified: String,

    /// Whether this is an async function.
    pub is_async: bool,
}

/// Kinds of Python symbols.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PythonSymbolKind {
    /// Function symbol.
    Function,
    /// Class symbol.
    Class,
    /// Method symbol (function inside a class).
    Method,
    /// Variable (module-level).
    Variable,
}

impl PythonSymbolKind {
    /// Convert to string for storage.
    pub fn as_str(&self) -> &'static str {
        match self {
            PythonSymbolKind::Function => "function",
            PythonSymbolKind::Class => "class",
            PythonSymbolKind::Method => "method",
            PythonSymbolKind::Variable => "variable",
        }
    }
}

/// Extract symbols and spans from a Python source file.
///
/// Uses tree-sitter-python to parse the file and extract:
/// - Functions with signatures and bodies
/// - Class definitions with methods
/// - Async functions
///
/// Returns a list of symbol entries ready for graph insertion.
pub fn extract_python_symbols(path: &Path, source: &[u8]) -> Result<Vec<PythonSymbol>> {
    // Create tree-sitter parser for Python
    let mut parser = tree_sitter::Parser::new();
    parser
        .set_language(&tree_sitter_python::language())
        .map_err(|e| SpliceError::Parse {
            file: path.to_path_buf(),
            message: format!("Failed to set Python language: {:?}", e),
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
    extract_symbols(tree.root_node(), source, &rope, &mut symbols, "module");

    Ok(symbols)
}

/// Extract symbols from AST nodes.
fn extract_symbols(
    node: tree_sitter::Node,
    source: &[u8],
    rope: &Rope,
    symbols: &mut Vec<PythonSymbol>,
    module_path: &str,
) {
    // Check if this node is a symbol we care about
    let kind = node.kind();
    let symbol_kind = match kind {
        "function_definition" => Some(PythonSymbolKind::Function),
        "class_definition" => Some(PythonSymbolKind::Class),
        _ => None,
    };

    // Check for async modifier
    let has_async = node.children(&mut node.walk()).any(|c| c.kind() == "async");

    if let Some(kind) = symbol_kind {
        if let Some(symbol) = extract_symbol(node, source, rope, kind, module_path, has_async) {
            let name = symbol.name.clone();

            symbols.push(symbol);

            // For classes, update the module path for children (nested classes/methods)
            if kind == PythonSymbolKind::Class {
                let new_module_path = format!("{}::{}", module_path, name);

                // Extract symbols from the class body (block)
                if let Some(block) = node.child_by_field_name("body") {
                    extract_symbols(block, source, rope, symbols, &new_module_path);
                }

                // Don't recurse into children again since we handled the block
                return;
            }
        }
    }

    // Recursively process children (only for non-class bodies, which are handled above)
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        // Skip class bodies since we handle them above
        if kind == "class_definition" && child.kind() == "block" {
            continue;
        }
        extract_symbols(child, source, rope, symbols, module_path);
    }
}

/// Extract a single symbol from a tree-sitter node.
fn extract_symbol(
    node: tree_sitter::Node,
    source: &[u8],
    rope: &Rope,
    kind: PythonSymbolKind,
    module_path: &str,
    is_async: bool,
) -> Option<PythonSymbol> {
    // Get symbol name from identifier child
    let name = node
        .children(&mut node.walk())
        .find(|c| c.kind() == "identifier")?
        .utf8_text(source)
        .ok()?
        .to_string();

    // Compute byte spans
    let byte_start = node.start_byte();
    let byte_end = node.end_byte();

    // Convert bytes to line/col using ropey
    let start_char = rope.byte_to_char(byte_start);
    let end_char = rope.byte_to_char(byte_end);

    let line_start = rope.char_to_line(start_char);
    let line_end = rope.char_to_line(end_char);

    // Column is byte offset within the line (0-based)
    let line_start_byte = rope.line_to_byte(line_start);
    let line_end_byte = rope.line_to_byte(line_end);

    let col_start = byte_start - line_start_byte;
    let col_end = byte_end - line_end_byte;

    // Extract parameters for functions
    let parameters = extract_parameters(node, source);

    // Build fully qualified name
    let fully_qualified = format!("{}::{}", module_path, name);

    Some(PythonSymbol {
        name,
        kind,
        byte_start,
        byte_end,
        line_start: line_start + 1, // Convert to 1-based
        line_end: line_end + 1,     // Convert to 1-based
        col_start,
        col_end,
        parameters,
        module_path: module_path.to_string(),
        fully_qualified,
        is_async,
    })
}

/// Extract parameter names from a function definition.
fn extract_parameters(node: tree_sitter::Node, source: &[u8]) -> Vec<String> {
    let mut parameters = Vec::new();

    // Find the parameters node
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        if child.kind() == "parameters" {
            // Extract identifiers from parameters and typed_parameter
            let mut param_cursor = child.walk();
            for param in child.children(&mut param_cursor) {
                match param.kind() {
                    "identifier" => {
                        if let Ok(name) = param.utf8_text(source) {
                            // Filter out 'self' as it's implicit
                            if name != "self" {
                                parameters.push(name.to_string());
                            }
                        }
                    }
                    "typed_parameter" | "default_parameter" | "typed_default_parameter" => {
                        // For typed parameters like `name: str`, extract the identifier
                        let mut sub_cursor = param.walk();
                        for sub_child in param.children(&mut sub_cursor) {
                            if sub_child.kind() == "identifier" {
                                if let Ok(name) = sub_child.utf8_text(source) {
                                    if name != "self" {
                                        parameters.push(name.to_string());
                                    }
                                }
                            }
                        }
                    }
                    "," | "(" | ")" => continue,
                    _ => {}
                }
            }
        }
    }

    parameters
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_simple_function_basic() {
        let source = b"def foo():\n    pass\n";
        let path = Path::new("test.py");
        let result = extract_python_symbols(path, source);
        assert!(result.is_ok());
        let symbols = result.unwrap();
        assert_eq!(symbols.len(), 1);
        assert_eq!(symbols[0].name, "foo");
        assert_eq!(symbols[0].kind, PythonSymbolKind::Function);
    }
}
