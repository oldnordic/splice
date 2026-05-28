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
/// ```rust,ignore
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
        if parent.kind() == "source_file" || !parent.is_named() {
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
/// ```rust,ignore
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
/// ```rust,ignore
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
            return Some((parent.start_byte(), parent.end_byte()));
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
/// ```rust,ignore
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
/// ```rust,ignore
/// use splice::expand::tree_walker::extract_leading_doc_comments;
///
/// let function_node = /* function_item node */;
/// let source = b"/// Example docs\nfn example() {}";
///
/// let comments = extract_leading_doc_comments(function_node, source);
/// assert_eq!(comments, vec!["/// Example docs"]);
/// ```
pub fn extract_leading_doc_comments(node: tree_sitter::Node, source: &[u8]) -> Vec<String> {
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
/// ```rust,ignore
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
        } else if kind == "\n" || !prev.is_named() {
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
#[path = "tree_walker_tests.rs"]
mod tree_walker_tests;
