//! Rust-specific reference finding using tree-sitter-rust.
//!
//! This module traverses the AST to find identifier references
//! that point to a specific symbol definition.

use super::cross_file::find_cross_file_references;
use super::scope::{build_scope_map, ScopeMap};
use crate::error::{Result, SpliceError};
use crate::graph::CodeGraph;
use crate::ingest::rust::{extract_rust_symbols, RustSymbol, RustSymbolKind, Visibility};
use crate::io_ext;
use crate::resolve::references::{Reference, ReferenceContext, ReferenceSet, SymbolDefinition};
use ropey::Rope;
use std::path::Path;

/// Find all references to a Rust symbol.
///
/// # Algorithm
/// 1. Parse the file with tree-sitter to find the symbol definition
/// 2. Traverse the AST finding all identifier nodes matching the symbol name
/// 3. Filter out the definition itself
/// 4. Collect matching references with context
pub fn find_rust_references(
    _graph: &CodeGraph,
    file_path: &Path,
    symbol_name: &str,
    symbol_kind: Option<RustSymbolKind>,
) -> Result<ReferenceSet> {
    // Step 1: Read and parse the source file
    let source = io_ext::read(file_path)?;
    let rope = Rope::from_str(std::str::from_utf8(&source)?);

    // Step 2: Extract all symbols to find the target definition
    let symbols = extract_rust_symbols(file_path, &source)?;

    // Find the target symbol definition
    let target_symbol = symbols
        .iter()
        .find(|s| s.name == symbol_name && symbol_kind.is_none_or(|k| s.kind == k))
        .ok_or_else(|| SpliceError::symbol_not_found(symbol_name, Some(file_path)))?;

    // Step 3: Find same-file references
    let same_file_refs = find_same_file_references(&source, &rope, target_symbol, file_path)?;

    // Step 4: Find cross-file references (if public)
    let (cross_file_refs, has_glob_ambiguity) = if target_symbol.visibility != Visibility::Private {
        find_cross_file_references(file_path, target_symbol)?
    } else {
        (Vec::new(), false)
    };

    // Step 5: Combine and sort references (by byte offset descending for deletion order)
    let mut all_refs = same_file_refs;
    all_refs.extend(cross_file_refs);
    all_refs.sort_by_key(|r| std::cmp::Reverse(r.byte_start));

    Ok(ReferenceSet {
        references: all_refs,
        definition: SymbolDefinition {
            name: target_symbol.name.clone(),
            kind: target_symbol.kind,
            file_path: file_path.to_str().unwrap_or("").to_string(),
            byte_start: target_symbol.byte_start,
            byte_end: target_symbol.byte_end,
            is_public: target_symbol.visibility != Visibility::Private,
        },
        has_glob_ambiguity,
    })
}

/// Find references within the same file.
fn find_same_file_references(
    source: &[u8],
    rope: &Rope,
    target_symbol: &RustSymbol,
    file_path: &Path,
) -> Result<Vec<Reference>> {
    let mut references = Vec::new();

    // Build scope map for shadowing detection
    let scope_map = build_scope_map(source)?;

    // Parse the file
    let mut parser = tree_sitter::Parser::new();
    parser
        .set_language(&tree_sitter_rust::LANGUAGE.into())
        .map_err(|e| SpliceError::Parse {
            file: file_path.to_path_buf(),
            message: format!("Failed to set Rust language: {:?}", e),
        })?;

    let tree = parser
        .parse(source, None)
        .ok_or_else(|| SpliceError::Parse {
            file: file_path.to_path_buf(),
            message: "Parse failed - no tree returned".to_string(),
        })?;

    // Walk the AST looking for identifier nodes
    find_identifiers_recursive(
        tree.root_node(),
        source,
        rope,
        target_symbol,
        file_path,
        &scope_map,
        &mut references,
    );

    // Filter out the definition itself
    references.retain(|r| {
        !(r.byte_start >= target_symbol.byte_start && r.byte_end <= target_symbol.byte_end)
    });

    Ok(references)
}

/// Find references to a symbol in a specific file.
///
/// This is a simplified version of find_same_file_references that doesn't
/// do symbol extraction (since we already know the target symbol info).
pub(crate) fn find_references_in_file(
    source: &[u8],
    rope: &Rope,
    target_symbol: &RustSymbol,
    file_path: &Path,
) -> Result<Vec<Reference>> {
    let mut references = Vec::new();

    // Build scope map for shadowing detection
    let scope_map = build_scope_map(source)?;

    // Parse the file
    let mut parser = tree_sitter::Parser::new();
    parser
        .set_language(&tree_sitter_rust::LANGUAGE.into())
        .map_err(|e| SpliceError::Parse {
            file: file_path.to_path_buf(),
            message: format!("Failed to set Rust language: {:?}", e),
        })?;

    let tree = parser
        .parse(source, None)
        .ok_or_else(|| SpliceError::Parse {
            file: file_path.to_path_buf(),
            message: "Parse failed - no tree returned".to_string(),
        })?;

    // Walk the AST looking for identifier nodes
    find_identifiers_recursive(
        tree.root_node(),
        source,
        rope,
        target_symbol,
        file_path,
        &scope_map,
        &mut references,
    );

    Ok(references)
}

