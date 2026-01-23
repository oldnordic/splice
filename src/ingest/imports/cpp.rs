//! C/C++ include directive extraction.
//!
//! Uses tree-sitter-cpp to parse and extract `#include` directives.

use crate::error::{Result, SpliceError};
use crate::ingest::imports::ImportKind;
use std::path::Path;

/// Strip surrounding quotes or angle brackets from a string.
/// Character-safe for UTF-8: works with char indices, not byte offsets.
fn strip_quotes_or_angle_brackets(text: &str) -> String {
    let chars: Vec<char> = text.chars().collect();
    if chars.len() >= 2 {
        // Check for <...> or "..." or '...' pattern
        match (chars.first(), chars.last()) {
            (Some('<'), Some('>')) | (Some('"'), Some('"')) | (Some('\''), Some('\'')) => {
                chars[1..chars.len() - 1].iter().collect()
            }
            _ => text.to_string(),
        }
    } else {
        text.to_string()
    }
}

/// Extract include directives from a C/C++ source file.
///
/// Uses tree-sitter-cpp to parse the file and extract all `#include` directives.
///
/// # Examples
///
/// ```
/// # use splice::ingest::imports::{extract_cpp_imports, ImportKind};
/// # use std::path::Path;
/// let source = b"#include <stdio.h>\n";
/// let imports = extract_cpp_imports(Path::new("test.c"), source)?;
/// assert_eq!(imports[0].import_kind, ImportKind::CppSystemInclude);
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
pub fn extract_cpp_imports(path: &Path, source: &[u8]) -> Result<Vec<super::ImportFact>> {
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

    // Extract includes from the AST
    let mut imports = Vec::new();
    extract_include_statements(tree.root_node(), source, &mut imports);

    Ok(imports)
}

/// Extract include statements from AST nodes.
fn extract_include_statements(
    node: tree_sitter::Node,
    source: &[u8],
    imports: &mut Vec<super::ImportFact>,
) {
    // Check if this node is a preproc_include
    if node.kind() == "preproc_include" {
        if let Some(include) = extract_preproc_include(node, source) {
            imports.push(include);
        }
    }

    // Recursively process children
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        extract_include_statements(child, source, imports);
    }
}

/// Extract a single preproc_include from a tree-sitter node.
fn extract_preproc_include(node: tree_sitter::Node, source: &[u8]) -> Option<super::ImportFact> {
    let byte_start = node.start_byte();
    let byte_end = node.end_byte();

    // Check for system_lib_string (<...>) or string_literal ("...")
    let mut cursor = node.walk();
    let mut path = String::new();
    let mut is_system = false;

    for child in node.children(&mut cursor) {
        match child.kind() {
            "system_lib_string" => {
                // <stdio.h>
                is_system = true;
                if let Ok(text) = child.utf8_text(source) {
                    path = strip_quotes_or_angle_brackets(text);
                }
            }
            "string_literal" => {
                // "myheader.h"
                is_system = false;
                if let Ok(text) = child.utf8_text(source) {
                    path = strip_quotes_or_angle_brackets(text);
                }
            }
            _ => {}
        }
    }

    if path.is_empty() {
        return None;
    }

    let import_kind = if is_system {
        ImportKind::CppSystemInclude
    } else {
        ImportKind::CppLocalInclude
    };

    // For includes, the path is just the header name
    // We could normalize it (e.g., "boost/filesystem.hpp" -> ["boost", "filesystem.hpp"])
    let path_parts: Vec<String> = path.split('/').map(|s| s.to_string()).collect();

    Some(super::ImportFact {
        file_path: std::path::PathBuf::new(),
        import_kind,
        path: path_parts,
        imported_names: vec![path.clone()],
        is_glob: false,
        is_reexport: false,
        byte_span: (byte_start, byte_end),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_system_include() -> std::result::Result<(), Box<dyn std::error::Error>> {
        let source = b"#include <stdio.h>\n";
        let path = Path::new("test.c");
        let result = extract_cpp_imports(path, source);
        assert!(result.is_ok());
        let imports = result?;
        assert_eq!(imports.len(), 1);
        assert_eq!(imports[0].import_kind, ImportKind::CppSystemInclude);
        assert_eq!(imports[0].path, vec!["stdio.h"]);
        Ok(())
    }

    #[test]
    fn test_extract_local_include() -> std::result::Result<(), Box<dyn std::error::Error>> {
        let source = b"#include \"myheader.h\"\n";
        let path = Path::new("test.c");
        let result = extract_cpp_imports(path, source);
        assert!(result.is_ok());
        let imports = result?;
        assert_eq!(imports.len(), 1);
        assert_eq!(imports[0].import_kind, ImportKind::CppLocalInclude);
        assert_eq!(imports[0].path, vec!["myheader.h"]);
        Ok(())
    }
}
