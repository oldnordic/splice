//! Graph schema definitions for code symbols and spans.
//!
//! This module defines the labels and properties used to store
//! Rust code constructs in the code graph.

use sqlitegraph::{Label, PropertyKey};

/// Label for Rust function symbols.
pub fn label_function() -> Label {
    Label("rust_function".into())
}

/// Label for Rust struct symbols.
pub fn label_struct() -> Label {
    Label("rust_struct".into())
}

/// Label for Rust enum symbols.
pub fn label_enum() -> Label {
    Label("rust_enum".into())
}

/// Label for Rust impl blocks.
pub fn label_impl() -> Label {
    Label("rust_impl".into())
}

/// Label for Rust modules.
pub fn label_module() -> Label {
    Label("rust_module".into())
}

/// Label for Rust traits.
pub fn label_trait() -> Label {
    Label("rust_trait".into())
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

/// Edge type for containment relationships (module contains function).
pub const EDGE_CONTAINS: &str = "contains";

/// Edge type for implementation relationships (impl implements trait).
pub const EDGE_IMPLEMENTS: &str = "implements";

/// Edge type for calls relationships (function calls function).
pub const EDGE_CALLS: &str = "calls";

/// Edge type for file defines symbol relationships.
pub const EDGE_DEFINES: &str = "defines";
