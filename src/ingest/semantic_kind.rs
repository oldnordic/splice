//! Unified semantic kind detection across all supported languages.
//!
//! Maps language-specific tree-sitter node types to standardized semantic kinds
//! for consistent LLM consumption.

use crate::ingest::detect::Language;

/// Standardized semantic kinds across all languages.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SemanticKind {
    /// Function or method definition
    Function,
    /// Class, struct, or interface definition
    Type,
    /// Variable, field, or parameter
    Variable,
    /// Module or namespace
    Module,
    /// Enum or enumerator
    Enum,
    /// Trait or interface
    Trait,
    /// Type alias
    TypeAlias,
    /// Constant or static
    Constant,
    /// Constructor
    Constructor,
    /// Unknown kind (fallback for unmapped node types)
    Unknown,
}

impl SemanticKind {
    /// Convert semantic kind to string identifier (lowercase).
    pub fn as_str(&self) -> &'static str {
        match self {
            SemanticKind::Function => "function",
            SemanticKind::Type => "type",
            SemanticKind::Variable => "variable",
            SemanticKind::Module => "module",
            SemanticKind::Enum => "enum",
            SemanticKind::Trait => "trait",
            SemanticKind::TypeAlias => "type_alias",
            SemanticKind::Constant => "constant",
            SemanticKind::Constructor => "constructor",
            SemanticKind::Unknown => "unknown",
        }
    }
}

/// Detect semantic kind from tree-sitter node type and language.
///
/// Maps language-specific tree-sitter node types to standardized semantic kinds.
/// Unknown node types return `SemanticKind::Unknown` as a safe fallback.
///
/// # Arguments
///
/// * `node_type` - Tree-sitter node kind (e.g., "function_item", "struct_item")
/// * `language` - Programming language
///
/// # Returns
///
/// Standardized `SemanticKind` or `SemanticKind::Unknown` for unmapped types
///
/// # Examples
///
/// ```
/// use splice::ingest::semantic_kind::{detect_semantic_kind, SemanticKind};
/// use splice::ingest::detect::Language;
///
/// let kind = detect_semantic_kind("function_item", Language::Rust);
/// assert_eq!(kind, SemanticKind::Function);
/// ```
pub fn detect_semantic_kind(node_type: &str, language: Language) -> SemanticKind {
    match language {
        Language::Rust => detect_rust_kind(node_type),
        Language::Python => detect_python_kind(node_type),
        Language::JavaScript => detect_javascript_kind(node_type),
        Language::TypeScript => detect_typescript_kind(node_type),
        Language::Java => detect_java_kind(node_type),
        Language::C => detect_c_kind(node_type),
        Language::Cpp => detect_cpp_kind(node_type),
    }
}

// Rust-specific mappings (tree-sitter-rust grammar)
fn detect_rust_kind(node_type: &str) -> SemanticKind {
    match node_type {
        // Functions
        "function_item" => SemanticKind::Function,

        // Types
        "struct_item" | "enum_item" => SemanticKind::Type,

        // Traits
        "trait_item" => SemanticKind::Trait,

        // Impl blocks (map to trait for consistency)
        "impl_item" => SemanticKind::Trait,

        // Modules
        "mod_item" => SemanticKind::Module,

        // Type aliases
        "type_item" => SemanticKind::TypeAlias,

        // Constants
        "const_item" | "static_item" => SemanticKind::Constant,

        // Fallback
        _ => SemanticKind::Unknown,
    }
}

// Python-specific mappings (tree-sitter-python grammar)
fn detect_python_kind(node_type: &str) -> SemanticKind {
    match node_type {
        // Functions
        "function_definition" => SemanticKind::Function,

        // Types
        "class_definition" => SemanticKind::Type,

        // Modules
        "import_statement" | "import_from_statement" => SemanticKind::Module,

        // Variables (assignment expressions)
        "assignment" | "annotated_assignment" => SemanticKind::Variable,

        // Type aliases (PEP 695, Python 3.12+)
        "type_alias_statement" => SemanticKind::TypeAlias,

        // Fallback
        _ => SemanticKind::Unknown,
    }
}

// JavaScript-specific mappings (tree-sitter-javascript grammar)
fn detect_javascript_kind(node_type: &str) -> SemanticKind {
    match node_type {
        // Functions
        "function_declaration" | "function_expression" | "arrow_function" => SemanticKind::Function,

        // Classes
        "class_declaration" | "class_expression" => SemanticKind::Type,

        // Variables
        "variable_declaration" | "lexical_declaration" => SemanticKind::Variable,

        // Enums (JavaScript doesn't have native enums, but some TypeScript patterns)
        "enum_declaration" => SemanticKind::Enum,

        // Modules
        "import_statement" | "export_statement" => SemanticKind::Module,

        // Fallback
        _ => SemanticKind::Unknown,
    }
}

