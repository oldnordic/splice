//! TypeScript-specific tree-sitter parsing logic.
//!
//! This module contains tree-sitter-typescript integration for extracting
//! interfaces, types, enums, namespaces, classes, functions, and other TypeScript
//! constructs with byte spans.

use crate::error::{Result, SpliceError};
use ropey::Rope;
use std::path::Path;

/// Represents a TypeScript symbol with its byte and line/col spans.
#[derive(Debug, Clone, PartialEq)]
pub struct TypeScriptSymbol {
    /// Local symbol name (e.g., `User`).
    pub name: String,

    /// Symbol kind (interface, type, enum, namespace, class, function, etc.).
    pub kind: TypeScriptSymbolKind,

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

    /// Function/method parameters (for functions and methods).
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

/// Kinds of TypeScript symbols.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TypeScriptSymbolKind {
    /// Interface declaration.
    Interface,
    /// Type alias declaration.
    TypeAlias,
    /// Enum declaration.
    Enum,
    /// Namespace declaration.
    Namespace,
    /// Function declaration.
    Function,
    /// Class declaration.
    Class,
    /// Method declaration.
    Method,
    /// Variable declaration (const/let/var).
    Variable,
    /// Arrow function (assigned to variable).
    ArrowFunction,
}

impl TypeScriptSymbolKind {
    /// Convert to string for storage.
    pub fn as_str(&self) -> &'static str {
        match self {
            TypeScriptSymbolKind::Interface => "interface",
            TypeScriptSymbolKind::TypeAlias => "type_alias",
            TypeScriptSymbolKind::Enum => "enum",
            TypeScriptSymbolKind::Namespace => "namespace",
            TypeScriptSymbolKind::Function => "function",
            TypeScriptSymbolKind::Class => "class",
            TypeScriptSymbolKind::Method => "method",
            TypeScriptSymbolKind::Variable => "variable",
            TypeScriptSymbolKind::ArrowFunction => "arrow_function",
        }
    }
}

