//! Java-specific tree-sitter parsing logic.
//!
//! This module contains tree-sitter-java integration for extracting
//! classes, interfaces, enums, methods, constructors, fields, and other Java constructs with byte spans.

use crate::error::{Result, SpliceError};
use ropey::Rope;
use std::path::Path;

/// Represents a Java symbol with its byte and line/col spans.
#[derive(Debug, Clone, PartialEq)]
pub struct JavaSymbol {
    /// Local symbol name (e.g., `MyClass`).
    pub name: String,

    /// Symbol kind (class, interface, enum, method, constructor, field).
    pub kind: JavaSymbolKind,

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

    /// Method/constructor parameters.
    pub parameters: Vec<String>,

    /// Class/interface path (e.g., `Outer.Inner`).
    pub container_path: String,

    /// Fully qualified name (e.g., `Outer.Inner.method`).
    pub fully_qualified: String,

    /// Whether this has public modifier.
    pub is_public: bool,

    /// Whether this has static modifier.
    pub is_static: bool,
}

/// Kinds of Java symbols.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum JavaSymbolKind {
    /// Class symbol.
    Class,
    /// Interface symbol.
    Interface,
    /// Enum symbol.
    Enum,
    /// Method symbol.
    Method,
    /// Constructor symbol.
    Constructor,
    /// Field symbol.
    Field,
}

impl JavaSymbolKind {
    /// Convert to string for storage.
    pub fn as_str(&self) -> &'static str {
        match self {
            JavaSymbolKind::Class => "class",
            JavaSymbolKind::Interface => "interface",
            JavaSymbolKind::Enum => "enum",
            JavaSymbolKind::Method => "method",
            JavaSymbolKind::Constructor => "constructor",
            JavaSymbolKind::Field => "field",
        }
    }
}

/// Extract symbols and spans from a Java source file.
///
/// Uses tree-sitter-java to parse the file and extract:
/// - Class declarations
/// - Interface declarations
/// - Enum declarations
/// - Method declarations
/// - Constructor declarations
/// - Field declarations
///
/// Returns a list of symbol entries ready for graph insertion.
pub fn extract_java_symbols(path: &Path, source: &[u8]) -> Result<Vec<JavaSymbol>> {
    let mut parser = tree_sitter::Parser::new();
    parser
        .set_language(&tree_sitter_java::language())
        .map_err(|e| SpliceError::Parse {
            file: path.to_path_buf(),
            message: format!("Failed to set Java language: {:?}", e),
        })?;

    let tree = parser
        .parse(source, None)
        .ok_or_else(|| SpliceError::Parse {
            file: path.to_path_buf(),
            message: "Parse failed - no tree returned".to_string(),
        })?;

    let rope = Rope::from_str(std::str::from_utf8(source)?);

    let mut symbols = Vec::new();
    extract_symbols(tree.root_node(), source, &rope, &mut symbols, "");

    Ok(symbols)
}

/// Extract symbols from AST nodes.
fn extract_symbols(
    node: tree_sitter::Node,
    source: &[u8],
    rope: &Rope,
    symbols: &mut Vec<JavaSymbol>,
    container_path: &str,
) {
    let kind = node.kind();

    // Check for modifiers
    let is_public = has_modifier(node, "public");
    let is_static = has_modifier(node, "static");

    // Determine symbol kind
    let symbol_kind = match kind {
        "class_declaration" => Some(JavaSymbolKind::Class),
        "interface_declaration" => Some(JavaSymbolKind::Interface),
        "enum_declaration" => Some(JavaSymbolKind::Enum),
        "method_declaration" => Some(JavaSymbolKind::Method),
        "constructor_declaration" => Some(JavaSymbolKind::Constructor),
        "field_declaration" => Some(JavaSymbolKind::Field),
        _ => None,
    };

    if let Some(kind) = symbol_kind {
        if let Some(symbol) = extract_symbol(
            node,
            source,
            rope,
            kind,
            container_path,
            is_public,
            is_static,
        ) {
            let name = symbol.name.clone();

            symbols.push(symbol);

            // For classes, interfaces, and enums, extract nested symbols
            if matches!(
                kind,
                JavaSymbolKind::Class | JavaSymbolKind::Interface | JavaSymbolKind::Enum
            ) {
                let new_container = if container_path.is_empty() {
                    name.clone()
                } else {
                    format!("{}.{}", container_path, name)
                };

                // Extract from class/interface/enum body
                if let Some(body) = node.child_by_field_name("body") {
                    extract_symbols(body, source, rope, symbols, &new_container);
                }

                return;
            }
        }
    }

    // Recursively process children (unless we already handled class/interface/enum bodies)
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        // Skip bodies of classes/interfaces/enums as we handle them above
        if matches!(
            kind,
            "class_declaration" | "interface_declaration" | "enum_declaration"
        ) && matches!(child.kind(), "class_body" | "interface_body" | "enum_body")
        {
            continue;
        }
        // Skip declarator children of field_declaration (already handled in extract_name)
        if kind == "field_declaration" && child.kind() == "variable_declarator" {
            continue;
        }
        extract_symbols(child, source, rope, symbols, container_path);
    }
}

/// Check if a node has a specific modifier (public, private, static, etc.).
fn has_modifier(node: tree_sitter::Node, modifier: &str) -> bool {
    // Check for modifiers child
    for child in node.children(&mut node.walk()) {
        if child.kind() == "modifiers" {
            for modifier_node in child.children(&mut child.walk()) {
                if modifier_node.kind() == modifier {
                    return true;
                }
            }
        }
    }
    false
}

