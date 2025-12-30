//! TypeScript import statement extraction.
//!
//! Uses tree-sitter-typescript to parse and extract ES6 `import` statements and
//! TypeScript-specific type-only imports.

use crate::error::{Result, SpliceError};
use crate::ingest::imports::ImportKind;
use std::path::Path;

/// Extract import statements from a TypeScript source file.
///
/// Uses tree-sitter-typescript to parse the file and extract:
/// - ES6 `import` statements (named, default, namespace, side-effect)
/// - Type-only imports (`import type { Foo } from 'bar'`)
/// - Type-only default imports (`import type Foo from 'bar'`)
///
/// Automatically detects file extension to use the correct parser:
/// - `.ts` files use `language_typescript()`
/// - `.tsx` files use `language_tsx()`
///
/// # Examples
///
/// ```
/// # use splice::ingest::imports::{extract_typescript_imports, ImportKind};
/// # use std::path::Path;
/// let source = b"import { foo } from 'bar';\n";
/// let imports = extract_typescript_imports(Path::new("test.ts"), source)?;
/// assert_eq!(imports[0].import_kind, ImportKind::JsImport);
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
pub fn extract_typescript_imports(path: &Path, source: &[u8]) -> Result<Vec<super::ImportFact>> {
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
    let kind = node.kind();

    // Check for ES6 import statements
    if kind == "import_statement" {
        if let Some(import) = extract_import_statement(node, source) {
            imports.push(import);
        }
        return; // Don't recurse into import_statement
    }

    // Check for CommonJS require calls
    if kind == "variable_declarator" {
        if let Some(import) = extract_require_call(node, source) {
            imports.push(import);
        }
        return; // Don't recurse into processed variable_declarator
    }

    // Recursively process children
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        extract_import_statements(child, source, imports);
    }
}