/// Recursively find identifier references.
fn find_identifiers_recursive(
    node: tree_sitter::Node,
    source: &[u8],
    rope: &Rope,
    target_symbol: &RustSymbol,
    file_path: &Path,
    scope_map: &ScopeMap,
    references: &mut Vec<Reference>,
) {
    let kind = node.kind();

    // Check if this node could be a reference
    match kind {
        "identifier" => {
            // Skip if parent is a call_expression (already handled there)
            let parent = node.parent();
            if let Some(p) = parent {
                if p.kind() == "call_expression" {
                    // Already handled in call_expression case, skip
                    // But still recurse to find other references in arguments
                    let mut cursor = node.walk();
                    for child in node.children(&mut cursor) {
                        find_identifiers_recursive(
                            child,
                            source,
                            rope,
                            target_symbol,
                            file_path,
                            scope_map,
                            references,
                        );
                    }
                    return;
                }
            }

            if let Ok(text) = node.utf8_text(source) {
                if text == target_symbol.name {
                    // Check if this identifier is shadowed by a local definition
                    if scope_map.is_shadowed_at(&target_symbol.name, node.start_byte()) {
                        // Shadowed - don't count as a reference to the target symbol
                        // But still recurse to find other references
                        let mut cursor = node.walk();
                        for child in node.children(&mut cursor) {
                            find_identifiers_recursive(
                                child,
                                source,
                                rope,
                                target_symbol,
                                file_path,
                                scope_map,
                                references,
                            );
                        }
                        return;
                    }

                    let context = extract_context(node, source);

                    let start_char = rope.byte_to_char(node.start_byte());
                    let line = rope.char_to_line(start_char);
                    let line_byte = rope.line_to_byte(line);
                    let col = node.start_byte() - line_byte;

                    references.push(Reference {
                        file_path: file_path.to_str().unwrap_or("").to_string(),
                        byte_start: node.start_byte(),
                        byte_end: node.end_byte(),
                        line: line + 1,
                        column: col,
                        context,
                        match_id: None,
                    });
                }
            }
        }
        "scoped_identifier" | "scoped_type_identifier" => {
            // Check if the last segment matches our symbol name
            if let Ok(text) = node.utf8_text(source) {
                if text.ends_with(&format!("::{}", target_symbol.name)) {
                    let context = extract_context(node, source);

                    let start_char = rope.byte_to_char(node.start_byte());
                    let line = rope.char_to_line(start_char);
                    let line_byte = rope.line_to_byte(line);
                    let col = node.start_byte() - line_byte;

                    references.push(Reference {
                        file_path: file_path.to_str().unwrap_or("").to_string(),
                        byte_start: node.start_byte(),
                        byte_end: node.end_byte(),
                        line: line + 1,
                        column: col,
                        context,
                        match_id: None,
                    });
                }
            }
        }
        "call_expression" => {
            // Check if function being called is our target
            if let Some(func) = node.child_by_field_name("function") {
                let func_kind = func.kind();
                if func_kind == "identifier"
                    || func_kind == "scoped_identifier"
                    || func_kind == "field_expression"
                {
                    if let Ok(text) = func.utf8_text(source) {
                        let matches = if func_kind == "identifier" {
                            text == target_symbol.name
                        } else if func_kind == "field_expression" {
                            // field_expression: receiver.method_name()
                            // Extract the field name (method name)
                            if let Some(field) = func.child_by_field_name("field") {
                                if let Ok(field_text) = field.utf8_text(source) {
                                    field_text == target_symbol.name
                                } else {
                                    false
                                }
                            } else {
                                false
                            }
                        } else {
                            text.ends_with(&format!("::{}", target_symbol.name))
                        };

                        if matches && target_symbol.kind == RustSymbolKind::Function {
                            // For unqualified calls, check for shadowing
                            if func_kind == "identifier"
                                && scope_map.is_shadowed_at(&target_symbol.name, func.start_byte())
                            {
                                // Shadowed - don't count as a reference
                            } else {
                                let context = ReferenceContext::FunctionCall {
                                    is_qualified: func_kind == "scoped_identifier"
                                        || func_kind == "field_expression",
                                };

                                // For field_expression, use the field node's position
                                let (start, end) = if func_kind == "field_expression" {
                                    if let Some(field) = func.child_by_field_name("field") {
                                        (field.start_byte(), field.end_byte())
                                    } else {
                                        (func.start_byte(), func.end_byte())
                                    }
                                } else {
                                    (func.start_byte(), func.end_byte())
                                };

                                let start_char = rope.byte_to_char(start);
                                let line = rope.char_to_line(start_char);
                                let line_byte = rope.line_to_byte(line);
                                let col = start - line_byte;

                                references.push(Reference {
                                    file_path: file_path.to_str().unwrap_or("").to_string(),
                                    byte_start: start,
                                    byte_end: end,
                                    line: line + 1,
                                    column: col,
                                    context,
                                    match_id: None,
                                });
                            }
                        }
                    }
                }
            }

            // Recurse into arguments
            let mut cursor = node.walk();
            for child in node.children(&mut cursor) {
                find_identifiers_recursive(
                    child,
                    source,
                    rope,
                    target_symbol,
                    file_path,
                    scope_map,
                    references,
                );
            }
        }
        _ => {
            // Recurse into other nodes
            let mut cursor = node.walk();
            for child in node.children(&mut cursor) {
                find_identifiers_recursive(
                    child,
                    source,
                    rope,
                    target_symbol,
                    file_path,
                    scope_map,
                    references,
                );
            }
        }
    }
}