// TypeScript-specific mappings (tree-sitter-typescript grammar)
fn detect_typescript_kind(node_type: &str) -> SemanticKind {
    match node_type {
        // Functions
        "function_declaration" | "function_expression" | "arrow_function" => SemanticKind::Function,

        // Methods
        "method_definition" => SemanticKind::Function,

        // Classes
        "class_declaration" | "class_expression" => SemanticKind::Type,

        // Interfaces
        "interface_declaration" => SemanticKind::Trait,

        // Enums
        "enum_declaration" => SemanticKind::Enum,

        // Type aliases
        "type_alias_declaration" => SemanticKind::TypeAlias,

        // Namespaces
        "namespace_declaration" => SemanticKind::Module,

        // Variables
        "variable_declaration" | "lexical_declaration" => SemanticKind::Variable,

        // Constructors
        "constructor_parameters" => SemanticKind::Constructor,

        // Fallback
        _ => SemanticKind::Unknown,
    }
}

// Java-specific mappings (tree-sitter-java grammar)
fn detect_java_kind(node_type: &str) -> SemanticKind {
    match node_type {
        // Methods
        "method_declaration" => SemanticKind::Function,

        // Classes and interfaces
        "class_declaration" | "interface_declaration" => SemanticKind::Type,

        // Enums
        "enum_declaration" => SemanticKind::Enum,

        // Fields
        "field_declaration" => SemanticKind::Variable,

        // Constructors
        "constructor_declaration" => SemanticKind::Constructor,

        // Fallback
        _ => SemanticKind::Unknown,
    }
}

// C-specific mappings (tree-sitter-c grammar)
fn detect_c_kind(node_type: &str) -> SemanticKind {
    match node_type {
        // Functions
        "function_definition" | "function_declarator" => SemanticKind::Function,

        // Types
        "struct_specifier" | "union_specifier" => SemanticKind::Type,

        // Enums
        "enum_specifier" => SemanticKind::Enum,

        // Variables
        "declaration" => SemanticKind::Variable,

        // Type aliases (typedef)
        "type_definition" => SemanticKind::TypeAlias,

        // Fallback
        _ => SemanticKind::Unknown,
    }
}