/// Extract a single import_statement from a tree-sitter node.
fn extract_import_statement(node: tree_sitter::Node, source: &[u8]) -> Option<super::ImportFact> {
    let byte_start = node.start_byte();
    let byte_end = node.end_byte();

    let mut source_path = String::new();
    let mut imported_names = Vec::new();
    let mut import_kind = ImportKind::JsImport;
    let mut is_glob = false;
    let mut is_type_only = false;

    // Check if this is a type-only import by looking at source text
    // The pattern is: "import type" followed by space or identifier
    if let Ok(text) = std::str::from_utf8(source) {
        // Get the text of this import statement
        let import_text = &text[byte_start..byte_end.min(text.len())];
        // Check for "import type " pattern at the start
        if import_text.trim_start().starts_with("import type ")
            || import_text.trim_start().starts_with("import\ttype\t")
            || import_text.trim_start().starts_with("import type{")
        {
            is_type_only = true;
        }
    }

    // Get the source string (it's a named field)
    if let Some(source_node) = node.child_by_field_name("source") {
        // The string node contains string_fragment
        for sub_child in source_node.children(&mut source_node.walk()) {
            if sub_child.kind() == "string_fragment" {
                if let Ok(text) = sub_child.utf8_text(source) {
                    source_path = text.to_string();
                }
            }
        }
    }

    // Get the import_clause (it's a direct child, not a named field)
    for child in node.children(&mut node.walk()) {
        if child.kind() == "import_clause" {
            // Determine the type of import and extract names
            for sub_child in child.children(&mut child.walk()) {
                match sub_child.kind() {
                    "identifier" => {
                        // Default import: `import foo from 'bar'`
                        if let Ok(name) = sub_child.utf8_text(source) {
                            imported_names.push(name.to_string());
                            import_kind = if is_type_only {
                                ImportKind::TsTypeDefaultImport
                            } else {
                                ImportKind::JsDefaultImport
                            };
                        }
                    }
                    "named_imports" => {
                        // Named imports: `import { foo, bar } from 'baz'`
                        import_kind = if is_type_only {
                            ImportKind::TsTypeImport
                        } else {
                            ImportKind::JsImport
                        };
                        for named in sub_child.children(&mut sub_child.walk()) {
                            if named.kind() == "import_specifier" {
                                // Get the local name (identifier after "as" if present)
                                let mut name_to_add = String::new();

                                if let Some(local_name_node) = named.child_by_field_name("alias") {
                                    if let Ok(name) = local_name_node.utf8_text(source) {
                                        name_to_add = name.to_string();
                                    }
                                } else if let Some(name_node) = named.child_by_field_name("name") {
                                    if let Ok(name) = name_node.utf8_text(source) {
                                        name_to_add = name.to_string();
                                    }
                                } else if name_to_add.is_empty() {
                                    // Fallback: iterate children
                                    for name_node in named.children(&mut named.walk()) {
                                        if name_node.kind() == "identifier"
                                            || name_node.kind() == "property_identifier"
                                        {
                                            if let Ok(name) = name_node.utf8_text(source) {
                                                name_to_add = name.to_string();
                                            }
                                        }
                                    }
                                }

                                if !name_to_add.is_empty() {
                                    imported_names.push(name_to_add);
                                }
                            }
                        }
                    }
                    "namespace_import" => {
                        // Namespace import: `import * as foo from 'bar'`
                        import_kind = ImportKind::JsNamespaceImport;
                        is_glob = true;
                        for name_node in sub_child.children(&mut sub_child.walk()) {
                            if name_node.kind() == "identifier" {
                                if let Ok(name) = name_node.utf8_text(source) {
                                    imported_names.push(name.to_string());
                                }
                            }
                        }
                    }
                    _ => {}
                }
            }
        }
    }

    // Handle side-effect imports: `import 'bar'`
    if imported_names.is_empty() && !source_path.is_empty() {
        import_kind = ImportKind::JsSideEffectImport;
    }

    if source_path.is_empty() {
        return None;
    }

    // Parse the source path to extract segments
    let path_parts: Vec<String> = source_path.split('/').map(|s| s.to_string()).collect();

    Some(super::ImportFact {
        file_path: std::path::PathBuf::new(),
        import_kind,
        path: path_parts,
        imported_names,
        is_glob,
        is_reexport: false,
        byte_span: (byte_start, byte_end),
    })
}