/// Extract context information from a reference node.
fn extract_context(node: tree_sitter::Node, _source: &[u8]) -> ReferenceContext {
    let parent = match node.parent() {
        Some(p) => p,
        None => return ReferenceContext::Identifier,
    };

    let parent_kind = parent.kind();

    match parent_kind {
        "call_expression" => ReferenceContext::FunctionCall {
            is_qualified: node.kind() == "scoped_identifier",
        },
        "use_declaration" => ReferenceContext::ImportStatement,
        "field_expression" => ReferenceContext::FieldAccess,
        "type_identifier" | "generic_type" | "type_arguments" => ReferenceContext::TypeReference,
        _ => ReferenceContext::Identifier,
    }
}

#[cfg(test)]
mod tests {
    use super::super::cross_file::{find_workspace_root, import_path_matches_target};
    use super::*;
    use std::io::Write;
    use tempfile::{NamedTempFile, TempDir};

    fn create_test_graph() -> CodeGraph {
        let temp_dir = TempDir::new().unwrap();
        let graph_path = temp_dir.path().join("test_graph.db");
        let graph = CodeGraph::open(&graph_path).unwrap();
        // Keep temp_dir alive by leaking it
        // Safe in tests - OS will clean up on process exit
        std::mem::forget(temp_dir);
        graph
    }

    #[test]
    fn test_find_same_file_function_references() {
        let source = r#"
fn helper() -> i32 {
    42
}

fn main() {
    let x = helper();
    let y = helper();
    println!("{}", helper());
}
"#;

        let mut temp_file = NamedTempFile::new().unwrap();
        write!(temp_file, "{}", source).unwrap();

        let graph = create_test_graph();
        let refs = find_rust_references(
            &graph,
            temp_file.path(),
            "helper",
            Some(RustSymbolKind::Function),
        )
        .unwrap();

        // Should find 3 references (not including definition)
        assert_eq!(refs.references.len(), 3);
    }

    #[test]
    fn test_qualified_path_references() {
        let source = r#"
fn helper() -> i32 {
    42
}

fn main() {
    let x = helper();              // Unqualified
    let y = crate::helper();       // Qualified - but this won't resolve in same file
}
"#;

        let mut temp_file = NamedTempFile::new().unwrap();
        write!(temp_file, "{}", source).unwrap();

        let graph = create_test_graph();
        let refs = find_rust_references(
            &graph,
            temp_file.path(),
            "helper",
            Some(RustSymbolKind::Function),
        )
        .unwrap();

        // Should find at least 1 unqualified reference
        assert!(!refs.references.is_empty());
    }

    #[test]
    fn test_no_references_to_symbol() {
        let source = r#"
fn unused() -> i32 {
    42
}

fn main() {
    println!("Hello");
}
"#;

        let mut temp_file = NamedTempFile::new().unwrap();
        write!(temp_file, "{}", source).unwrap();

        let graph = create_test_graph();
        let refs = find_rust_references(
            &graph,
            temp_file.path(),
            "unused",
            Some(RustSymbolKind::Function),
        )
        .unwrap();

        // Should find 0 references
        assert_eq!(refs.references.len(), 0);
    }

    #[test]
    fn test_symbol_not_found() {
        let source = "fn main() {}";

        let mut temp_file = NamedTempFile::new().unwrap();
        write!(temp_file, "{}", source).unwrap();

        let graph = create_test_graph();
        let result = find_rust_references(
            &graph,
            temp_file.path(),
            "nonexistent",
            Some(RustSymbolKind::Function),
        );

        assert!(result.is_err());
    }

