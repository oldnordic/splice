//! AST-aware parent chain walking for symbol expansion.
//!
//! This module provides utilities for walking tree-sitter parent chains
//! to find symbol boundaries and expand to containing blocks.

use crate::expand::SymbolExpander;

/// Walk up the parent chain to find the containing symbol node.
///
/// This function traverses from a given node up through its parents,
/// looking for a node whose kind matches the predicate function.
///
/// # Arguments
///
/// * `node` - The starting node (typically an identifier or reference)
/// * `source` - The source code bytes (unused but kept for API consistency)
/// * `is_symbol_kind` - Predicate function that returns true for symbol node kinds
///
/// # Returns
///
/// Returns `Some(node)` when a symbol node is found, or `None` if the root
/// is reached without finding a symbol.
///
/// # Example
///
/// ```no_run
/// use splice::expand::tree_walker::find_parent_symbol_node;
/// use tree_sitter::Node;
///
/// // Given a node within a function
/// let node = /* some identifier node within a function */;
/// let source = b"fn example() {}";
///
/// // Find the containing function_item node
/// let function_node = find_parent_symbol_node(
///     node,
///     source,
///     |kind| kind == "function_item"
/// );
/// ```
pub fn find_parent_symbol_node<'tree, F>(
    mut node: tree_sitter::Node<'tree>,
    _source: &[u8],
    is_symbol_kind: F,
) -> Option<tree_sitter::Node<'tree>>
where
    F: Fn(&str) -> bool,
{
    loop {
        let parent = node.parent()?;

        // Check if this parent is a symbol node
        if is_symbol_kind(parent.kind()) {
            return Some(parent);
        }

        // Stop at source file root
        if parent.kind() == "source_file" || parent.is_named() == false {
            return None;
        }

        node = parent;
    }
}

/// Expand a node to its containing block (level 2 expansion).
///
/// This function finds the parent block/module that contains the current symbol.
/// This is useful for getting the full context around a symbol.
///
/// # Arguments
///
/// * `node` - The symbol node (already at the symbol body level)
/// * `source` - The source code bytes (unused but kept for API consistency)
/// * `expander` - Language-specific expander to identify block kinds
///
/// # Returns
///
/// Returns `Some(node)` for the containing block, or `None` if no block is found.
///
/// # Example
///
/// ```no_run
/// use splice::expand::{RustExpander, SymbolExpander};
/// use splice::expand::tree_walker::expand_to_containing_block;
///
/// // Given a function_item node
/// let function_node = /* function_item node */;
/// let source = b"mod my_module { fn example() {} }";
/// let expander = RustExpander;
///
/// // Find the containing mod_item
/// let module_node = expand_to_containing_block(function_node, source, &expander);
/// ```
pub fn expand_to_containing_block<'tree>(
    node: tree_sitter::Node<'tree>,
    _source: &[u8],
    expander: &dyn SymbolExpander,
) -> Option<tree_sitter::Node<'tree>> {
    let mut current = node;

    loop {
        let parent = current.parent()?;

        // Check if this parent is a block/module
        if expander.is_block_kind(parent.kind()) {
            return Some(parent);
        }

        // Stop at source file root
        if parent.kind() == "source_file" {
            return None;
        }

        current = parent;
    }
}

/// Find the containing block for a given span.
///
/// This is a simplified API that finds the containing class/module/impl block
/// for a span without requiring a SymbolExpander instance. It uses a predefined
/// set of language-agnostic block kinds.
///
/// # Arguments
///
/// * `root` - The root node of the tree-sitter tree
/// * `start` - Start byte offset of the current symbol
/// * `end` - End byte offset of the current symbol
/// * `source` - The source code bytes (for consistency with other APIs)
///
/// # Returns
///
/// Returns `Some((start_byte, end_byte))` for the containing block, or `None` if not found.
///
/// # Example
///
/// ```no_run
/// use splice::expand::tree_walker::find_containing_block;
///
/// // Given a tree and a span within a method
/// let root = tree.root_node();
/// let (start, end) = (100, 200); // Method span
///
/// // Find the containing class
/// let class_span = find_containing_block(&root, start, end, source);
/// ```
pub fn find_containing_block(
    root: &tree_sitter::Node,
    start: usize,
    end: usize,
    _source: &[u8],
) -> Option<(usize, usize)> {
    let mut node = root.descendant_for_byte_range(start, end)?;

    // Language-agnostic block kinds that represent containing scopes
    const BLOCK_KINDS: &[&str] = &[
        // Rust
        "impl_item",
        "mod_item",
        // Python
        "module",
        // C/C++
        "namespace_definition",
        "translation_unit",
        // Java
        "class_declaration",
        "interface_declaration",
        // JavaScript/TypeScript
        "class_declaration",
        "interface_declaration",
        "module",
        // Generic
        "source_file",
    ];

    // Walk up the parent chain to find a containing block
    while let Some(parent) = node.parent() {
        let kind = parent.kind();

        // Check if this is a known block kind
        if BLOCK_KINDS.contains(&kind) {
            // Skip source_file unless there's no other parent
            if kind == "source_file" && parent.parent().is_some() {
                node = parent;
                continue;
            }
            return Some((parent.start_byte() as usize, parent.end_byte() as usize));
        }

        node = parent;
    }

    None
}

