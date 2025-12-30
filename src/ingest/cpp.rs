//! C/C++-specific tree-sitter parsing logic.
//!
//! This module contains tree-sitter-cpp integration for extracting
//! functions, classes, structs, namespaces, enums, and other C/C++ constructs with byte spans.

use crate::error::{Result, SpliceError};
use ropey::Rope;
use std::path::Path;

/// Represents a C/C++ symbol with its byte and line/col spans.
#[derive(Debug, Clone, PartialEq)]
pub struct CppSymbol {
    /// Local symbol name (e.g., `foo`).
    pub name: String,

    /// Symbol kind (function, class, struct, namespace, enum, method, etc.).
    pub kind: CppSymbolKind,

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

    /// Namespace path (e.g., `std::vector` or `Outer::Inner`).
    pub namespace_path: String,

    /// Fully qualified name (e.g., `std::vector::iterator`).
    pub fully_qualified: String,

    /// Whether this is a template declaration.
    pub is_template: bool,
}

/// Kinds of C/C++ symbols.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CppSymbolKind {
    /// Function symbol.
    Function,
    /// Class symbol.
    Class,
    /// Struct symbol.
    Struct,
    /// Namespace symbol.
    Namespace,
    /// Enum symbol.
    Enum,
    /// Enumerator (enum value).
    Enumerator,
    /// Method symbol (function inside a class/struct).
    Method,
    /// Field/member variable.
    Field,
    /// Template function.
    TemplateFunction,
    /// Template class.
    TemplateClass,
}

impl CppSymbolKind {
    /// Convert to string for storage.
    pub fn as_str(&self) -> &'static str {
        match self {
            CppSymbolKind::Function => "function",
            CppSymbolKind::Class => "class",
            CppSymbolKind::Struct => "struct",
            CppSymbolKind::Namespace => "namespace",
            CppSymbolKind::Enum => "enum",
            CppSymbolKind::Enumerator => "enumerator",
            CppSymbolKind::Method => "method",
            CppSymbolKind::Field => "field",
            CppSymbolKind::TemplateFunction => "template_function",
            CppSymbolKind::TemplateClass => "template_class",
        }
    }
}

