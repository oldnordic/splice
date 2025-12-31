//! Rust-specific reference finding using tree-sitter-rust.
//!
//! This module traverses the AST to find identifier references
//! that point to a specific symbol definition.

use crate::error::{Result, SpliceError};
use crate::graph::CodeGraph;
use crate::ingest::imports::extract_rust_imports;
use crate::ingest::rust::{extract_rust_symbols, RustSymbol, RustSymbolKind, Visibility};
use crate::resolve::references::{Reference, ReferenceContext, ReferenceSet, SymbolDefinition};
use ropey::Rope;
use std::path::{Path, PathBuf};

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
    let source = std::fs::read(file_path)?;
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
    #[allow(dead_code)]
    parent: Option<usize>,
}

/// A map of all scopes in a file, used to detect shadowing.
#[derive(Debug, Clone)]
struct ScopeMap {
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
            parent,
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
    fn is_shadowed_at(&self, name: &str, byte_offset: usize) -> bool {
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
fn build_scope_map(source: &[u8]) -> Result<ScopeMap> {
    let mut scope_map = ScopeMap::new();
    let mut parser = tree_sitter::Parser::new();
    parser
        .set_language(&tree_sitter_rust::language())
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
        .set_language(&tree_sitter_rust::language())
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

/// Represents a re-export of a symbol.
#[derive(Debug, Clone)]
struct Reexport {
    /// The module path that re-exports the symbol (e.g., "crate::mod_a")
    reexporting_module: String,
    /// The name the symbol is re-exported as (might differ with `as`)
    #[allow(dead_code)]
    reexported_name: String,
    /// The original module path being re-exported (e.g., "crate::utils")
    #[allow(dead_code)]
    original_module: String,
    /// The original symbol name being re-exported
    #[allow(dead_code)]
    original_name: String,
}

/// Build a map of all re-exports in the workspace.
///
/// Returns a map from (module_path, symbol_name) to list of re-exports.
fn build_reexport_map(
    workspace_root: &Path,
    rust_files: &[PathBuf],
) -> Result<std::collections::HashMap<(String, String), Vec<Reexport>>> {
    let mut reexport_map: std::collections::HashMap<(String, String), Vec<Reexport>> =
        std::collections::HashMap::new();

    for file_path in rust_files {
        let source = match std::fs::read(file_path) {
            Ok(s) => s,
            Err(_) => continue,
        };

        let imports = match extract_rust_imports(file_path, &source) {
            Ok(i) => i,
            Err(_) => continue,
        };

        // Get the module path of this file
        let module_path = match module_path_from_file(workspace_root, file_path) {
            Ok(m) => m,
            Err(_) => continue,
        };

        // Process re-exports (imports with is_reexport = true)
        for import in imports {
            if !import.is_reexport {
                continue;
            }

            // Build the full path of the module being re-exported
            let imported_module = import.path.join("::");

            // For each re-exported name, record the re-export
            for name in &import.imported_names {
                if name == "*" {
                    // Glob re-export - everything from the imported module is re-exported
                    // We'll handle this as a special case by checking against the module path only
                    continue;
                }

                // The re-export is: `pub use <imported_module>::<name> as <name>` (or similar)
                // This creates a re-export from `module_path` of the symbol `<imported_module>::<name>`
                let reexport = Reexport {
                    reexporting_module: module_path.clone(),
                    reexported_name: name.clone(),
                    original_module: imported_module.clone(),
                    original_name: name.clone(),
                };

                let key = (imported_module.clone(), name.clone());
                reexport_map.entry(key).or_default().push(reexport);
            }
        }
    }

    Ok(reexport_map)
}

/// Get the module path for a file path.
///
/// Converts /path/to/workspace/src/utils/helpers.rs to "crate::utils::helpers"
fn module_path_from_file(workspace_root: &Path, file_path: &Path) -> Result<String> {
    // Get the relative path from workspace root
    let relative = file_path
        .strip_prefix(workspace_root)
        .map_err(|_| SpliceError::Other("File not in workspace".to_string()))?;

    // Convert to string and handle src/ directory
    let path_str = relative
        .to_str()
        .ok_or_else(|| SpliceError::Other("Invalid UTF-8 in path".to_string()))?;

    // Remove .rs extension and convert slashes to ::
    let module_path = path_str
        .trim_end_matches(".rs")
        .replace("/", "::")
        .replace("\\", "::");

    // Remove mod.rs if present (Rust convention for module modules)
    let module_path = module_path.replace("::mod", "");

    // Add crate:: prefix if not present
    let module_path = if module_path.starts_with("crate::") {
        module_path
    } else if module_path.starts_with("lib::") || module_path.starts_with("src::") {
        // Remove leading lib or src, add crate
        let rest = module_path
            .split("::")
            .skip(1)
            .collect::<Vec<_>>()
            .join("::");
        format!("crate::{}", rest)
    } else {
        format!("crate::{}", module_path)
    };

    Ok(module_path)
}

/// Check if a module re-exports the target symbol.
///
/// Returns true if the given module re-exports the symbol from the original module.
fn module_reexports_symbol(
    module_path: &str,
    target_module: &str,
    target_symbol: &str,
    reexport_map: &std::collections::HashMap<(String, String), Vec<Reexport>>,
) -> bool {
    // Check if this module directly re-exports the target symbol
    let key = (target_module.to_string(), target_symbol.to_string());
    if let Some(reexports) = reexport_map.get(&key) {
        for reexport in reexports {
            if reexport.reexporting_module == module_path {
                return true;
            }
        }
    }

    false
}

/// Find cross-file references to a symbol.
///
/// This function:
/// 1. Finds the workspace root (directory containing Cargo.toml)
/// 2. Finds all .rs files in the workspace
/// 3. Builds a re-export map to track which modules re-export which symbols
/// 4. For each file, extracts imports and checks if they match the target symbol's module
///    (including modules that re-export the symbol)
/// 5. For matching files, searches for references to the symbol
///
/// # Arguments
/// * `definition_file` - Path to the file containing the symbol definition
/// * `target_symbol` - The symbol to find references for
///
/// # Returns
/// * Vector of references from other files
/// * Boolean indicating if glob imports were found (reduces confidence)
fn find_cross_file_references(
    definition_file: &Path,
    target_symbol: &RustSymbol,
) -> Result<(Vec<Reference>, bool)> {
    let mut all_references = Vec::new();
    let mut has_glob_ambiguity = false;

    // Step 1: Find workspace root
    let workspace_root = find_workspace_root(definition_file)?;

    // Step 2: Find all .rs files in workspace
    let rust_files = find_all_rust_files(&workspace_root)?;

    // Step 3: Build re-export map to track re-exported symbols
    let reexport_map = match build_reexport_map(&workspace_root, &rust_files) {
        Ok(m) => m,
        Err(e) => {
            // Log error but continue without re-export tracking
            eprintln!("Warning: failed to build re-export map: {}", e);
            std::collections::HashMap::new()
        }
    };

    // Step 4: Get the module path of the target symbol
    let target_module = &target_symbol.module_path;

    // Step 5: For each file (except the definition file), check imports and search
    for file_path in rust_files {
        // Skip the definition file (already handled in same-file search)
        if file_path == definition_file {
            continue;
        }

        // Read source
        let source = match std::fs::read(&file_path) {
            Ok(s) => s,
            Err(_) => continue, // Skip files we can't read
        };

        // Extract imports from this file
        let imports = match extract_rust_imports(&file_path, &source) {
            Ok(i) => i,
            Err(_) => continue, // Skip files that fail to parse
        };

        // Check if any import matches the target module directly
        let (matches, has_glob) =
            import_matches_module(&imports, target_module, &target_symbol.name);

        // Also check if any import is from a module that re-exports the target symbol
        let matches_reexport =
            check_reexport_matches(&imports, target_module, &target_symbol.name, &reexport_map);

        if has_glob {
            has_glob_ambiguity = true;
        }

        if matches || matches_reexport {
            // This file imports from the target module (or a re-exporting module), search for references
            let rope = Rope::from_str(std::str::from_utf8(&source)?);
            let refs = find_references_in_file(&source, &rope, target_symbol, &file_path)?;
            all_references.extend(refs);
        }
    }

    Ok((all_references, has_glob_ambiguity))
}

/// Check if any import is from a module that re-exports the target symbol.
fn check_reexport_matches(
    imports: &[crate::ingest::imports::ImportFact],
    target_module: &str,
    target_symbol: &str,
    reexport_map: &std::collections::HashMap<(String, String), Vec<Reexport>>,
) -> bool {
    for import in imports {
        let imported_module = import.path.join("::");

        // For each name imported, check if it's a re-export of our target symbol
        for name in &import.imported_names {
            if name == "*" {
                // Glob import - check if this module re-exports the target
                if module_reexports_symbol(
                    &imported_module,
                    target_module,
                    target_symbol,
                    reexport_map,
                ) {
                    return true;
                }
            } else if name == target_symbol {
                // Check if this is a re-export of our target symbol
                if module_reexports_symbol(
                    &imported_module,
                    target_module,
                    target_symbol,
                    reexport_map,
                ) {
                    return true;
                }
            }
        }
    }
    false
}

/// Find the workspace root by searching upward for Cargo.toml.
fn find_workspace_root(start_path: &Path) -> Result<PathBuf> {
    let mut current = start_path
        .parent()
        .ok_or_else(|| SpliceError::Other("Cannot determine workspace root".to_string()))?;

    loop {
        let cargo_toml = current.join("Cargo.toml");
        if cargo_toml.exists() {
            return Ok(current.to_path_buf());
        }

        match current.parent() {
            Some(parent) => current = parent,
            None => {
                return Err(SpliceError::Other(
                    "Cargo.toml not found in any parent directory".to_string(),
                ))
            }
        }
    }
}

/// Find all .rs files in the workspace directory.
///
/// Excludes common build/output directories:
/// - target/
/// - .git/
/// - Any directory starting with "."
fn find_all_rust_files(workspace_root: &Path) -> Result<Vec<PathBuf>> {
    let mut rust_files = Vec::new();

    fn visit_dirs(dir: &Path, rust_files: &mut Vec<PathBuf>) -> Result<()> {
        // Skip certain directories
        if dir
            .file_name()
            .map(|n| n.to_str().unwrap_or(""))
            .unwrap_or("")
            == "target"
        {
            return Ok(());
        }
        if dir
            .file_name()
            .map(|n| n.to_str().unwrap_or(""))
            .unwrap_or("")
            == ".git"
        {
            return Ok(());
        }
        // Skip hidden directories
        if dir
            .file_name()
            .and_then(|n| n.to_str())
            .map(|s| s.starts_with('.'))
            .unwrap_or(false)
        {
            return Ok(());
        }

        let entries = match std::fs::read_dir(dir) {
            Ok(e) => e,
            Err(_) => return Ok(()), // Skip directories we can't read
        };

        for entry in entries {
            let entry = match entry {
                Ok(e) => e,
                Err(_) => continue,
            };
            let path = entry.path();

            if path.is_dir() {
                visit_dirs(&path, rust_files)?;
            } else if path.extension().and_then(|s| s.to_str()) == Some("rs") {
                rust_files.push(path);
            }
        }
        Ok(())
    }

    visit_dirs(workspace_root, &mut rust_files)?;
    Ok(rust_files)
}

/// Check if any import in the list matches the target module and symbol.
///
/// Returns (matches, has_glob) where:
/// - matches: true if any import could reference the target symbol
/// - has_glob: true if a glob import was found (reduces confidence)
fn import_matches_module(
    imports: &[crate::ingest::imports::ImportFact],
    target_module: &str,
    target_symbol_name: &str,
) -> (bool, bool) {
    let mut matches = false;
    let mut has_glob = false;

    for import in imports {
        if import.is_glob {
            has_glob = true;
            // Check if glob import matches the module
            let import_path = import.path.join("::");
            if import_path_matches_target(&import_path, target_module) {
                matches = true;
            }
        } else {
            // Check if any imported name matches our target symbol
            if import
                .imported_names
                .contains(&target_symbol_name.to_string())
            {
                let import_path = import.path.join("::");
                if import_path_matches_target(&import_path, target_module) {
                    matches = true;
                }
            }
        }
    }

    (matches, has_glob)
}

/// Check if an import path matches the target module.
///
/// The import path matches if:
/// - It exactly equals the target module
/// - It's a parent of the target module (e.g., "crate::utils" matches "crate::utils::helpers")
fn import_path_matches_target(import_path: &str, target_module: &str) -> bool {
    // Direct match
    if import_path == target_module {
        return true;
    }

    // Import is a parent module
    // e.g., import "crate::utils" matches target "crate::utils::helper"
    if target_module.starts_with(&format!("{}::", import_path)) {
        return true;
    }

    // Target is a parent module
    // e.g., import "crate::utils::helper" matches target "crate::utils"
    // (This happens when importing a specific symbol from the module)
    if import_path.starts_with(&format!("{}::", target_module)) {
        return true;
    }

    false
}

/// Find references to a symbol in a specific file.
///
/// This is a simplified version of find_same_file_references that doesn't
/// do symbol extraction (since we already know the target symbol info).
fn find_references_in_file(
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
        .set_language(&tree_sitter_rust::language())
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
    use super::*;
    use std::io::Write;
    use tempfile::NamedTempFile;

    fn create_test_graph() -> CodeGraph {
        let temp = NamedTempFile::new().unwrap();
        CodeGraph::open(temp.path()).unwrap()
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
