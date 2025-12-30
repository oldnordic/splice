//! Rust import statement extraction.
//!
//! Uses tree-sitter-rust to parse and extract `use` statements.

use crate::error::{Result, SpliceError};
use crate::ingest::imports::ImportKind;
use std::path::Path;

/// Extract import statements from a Rust source file.
///
/// Uses tree-sitter-rust to parse the file and extract all `use` statements.
///
/// # Examples
///
/// ```
/// # use splice::ingest::imports::{extract_rust_imports, ImportKind};
/// # use std::path::Path;
/// let source = b"use crate::foo::bar;\n";
/// let imports = extract_rust_imports(Path::new("test.rs"), source)?;
/// assert_eq!(imports[0].import_kind, ImportKind::UseCrate);
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
pub fn extract_rust_imports(path: &Path, source: &[u8]) -> Result<Vec<super::ImportFact>> {
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

    // Extract imports from the AST
    let mut imports = Vec::new();
    extract_use_statements(tree.root_node(), source, &mut imports);

    Ok(imports)
}

/// Extract use statements from AST nodes.
fn extract_use_statements(
    node: tree_sitter::Node,
    source: &[u8],
    imports: &mut Vec<super::ImportFact>,
) {
    // Check if this node is a use declaration
    if node.kind() == "use_declaration" {
        if let Some(import) = extract_use_declaration(node, source) {
            imports.push(import);
        }
    }

    // Recursively process children
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        extract_use_statements(child, source, imports);
    }
}

/// Extract a single use declaration from a tree-sitter node.
fn extract_use_declaration(node: tree_sitter::Node, source: &[u8]) -> Option<super::ImportFact> {
    let byte_start = node.start_byte();
    let byte_end = node.end_byte();

    // Check if this is a pub use (re-export)
    // Look for a "visibility" child or a "pub" token
    let is_reexport = check_is_reexport(node, source);

    // Get the argument of the use statement
    let argument = node.child_by_field_name("argument")?;
    let kind = argument.kind();

    match kind {
        "scoped_identifier" => {
            extract_scoped_identifier(argument, source, byte_start, byte_end, is_reexport)
        }
        "use_wildcard" => extract_use_wildcard(argument, source, byte_start, byte_end, is_reexport),
        "scoped_use_list" => {
            extract_scoped_use_list(argument, source, byte_start, byte_end, is_reexport)
        }
        "use_as_clause" => {
            extract_use_as_clause(argument, source, byte_start, byte_end, is_reexport)
        }
        _ => None,
    }
}

/// Check if a use_declaration is a re-export (pub use).
fn check_is_reexport(node: tree_sitter::Node, source: &[u8]) -> bool {
    // Walk through children to find a visibility modifier
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        match child.kind() {
            "visibility_modifier" => return true,
            // Tree-sitter-rust represents "pub" as a visibility_modifier
            // Also check for the literal "pub" token in some tree-sitter versions
            _ => {
                if let Ok(text) = child.utf8_text(source) {
                    if text == "pub" {
                        return true;
                    }
                }
            }
        }
    }
    false
}

/// Helper to extract path segments from a scoped_identifier chain.
///
/// For `use crate::foo::bar;`: Returns (["crate", "foo"], "bar")
fn extract_scoped_path(node: tree_sitter::Node, source: &[u8]) -> Option<(Vec<String>, String)> {
    let mut segments = Vec::new();

    // Walk up the scoped_identifier chain
    let mut current = Some(node);
    while let Some(n) = current {
        if n.kind() == "scoped_identifier" {
            // Extract the name from this level and add to segments
            if let Some(name_field) = n.child_by_field_name("name") {
                if let Ok(text) = name_field.utf8_text(source) {
                    segments.push(text.to_string());
                }
            }
            // Continue up the path
            current = n.child_by_field_name("path");
        } else if n.kind() == "identifier"
            || n.kind() == "crate"
            || n.kind() == "super"
            || n.kind() == "self"
        {
            // Base identifier
            if let Ok(text) = n.utf8_text(source) {
                segments.push(text.to_string());
            }
            break;
        } else {
            break;
        }
    }

    // Reverse to get root-first order
    segments.reverse();

    // Split into path (everything except last) and imported name (last)
    if segments.is_empty() {
        return None;
    }

    let imported_name = segments.pop().unwrap_or_default();
    Some((segments, imported_name))
}