    #[test]
    fn test_import_path_matches_target() {
        // Direct match
        assert!(import_path_matches_target("crate::utils", "crate::utils"));

        // Import is parent of target
        assert!(import_path_matches_target(
            "crate::utils",
            "crate::utils::helpers"
        ));

        // Target is parent of import
        assert!(import_path_matches_target(
            "crate::utils",
            "crate::utils::helper"
        ));

        // No match
        assert!(!import_path_matches_target("crate::utils", "crate::other"));
        assert!(!import_path_matches_target(
            "crate::utils::helpers",
            "crate::other"
        ));
    }

    #[test]
    fn test_find_workspace_root() {
        use std::fs;
        use tempfile::TempDir;

        // Create a temporary workspace with Cargo.toml
        let temp_dir = TempDir::new().unwrap();
        let workspace = temp_dir.path();

        let cargo_toml = workspace.join("Cargo.toml");
        fs::write(&cargo_toml, "[package]\nname = \"test\"\n").unwrap();

        let src_dir = workspace.join("src");
        fs::create_dir_all(&src_dir).unwrap();

        let main_rs = src_dir.join("main.rs");
        fs::write(&main_rs, "fn main() {}").unwrap();

        // Should find workspace root from main.rs
        let found_root = find_workspace_root(&main_rs).unwrap();
        assert_eq!(found_root, workspace);
    }

    #[test]
    fn test_find_workspace_root_finds_pyproject_toml() {
        use std::fs;
        use tempfile::TempDir;

        // Multi-language: a pyproject.toml-rooted project should resolve too.
        let temp_dir = TempDir::new().unwrap();
        let workspace = temp_dir.path();

        let pyproject = workspace.join("pyproject.toml");
        fs::write(&pyproject, "[project]\nname = \"test\"\n").unwrap();

        let pkg_dir = workspace.join("pkg");
        fs::create_dir_all(&pkg_dir).unwrap();

        let module = pkg_dir.join("m.py");
        fs::write(&module, "").unwrap();

        let found_root = find_workspace_root(&module).unwrap();
        assert_eq!(found_root, workspace);
    }

    #[test]
    fn test_shadowing_by_local_function() {
        // Test that a local function shadowing the target is NOT counted as a reference
        let source = r#"
fn helper() -> i32 {
    42
}

fn main() {
    let x = helper();  // Should find this (references top-level helper)

    fn helper() -> i32 {  // Local function shadows the top-level one
        99
    }

    let y = helper();  // Should NOT find this (references local helper)
}
"#;

        let mut temp_file = NamedTempFile::new().unwrap();
        write!(temp_file, "{}", source).unwrap();

        let graph = create_test_graph();
        let refs = find_rust_references(
            &graph,
            temp_file.path(),
            "helper",
            Some(RustSymbolKind::Function),
        )
        .unwrap();

        // Should find only 1 reference (the first call before shadowing)
        assert_eq!(refs.references.len(), 1);
    }

    #[test]
    fn test_shadowing_by_closure_parameter() {
        // Test that a closure parameter shadowing the target is NOT counted
        let source = r#"
fn helper() -> i32 {
    42
}

fn main() {
    let x = helper();  // Should find this

    let f = |helper: i32| helper + 1;  // 'helper' here is a parameter, not a call
    let y = f(10);

    let z = helper();  // Should find this too
}
"#;

        let mut temp_file = NamedTempFile::new().unwrap();
        write!(temp_file, "{}", source).unwrap();

        let graph = create_test_graph();
        let refs = find_rust_references(
            &graph,
            temp_file.path(),
            "helper",
            Some(RustSymbolKind::Function),
        )
        .unwrap();

        // Should find 2 references (both calls outside the closure)
        assert_eq!(refs.references.len(), 2);
    }

    #[test]
    fn test_nested_scope_shadowing() {
        // Test shadowing in nested scopes
        let source = r#"
fn helper() -> i32 {
    42
}

fn main() {
    let x = helper();  // Should find

    {
        let y = helper();  // Should find (still in scope)

        fn helper() -> i32 {  // Shadows only within this block
            99
        }

        let z = helper();  // Should NOT find (shadowed)
    }

    let w = helper();  // Should find (outside shadowing scope)
}
"#;

        let mut temp_file = NamedTempFile::new().unwrap();
        write!(temp_file, "{}", source).unwrap();

        let graph = create_test_graph();
        let refs = find_rust_references(
            &graph,
            temp_file.path(),
            "helper",
            Some(RustSymbolKind::Function),
        )
        .unwrap();

        // Should find 3 references (first, second, and fourth calls)
        assert_eq!(refs.references.len(), 3);
    }

    // Note: Full cross-file reference testing requires a real Cargo workspace
    // because find_cross_file_references() searches for Cargo.toml and .rs files.
    // Integration tests should be added to tests/ directory with proper workspace setup.
}