// C++-specific mappings (tree-sitter-cpp grammar)
fn detect_cpp_kind(node_type: &str) -> SemanticKind {
    match node_type {
        // Functions
        "function_definition" | "function_declarator" => SemanticKind::Function,

        // Classes and structs
        "class_specifier" | "struct_specifier" => SemanticKind::Type,

        // Namespaces
        "namespace_definition" => SemanticKind::Module,

        // Enums
        "enum_specifier" => SemanticKind::Enum,

        // Variables
        "declaration" => SemanticKind::Variable,

        // Type aliases
        "type_definition" => SemanticKind::TypeAlias,

        // Constructors
        "constructor_definition" => SemanticKind::Constructor,

        // Template functions (special handling)
        "template_function" => SemanticKind::Function,

        // Template classes
        "template_class" => SemanticKind::Type,

        // Fallback
        _ => SemanticKind::Unknown,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rust_semantic_kinds() {
        assert_eq!(
            detect_semantic_kind("function_item", Language::Rust),
            SemanticKind::Function
        );
        assert_eq!(
            detect_semantic_kind("struct_item", Language::Rust),
            SemanticKind::Type
        );
        assert_eq!(
            detect_semantic_kind("enum_item", Language::Rust),
            SemanticKind::Type
        );
        assert_eq!(
            detect_semantic_kind("trait_item", Language::Rust),
            SemanticKind::Trait
        );
        assert_eq!(
            detect_semantic_kind("impl_item", Language::Rust),
            SemanticKind::Trait
        );
        assert_eq!(
            detect_semantic_kind("mod_item", Language::Rust),
            SemanticKind::Module
        );
        assert_eq!(
            detect_semantic_kind("type_item", Language::Rust),
            SemanticKind::TypeAlias
        );
        assert_eq!(
            detect_semantic_kind("const_item", Language::Rust),
            SemanticKind::Constant
        );
        assert_eq!(
            detect_semantic_kind("static_item", Language::Rust),
            SemanticKind::Constant
        );
    }

    #[test]
    fn test_python_semantic_kinds() {
        assert_eq!(
            detect_semantic_kind("function_definition", Language::Python),
            SemanticKind::Function
        );
        assert_eq!(
            detect_semantic_kind("class_definition", Language::Python),
            SemanticKind::Type
        );
        assert_eq!(
            detect_semantic_kind("assignment", Language::Python),
            SemanticKind::Variable
        );
        assert_eq!(
            detect_semantic_kind("type_alias_statement", Language::Python),
            SemanticKind::TypeAlias
        );
    }

    #[test]
    fn test_javascript_semantic_kinds() {
        assert_eq!(
            detect_semantic_kind("function_declaration", Language::JavaScript),
            SemanticKind::Function
        );
        assert_eq!(
            detect_semantic_kind("arrow_function", Language::JavaScript),
            SemanticKind::Function
        );
        assert_eq!(
            detect_semantic_kind("class_declaration", Language::JavaScript),
            SemanticKind::Type
        );
        assert_eq!(
            detect_semantic_kind("variable_declaration", Language::JavaScript),
            SemanticKind::Variable
        );
    }

    #[test]
    fn test_typescript_semantic_kinds() {
        assert_eq!(
            detect_semantic_kind("function_declaration", Language::TypeScript),
            SemanticKind::Function
        );
        assert_eq!(
            detect_semantic_kind("method_definition", Language::TypeScript),
            SemanticKind::Function
        );
        assert_eq!(
            detect_semantic_kind("interface_declaration", Language::TypeScript),
            SemanticKind::Trait
        );
        assert_eq!(
            detect_semantic_kind("enum_declaration", Language::TypeScript),
            SemanticKind::Enum
        );
        assert_eq!(
            detect_semantic_kind("type_alias_declaration", Language::TypeScript),
            SemanticKind::TypeAlias
        );
        assert_eq!(
            detect_semantic_kind("namespace_declaration", Language::TypeScript),
            SemanticKind::Module
        );
    }

    #[test]
    fn test_java_semantic_kinds() {
        assert_eq!(
            detect_semantic_kind("method_declaration", Language::Java),
            SemanticKind::Function
        );
        assert_eq!(
            detect_semantic_kind("class_declaration", Language::Java),
            SemanticKind::Type
        );
        assert_eq!(
            detect_semantic_kind("interface_declaration", Language::Java),
            SemanticKind::Type
        );
        assert_eq!(
            detect_semantic_kind("enum_declaration", Language::Java),
            SemanticKind::Enum
        );
        assert_eq!(
            detect_semantic_kind("field_declaration", Language::Java),
            SemanticKind::Variable
        );
        assert_eq!(
            detect_semantic_kind("constructor_declaration", Language::Java),
            SemanticKind::Constructor
        );
    }

    #[test]
    fn test_c_semantic_kinds() {
        assert_eq!(
            detect_semantic_kind("function_definition", Language::C),
            SemanticKind::Function
        );
        assert_eq!(
            detect_semantic_kind("struct_specifier", Language::C),
            SemanticKind::Type
        );
        assert_eq!(
            detect_semantic_kind("enum_specifier", Language::C),
            SemanticKind::Enum
        );
        assert_eq!(
            detect_semantic_kind("type_definition", Language::C),
            SemanticKind::TypeAlias
        );
    }

    #[test]
    fn test_cpp_semantic_kinds() {
        assert_eq!(
            detect_semantic_kind("function_definition", Language::Cpp),
            SemanticKind::Function
        );
        assert_eq!(
            detect_semantic_kind("class_specifier", Language::Cpp),
            SemanticKind::Type
        );
        assert_eq!(
            detect_semantic_kind("namespace_definition", Language::Cpp),
            SemanticKind::Module
        );
        assert_eq!(
            detect_semantic_kind("enum_specifier", Language::Cpp),
            SemanticKind::Enum
        );
        assert_eq!(
            detect_semantic_kind("constructor_definition", Language::Cpp),
            SemanticKind::Constructor
        );
        assert_eq!(
            detect_semantic_kind("template_function", Language::Cpp),
            SemanticKind::Function
        );
    }

    #[test]
    fn test_unknown_node_types() {
        // Unknown node types should return Unknown
        assert_eq!(
            detect_semantic_kind("unknown_node", Language::Rust),
            SemanticKind::Unknown
        );
        assert_eq!(
            detect_semantic_kind("weird_construct", Language::Python),
            SemanticKind::Unknown
        );
        assert_eq!(
            detect_semantic_kind("future_syntax", Language::Java),
            SemanticKind::Unknown
        );
    }

    #[test]
    fn test_semantic_kind_as_str() {
        assert_eq!(SemanticKind::Function.as_str(), "function");
        assert_eq!(SemanticKind::Type.as_str(), "type");
        assert_eq!(SemanticKind::Variable.as_str(), "variable");
        assert_eq!(SemanticKind::Module.as_str(), "module");
        assert_eq!(SemanticKind::Enum.as_str(), "enum");
        assert_eq!(SemanticKind::Trait.as_str(), "trait");
        assert_eq!(SemanticKind::TypeAlias.as_str(), "type_alias");
        assert_eq!(SemanticKind::Constant.as_str(), "constant");
        assert_eq!(SemanticKind::Constructor.as_str(), "constructor");
        assert_eq!(SemanticKind::Unknown.as_str(), "unknown");
    }
}
