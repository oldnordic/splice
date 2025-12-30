//! Python import statement extraction.
//!
//! Uses tree-sitter-python to parse and extract `import` and `from ... import` statements.

use crate::error::{Result, SpliceError};
use crate::ingest::imports::ImportKind;
use std::path::Path;

/// Extract import statements from a Python source file.
///
/// Uses tree-sitter-python to parse the file and extract all import statements.
///
/// # Examples
///
/// ```
/// # use splice::ingest::imports::{extract_python_imports, ImportKind};
/// # use std::path::Path;
/// let source = b"import os\n";
/// let imports = extract_python_imports(Path::new("test.py"), source)?;
/// assert_eq!(imports[0].import_kind, ImportKind::PythonImport);
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
pub fn extract_python_imports(path: &Path, source: &[u8]) -> Result<Vec<super::ImportFact>> {
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

    // Extract imports from the AST
    let mut imports = Vec::new();
    extract_import_statements(tree.root_node(), source, &mut imports);

    Ok(imports)
}

/// Extract import statements from AST nodes.
fn extract_import_statements(
    node: tree_sitter::Node,
    source: &[u8],
    imports: &mut Vec<super::ImportFact>,
) {
    match node.kind() {
        "import_statement" => {
            // import_statement can have multiple imports (e.g., `import os, sys`)
            if let Some(mut stmt_imports) = extract_import_statement(node, source) {
                imports.append(&mut stmt_imports);
            }
        }
        "import_from_statement" => {
            if let Some(import) = extract_import_from_statement(node, source) {
                imports.push(import);
            }
        }
        _ => {
            // Recursively process children
            let mut cursor = node.walk();
            for child in node.children(&mut cursor) {
                extract_import_statements(child, source, imports);
            }
        }
    }
}

/// Extract from import_statement: import os
/// Or: import os, sys (returns multiple)
/// Or: import os as operating_system
fn extract_import_statement(
    node: tree_sitter::Node,
    source: &[u8],
) -> Option<Vec<super::ImportFact>> {
    let byte_start = node.start_byte();
    let byte_end = node.end_byte();
    let mut result = Vec::new();

    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        match child.kind() {
            "dotted_name" => {
                // `import os.path` -> path = ["os", "path"], imported_name = "os"
                let path = extract_dotted_name_path(child, source);
                if !path.is_empty() {
                    let imported_name = path.first().unwrap().clone();
                    result.push(super::ImportFact {
                        file_path: std::path::PathBuf::new(),
                        import_kind: ImportKind::PythonImport,
                        path: path.clone(),
                        imported_names: vec![imported_name],
                        is_glob: false,
                        is_reexport: false,
                        byte_span: (byte_start, byte_end),
                    });
                }
            }
            "aliased_import" => {
                // `import os as operating_system`
                if let Some(import) = extract_aliased_import(
                    child,
                    source,
                    byte_start,
                    byte_end,
                    ImportKind::PythonImport,
                ) {
                    result.push(import);
                }
            }
            _ => {}
        }
    }

    if result.is_empty() {
        None
    } else {
        Some(result)
    }
}