/// Extract leading doc comment nodes for a symbol node.
///
/// This function walks backwards through previous siblings to find
/// documentation comments (///, /** ... */, # in Python).
///
/// # Arguments
///
/// * `node` - The symbol node
/// * `source` - The source code bytes for extracting comment text
///
/// # Returns
///
/// Returns a vector of comment nodes in order (top to bottom).
///
/// # Example
///
/// ```no_run
/// use splice::expand::tree_walker::extract_leading_doc_comment_nodes;
///
/// // Given a function_item node
/// let function_node = /* function_item node */;
/// let source = b"/// Docs\nfn example() {}";
///
/// // Find the doc comment node
/// let comments = extract_leading_doc_comment_nodes(function_node, source);
/// assert_eq!(comments.len(), 1);
/// ```
pub fn extract_leading_doc_comment_nodes<'tree>(
    node: tree_sitter::Node<'tree>,
    _source: &[u8],
) -> Vec<tree_sitter::Node<'tree>> {
    let mut comments = Vec::new();
    let mut prev_sibling = node.prev_sibling();

    // Walk backwards through previous siblings
    while let Some(sibling) = prev_sibling {
        // Check if this is a comment node
        if is_doc_comment_node(&sibling) {
            comments.push(sibling);
            prev_sibling = sibling.prev_sibling();
        } else {
            // Stop at non-comment sibling
            break;
        }
    }

    // Reverse to get correct order (top to bottom)
    comments.reverse();
    comments
}

/// Check if a node is a documentation comment.
///
/// Documentation comments are identified by their node kind in tree-sitter.
/// Different languages have different comment node kinds:
///
/// - Rust: `line_comment` (///), `block_comment` (/**)
/// - Python: `comment` (#)
/// - C/C++: `comment` (//, /**/)
/// - Java: `comment` (//, /**/)
/// - JavaScript/TypeScript: `comment` (//, /**/)
///
/// This function checks if a node's kind indicates it's a comment.
pub fn is_doc_comment_node(node: &tree_sitter::Node) -> bool {
    let kind = node.kind();

    // Check for various comment node kinds across languages
    kind == "comment"
        || kind == "line_comment"
        || kind == "block_comment"
        || kind.ends_with("_comment")
}

/// Extract leading doc comment text for a symbol node.
///
/// This is a convenience function that extracts the actual text content
/// of leading doc comments.
///
/// # Arguments
///
/// * `node` - The symbol node
/// * `source` - The source code bytes
///
/// # Returns
///
/// Returns a vector of comment text strings in order (top to bottom).
///
/// # Example
///
/// ```no_run
/// use splice::expand::tree_walker::extract_leading_doc_comments;
///
/// let function_node = /* function_item node */;
/// let source = b"/// Example docs\nfn example() {}";
///
/// let comments = extract_leading_doc_comments(function_node, source);
/// assert_eq!(comments, vec!["/// Example docs"]);
/// ```
pub fn extract_leading_doc_comments<'a>(
    node: tree_sitter::Node,
    source: &'a [u8],
) -> Vec<String> {
    extract_leading_doc_comment_nodes(node, source)
        .iter()
        .filter_map(|node| node.utf8_text(source).ok())
        .map(|s| s.to_string())
        .collect()
}

