use crate::error::{Result, SpliceError};
use std::path::PathBuf;

/// A symbol declared in a scope, with its declaration position.
#[derive(Debug, Clone)]
struct ScopedSymbol {
    /// Name of the symbol.
    name: String,
    /// Byte offset where this symbol is declared (shadowing starts here).
    declaration_pos: usize,
}

/// Scope information for tracking local symbols that shadow imported/global symbols.
///
/// Each scope has a byte range [start, end) and a set of symbol names defined in that scope.
#[derive(Debug, Clone)]
struct Scope {
    /// Byte offset where this scope starts (inclusive).
    start: usize,
    /// Byte offset where this scope ends (exclusive).
    end: usize,
    /// Symbols defined in this scope, with their declaration positions.
    symbols: Vec<ScopedSymbol>,
    /// Parent scope index in the scope map (for nested scopes).
    _parent: Option<usize>,
}

/// A map of all scopes in a file, used to detect shadowing.
#[derive(Debug, Clone)]
pub(crate) struct ScopeMap {
    /// All scopes in the file.
    scopes: Vec<Scope>,
}

impl ScopeMap {
    /// Create a new empty scope map.
    fn new() -> Self {
        Self { scopes: Vec::new() }
    }

    /// Add a scope to the map.
    fn add_scope(&mut self, start: usize, end: usize, parent: Option<usize>) -> usize {
        let idx = self.scopes.len();
        self.scopes.push(Scope {
            start,
            end,
            symbols: Vec::new(),
            _parent: parent,
        });
        idx
    }

    /// Add a symbol to a scope at a specific declaration position.
    fn add_symbol(&mut self, scope_idx: usize, name: String, declaration_pos: usize) {
        if let Some(scope) = self.scopes.get_mut(scope_idx) {
            scope.symbols.push(ScopedSymbol {
                name,
                declaration_pos,
            });
        }
    }

    /// Check if a reference at a given byte offset is shadowed by a local definition.
    ///
    /// Returns true if the symbol name is shadowed at that offset.
    /// A symbol shadows another if the reference is after the symbol's declaration
    /// and within the same scope.
    pub(crate) fn is_shadowed_at(&self, name: &str, byte_offset: usize) -> bool {
        // Find all scopes that contain this byte offset
        for scope in &self.scopes {
            if byte_offset >= scope.start && byte_offset < scope.end {
                // Check if this scope defines a symbol with the same name
                // The symbol must be declared before the reference
                for symbol in &scope.symbols {
                    if symbol.name == name && byte_offset >= symbol.declaration_pos {
                        return true;
                    }
                }
            }
        }
        false
    }
}

/// Build a scope map for the given source code.
///
/// This identifies all local scopes (functions, blocks, closures, match arms)
/// and tracks which symbols are defined in each scope.
pub(crate) fn build_scope_map(source: &[u8]) -> Result<ScopeMap> {
    let mut scope_map = ScopeMap::new();
    let mut parser = tree_sitter::Parser::new();
    parser
        .set_language(&tree_sitter_rust::LANGUAGE.into())
        .map_err(|e| SpliceError::Parse {
            file: PathBuf::from("<source>"),
            message: format!("Failed to set Rust language: {:?}", e),
        })?;

    let tree = parser
        .parse(source, None)
        .ok_or_else(|| SpliceError::Parse {
            file: PathBuf::from("<source>"),
            message: "Parse failed - no tree returned".to_string(),
        })?;

    // Add the file-level scope (covers the entire file)
    let file_scope = scope_map.add_scope(0, source.len(), None);

    // Build scopes by traversing the AST
    build_scopes_recursive(tree.root_node(), source, &mut scope_map, file_scope);

    Ok(scope_map)
}