/// Extract CommonJS require() calls from a variable_declarator node.
fn extract_require_call(node: tree_sitter::Node, source: &[u8]) -> Option<super::ImportFact> {
    let byte_start = node.start_byte();
    let byte_end = node.end_byte();

    let mut source_path = String::new();
    let mut variable_name = String::new();

    // Look for pattern: const x = require('module')
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        if child.kind() == "identifier" {
            if let Ok(name) = child.utf8_text(source) {
                variable_name = name.to_string();
            }
        }
        if child.kind() == "call_expression" {
            // Check if this is a require() call
            for sub_child in child.children(&mut child.walk()) {
                if sub_child.kind() == "identifier" {
                    if let Ok(name) = sub_child.utf8_text(source) {
                        if name != "require" {
                            return None;
                        }
                    }
                }
                if sub_child.kind() == "arguments" {
                    for arg in sub_child.children(&mut sub_child.walk()) {
                        if arg.kind() == "string" {
                            if let Ok(text) = arg.utf8_text(source) {
                                // Remove quotes from the string
                                if text.len() > 2 {
                                    source_path = text[1..text.len() - 1].to_string();
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    if source_path.is_empty() {
        return None;
    }

    let path_parts: Vec<String> = source_path.split('/').map(|s| s.to_string()).collect();

    Some(super::ImportFact {
        file_path: std::path::PathBuf::new(),
        import_kind: ImportKind::JsRequire,
        path: path_parts,
        imported_names: vec![variable_name],
        is_glob: false,
        is_reexport: false,
        byte_span: (byte_start, byte_end),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_named_import() {
        let source = b"import { Component } from 'react';\n";
        let path = Path::new("test.ts");
        let result = extract_typescript_imports(path, source);
        assert!(result.is_ok());
        let imports = result.unwrap();
        assert_eq!(imports.len(), 1);
        assert_eq!(imports[0].import_kind, ImportKind::JsImport);
        assert_eq!(imports[0].path, vec!["react"]);
        assert_eq!(imports[0].imported_names, vec!["Component"]);
    }

    #[test]
    fn test_extract_default_import() {
        let source = b"import React from 'react';\n";
        let path = Path::new("test.ts");
        let result = extract_typescript_imports(path, source);
        assert!(result.is_ok());
        let imports = result.unwrap();
        assert_eq!(imports.len(), 1);
        assert_eq!(imports[0].import_kind, ImportKind::JsDefaultImport);
        assert_eq!(imports[0].imported_names, vec!["React"]);
    }

    #[test]
    fn test_extract_namespace_import() {
        let source = b"import * as utils from './utils';\n";
        let path = Path::new("test.ts");
        let result = extract_typescript_imports(path, source);
        assert!(result.is_ok());
        let imports = result.unwrap();
        assert_eq!(imports.len(), 1);
        assert_eq!(imports[0].import_kind, ImportKind::JsNamespaceImport);
        assert!(imports[0].is_glob);
        assert_eq!(imports[0].imported_names, vec!["utils"]);
    }

    #[test]
    fn test_extract_side_effect_import() {
        let source = b"import 'polyfills';\n";
        let path = Path::new("test.ts");
        let result = extract_typescript_imports(path, source);
        assert!(result.is_ok());
        let imports = result.unwrap();
        assert_eq!(imports.len(), 1);
        assert_eq!(imports[0].import_kind, ImportKind::JsSideEffectImport);
        assert_eq!(imports[0].path, vec!["polyfills"]);
    }

    #[test]
    fn test_extract_type_only_named_import() {
        let source = b"import type { User, Admin } from './types';\n";
        let path = Path::new("test.ts");
        let result = extract_typescript_imports(path, source);
        assert!(result.is_ok());
        let imports = result.unwrap();
        assert_eq!(imports.len(), 1);
        assert_eq!(imports[0].import_kind, ImportKind::TsTypeImport);
        assert_eq!(imports[0].path, vec![".", "types"]);
        assert_eq!(imports[0].imported_names.len(), 2);
    }

    #[test]
    fn test_extract_type_only_default_import() {
        let source = b"import type UserModel from './models';\n";
        let path = Path::new("test.ts");
        let result = extract_typescript_imports(path, source);
        assert!(result.is_ok());
        let imports = result.unwrap();
        assert_eq!(imports.len(), 1);
        assert_eq!(imports[0].import_kind, ImportKind::TsTypeDefaultImport);
        assert_eq!(imports[0].imported_names, vec!["UserModel"]);
    }

    #[test]
    fn test_extract_multiple_imports() {
        let source = b"import { Foo } from 'bar';\nimport type Baz from 'qux';\n";
        let path = Path::new("test.ts");
        let result = extract_typescript_imports(path, source);
        assert!(result.is_ok());
        let imports = result.unwrap();
        assert_eq!(imports.len(), 2);
    }

    #[test]
    fn test_extract_import_with_alias() {
        let source = b"import { Button as Btn } from './ui';\n";
        let path = Path::new("test.ts");
        let result = extract_typescript_imports(path, source);
        assert!(result.is_ok());
        let imports = result.unwrap();
        assert_eq!(imports.len(), 1);
        // Should extract the local alias (Btn)
        assert_eq!(imports[0].imported_names, vec!["Btn"]);
    }

    #[test]
    fn test_typescript_import_has_byte_span() {
        let source = b"import { Foo } from 'bar';\n";
        let path = Path::new("test.ts");
        let result = extract_typescript_imports(path, source);
        assert!(result.is_ok());
        let imports = result.unwrap();
        assert_eq!(imports.len(), 1);
        // Byte span is set
        assert!(imports[0].byte_span.0 < imports[0].byte_span.1);
    }
}