/// Extract symbols and spans from a TypeScript source file.
///
/// Uses tree-sitter-typescript to parse the file and extract:
/// - Interface declarations
/// - Type alias declarations
/// - Enum declarations
/// - Namespace declarations
/// - Function declarations
/// - Class declarations
/// - Method definitions
/// - Variable declarations (const, let, var)
///
/// Automatically detects file extension to use the correct parser:
/// - `.ts` files use `language_typescript()`
/// - `.tsx` files use `language_tsx()`
///
/// Returns a list of symbol entries ready for graph insertion.
pub fn extract_typescript_symbols(path: &Path, source: &[u8]) -> Result<Vec<TypeScriptSymbol>> {
    let mut parser = tree_sitter::Parser::new();

    // Choose parser based on file extension
    let extension = path.extension().and_then(|e| e.to_str());
    let is_tsx = extension == Some("tsx");

    if is_tsx {
        parser
            .set_language(&tree_sitter_typescript::language_tsx())
            .map_err(|e| SpliceError::Parse {
                file: path.to_path_buf(),
                message: format!("Failed to set TSX language: {:?}", e),
            })?;
    } else {
        parser
            .set_language(&tree_sitter_typescript::language_typescript())
            .map_err(|e| SpliceError::Parse {
                file: path.to_path_buf(),
                message: format!("Failed to set TypeScript language: {:?}", e),
            })?;
    }

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
    symbols: &mut Vec<TypeScriptSymbol>,
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

    // Determine if this is an async function
    let is_async = has_modifier(node, "async");

    // Determine symbol kind
    let symbol_kind = match kind {
        "function_declaration" | "function_expression" => Some(TypeScriptSymbolKind::Function),
        "class_declaration" | "class_expression" => Some(TypeScriptSymbolKind::Class),
        "interface_declaration" => Some(TypeScriptSymbolKind::Interface),
        "type_alias_declaration" => Some(TypeScriptSymbolKind::TypeAlias),
        "enum_declaration" => Some(TypeScriptSymbolKind::Enum),
        "internal_module" => Some(TypeScriptSymbolKind::Namespace), // TypeScript uses internal_module
        "variable_declaration" | "lexical_declaration" => Some(TypeScriptSymbolKind::Variable),
        "method_definition" => Some(TypeScriptSymbolKind::Method),
        "arrow_function" => Some(TypeScriptSymbolKind::ArrowFunction),
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
            if kind == TypeScriptSymbolKind::Class || kind == TypeScriptSymbolKind::Namespace {
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
        if (kind == "class_declaration" || kind == "class_expression" || kind == "internal_module")
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
    kind: TypeScriptSymbolKind,
    container_path: &str,
    is_exported: bool,
    is_async: bool,
) -> Option<TypeScriptSymbol> {
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

    Some(TypeScriptSymbol {
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
        "interface_declaration"
        | "type_alias_declaration"
        | "enum_declaration"
        | "namespace_declaration"
        | "internal_module" => node
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
            // Handle required and optional parameters (TypeScript uses pattern field)
            if param.kind() == "required_parameter" || param.kind() == "optional_parameter" {
                // TypeScript uses "pattern" field for the identifier
                if let Some(pattern_node) = param.child_by_field_name("pattern") {
                    if let Ok(name) = pattern_node.utf8_text(source) {
                        parameters.push(name.to_string());
                    }
                }
            } else if param.kind() == "identifier" {
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
    fn test_extract_simple_interface() {
        let source = b"interface User {\n  name: string;\n}\n";
        let path = Path::new("test.ts");
        let result = extract_typescript_symbols(path, source);
        assert!(result.is_ok());
        let symbols = result.unwrap();
        assert_eq!(symbols.len(), 1);
        assert_eq!(symbols[0].name, "User");
        assert_eq!(symbols[0].kind, TypeScriptSymbolKind::Interface);
    }

    #[test]
    fn test_extract_type_alias() {
        let source = b"type UserId = string | number;\n";
        let path = Path::new("test.ts");
        let result = extract_typescript_symbols(path, source);
        assert!(result.is_ok());
        let symbols = result.unwrap();
        assert_eq!(symbols.len(), 1);
        assert_eq!(symbols[0].name, "UserId");
        assert_eq!(symbols[0].kind, TypeScriptSymbolKind::TypeAlias);
    }

    #[test]
    fn test_extract_enum() {
        let source = b"enum Color { Red, Green, Blue }\n";
        let path = Path::new("test.ts");
        let result = extract_typescript_symbols(path, source);
        assert!(result.is_ok());
        let symbols = result.unwrap();
        assert_eq!(symbols.len(), 1);
        assert_eq!(symbols[0].name, "Color");
        assert_eq!(symbols[0].kind, TypeScriptSymbolKind::Enum);
    }

    #[test]
    fn test_extract_namespace() {
        let source = b"namespace Utils { export function helper() {} }\n";
        let path = Path::new("test.ts");
        let result = extract_typescript_symbols(path, source);
        assert!(result.is_ok());
        let symbols = result.unwrap();
        assert!(!symbols.is_empty());
        assert_eq!(symbols[0].name, "Utils");
        assert_eq!(symbols[0].kind, TypeScriptSymbolKind::Namespace);
    }

    #[test]
    fn test_extract_class() {
        let source = b"class Person { name: string; }\n";
        let path = Path::new("test.ts");
        let result = extract_typescript_symbols(path, source);
        assert!(result.is_ok());
        let symbols = result.unwrap();
        assert_eq!(symbols.len(), 1);
        assert_eq!(symbols[0].name, "Person");
        assert_eq!(symbols[0].kind, TypeScriptSymbolKind::Class);
    }

    #[test]
    fn test_extract_function() {
        let source = b"function add(a: number, b: number): number { return a + b; }\n";
        let path = Path::new("test.ts");
        let result = extract_typescript_symbols(path, source);
        assert!(result.is_ok());
        let symbols = result.unwrap();
        assert_eq!(symbols.len(), 1);
        assert_eq!(symbols[0].name, "add");
        assert_eq!(symbols[0].kind, TypeScriptSymbolKind::Function);
        assert_eq!(symbols[0].parameters, vec!["a", "b"]);
    }

    #[test]
    fn test_extract_exported_interface() {
        let source = b"export interface IUser { id: string; }\n";
        let path = Path::new("test.ts");
        let result = extract_typescript_symbols(path, source);
        assert!(result.is_ok());
        let symbols = result.unwrap();
        assert_eq!(symbols.len(), 1);
        assert_eq!(symbols[0].name, "IUser");
        assert!(symbols[0].is_exported);
    }

    #[test]
    fn test_extract_variable() {
        let source = b"const count: number = 42;\n";
        let path = Path::new("test.ts");
        let result = extract_typescript_symbols(path, source);
        assert!(result.is_ok());
        let symbols = result.unwrap();
        assert_eq!(symbols.len(), 1);
        assert_eq!(symbols[0].name, "count");
        assert_eq!(symbols[0].kind, TypeScriptSymbolKind::Variable);
    }

    #[test]
    fn test_extract_async_function() {
        let source = b"async function fetchData(): Promise<string> { return 'data'; }\n";
        let path = Path::new("test.ts");
        let result = extract_typescript_symbols(path, source);
        assert!(result.is_ok());
        let symbols = result.unwrap();
        assert_eq!(symbols.len(), 1);
        assert_eq!(symbols[0].name, "fetchData");
        assert!(symbols[0].is_async);
    }

    #[test]
    fn test_extract_class_with_method() {
        let source = b"class Calculator { add(a: number, b: number): number { return a + b; } }\n";
        let path = Path::new("test.ts");
        let result = extract_typescript_symbols(path, source);
        assert!(result.is_ok());
        let symbols = result.unwrap();
        assert_eq!(symbols.len(), 2);
        assert_eq!(symbols[0].name, "Calculator");
        assert_eq!(symbols[0].kind, TypeScriptSymbolKind::Class);
        assert_eq!(symbols[1].name, "add");
        assert_eq!(symbols[1].kind, TypeScriptSymbolKind::Method);
    }

    #[test]
    fn test_symbol_has_byte_span() {
        let source = b"interface Foo {}\n";
        let path = Path::new("test.ts");
        let result = extract_typescript_symbols(path, source);
        assert!(result.is_ok());
        let symbols = result.unwrap();
        assert_eq!(symbols.len(), 1);
        assert!(symbols[0].byte_start < symbols[0].byte_end);
        assert_eq!(symbols[0].byte_start, 0);
    }
}
