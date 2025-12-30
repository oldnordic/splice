//! JavaScript/TypeScript-specific tree-sitter parsing logic.
//!
//! This module contains tree-sitter-javascript integration for extracting
//! functions, classes, variables, interfaces, types, and other JS/TS constructs with byte spans.

use crate::error::{Result, SpliceError};
use ropey::Rope;
use std::path::Path;

/// Represents a JavaScript/TypeScript symbol with its byte and line/col spans.
#[derive(Debug, Clone, PartialEq)]
pub struct JavaScriptSymbol {
    /// Local symbol name (e.g., `foo`).
    pub name: String,

    /// Symbol kind (function, class, variable, interface, type, etc.).
    pub kind: JavaScriptSymbolKind,

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

    /// Class/module path (e.g., `Outer.Inner`).
    pub container_path: String,

    /// Fully qualified name (e.g., `Outer.Inner.method`).
    pub fully_qualified: String,

    /// Whether this is an async function.
    pub is_async: bool,

    /// Whether this is exported (export keyword).
    pub is_exported: bool,
}

/// Kinds of JavaScript/TypeScript symbols.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum JavaScriptSymbolKind {
    /// Function symbol.
    Function,
    /// Class symbol.
    Class,
    /// Method symbol.
    Method,
    /// Variable (const/let/var).
    Variable,
    /// Arrow function.
    ArrowFunction,
    /// Interface (TypeScript).
    Interface,
    /// Type alias (TypeScript).
    TypeAlias,
    /// Enum (TypeScript).
    Enum,
    /// Namespace (TypeScript).
    Namespace,
}

impl JavaScriptSymbolKind {
    /// Convert to string for storage.
    pub fn as_str(&self) -> &'static str {
        match self {
            JavaScriptSymbolKind::Function => "function",
            JavaScriptSymbolKind::Class => "class",
            JavaScriptSymbolKind::Method => "method",
            JavaScriptSymbolKind::Variable => "variable",
            JavaScriptSymbolKind::ArrowFunction => "arrow_function",
            JavaScriptSymbolKind::Interface => "interface",
            JavaScriptSymbolKind::TypeAlias => "type_alias",
            JavaScriptSymbolKind::Enum => "enum",
            JavaScriptSymbolKind::Namespace => "namespace",
        }
    }
}

/// Extract symbols and spans from a JavaScript/TypeScript source file.
///
/// Uses tree-sitter-javascript to parse the file and extract:
/// - Function declarations and expressions
/// - Class declarations
/// - Variable declarations (const, let, var)
/// - Method definitions
/// - TypeScript interfaces, type aliases, enums, namespaces
/// - Arrow functions
///
/// Returns a list of symbol entries ready for graph insertion.
pub fn extract_javascript_symbols(path: &Path, source: &[u8]) -> Result<Vec<JavaScriptSymbol>> {
    let mut parser = tree_sitter::Parser::new();
    parser
        .set_language(&tree_sitter_javascript::language())
        .map_err(|e| SpliceError::Parse {
            file: path.to_path_buf(),
            message: format!("Failed to set JavaScript language: {:?}", e),
        })?;

    let tree = parser
        .parse(source, None)
        .ok_or_else(|| SpliceError::Parse {
            file: path.to_path_buf(),
            message: "Parse failed - no tree returned".to_string(),
        })?;

    let rope = Rope::from_str(std::str::from_utf8(source)?);

    let mut symbols = Vec::new();
    extract_symbols(tree.root_node(), source, &rope, &mut symbols, "", false);

    Ok(symbols)
}