/// Extract a single symbol from a tree-sitter node.
fn extract_symbol(
    node: tree_sitter::Node,
    source: &[u8],
    rope: &Rope,
    kind: JavaSymbolKind,
    container_path: &str,
    is_public: bool,
    is_static: bool,
) -> Option<JavaSymbol> {
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
        format!("{}.{}", container_path, name)
    };

    Some(JavaSymbol {
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
        is_public,
        is_static,
    })
}

/// Extract the name from a node.
fn extract_name(node: tree_sitter::Node, source: &[u8]) -> Option<String> {
    let kind = node.kind();

    match kind {
        "class_declaration" | "interface_declaration" | "enum_declaration" => node
            .child_by_field_name("name")
            .and_then(|n| n.utf8_text(source).ok().map(|s| s.to_string())),
        "method_declaration" | "constructor_declaration" => node
            .child_by_field_name("name")
            .and_then(|n| n.utf8_text(source).ok().map(|s| s.to_string())),
        "field_declaration" => {
            // For field declarations, get the name from the first declarator
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
        _ => None,
    }
}

/// Extract parameter names from a method/constructor.
fn extract_parameters(node: tree_sitter::Node, source: &[u8]) -> Vec<String> {
    let mut parameters = Vec::new();

    if let Some(params) = node.child_by_field_name("parameters") {
        for param in params.children(&mut params.walk()) {
            if param.kind() == "formal_parameter" {
                if let Some(name_node) = param.child_by_field_name("name") {
                    if let Ok(name) = name_node.utf8_text(source) {
                        parameters.push(name.to_string());
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
    fn test_extract_simple_class() {
        let source = b"class MyClass {}\n";
        let path = Path::new("test.java");
        let result = extract_java_symbols(path, source);
        assert!(result.is_ok());
        let symbols = result.unwrap();
        assert_eq!(symbols.len(), 1);
        assert_eq!(symbols[0].name, "MyClass");
        assert_eq!(symbols[0].kind.as_str(), "class");
    }

    #[test]
    fn test_extract_class_with_method() {
        let source = b"class MyClass { void method() {} }\n";
        let path = Path::new("test.java");
        let result = extract_java_symbols(path, source);
        assert!(result.is_ok());
        let symbols = result.unwrap();
        // Class + method = 2 symbols
        assert_eq!(symbols.len(), 2);
        assert_eq!(symbols[0].name, "MyClass");
        assert_eq!(symbols[0].kind.as_str(), "class");
        assert_eq!(symbols[1].name, "method");
        assert_eq!(symbols[1].kind.as_str(), "method");
    }

    #[test]
    fn test_extract_class_with_field() {
        let source = b"class MyClass { private int field; }\n";
        let path = Path::new("test.java");
        let result = extract_java_symbols(path, source);
        assert!(result.is_ok());
        let symbols = result.unwrap();
        assert_eq!(symbols.len(), 2);
        assert_eq!(symbols[0].name, "MyClass");
        assert_eq!(symbols[1].name, "field");
        assert_eq!(symbols[1].kind.as_str(), "field");
    }

    #[test]
    fn test_extract_interface() {
        let source = b"interface MyInterface { void method(); }\n";
        let path = Path::new("test.java");
        let result = extract_java_symbols(path, source);
        assert!(result.is_ok());
        let symbols = result.unwrap();
        assert_eq!(symbols.len(), 2);
        assert_eq!(symbols[0].name, "MyInterface");
        assert_eq!(symbols[0].kind.as_str(), "interface");
        assert_eq!(symbols[1].name, "method");
        assert_eq!(symbols[1].kind.as_str(), "method");
    }

    #[test]
    fn test_extract_enum() {
        let source = b"enum Color { RED, GREEN, BLUE }\n";
        let path = Path::new("test.java");
        let result = extract_java_symbols(path, source);
        assert!(result.is_ok());
        let symbols = result.unwrap();
        assert_eq!(symbols.len(), 1);
        assert_eq!(symbols[0].name, "Color");
        assert_eq!(symbols[0].kind.as_str(), "enum");
    }

    #[test]
    fn test_extract_class_with_constructor() {
        let source = b"class Foo { Foo() {} }\n";
        let path = Path::new("test.java");
        let result = extract_java_symbols(path, source);
        assert!(result.is_ok());
        let symbols = result.unwrap();
        assert_eq!(symbols.len(), 2);
        assert_eq!(symbols[0].name, "Foo");
        assert_eq!(symbols[0].kind.as_str(), "class");
        assert_eq!(symbols[1].name, "Foo");
        assert_eq!(symbols[1].kind.as_str(), "constructor");
    }

    #[test]
    fn test_extract_method_with_parameters() {
        let source = b"class MyClass { void add(int a, int b) {} }\n";
        let path = Path::new("test.java");
        let result = extract_java_symbols(path, source);
        assert!(result.is_ok());
        let symbols = result.unwrap();
        assert_eq!(symbols.len(), 2);
        assert_eq!(symbols[1].parameters, vec!["a", "b"]);
    }

    #[test]
    fn test_extract_public_class() {
        let source = b"public class MyClass {}\n";
        let path = Path::new("test.java");
        let result = extract_java_symbols(path, source);
        assert!(result.is_ok());
        let symbols = result.unwrap();
        assert_eq!(symbols.len(), 1);
        assert_eq!(symbols[0].name, "MyClass");
        assert!(symbols[0].is_public);
    }

    #[test]
    fn test_extract_static_method() {
        let source = b"class MyClass { static void method() {} }\n";
        let path = Path::new("test.java");
        let result = extract_java_symbols(path, source);
        assert!(result.is_ok());
        let symbols = result.unwrap();
        assert_eq!(symbols.len(), 2);
        assert_eq!(symbols[1].name, "method");
        assert!(symbols[1].is_static);
    }
}