/// Extract from import_from_statement: from os import path
/// Or: from os import path, environ
/// Or: from . import helper
/// Or: from os import *
fn extract_import_from_statement(
    node: tree_sitter::Node,
    source: &[u8],
) -> Option<super::ImportFact> {
    let byte_start = node.start_byte();
    let byte_end = node.end_byte();

    let mut cursor = node.walk();
    let mut path = Vec::new();
    let mut import_kind = ImportKind::PythonFrom;
    let mut imported_names = Vec::new();
    let mut is_glob = false;

    let mut stage = 0; // 0=before from, 1=after from before import, 2=after import

    for child in node.children(&mut cursor) {
        match child.kind() {
            "from" => {
                stage = 1;
                continue;
            }
            "import" => {
                stage = 2;
                continue;
            }
            "relative_import" => {
                // Extract both the dots and any module name
                let text = child.utf8_text(source).ok()?;
                let relative_level = text.matches('.').count();

                import_kind = match relative_level {
                    1 => ImportKind::PythonFromRelative,
                    2 => ImportKind::PythonFromParent,
                    _ => ImportKind::PythonFromAncestor,
                };

                // Check if relative_import contains a dotted_name (e.g., "..utils")
                let mut dot_cursor = child.walk();
                let mut has_module_name = false;
                for sub_child in child.children(&mut dot_cursor) {
                    if sub_child.kind() == "dotted_name" {
                        let module_path = extract_dotted_name_path(sub_child, source);
                        if !module_path.is_empty() {
                            // Add the dot prefix as first element, then module components
                            path.push(".".repeat(relative_level));
                            path.extend(module_path);
                            has_module_name = true;
                        }
                    }
                }
                if !has_module_name {
                    path.push(".".repeat(relative_level));
                }
            }
            "dotted_name" => {
                let name_path = extract_dotted_name_path(child, source);
                if name_path.is_empty() {
                    continue;
                }
                if stage == 1 {
                    // Module path after `from`, before `import`
                    path.extend(name_path);
                } else if stage == 2 {
                    // Imported names after `import`
                    // For dotted names, use the last component as the imported name
                    // e.g., `from os.path import join` -> "join"
                    imported_names.push(name_path.last().unwrap().clone());
                }
            }
            "aliased_import" => {
                if stage == 2 {
                    if let Some(alias) = extract_alias_name(child, source) {
                        imported_names.push(alias);
                    }
                }
            }
            "wildcard_import" => {
                imported_names.push("*".to_string());
                is_glob = true;
            }
            _ => {}
        }
    }

    Some(super::ImportFact {
        file_path: std::path::PathBuf::new(),
        import_kind,
        path,
        imported_names,
        is_glob,
        is_reexport: false,
        byte_span: (byte_start, byte_end),
    })
}

/// Extract path segments from a dotted_name node.
/// For `os.path.join`, returns ["os", "path", "join"]
fn extract_dotted_name_path(node: tree_sitter::Node, source: &[u8]) -> Vec<String> {
    let mut path = Vec::new();
    let mut cursor = node.walk();

    for child in node.children(&mut cursor) {
        if child.kind() == "identifier" {
            if let Ok(text) = child.utf8_text(source) {
                path.push(text.to_string());
            }
        }
    }

    path
}

/// Extract the alias from an aliased_import node.
/// For `os as operating_system`, returns "operating_system"
fn extract_alias_name(node: tree_sitter::Node, source: &[u8]) -> Option<String> {
    let mut cursor = node.walk();

    // aliased_import structure: dotted_name, "as", identifier
    // We want the identifier after "as"
    let children: Vec<_> = node.children(&mut cursor).collect();
    if children.len() >= 3 {
        // Third child should be the alias identifier
        if let Ok(alias) = children[2].utf8_text(source) {
            return Some(alias.to_string());
        }
    }

    None
}

/// Extract an aliased_import as a complete ImportFact.
fn extract_aliased_import(
    node: tree_sitter::Node,
    source: &[u8],
    byte_start: usize,
    byte_end: usize,
    kind: ImportKind,
) -> Option<super::ImportFact> {
    let mut cursor = node.walk();
    let children: Vec<_> = node.children(&mut cursor).collect();

    // First child is dotted_name (module path)
    let mut path = Vec::new();
    if let Some(dotted_name) = children.first() {
        path = extract_dotted_name_path(*dotted_name, source);
    }

    // Third child is the alias identifier
    let imported_name = if children.len() >= 3 {
        children[2].utf8_text(source).ok()?.to_string()
    } else {
        path.first()?.clone()
    };

    Some(super::ImportFact {
        file_path: std::path::PathBuf::new(),
        import_kind: kind,
        path,
        imported_names: vec![imported_name],
        is_glob: false,
        is_reexport: false,
        byte_span: (byte_start, byte_end),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_simple_import_basic() {
        let source = b"import os\n";
        let path = Path::new("test.py");
        let result = extract_python_imports(path, source);
        assert!(result.is_ok());
        let imports = result.unwrap();
        assert_eq!(imports.len(), 1);
        assert_eq!(imports[0].import_kind, ImportKind::PythonImport);
    }
}