/// Extract symbols from AST nodes.
fn extract_symbols(
    node: tree_sitter::Node,
    source: &[u8],
    rope: &Rope,
    symbols: &mut Vec<JavaScriptSymbol>,
    container_path: &str,
    is_exported: bool,
) {
    let kind = node.kind();

    // Check for export_statement wrapper
    if kind == "export_statement" {
        for child in node.children(&mut node.walk()) {
            extract_symbols(child, source, rope, symbols, container_path, true);
        }
        return;
    }

    // Determine symbol kind and check for async
    let is_async = has_modifier(node, "async");
    let symbol_kind = match kind {
        "function_declaration" | "function_expression" => Some(JavaScriptSymbolKind::Function),
        "class_declaration" | "class_expression" => Some(JavaScriptSymbolKind::Class),
        "variable_declaration" | "variable_declarator" | "lexical_declaration" => {
            Some(JavaScriptSymbolKind::Variable)
        }
        "method_definition" => Some(JavaScriptSymbolKind::Method),
        "arrow_function" => Some(JavaScriptSymbolKind::ArrowFunction),
        "interface_declaration" => Some(JavaScriptSymbolKind::Interface),
        "type_alias_declaration" => Some(JavaScriptSymbolKind::TypeAlias),
        "enum_declaration" => Some(JavaScriptSymbolKind::Enum),
        "namespace_declaration" => Some(JavaScriptSymbolKind::Namespace),
        _ => None,
    };

    if let Some(kind) = symbol_kind {
        if let Some(symbol) = extract_symbol(
            node,
            source,
            rope,
            kind,
            container_path,
            is_exported,
            is_async,
        ) {
            let name = symbol.name.clone();

            symbols.push(symbol);

            // For classes and namespaces, extract nested symbols
            if kind == JavaScriptSymbolKind::Class || kind == JavaScriptSymbolKind::Namespace {
                let new_container = if container_path.is_empty() {
                    name.clone()
                } else {
                    format!("{}::{}", container_path, name)
                };

                // Extract from class body or namespace body
                if let Some(body) = node.child_by_field_name("body") {
                    extract_symbols(body, source, rope, symbols, &new_container, is_exported);
                }

                return;
            }
        }
    }

    // Recursively process children (unless we already handled class/namespace bodies)
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        // Skip bodies of classes/namespaces as we handle them above
        if (kind == "class_declaration"
            || kind == "class_expression"
            || kind == "namespace_declaration")
            && (child.kind() == "class_body" || child.kind() == "statement_block")
        {
            continue;
        }
        // Skip variable_declarator children of variable_declaration/lexical_declaration (already handled in extract_name)
        if (kind == "variable_declaration" || kind == "lexical_declaration")
            && child.kind() == "variable_declarator"
        {
            continue;
        }
        extract_symbols(child, source, rope, symbols, container_path, is_exported);
    }
}

/// Check if a node has a specific modifier (async, static, etc.).
fn has_modifier(node: tree_sitter::Node, modifier: &str) -> bool {
    for child in node.children(&mut node.walk()) {
        if child.kind() == modifier {
            return true;
        }
    }
    false
}

/// Extract a single symbol from a tree-sitter node.
fn extract_symbol(
    node: tree_sitter::Node,
    source: &[u8],
    rope: &Rope,
    kind: JavaScriptSymbolKind,
    container_path: &str,
    is_exported: bool,
    is_async: bool,
) -> Option<JavaScriptSymbol> {
    let name = extract_name(node, source)?;

    let byte_start = node.start_byte();
    let byte_end = node.end_byte();

    let start_char = rope.byte_to_char(byte_start);
    let end_char = rope.byte_to_char(byte_end);

    let line_start = rope.char_to_line(start_char);
    let line_end = rope.char_to_line(end_char);

    let line_start_byte = rope.line_to_byte(line_start);
    let line_end_byte = rope.line_to_byte(line_end);

    let col_start = byte_start - line_start_byte;
    let col_end = byte_end - line_end_byte;

    let parameters = extract_parameters(node, source);

    let fully_qualified = if container_path.is_empty() {
        name.clone()
    } else {
        format!("{}::{}", container_path, name)
    };

    Some(JavaScriptSymbol {
        name,
        kind,
        byte_start,
        byte_end,
        line_start: line_start + 1,
        line_end: line_end + 1,
        col_start,
        col_end,
        parameters,
        container_path: container_path.to_string(),
        fully_qualified,
        is_async,
        is_exported,
    })
}

