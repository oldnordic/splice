//! Java import statement extraction.
//!
//! Uses tree-sitter-java to parse and extract `import` statements.

use crate::error::{Result, SpliceError};
use crate::ingest::imports::ImportKind;
use std::path::Path;

/// Extract import statements from a Java source file.
///
/// Uses tree-sitter-java to parse the file and extract all `import` statements,
/// including regular imports and static imports.
///
/// # Examples
///
/// ```
/// # use splice::ingest::imports::{extract_java_imports, ImportKind};
/// # use std::path::Path;
/// let source = b"import java.util.List;\n";
/// let imports = extract_java_imports(Path::new("test.java"), source)?;
/// assert_eq!(imports[0].import_kind, ImportKind::JavaImport);
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
pub fn extract_java_imports(path: &Path, source: &[u8]) -> Result<Vec<super::ImportFact>> {
    // Create tree-sitter parser for Java
    let mut parser = tree_sitter::Parser::new();
    parser
        .set_language(&tree_sitter_java::language())
        .map_err(|e| SpliceError::Parse {
            file: path.to_path_buf(),
            message: format!("Failed to set Java language: {:?}", e),
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
    // Check if this node is an import_declaration
    if node.kind() == "import_declaration" {
        if let Some(import) = extract_import_declaration(node, source) {
            imports.push(import);
        }
        return; // Don't recurse into import_declaration
    }

    // Recursively process children
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        extract_import_statements(child, source, imports);
    }
}

/// Extract a single import_declaration from a tree-sitter node.
fn extract_import_declaration(node: tree_sitter::Node, source: &[u8]) -> Option<super::ImportFact> {
    let byte_start = node.start_byte();
    let byte_end = node.end_byte();

    let mut is_static = false;
    let mut path = Vec::new();
    let mut is_glob = false;

    // Check for static modifier and extract the scoped_identifier path
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        if child.kind() == "static" {
            is_static = true;
        } else if child.kind() == "scoped_identifier" || child.kind() == "identifier" {
            // Extract the path segments
            extract_path_segments(child, source, &mut path);
        } else if child.kind() == "asterisk" {
            is_glob = true;
        }
    }

    if path.is_empty() {
        return None;
    }

    let import_kind = if is_static {
        ImportKind::JavaStaticImport
    } else {
        ImportKind::JavaImport
    };

    // For Java imports, the path includes the full class name
    // The last element could be a class name or * for wildcard
    Some(super::ImportFact {
        file_path: std::path::PathBuf::new(),
        import_kind,
        path,
        imported_names: Vec::new(), // Java imports don't specify local names
        is_glob,
        is_reexport: false,
        byte_span: (byte_start, byte_end),
    })
}

/// Extract path segments from a scoped_identifier or identifier.
fn extract_path_segments(node: tree_sitter::Node, source: &[u8], path: &mut Vec<String>) {
    let kind = node.kind();

    if kind == "identifier" {
        if let Ok(name) = node.utf8_text(source) {
            path.push(name.to_string());
        }
    } else if kind == "scoped_identifier" {
        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            if child.kind() == "identifier" || child.kind() == "scoped_identifier" {
                extract_path_segments(child, source, path);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_simple_import() {
        let source = b"import java.util.List;\n";
        let path = Path::new("test.java");
        let result = extract_java_imports(path, source);
        assert!(result.is_ok());
        let imports = result.unwrap();
        assert_eq!(imports.len(), 1);
        assert_eq!(imports[0].import_kind, ImportKind::JavaImport);
        assert_eq!(imports[0].path, vec!["java", "util", "List"]);
    }

    #[test]
    fn test_extract_static_import() {
        let source = b"import static java.lang.Math.PI;\n";
        let path = Path::new("test.java");
        let result = extract_java_imports(path, source);
        assert!(result.is_ok());
        let imports = result.unwrap();
        assert_eq!(imports.len(), 1);
        assert_eq!(imports[0].import_kind, ImportKind::JavaStaticImport);
        assert_eq!(imports[0].path, vec!["java", "lang", "Math", "PI"]);
    }

    #[test]
    fn test_extract_wildcard_import() {
        let source = b"import java.util.*;\n";
        let path = Path::new("test.java");
        let result = extract_java_imports(path, source);
        assert!(result.is_ok());
        let imports = result.unwrap();
        assert_eq!(imports.len(), 1);
        assert_eq!(imports[0].import_kind, ImportKind::JavaImport);
        assert!(imports[0].is_glob);
    }

    #[test]
    fn test_extract_static_wildcard_import() {
        let source = b"import static java.lang.Math.*;\n";
        let path = Path::new("test.java");
        let result = extract_java_imports(path, source);
        assert!(result.is_ok());
        let imports = result.unwrap();
        assert_eq!(imports.len(), 1);
        assert_eq!(imports[0].import_kind, ImportKind::JavaStaticImport);
        assert!(imports[0].is_glob);
    }

    #[test]
    fn test_extract_multiple_imports() {
        let source = b"import java.util.List;\nimport java.util.ArrayList;\n";
        let path = Path::new("test.java");
        let result = extract_java_imports(path, source);
        assert!(result.is_ok());
        let imports = result.unwrap();
        assert_eq!(imports.len(), 2);
        assert_eq!(imports[0].path, vec!["java", "util", "List"]);
        assert_eq!(imports[1].path, vec!["java", "util", "ArrayList"]);
    }

    #[test]
    fn test_import_has_byte_span() {
        let source = b"import java.util.List;\n";
        let path = Path::new("test.java");
        let result = extract_java_imports(path, source);
        assert!(result.is_ok());
        let imports = result.unwrap();
        assert_eq!(imports.len(), 1);
        // The import_declaration node ends before the trailing newline
        assert_eq!(imports[0].byte_span, (0, 22));
    }
}