/// Extract symbols and spans from a C/C++ source file.
///
/// Uses tree-sitter-cpp to parse the file and extract:
/// - Functions with signatures and bodies
/// - Class definitions with methods
/// - Struct definitions
/// - Namespace definitions
/// - Enum definitions
/// - Template declarations
///
/// Returns a list of symbol entries ready for graph insertion.
pub fn extract_cpp_symbols(path: &Path, source: &[u8]) -> Result<Vec<CppSymbol>> {
    // Create tree-sitter parser for C/C++
    let mut parser = tree_sitter::Parser::new();
    parser
        .set_language(&tree_sitter_cpp::language())
        .map_err(|e| SpliceError::Parse {
            file: path.to_path_buf(),
            message: format!("Failed to set C++ language: {:?}", e),
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
    extract_symbols(tree.root_node(), source, &rope, &mut symbols, "");

    Ok(symbols)
}

/// Extract symbols from AST nodes.
fn extract_symbols(
    node: tree_sitter::Node,
    source: &[u8],
    rope: &Rope,
    symbols: &mut Vec<CppSymbol>,
    namespace_path: &str,
) {
    let kind = node.kind();

    // Handle template_declaration - unwrap to get the actual declaration
    if kind == "template_declaration" {
        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            match child.kind() {
                "function_definition" | "class_specifier" | "struct_specifier" => {
                    // Extract with is_template=true
                    extract_symbol_with_template(
                        child,
                        source,
                        rope,
                        symbols,
                        namespace_path,
                        true,
                    );
                }
                _ => {}
            }
        }
        // Don't recurse into children of template_declaration
        return;
    }

    // Determine symbol kind
    let symbol_kind = match kind {
        "function_definition" => Some(CppSymbolKind::Function),
        "class_specifier" => Some(CppSymbolKind::Class),
        "struct_specifier" => Some(CppSymbolKind::Struct),
        "namespace_definition" => Some(CppSymbolKind::Namespace),
        "enum_specifier" => Some(CppSymbolKind::Enum),
        "declaration" => {
            // Check if this is a function declaration (has function_declarator)
            let mut has_func_declarator = false;
            let mut cursor = node.walk();
            for child in node.children(&mut cursor) {
                if child.kind() == "function_declarator" {
                    has_func_declarator = true;
                    break;
                }
            }
            if has_func_declarator {
                Some(CppSymbolKind::Function)
            } else {
                None
            }
        }
        _ => None,
    };

    // Extract the symbol if found
    if let Some(kind) = symbol_kind {
        if let Some(symbol) = extract_symbol(node, source, rope, kind, namespace_path, false) {
            let name = symbol.name.clone();

            symbols.push(symbol);

            // For classes, structs, and namespaces, extract nested symbols
            if matches!(
                kind,
                CppSymbolKind::Class | CppSymbolKind::Struct | CppSymbolKind::Namespace
            ) {
                let new_namespace = if namespace_path.is_empty() {
                    name.clone()
                } else {
                    format!("{}::{}", namespace_path, name)
                };

                // Extract symbols from the body/declaration_list
                // Namespaces use "declaration_list", classes/structs use "field_declaration_list"
                let body_field = node.child_by_field_name("body");

                if let Some(body) = body_field {
                    // For namespaces, check declaration_list
                    if kind == CppSymbolKind::Namespace {
                        // body is declaration_list, iterate through declarations
                        for decl in body.children(&mut body.walk()) {
                            if decl.kind() == "declaration" {
                                // Recurse into the declaration to find definitions
                                extract_symbols(decl, source, rope, symbols, &new_namespace);
                            } else {
                                // Also handle nodes directly (for function_declarator etc)
                                extract_symbols(decl, source, rope, symbols, &new_namespace);
                            }
                        }
                    } else {
                        // For classes/structs, body IS the field_declaration_list
                        for field in body.children(&mut body.walk()) {
                            // Nested classes/structs are wrapped in field_declaration
                            match field.kind() {
                                "field_declaration" => {
                                    // Check for nested class_specifier or struct_specifier inside
                                    let mut cursor = field.walk();
                                    for nested in field.children(&mut cursor) {
                                        match nested.kind() {
                                            "class_specifier"
                                            | "struct_specifier"
                                            | "function_definition" => {
                                                extract_symbols(
                                                    nested,
                                                    source,
                                                    rope,
                                                    symbols,
                                                    &new_namespace,
                                                );
                                            }
                                            _ => {}
                                        }
                                    }
                                }
                                "class_specifier" | "struct_specifier" | "function_definition" => {
                                    extract_symbols(field, source, rope, symbols, &new_namespace);
                                }
                                _ => {}
                            }
                        }
                    }
                }

                return;
            }
        }
    }

    // Recursively process children
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        // Skip bodies of classes/structs/namespaces as we handle them above
        if matches!(
            kind,
            "class_specifier" | "struct_specifier" | "namespace_definition"
        ) && (child.kind() == "field_declaration_list" || child.kind() == "declaration_list")
        {
            continue;
        }
        // Skip function_declarator children of declarations (already handled)
        if kind == "declaration" && child.kind() == "function_declarator" {
            continue;
        }
        extract_symbols(child, source, rope, symbols, namespace_path);
    }
}

/// Extract a symbol with template flag.
fn extract_symbol_with_template(
    node: tree_sitter::Node,
    source: &[u8],
    rope: &Rope,
    symbols: &mut Vec<CppSymbol>,
    namespace_path: &str,
    is_template: bool,
) {
    let kind = match node.kind() {
        "function_definition" => CppSymbolKind::TemplateFunction,
        "class_specifier" => CppSymbolKind::TemplateClass,
        "struct_specifier" => CppSymbolKind::Struct, // Template structs use Struct kind
        _ => return,
    };

    if let Some(symbol) = extract_symbol(node, source, rope, kind, namespace_path, is_template) {
        let name = symbol.name.clone();
        symbols.push(symbol);

        // For template classes, also extract nested symbols
        if kind == CppSymbolKind::TemplateClass {
            let new_namespace = if namespace_path.is_empty() {
                name.clone()
            } else {
                format!("{}::{}", namespace_path, name)
            };

            if let Some(body) = node.child_by_field_name("body") {
                for child in body.children(&mut body.walk()) {
                    if child.kind() == "field_declaration_list" {
                        for field in child.children(&mut child.walk()) {
                            // Nested classes/structs are wrapped in field_declaration
                            match field.kind() {
                                "field_declaration" => {
                                    for nested in field.children(&mut field.walk()) {
                                        match nested.kind() {
                                            "class_specifier"
                                            | "struct_specifier"
                                            | "function_definition" => {
                                                extract_symbols(
                                                    nested,
                                                    source,
                                                    rope,
                                                    symbols,
                                                    &new_namespace,
                                                );
                                            }
                                            _ => {}
                                        }
                                    }
                                }
                                "class_specifier" | "struct_specifier" | "function_definition" => {
                                    extract_symbols(field, source, rope, symbols, &new_namespace);
                                }
                                _ => {}
                            }
                        }
                    }
                }
            }
        }
    }
}