/// Recursively build scopes from AST nodes.
fn build_scopes_recursive(
    node: tree_sitter::Node,
    source: &[u8],
    scope_map: &mut ScopeMap,
    current_scope: usize,
) {
    match node.kind() {
        // Function items create a new scope
        "function_item" => {
            if let Some(body) = node.child_by_field_name("body") {
                // Extract function name
                let func_name = node
                    .child_by_field_name("name")
                    .and_then(|n| n.utf8_text(source).ok())
                    .map(|s| s.to_string());

                let scope_idx =
                    scope_map.add_scope(body.start_byte(), body.end_byte(), Some(current_scope));

                // If this is a nested function (not at file scope), the function name
                // shadows in the parent scope from its declaration point.
                // We detect nested functions by checking if the current_scope is not the file scope.
                // The file scope is always at index 0 and has start=0.
                let is_nested_function = current_scope > 0;
                if is_nested_function {
                    if let Some(name) = func_name {
                        // Nested function name shadows in parent scope from declaration point
                        scope_map.add_symbol(current_scope, name, node.start_byte());
                    }
                }

                // Extract parameters
                if let Some(params) = node.child_by_field_name("parameters") {
                    for (i, name) in
                        extract_param_names(params, source, &mut std::collections::HashSet::new())
                            .into_iter()
                            .enumerate()
                    {
                        // Parameters are declared at the start of the function body
                        scope_map.add_symbol(scope_idx, name, body.start_byte() + i);
                    }
                }

                // Recurse into body to find nested scopes
                let mut cursor = body.walk();
                for child in body.children(&mut cursor) {
                    build_scopes_recursive(child, source, scope_map, scope_idx);
                }
                return;
            }
        }
        // Closure expressions create a new scope
        "closure_expression" => {
            let scope_idx =
                scope_map.add_scope(node.start_byte(), node.end_byte(), Some(current_scope));

            // Extract parameters - declared at closure start
            if let Some(params) = node.child_by_field_name("parameters") {
                for (i, name) in
                    extract_param_names(params, source, &mut std::collections::HashSet::new())
                        .into_iter()
                        .enumerate()
                {
                    scope_map.add_symbol(scope_idx, name, node.start_byte() + i);
                }
            }

            // Recurse
            let mut cursor = node.walk();
            for child in node.children(&mut cursor) {
                build_scopes_recursive(child, source, scope_map, scope_idx);
            }
            return;
        }
        // Let declarations introduce new symbols in the current scope
        "let_declaration" => {
            if let Some(name) = extract_let_binding_name(node, source) {
                scope_map.add_symbol(current_scope, name, node.start_byte());
            }
        }
        // Match arms can introduce pattern bindings
        "match_arm" => {
            if let Some(pattern) = node.child_by_field_name("pattern") {
                let bindings = extract_pattern_bindings(pattern, source);
                for binding in bindings {
                    scope_map.add_symbol(current_scope, binding, node.start_byte());
                }
            }
        }
        // Block expressions create a new scope
        "block" => {
            let scope_idx =
                scope_map.add_scope(node.start_byte(), node.end_byte(), Some(current_scope));

            // Recurse into children with the new block scope
            let mut cursor = node.walk();
            for child in node.children(&mut cursor) {
                build_scopes_recursive(child, source, scope_map, scope_idx);
            }
            return;
        }
        _ => {}
    }

    // Recurse into children
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        build_scopes_recursive(child, source, scope_map, current_scope);
    }
}

/// Extract parameter names from a parameters node.
fn extract_param_names(
    node: tree_sitter::Node,
    source: &[u8],
    _seen: &mut std::collections::HashSet<String>,
) -> Vec<String> {
    let mut names = Vec::new();

    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        match child.kind() {
            "parameter" => {
                // Try to get the name field
                if let Some(name_node) = child.child_by_field_name("name") {
                    if let Ok(name) = name_node.utf8_text(source) {
                        names.push(name.to_string());
                    }
                } else {
                    // No name field, try to find identifier in children
                    let mut inner_cursor = child.walk();
                    for inner_child in child.children(&mut inner_cursor) {
                        if inner_child.kind() == "identifier" {
                            if let Ok(name) = inner_child.utf8_text(source) {
                                names.push(name.to_string());
                                break;
                            }
                        }
                    }
                }
            }
            "," => continue,
            _ => {}
        }
    }

    names
}

/// Extract the binding name from a let declaration.
fn extract_let_binding_name(node: tree_sitter::Node, source: &[u8]) -> Option<String> {
    // Try the pattern field first (covers let x = ... and let (x, y) = ...)
    if let Some(pattern) = node.child_by_field_name("pattern") {
        // For simple identifiers
        if pattern.kind() == "identifier" {
            if let Ok(name) = pattern.utf8_text(source) {
                return Some(name.to_string());
            }
        }
        // For tuple/struct patterns, extract the first identifier
        let mut cursor = pattern.walk();
        for child in pattern.children(&mut cursor) {
            if child.kind() == "identifier" {
                if let Ok(name) = child.utf8_text(source) {
                    return Some(name.to_string());
                }
            }
        }
    }
    None
}

/// Extract all pattern bindings from a match pattern.
fn extract_pattern_bindings(node: tree_sitter::Node, source: &[u8]) -> Vec<String> {
    let mut bindings = Vec::new();

    match node.kind() {
        "identifier" => {
            if let Ok(name) = node.utf8_text(source) {
                bindings.push(name.to_string());
            }
        }
        "tuple_pattern" | "struct_pattern" => {
            let mut cursor = node.walk();
            for child in node.children(&mut cursor) {
                if child.kind() == "identifier" {
                    if let Ok(name) = child.utf8_text(source) {
                        bindings.push(name.to_string());
                    }
                }
            }
        }
        _ => {
            let mut cursor = node.walk();
            for child in node.children(&mut cursor) {
                bindings.extend(extract_pattern_bindings(child, source));
            }
        }
    }

    bindings
}