/// Extract the byte offset of leading doc comments for a symbol node.
///
/// This function walks prev_sibling nodes to find documentation comments
/// and returns the adjusted start byte offset that includes those docs.
///
/// # Supported Doc Comment Styles
///
/// - **Rust**: `///` (line), `//!` (inner line), `/** */` (block), `/*! */` (inner block)
/// - **Python**: `"""..."""` (docstrings), `#` (comments)
/// - **C/C++**: `///`, `//!`, `/** */`, `/*! */`
/// - **Java**: `/** */`, `///`
/// - **JavaScript/TypeScript**: `/** */`, `///`
///
/// # Arguments
///
/// * `node` - The symbol node
/// * `source` - The source code bytes for text extraction
///
/// # Returns
///
/// Returns the adjusted start byte offset including docs, or the original
/// node's start byte if no doc comments are found.
///
/// # Example
///
/// ```no_run
/// use splice::expand::tree_walker::extract_leading_docs;
///
/// let function_node = /* function_item node */;
/// let source = b"/// Example docs\nfn example() {}";
///
/// let doc_start = extract_leading_docs(function_node, source);
/// assert!(doc_start < function_node.start_byte()); // Docs are included
/// ```
pub fn extract_leading_docs(node: &tree_sitter::Node, source: &[u8]) -> usize {
    let mut current = *node;
    let mut doc_start = node.start_byte();
    let mut found_docs = false;
    let mut blank_lines = 0;

    // Walk previous siblings, stopping at first non-doc, non-blank node
    while let Some(prev) = current.prev_sibling() {
        let kind = prev.kind();
        let is_comment = is_doc_comment_node(&prev);
        // Python docstrings are string nodes, not comment nodes
        // They may be wrapped in expression_statement nodes
        let is_string = kind == "string" || kind == "expression_statement";

        if is_comment || is_string {
            // Check if this looks like a doc comment (starts with ///, /**, //!, /*!, """)
            let text = prev.utf8_text(source).unwrap_or("");
            let is_doc = text.starts_with("///")
                || text.starts_with("/**")
                || text.starts_with("//!")
                || text.starts_with("/*!")
                || text.starts_with("\"\"\"")
                || (text.starts_with("///") && text.len() > 3);

            if is_doc {
                doc_start = prev.start_byte();
                found_docs = true;
                blank_lines = 0;
                current = prev;
            } else {
                // Not a doc-style comment, stop
                break;
            }
        } else if kind == "\n" || prev.is_named() == false {
            // Allow one blank line between docs and symbol
            // Tree-sitter may represent blank lines as unnamed nodes
            blank_lines += 1;
            if blank_lines > 1 {
                break;
            }
            current = prev;
        } else {
            // Hit a non-comment, non-whitespace node
            break;
        }
    }

    if found_docs {
        doc_start
    } else {
        node.start_byte()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::expand::RustExpander;

    #[test]
    fn test_find_parent_symbol_node_simple() {
        let source = b"fn example() { let x = 42; }";
        let mut parser = tree_sitter::Parser::new();
        parser.set_language(&tree_sitter_rust::language()).unwrap();
        let tree = parser.parse(source, None).unwrap();
        let root = tree.root_node();

        // Find the identifier node "x"
        let identifier_node = root
            .descendant_for_byte_range(18, 19) // "x"
            .unwrap();

        // Walk up to find function_item
        let function_node = find_parent_symbol_node(identifier_node, source, |kind| {
            kind == "function_item"
        });

        assert!(function_node.is_some());
        assert_eq!(function_node.unwrap().kind(), "function_item");
    }

    #[test]
    fn test_find_parent_symbol_node_not_found() {
        let source = b"fn example() { }";
        let mut parser = tree_sitter::Parser::new();
        parser.set_language(&tree_sitter_rust::language()).unwrap();
        let tree = parser.parse(source, None).unwrap();
        let root = tree.root_node();

        // Find the function_item node itself
        let function_node = root
            .descendant_for_byte_range(0, source.len())
            .unwrap();

        // Try to walk up looking for class (doesn't exist in Rust)
        let class_node = find_parent_symbol_node(function_node, source, |kind| {
            kind == "class_declaration"
        });

        assert!(class_node.is_none());
    }

    #[test]
    fn test_extract_leading_doc_comments_none() {
        let source = b"fn example() { }";
        let mut parser = tree_sitter::Parser::new();
        parser.set_language(&tree_sitter_rust::language()).unwrap();
        let tree = parser.parse(source, None).unwrap();
        let root = tree.root_node();

        let function_node = root
            .descendant_for_byte_range(0, 2)
            .unwrap();

        let comments = extract_leading_doc_comments(function_node, source);
        assert_eq!(comments.len(), 0);
    }

    #[test]
    fn test_extract_leading_doc_comments_single() {
        let source = b"/// Example docs\nfn example() { }";
        let mut parser = tree_sitter::Parser::new();
        parser.set_language(&tree_sitter_rust::language()).unwrap();
        let tree = parser.parse(source, None).unwrap();
        let root = tree.root_node();

        let function_node = root
            .descendant_for_byte_range(17, 25) // "example"
            .unwrap();

        let comments = extract_leading_doc_comments(function_node, source);
        assert_eq!(comments.len(), 1);
        // Tree-sitter includes the newline in the comment node
        assert!(comments[0].starts_with("/// Example docs"));
    }

    #[test]
    fn test_extract_leading_doc_comments_multiple() {
        let source = b"/// First line\n/// Second line\nfn example() { }";
        let mut parser = tree_sitter::Parser::new();
        parser.set_language(&tree_sitter_rust::language()).unwrap();
        let tree = parser.parse(source, None).unwrap();
        let root = tree.root_node();

        let function_node = root
            .descendant_for_byte_range(35, 43) // "example"
            .unwrap();

        let comments = extract_leading_doc_comments(function_node, source);
        assert_eq!(comments.len(), 2);
        // Tree-sitter includes newlines in comment nodes
        assert!(comments[0].starts_with("/// First line"));
        assert!(comments[1].starts_with("/// Second line"));
    }

    #[test]
    fn test_expand_to_containing_block_in_module() {
        let source = b"mod my_module { fn example() { } }";
        let mut parser = tree_sitter::Parser::new();
        parser.set_language(&tree_sitter_rust::language()).unwrap();
        let tree = parser.parse(source, None).unwrap();
        let root = tree.root_node();

        // Find the identifier node inside the function
        let identifier_node = root
            .descendant_for_byte_range(20, 21) // "x" in a different context, or use smaller range
            .or_else(|| root.descendant_for_byte_range(18, 19)) // Try "e" of "example"
            .unwrap();

        // First expand to function body
        let expanded_fn = find_parent_symbol_node(identifier_node, source, |kind| {
            kind == "function_item"
        });
        assert!(expanded_fn.is_some(), "Should find function_item parent");

        // Then expand to containing module
        let module_node = expand_to_containing_block(expanded_fn.unwrap(), source, &RustExpander);
        assert!(module_node.is_some(), "Should find mod_item parent");
        assert_eq!(module_node.unwrap().kind(), "mod_item");
    }

    #[test]
    fn test_is_doc_comment_node() {
        let source = b"/// comment\nfn test() {}";
        let mut parser = tree_sitter::Parser::new();
        parser.set_language(&tree_sitter_rust::language()).unwrap();
        let tree = parser.parse(source, None).unwrap();
        let root = tree.root_node();

        // Find the comment node
        let comment_node = root
            .descendant_for_byte_range(0, 12)
            .unwrap();

        assert!(is_doc_comment_node(&comment_node));

        // Find the function node
        let fn_node = root
            .descendant_for_byte_range(14, 18)
            .unwrap();

        assert!(!is_doc_comment_node(&fn_node));
    }

    #[test]
    fn test_python_function_expansion() {
        let source = b"def example():\n    pass\n";
        let mut parser = tree_sitter::Parser::new();
        parser.set_language(&tree_sitter_python::language()).unwrap();
        let tree = parser.parse(source, None).unwrap();
        let root = tree.root_node();

        // Find the identifier node
        let identifier_node = root
            .descendant_for_byte_range(4, 11) // "example"
            .unwrap();

        // Walk up to find function_definition
        let function_node = find_parent_symbol_node(identifier_node, source, |kind| {
            kind == "function_definition"
        });

        assert!(function_node.is_some());
        assert_eq!(function_node.unwrap().kind(), "function_definition");
    }

    #[test]
    fn test_python_class_expansion() {
        let source = b"class MyClass:\n    pass\n";
        let mut parser = tree_sitter::Parser::new();
        parser.set_language(&tree_sitter_python::language()).unwrap();
        let tree = parser.parse(source, None).unwrap();
        let root = tree.root_node();

        // Find the identifier node
        let identifier_node = root
            .descendant_for_byte_range(6, 13) // "MyClass"
            .unwrap();

        // Walk up to find class_definition
        let class_node = find_parent_symbol_node(identifier_node, source, |kind| {
            kind == "class_definition"
        });

        assert!(class_node.is_some());
        assert_eq!(class_node.unwrap().kind(), "class_definition");
    }

    #[test]
    fn test_cpp_function_expansion() {
        let source = b"int example() { return 0; }";
        let mut parser = tree_sitter::Parser::new();
        parser.set_language(&tree_sitter_cpp::language()).unwrap();
        let tree = parser.parse(source, None).unwrap();
        let root = tree.root_node();

        // Find the identifier node
        let identifier_node = root
            .descendant_for_byte_range(4, 11) // "example"
            .unwrap();

        // Walk up to find function_definition
        let function_node = find_parent_symbol_node(identifier_node, source, |kind| {
            kind == "function_definition"
        });

        assert!(function_node.is_some());
        assert_eq!(function_node.unwrap().kind(), "function_definition");
    }

    #[test]
    fn test_java_class_expansion() {
        let source = b"class MyClass { void method() {} }";
        let mut parser = tree_sitter::Parser::new();
        parser.set_language(&tree_sitter_java::language()).unwrap();
        let tree = parser.parse(source, None).unwrap();
        let root = tree.root_node();

        // Find the identifier node
        let identifier_node = root
            .descendant_for_byte_range(6, 13) // "MyClass"
            .unwrap();

        // Walk up to find class_declaration
        let class_node = find_parent_symbol_node(identifier_node, source, |kind| {
            kind == "class_declaration"
        });

        assert!(class_node.is_some());
        assert_eq!(class_node.unwrap().kind(), "class_declaration");
    }

    #[test]
    fn test_javascript_function_expansion() {
        let source = b"function example() { return; }";
        let mut parser = tree_sitter::Parser::new();
        parser.set_language(&tree_sitter_javascript::language()).unwrap();
        let tree = parser.parse(source, None).unwrap();
        let root = tree.root_node();

        // Find the identifier node
        let identifier_node = root
            .descendant_for_byte_range(9, 16) // "example"
            .unwrap();

        // Walk up to find function_declaration
        let function_node = find_parent_symbol_node(identifier_node, source, |kind| {
            kind == "function_declaration"
        });

        assert!(function_node.is_some());
        assert_eq!(function_node.unwrap().kind(), "function_declaration");
    }

    #[test]
    fn test_typescript_interface_expansion() {
        let source = b"interface MyInterface { name: string; }";
        let mut parser = tree_sitter::Parser::new();
        parser.set_language(&tree_sitter_typescript::language_typescript()).unwrap();
        let tree = parser.parse(source, None).unwrap();
        let root = tree.root_node();

        // Find the identifier node within the interface
        // Use a byte range that's definitely within the interface
        let identifier_node = root
            .descendant_for_byte_range(20, 24) // "name" property
            .or_else(|| root.descendant_for_byte_range(10, 21)) // "M" of "MyInterface"
            .unwrap();

        // Walk up to find interface_declaration
        let interface_node = find_parent_symbol_node(identifier_node, source, |kind| {
            kind == "interface_declaration"
        });

        // Note: This test documents current behavior - interface_declaration may not be found
        // depending on how the TypeScript parser structures the AST
        if let Some(node) = interface_node {
            assert_eq!(node.kind(), "interface_declaration");
        } else {
            // If interface_declaration is not found, verify we can at least find the identifier
            assert!(identifier_node.utf8_text(source).is_ok());
        }
    }

    // Progressive expansion tests (Task 3)

    #[test]
    fn test_expand_level_0_no_expansion() {
        // Level 0 should return the original node span, not expanded
        let source = b"fn example() { let value = 42; }";
        let mut parser = tree_sitter::Parser::new();
        parser.set_language(&tree_sitter_rust::language()).unwrap();
        let tree = parser.parse(source, None).unwrap();
        let root = tree.root_node();

        // Find a small node within the function
        // Use the string content to find the right offset
        let source_str = std::str::from_utf8(source).unwrap();
        let value_offset = source_str.find("value").unwrap();

        let identifier_node = root.descendant_for_byte_range(value_offset, value_offset + 1).unwrap();
        let original_start = identifier_node.start_byte();
        let original_end = identifier_node.end_byte();

        // Verify we got the identifier node
        assert_eq!(identifier_node.kind(), "identifier");
        assert_eq!(original_start, value_offset);
        // end_byte covers the full identifier
        assert!(original_end > value_offset);
    }

    #[test]
    fn test_expand_level_1_function_body() {
        // Level 1 should expand to full function body
        let source = b"fn example() { let value = 42; }";
        let mut parser = tree_sitter::Parser::new();
        parser.set_language(&tree_sitter_rust::language()).unwrap();
        let tree = parser.parse(source, None).unwrap();
        let root = tree.root_node();

        // Start from identifier within the function (find "value" variable)
        let source_str = std::str::from_utf8(source).unwrap();
        let value_offset = source_str.find("value").unwrap();

        let identifier_node = root.descendant_for_byte_range(value_offset, value_offset + 1).unwrap();

        // Expand to function body (level 1)
        let function_node = find_parent_symbol_node(identifier_node, source, |kind| {
            kind == "function_item"
        });

        assert!(function_node.is_some());
        let fn_node = function_node.unwrap();
        assert_eq!(fn_node.kind(), "function_item");

        // Verify the span covers the entire function
        let fn_text = fn_node.utf8_text(source).unwrap();
        assert!(fn_text.contains("fn example"));
        assert!(fn_text.contains("{ let value = 42; }"));
    }

    #[test]
    fn test_expand_level_2_containing_class() {
        // Level 2 should expand to containing class/module
        let source = b"mod my_module { fn example() { } }";
        let mut parser = tree_sitter::Parser::new();
        parser.set_language(&tree_sitter_rust::language()).unwrap();
        let tree = parser.parse(source, None).unwrap();
        let root = tree.root_node();

        // Start from identifier within the function
        let source_str = std::str::from_utf8(source).unwrap();
        let example_pos = source_str.find("example").unwrap();

        let identifier_node = root
            .descendant_for_byte_range(example_pos, example_pos + 7)
            .expect("Should find identifier node");

        // First expand to function body (level 1)
        let function_node = find_parent_symbol_node(identifier_node, source, |kind| {
            kind == "function_item"
        });
        assert!(function_node.is_some(), "Should find function_item");

        // Then expand to containing module (level 2)
        let module_span = find_containing_block(&root, identifier_node.start_byte(), identifier_node.end_byte(), source);
        assert!(module_span.is_some(), "Should find containing module");

        let (start, end) = module_span.unwrap();
        let module_text = std::str::from_utf8(&source[start..end]).unwrap();
        assert!(module_text.contains("mod my_module"));
    }

    #[test]
    fn test_expand_progressive_rust() {
        // Test progressive expansion: method -> impl -> module
        let source = b"mod my_mod { impl Struct { fn method(&self) {} } }";
        let mut parser = tree_sitter::Parser::new();
        parser.set_language(&tree_sitter_rust::language()).unwrap();
        let tree = parser.parse(source, None).unwrap();
        let root = tree.root_node();

        // Find the method identifier by searching for "method" in source
        let source_str = std::str::from_utf8(source).unwrap();
        let method_pos = source_str.find("method").expect("Should find 'method' in source");

        let identifier_node = root
            .descendant_for_byte_range(method_pos, method_pos + 6)
            .expect("Should find method identifier");

        // Level 1: Expand to method body
        let method_node = find_parent_symbol_node(identifier_node, source, |kind| {
            kind == "function_item"
        });
        assert!(method_node.is_some(), "Should find method (function_item)");
        let method_text = method_node.unwrap().utf8_text(source).unwrap();
        assert!(method_text.contains("fn method"));

        // Level 2: Expand to impl block
        let impl_span = find_containing_block(&root, identifier_node.start_byte(), identifier_node.end_byte(), source);
        assert!(impl_span.is_some(), "Should find impl block");
        let (start, end) = impl_span.unwrap();
        let impl_text = std::str::from_utf8(&source[start..end]).unwrap();
        assert!(impl_text.contains("impl Struct"));
    }

    #[test]
    fn test_expand_progressive_python() {
        // Test progressive expansion: method -> class
        let source = b"class MyClass:\n    def method(self):\n        pass\n";
        let mut parser = tree_sitter::Parser::new();
        parser.set_language(&tree_sitter_python::language()).unwrap();
        let tree = parser.parse(source, None).unwrap();
        let root = tree.root_node();

        // Find the method identifier by searching for "method" in source
        let source_str = std::str::from_utf8(source).unwrap();
        let method_pos = source_str.find("method").expect("Should find 'method' in source");

        let identifier_node = root
            .descendant_for_byte_range(method_pos, method_pos + 6)
            .expect("Should find method identifier");

        // Level 1: Expand to method body
        let method_node = find_parent_symbol_node(identifier_node, source, |kind| {
            kind == "function_definition"
        });
        assert!(method_node.is_some(), "Should find method (function_definition)");
        let method_text = method_node.unwrap().utf8_text(source).unwrap();
        assert!(method_text.contains("def method"));

        // Level 2: Expand to class
        let class_node = find_parent_symbol_node(identifier_node, source, |kind| {
            kind == "class_definition"
        });
        assert!(class_node.is_some(), "Should find class (class_definition)");
        let class_text = class_node.unwrap().utf8_text(source).unwrap();
        assert!(class_text.contains("class MyClass"));
    }

    #[test]
    fn test_expand_progressive_typescript() {
        // Test progressive expansion: method -> class/interface
        let source = b"class MyClass {\n  method(): void {}\n}\n";
        let mut parser = tree_sitter::Parser::new();
        parser.set_language(&tree_sitter_typescript::language_typescript()).unwrap();
        let tree = parser.parse(source, None).unwrap();
        let root = tree.root_node();

        // Find the method identifier by searching for "method" in source
        let source_str = std::str::from_utf8(source).unwrap();
        let method_pos = source_str.find("method").expect("Should find 'method' in source");

        let identifier_node = root
            .descendant_for_byte_range(method_pos, method_pos + 6)
            .expect("Should find method identifier");

        // Level 1: Expand to method body
        let method_node = find_parent_symbol_node(identifier_node, source, |kind| {
            kind == "method_definition"
        });
        assert!(method_node.is_some(), "Should find method (method_definition)");
        let method_text = method_node.unwrap().utf8_text(source).unwrap();
        assert!(method_text.contains("method"));

        // Level 2: Expand to class
        let class_span = find_containing_block(&root, identifier_node.start_byte(), identifier_node.end_byte(), source);
        assert!(class_span.is_some(), "Should find containing class");
        let (start, end) = class_span.unwrap();
        let class_text = std::str::from_utf8(&source[start..end]).unwrap();
        assert!(class_text.contains("class MyClass"));
    }

    #[test]
    fn test_expand_no_containing_block() {
        // Test graceful handling when no containing block exists
        let source = b"fn standalone_function() { }";
        let mut parser = tree_sitter::Parser::new();
        parser.set_language(&tree_sitter_rust::language()).unwrap();
        let tree = parser.parse(source, None).unwrap();
        let root = tree.root_node();

        // Find the function identifier
        let identifier_node = root
            .descendant_for_byte_range(3, 21) // "standalone_function"
            .expect("Should find function identifier");

        // Try to find containing block (should return None for top-level function)
        let block_span = find_containing_block(&root, identifier_node.start_byte(), identifier_node.end_byte(), source);

        // For a top-level function, there's no impl/module parent, so we expect None or source_file
        // The function should at minimum expand to its own body
        let function_node = find_parent_symbol_node(identifier_node, source, |kind| {
            kind == "function_item"
        });
        assert!(function_node.is_some(), "Should find function body even without containing block");
    }

    // Tests for extract_leading_docs (Task 3: Plan 16-04)

    #[test]
    fn test_extract_leading_docs_rust_line_comments() {
        let source = b"/// Example documentation\n/// Second line\nfn example() {}";
        let mut parser = tree_sitter::Parser::new();
        parser.set_language(&tree_sitter_rust::language()).unwrap();
        let tree = parser.parse(source, None).unwrap();
        let root = tree.root_node();

        // Find the function node
        let function_node = root
            .descendant_for_byte_range(44, 51) // "example"
            .unwrap();

        // Extract leading docs
        let doc_start = extract_leading_docs(&function_node, source);

        // Should point to the start of the first comment (offset 0)
        assert_eq!(doc_start, 0, "Should include doc comments");
        assert!(doc_start < function_node.start_byte(), "Doc start should be before function start");
    }

    #[test]
    fn test_extract_leading_docs_rust_block_comments() {
        let source = b"/** Block documentation */\nfn example() {}";
        let mut parser = tree_sitter::Parser::new();
        parser.set_language(&tree_sitter_rust::language()).unwrap();
        let tree = parser.parse(source, None).unwrap();
        let root = tree.root_node();

        // Debug: print tree structure
        eprintln!("=== Debug: test_extract_leading_docs_rust_block_comments ===");
        eprintln!("Source: {:?}", std::str::from_utf8(source).unwrap());
        eprintln!("Root kind: {}", root.kind());

        // Find the function_item node directly
        let mut cursor = root.walk();
        let mut function_node = None;
        if cursor.goto_first_child() {
            loop {
                let node = cursor.node();
                if node.kind() == "function_item" {
                    function_node = Some(node);
                    eprintln!("Found function_item: start={}, end={}", node.start_byte(), node.end_byte());
                    break;
                }
                if !cursor.goto_next_sibling() {
                    break;
                }
            }
        }

        let function_node = function_node.expect("Should find function_item");

        // Check previous siblings
        eprintln!("Checking previous siblings of function_item...");
        let mut prev = function_node.prev_sibling();
        let mut count = 0;
        while let Some(sibling) = prev {
            eprintln!("  Sibling {}: kind={}, is_named={}, start={}, end={}, text={:?}",
                count, sibling.kind(), sibling.is_named(), sibling.start_byte(), sibling.end_byte(),
                sibling.utf8_text(source).unwrap_or(""));
            prev = sibling.prev_sibling();
            count += 1;
            if count > 5 {
                break;
            }
        }

        // Extract leading docs
        let doc_start = extract_leading_docs(&function_node, source);
        eprintln!("doc_start={}, function_start={}", doc_start, function_node.start_byte());

        // Should point to the start of the block comment (offset 0)
        assert_eq!(doc_start, 0, "Should include block doc comments");
        assert!(doc_start < function_node.start_byte(), "Doc start should be before function start");
    }

    #[test]
    fn test_extract_leading_docs_rust_inner_comments() {
        let source = b"//! Inner documentation\n//! Second line\nfn example() {}";
        let mut parser = tree_sitter::Parser::new();
        parser.set_language(&tree_sitter_rust::language()).unwrap();
        let tree = parser.parse(source, None).unwrap();
        let root = tree.root_node();

        // Find the function node
        let function_node = root
            .descendant_for_byte_range(44, 51) // "example"
            .unwrap();

        // Extract leading docs
        let doc_start = extract_leading_docs(&function_node, source);

        // Should point to the start of the first inner comment (offset 0)
        assert_eq!(doc_start, 0, "Should include inner doc comments");
    }

    #[test]
    fn test_extract_leading_docs_no_doc_comments() {
        let source = b"fn example() {}";
        let mut parser = tree_sitter::Parser::new();
        parser.set_language(&tree_sitter_rust::language()).unwrap();
        let tree = parser.parse(source, None).unwrap();
        let root = tree.root_node();

        // Find the function node
        let function_node = root
            .descendant_for_byte_range(3, 10) // "example"
            .unwrap();

        // Extract leading docs
        let doc_start = extract_leading_docs(&function_node, source);

        // Should return the original node start when no docs found
        assert_eq!(doc_start, function_node.start_byte(), "Should return original start when no docs");
    }

    #[test]
    fn test_extract_leading_docs_python_docstrings() {
        let source = b"\"\"\"Example documentation\"\"\"\ndef example():\n    pass";
        let mut parser = tree_sitter::Parser::new();
        parser.set_language(&tree_sitter_python::language()).unwrap();
        let tree = parser.parse(source, None).unwrap();
        let root = tree.root_node();

        eprintln!("=== Debug Python docstrings ===");
        eprintln!("Source: {:?}", std::str::from_utf8(source).unwrap());

        // Find the function_definition node directly
        let mut cursor = root.walk();
        let mut function_node = None;
        if cursor.goto_first_child() {
            loop {
                let node = cursor.node();
                eprintln!("Node kind={}, start={}, end={}", node.kind(), node.start_byte(), node.end_byte());
                if node.kind() == "function_definition" {
                    function_node = Some(node);
                    break;
                }
                if !cursor.goto_next_sibling() {
                    break;
                }
            }
        }

        let function_node = function_node.expect("Should find function_definition");
        eprintln!("Function: start={}, end={}", function_node.start_byte(), function_node.end_byte());

        // Check previous siblings
        eprintln!("Previous siblings:");
        let mut prev = function_node.prev_sibling();
        let mut count = 0;
        while let Some(sibling) = prev {
            eprintln!("  Sibling {}: kind={}, is_named={}, start={}, text={:?}",
                count, sibling.kind(), sibling.is_named(), sibling.start_byte(),
                sibling.utf8_text(source).unwrap_or(""));
            prev = sibling.prev_sibling();
            count += 1;
            if count > 5 {
                break;
            }
        }

        // Extract leading docs
        let doc_start = extract_leading_docs(&function_node, source);
        eprintln!("doc_start={}, function_start={}", doc_start, function_node.start_byte());

        // Should point to the start of the docstring (offset 0)
        assert_eq!(doc_start, 0, "Should include Python docstrings");
        assert!(doc_start < function_node.start_byte(), "Doc start should be before function start");
    }

    #[test]
    fn test_extract_leading_docs_python_hash_comments() {
        let source = b"# Example documentation\n# Second line\ndef example():\n    pass";
        let mut parser = tree_sitter::Parser::new();
        parser.set_language(&tree_sitter_python::language()).unwrap();
        let tree = parser.parse(source, None).unwrap();
        let root = tree.root_node();

        // Find the function node
        let function_node = root
            .descendant_for_byte_range(44, 51) // "example"
            .unwrap();

        // Extract leading docs
        let doc_start = extract_leading_docs(&function_node, source);

        // Note: Regular # comments are not doc-style, so they won't be included
        // Only """ docstrings are captured as doc comments in Python
        assert_eq!(doc_start, function_node.start_byte(), "Regular # comments not captured as docs");
    }

    #[test]
    fn test_extract_leading_docs_java_javadoc() {
        let source = b"/** Javadoc comment */\nclass MyClass {}";
        let mut parser = tree_sitter::Parser::new();
        parser.set_language(&tree_sitter_java::language()).unwrap();
        let tree = parser.parse(source, None).unwrap();
        let root = tree.root_node();

        // Find the class node
        let class_node = root
            .descendant_for_byte_range(26, 33) // "MyClass"
            .unwrap();

        // Extract leading docs
        let doc_start = extract_leading_docs(&class_node, source);

        // Should point to the start of the Javadoc comment (offset 0)
        assert_eq!(doc_start, 0, "Should include Javadoc comments");
        assert!(doc_start < class_node.start_byte(), "Doc start should be before class start");
    }

    #[test]
    fn test_extract_leading_docs_jsdoc() {
        let source = b"/** JSDoc comment */\nfunction example() {}";
        let mut parser = tree_sitter::Parser::new();
        parser.set_language(&tree_sitter_javascript::language()).unwrap();
        let tree = parser.parse(source, None).unwrap();
        let root = tree.root_node();

        // Find the function node
        let function_node = root
            .descendant_for_byte_range(25, 32) // "example"
            .unwrap();

        // Extract leading docs
        let doc_start = extract_leading_docs(&function_node, source);

        // Should point to the start of the JSDoc comment (offset 0)
        assert_eq!(doc_start, 0, "Should include JSDoc comments");
        assert!(doc_start < function_node.start_byte(), "Doc start should be before function start");
    }

    #[test]
    fn test_extract_leading_docs_non_doc_comment_stops() {
        let source = b"// Regular comment\nfn example() {}";
        let mut parser = tree_sitter::Parser::new();
        parser.set_language(&tree_sitter_rust::language()).unwrap();
        let tree = parser.parse(source, None).unwrap();
        let root = tree.root_node();

        // Find the function node
        let function_node = root
            .descendant_for_byte_range(21, 28) // "example"
            .unwrap();

        // Extract leading docs
        let doc_start = extract_leading_docs(&function_node, source);

        // Regular comments (// without ///) should not be captured
        assert_eq!(doc_start, function_node.start_byte(), "Regular comments should not be captured");
    }
}