/// Extract the name from a node.
fn extract_name(node: tree_sitter::Node, source: &[u8]) -> Option<String> {
    let kind = node.kind();

    match kind {
        "function_declaration" | "function_expression" => node
            .child_by_field_name("name")
            .and_then(|n| n.utf8_text(source).ok().map(|s| s.to_string())),
        "class_declaration" | "class_expression" => node
            .child_by_field_name("name")
            .and_then(|n| n.utf8_text(source).ok().map(|s| s.to_string())),
        "variable_declaration" | "lexical_declaration" => {
            // For variable declarations, get the name from the first declarator
            for child in node.children(&mut node.walk()) {
                if child.kind() == "variable_declarator" {
                    if let Some(name_node) = child.child_by_field_name("name") {
                        if let Ok(name) = name_node.utf8_text(source) {
                            return Some(name.to_string());
                        }
                    }
                }
            }
            None
        }
        "method_definition" => node
            .child_by_field_name("name")
            .and_then(|n| n.utf8_text(source).ok().map(|s| s.to_string())),
        "interface_declaration"
        | "type_alias_declaration"
        | "enum_declaration"
        | "namespace_declaration" => node
            .child_by_field_name("name")
            .and_then(|n| n.utf8_text(source).ok().map(|s| s.to_string())),
        "arrow_function" => {
            // Arrow functions are usually assigned to a variable
            // We'll return None for anonymous arrows
            None
        }
        _ => None,
    }
}

/// Extract parameter names from a function/method.
fn extract_parameters(node: tree_sitter::Node, source: &[u8]) -> Vec<String> {
    let mut parameters = Vec::new();

    if let Some(params) = node.child_by_field_name("parameters") {
        for param in params.children(&mut params.walk()) {
            if param.kind() == "identifier" {
                if let Ok(name) = param.utf8_text(source) {
                    parameters.push(name.to_string());
                }
            } else if param.kind() == "rest_pattern" {
                for child in param.children(&mut param.walk()) {
                    if child.kind() == "identifier" {
                        if let Ok(name) = child.utf8_text(source) {
                            parameters.push(format!("...{}", name));
                        }
                    }
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
    fn test_extract_simple_function() {
        let source = b"function foo() { return 42; }\n";
        let path = Path::new("test.js");
        let result = extract_javascript_symbols(path, source);
        assert!(result.is_ok());
        let symbols = result.unwrap();
        assert_eq!(symbols.len(), 1);
        assert_eq!(symbols[0].name, "foo");
        assert_eq!(symbols[0].kind, JavaScriptSymbolKind::Function);
    }

    #[test]
    fn test_extract_class_declaration() {
        let source = b"class MyClass { constructor() {} method() {} }\n";
        let path = Path::new("test.js");
        let result = extract_javascript_symbols(path, source);
        assert!(result.is_ok());
        let symbols = result.unwrap();
        // Class + constructor + method = 3 symbols
        assert_eq!(symbols.len(), 3);
        assert_eq!(symbols[0].name, "MyClass");
        assert_eq!(symbols[0].kind, JavaScriptSymbolKind::Class);
    }

    #[test]
    fn test_extract_variable_declaration() {
        let source = b"const x = 10;\n";
        let path = Path::new("test.js");
        let result = extract_javascript_symbols(path, source);
        assert!(result.is_ok());
        let symbols = result.unwrap();
        assert_eq!(symbols.len(), 1);
        assert_eq!(symbols[0].name, "x");
        assert_eq!(symbols[0].kind, JavaScriptSymbolKind::Variable);
    }

    #[test]
    fn test_extract_exported_function() {
        let source = b"export function foo() { return 42; }\n";
        let path = Path::new("test.js");
        let result = extract_javascript_symbols(path, source);
        assert!(result.is_ok());
        let symbols = result.unwrap();
        assert_eq!(symbols.len(), 1);
        assert_eq!(symbols[0].name, "foo");
        assert!(symbols[0].is_exported);
    }

    #[test]
    fn test_extract_async_function() {
        let source = b"async function bar() { await Promise.resolve(); }\n";
        let path = Path::new("test.js");
        let result = extract_javascript_symbols(path, source);
        assert!(result.is_ok());
        let symbols = result.unwrap();
        assert_eq!(symbols.len(), 1);
        assert_eq!(symbols[0].name, "bar");
        assert!(symbols[0].is_async);
    }

    #[test]
    fn test_extract_function_with_parameters() {
        let source = b"function add(a, b) { return a + b; }\n";
        let path = Path::new("test.js");
        let result = extract_javascript_symbols(path, source);
        assert!(result.is_ok());
        let symbols = result.unwrap();
        assert_eq!(symbols.len(), 1);
        assert_eq!(symbols[0].parameters, vec!["a", "b"]);
    }

    // TODO: TypeScript-specific tests require tree-sitter-typescript parser
    // For now, these are skipped since we're using the JavaScript parser
}
