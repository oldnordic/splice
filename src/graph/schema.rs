//! Graph schema definitions for code symbols and spans.
//!
//! This module defines the labels and properties used to store
//! code constructs in the code graph. Labels are language-agnostic
//! to support multi-language code analysis.

use sqlitegraph::{Label, PropertyKey};

/// Label for function symbols (all languages).
pub fn label_function() -> Label {
    Label("symbol_function".into())
}

/// Label for class/struct symbols (all languages).
pub fn label_class() -> Label {
    Label("symbol_class".into())
}

/// Label for enum symbols (all languages).
pub fn label_enum() -> Label {
    Label("symbol_enum".into())
}

/// Label for interface symbols (Java, TypeScript).
pub fn label_interface() -> Label {
    Label("symbol_interface".into())
}

/// Label for impl/trait symbols.
pub fn label_impl() -> Label {
    Label("symbol_impl".into())
}

/// Label for module/namespace symbols (all languages).
pub fn label_module() -> Label {
    Label("symbol_module".into())
}

/// Label for trait symbols (Rust).
pub fn label_trait() -> Label {
    Label("symbol_trait".into())
}

/// Label for variable/field symbols (all languages).
pub fn label_variable() -> Label {
    Label("symbol_variable".into())
}

/// Label for method symbols (class/struct methods).
pub fn label_method() -> Label {
    Label("symbol_method".into())
}

/// Label for constructor symbols.
pub fn label_constructor() -> Label {
    Label("symbol_constructor".into())
}

/// Label for type alias symbols.
pub fn label_type_alias() -> Label {
    Label("symbol_type_alias".into())
}

/// Label for File nodes.
pub fn label_file() -> Label {
    Label("file".into())
}

/// Property key for symbol name.
pub fn prop_name() -> PropertyKey {
    PropertyKey("name".into())
}

/// Property key for start byte offset.
pub fn prop_start() -> PropertyKey {
    PropertyKey("start_byte".into())
}

/// Property key for end byte offset.
pub fn prop_end() -> PropertyKey {
    PropertyKey("end_byte".into())
}

/// Property key for file path.
pub fn prop_file() -> PropertyKey {
    PropertyKey("file".into())
}

/// Property key for symbol kind (function, struct, etc.).
pub fn prop_kind() -> PropertyKey {
    PropertyKey("kind".into())
}

/// Property key for programming language.
pub fn prop_language() -> PropertyKey {
    PropertyKey("language".into())
}

/// Edge type for containment relationships (module contains function).
pub const EDGE_CONTAINS: &str = "contains";

/// Edge type for implementation relationships (impl implements trait).
pub const EDGE_IMPLEMENTS: &str = "implements";

/// Edge type for calls relationships (function calls function).
pub const EDGE_CALLS: &str = "calls";

/// Edge type for file defines symbol relationships.
pub const EDGE_DEFINES: &str = "defines";

/// Map symbol kind string to generic label.
///
/// This function maps language-agnostic symbol kinds to their
/// corresponding graph labels.
pub fn kind_to_label(kind: &str) -> Label {
    match kind {
        "function" => label_function(),
        "method" => label_method(),
        "class" | "struct" => label_class(),
        "interface" => label_interface(),
        "enum" => label_enum(),
        "impl" => label_impl(),
        "trait" => label_trait(),
        "module" | "namespace" => label_module(),
        "variable" | "field" | "const" | "static" => label_variable(),
        "constructor" => label_constructor(),
        "type_alias" => label_type_alias(),
        _ => label_function(), // Default fallback
    }
}