/// Determine import kind from path.
fn import_kind_from_path(path: &[String]) -> ImportKind {
    if path.first().map(|s| s.as_str()) == Some("crate") {
        ImportKind::UseCrate
    } else if path.first().map(|s| s.as_str()) == Some("super") {
        ImportKind::UseSuper
    } else if path.first().map(|s| s.as_str()) == Some("self") {
        ImportKind::UseSelf
    } else {
        ImportKind::PlainUse
    }
}

/// Extract from scoped_identifier: use crate::foo::bar;
fn extract_scoped_identifier(
    node: tree_sitter::Node,
    source: &[u8],
    byte_start: usize,
    byte_end: usize,
    is_reexport: bool,
) -> Option<super::ImportFact> {
    let (path, imported_name) = extract_scoped_path(node, source)?;
    let import_kind = import_kind_from_path(&path);

    Some(super::ImportFact {
        file_path: std::path::PathBuf::new(),
        import_kind,
        path,
        imported_names: vec![imported_name],
        is_glob: false,
        is_reexport,
        byte_span: (byte_start, byte_end),
    })
}

/// Extract from use_wildcard: use crate::module::*;
fn extract_use_wildcard(
    node: tree_sitter::Node,
    source: &[u8],
    byte_start: usize,
    byte_end: usize,
    is_reexport: bool,
) -> Option<super::ImportFact> {
    // use_wildcard contains a scoped_identifier as a child (not in argument field)
    let mut cursor = node.walk();
    let scoped = node
        .children(&mut cursor)
        .find(|n| n.kind() == "scoped_identifier")?;

    // For glob imports, the path includes the module name
    let (mut path, imported_name) = extract_scoped_path(scoped, source)?;
    path.push(imported_name);

    let import_kind = import_kind_from_path(&path);

    Some(super::ImportFact {
        file_path: std::path::PathBuf::new(),
        import_kind,
        path,
        imported_names: vec!["*".to_string()],
        is_glob: true,
        is_reexport,
        byte_span: (byte_start, byte_end),
    })
}

/// Extract from scoped_use_list: use crate::module::{foo, bar};
fn extract_scoped_use_list(
    node: tree_sitter::Node,
    source: &[u8],
    byte_start: usize,
    byte_end: usize,
    is_reexport: bool,
) -> Option<super::ImportFact> {
    let path_node = node.child_by_field_name("path")?;
    let list_node = node.child_by_field_name("list")?;

    // Extract path - the module name is part of the path
    let (mut path, imported_name) = extract_scoped_path(path_node, source)?;
    path.push(imported_name);

    let import_kind = import_kind_from_path(&path);

    // Extract imported names from use_list
    let mut imported_names = Vec::new();
    let mut cursor = list_node.walk();
    for child in list_node.children(&mut cursor) {
        match child.kind() {
            "identifier" => {
                if let Ok(text) = child.utf8_text(source) {
                    imported_names.push(text.to_string());
                }
            }
            "use_as_clause" => {
                if let Some(alias) = child.child_by_field_name("alias") {
                    if let Ok(text) = alias.utf8_text(source) {
                        imported_names.push(text.to_string());
                    }
                }
            }
            "," | "{" | "}" => continue,
            _ => {}
        }
    }

    Some(super::ImportFact {
        file_path: std::path::PathBuf::new(),
        import_kind,
        path,
        imported_names,
        is_glob: false,
        is_reexport,
        byte_span: (byte_start, byte_end),
    })
}

/// Extract from use_as_clause: use crate::foo as Bar;
fn extract_use_as_clause(
    node: tree_sitter::Node,
    source: &[u8],
    byte_start: usize,
    byte_end: usize,
    is_reexport: bool,
) -> Option<super::ImportFact> {
    let path_node = node.child_by_field_name("path")?;
    let alias_node = node.child_by_field_name("alias")?;
    let alias = alias_node.utf8_text(source).ok()?.to_string();

    // For renamed imports, path doesn't include the original name
    let (path, _imported_name) = extract_scoped_path(path_node, source)?;
    let import_kind = import_kind_from_path(&path);

    Some(super::ImportFact {
        file_path: std::path::PathBuf::new(),
        import_kind,
        path,
        imported_names: vec![alias],
        is_glob: false,
        is_reexport,
        byte_span: (byte_start, byte_end),
    })
}