/// Extract a single symbol from a tree-sitter node.
fn extract_symbol(
    node: tree_sitter::Node,
    source: &[u8],
    rope: &Rope,
    kind: CppSymbolKind,
    namespace_path: &str,
    is_template: bool,
) -> Option<CppSymbol> {
    let name = extract_name(node, source)?;

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
    let fully_qualified = if namespace_path.is_empty() {
        name.clone()
    } else {
        format!("{}::{}", namespace_path, name)
    };

    // Detect if this is a method (function inside a class/struct)
    // A function is a method if it's inside a namespace that starts with uppercase
    // This is a heuristic - in practice, you'd need more sophisticated tracking
    let actual_kind = if kind == CppSymbolKind::Function && !namespace_path.is_empty() {
        // Check if parent namespace starts with uppercase (likely a class)
        let first_char = namespace_path.chars().next();
        if matches!(first_char, Some(c) if c.is_uppercase()) {
            CppSymbolKind::Method
        } else {
            CppSymbolKind::Function
        }
    } else {
        kind
    };

    Some(CppSymbol {
        name,
        kind: actual_kind,
        byte_start,
        byte_end,
        line_start: line_start + 1, // Convert to 1-based
        line_end: line_end + 1,     // Convert to 1-based
        col_start,
        col_end,
        parameters,
        namespace_path: namespace_path.to_string(),
        fully_qualified,
        is_template,
    })
}

/// Extract the name from a node.
fn extract_name(node: tree_sitter::Node, source: &[u8]) -> Option<String> {
    let kind = node.kind();

    match kind {
        "function_definition" | "declaration" => {
            // Find the identifier in function_declarator
            for child in node.children(&mut node.walk()) {
                if child.kind() == "function_declarator" {
                    return extract_name(child, source);
                }
            }
            None
        }
        "function_declarator" => {
            // Find the identifier
            for child in node.children(&mut node.walk()) {
                if child.kind() == "identifier" || child.kind() == "field_identifier" {
                    return child.utf8_text(source).ok().map(|s| s.to_string());
                }
            }
            None
        }
        "class_specifier" | "struct_specifier" | "enum_specifier" => {
            // Find type_identifier
            for child in node.children(&mut node.walk()) {
                if child.kind() == "type_identifier" {
                    return child.utf8_text(source).ok().map(|s| s.to_string());
                }
            }
            None
        }
        "namespace_definition" => {
            // Find the namespace name - uses namespace_identifier
            for child in node.children(&mut node.walk()) {
                if child.kind() == "namespace_identifier" {
                    return child.utf8_text(source).ok().map(|s| s.to_string());
                }
            }
            None
        }
        _ => None,
    }
}

/// Extract parameter names from a function definition.
fn extract_parameters(node: tree_sitter::Node, source: &[u8]) -> Vec<String> {
    let mut parameters = Vec::new();

    // Find the parameter_list
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        if child.kind() == "function_declarator" {
            let mut param_cursor = child.walk();
            for param_child in child.children(&mut param_cursor) {
                if param_child.kind() == "parameter_list" {
                    // Extract identifiers from parameter_declaration
                    let mut list_cursor = param_child.walk();
                    for param in param_child.children(&mut list_cursor) {
                        if param.kind() == "parameter_declaration" {
                            // Find the identifier in the parameter
                            let mut decl_cursor = param.walk();
                            for decl_child in param.children(&mut decl_cursor) {
                                if decl_child.kind() == "identifier" {
                                    if let Ok(name) = decl_child.utf8_text(source) {
                                        parameters.push(name.to_string());
                                    }
                                }
                            }
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
    fn test_extract_simple_function_basic() {
        let source = b"int foo() { return 42; }\n";
        let path = Path::new("test.cpp");
        let result = extract_cpp_symbols(path, source);
        assert!(result.is_ok());
        let symbols = result.unwrap();
        assert_eq!(symbols.len(), 1);
        assert_eq!(symbols[0].name, "foo");
        assert_eq!(symbols[0].kind, CppSymbolKind::Function);
    }
}
