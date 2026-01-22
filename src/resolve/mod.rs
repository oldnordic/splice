//! Deterministic symbol resolution with ambiguity detection.
//!
//! This module provides file-aware, deterministic symbol resolution.
//! Name-only resolution is forbidden unless uniquely provable.
//! Supports multi-language code analysis.
//! Provides enhanced symbol lookup with "did you mean" functionality.

pub mod cross_file;
pub mod module_resolver;
pub mod references;

use crate::error::{Result, SpliceError};
use crate::graph::CodeGraph;
use serde::Serialize;
use sqlitegraph::NodeId;
use std::path::Path;

/// A resolved symbol with complete location information.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct ResolvedSpan {
    /// Graph node ID for this symbol.
    #[serde(skip_serializing)]
    pub node_id: NodeId,

    /// Unique ID for this resolution attempt (generated once per resolve call).
    pub match_id: String,

    /// Symbol name.
    pub name: String,

    /// Symbol kind (function, struct, etc.) as language-agnostic string.
    pub kind: String,

    /// Programming language (rust, python, etc.).
    pub language: Option<String>,

    /// File containing this symbol.
    pub file_path: String,

    /// Start byte offset.
    pub byte_start: usize,

    /// End byte offset.
    pub byte_end: usize,

    /// Start line (1-based).
    pub line_start: usize,

    /// End line (1-based).
    pub line_end: usize,

    /// Start column (0-based, in bytes).
    pub col_start: usize,

    /// End column (0-based, in bytes).
    pub col_end: usize,
}

/// Resolve a symbol to its span with file-aware disambiguation.
///
/// # Arguments
/// * `graph` - The code graph database
/// * `file` - Optional file path to disambiguate symbols with same name
/// * `kind` - Optional symbol kind filter (function, struct, class, etc.)
/// * `name` - Symbol name to resolve
///
/// # Resolution Rules
/// 1. If `file` is Some, resolves the symbol within that specific file
/// 2. If `file` is None AND multiple matches exist across files → returns AmbiguousSymbol error
/// 3. If `file` is None AND exactly one match exists globally → returns that symbol
///
/// # Errors
/// - `AmbiguousSymbol` - When name-only resolution finds multiple matches across files
/// - `SymbolNotFound` - When no symbol matches the query
pub fn resolve_symbol(
    graph: &CodeGraph,
    file: Option<&Path>,
    kind: Option<&str>,
    name: &str,
) -> Result<ResolvedSpan> {
    use uuid::Uuid;

    // Generate match_id for this resolution attempt
    let match_id = Uuid::new_v4().to_string();

    // Build cache key for lookup
    let _cache_key = if let Some(file_path) = file {
        let file_str = file_path
            .to_str()
            .ok_or_else(|| SpliceError::Other(format!("Invalid UTF-8 in path: {:?}", file_path)))?;
        format!("{}::{}", file_str, name)
    } else {
        name.to_string()
    };

    // For file-specific resolution, use the cache directly
    if let Some(file_path) = file {
        return resolve_symbol_in_file(graph, file_path, kind, name, &match_id);
    }

    // Name-only resolution: check for ambiguity
    let all_matches = graph.find_symbols_by_name(name);

    if all_matches.is_empty() {
        // Symbol not found - try to provide suggestions
        let all_symbols = graph.all_symbol_names();
        let suggestions = crate::suggestions::suggest_similar_symbols(name, &all_symbols, 3);

        let hint = if suggestions.is_empty() {
            format!(
                "Symbol '{}' not found. Run `splice ingest` to index the codebase.",
                name
            )
        } else {
            format!("Did you mean: {}?", suggestions.join(", "))
        };

        return Err(SpliceError::SymbolNotFound {
            message: format!("Symbol '{}' not found", name),
            symbol: name.to_string(),
            file: None,
            hint,
        });
    }

    if all_matches.len() > 1 {
        // Multiple matches → ambiguous
        let files: Vec<String> = all_matches
            .into_iter()
            .filter_map(|(_id, path)| path)
            .collect();

        return Err(SpliceError::AmbiguousSymbol {
            name: name.to_string(),
            files,
        });
    }

    // Exactly one match → safe to return
    let (node_id, file_path) = all_matches.into_iter().next().unwrap();
    let file_path_str =
        file_path.ok_or_else(|| SpliceError::Other("Symbol node missing file_path".to_string()))?;

    // Get node data from graph
    let node = graph.inner().get_node(node_id.as_i64())?;

    // Extract span data
    let byte_start = node
        .data
        .get("byte_start")
        .and_then(|v| v.as_u64())
        .ok_or_else(|| SpliceError::Other("Missing byte_start property".to_string()))?
        as usize;

    let byte_end = node
        .data
        .get("byte_end")
        .and_then(|v| v.as_u64())
        .ok_or_else(|| SpliceError::Other("Missing byte_end property".to_string()))?
        as usize;

    // Extract kind (language-agnostic string)
    let kind_str = node
        .data
        .get("kind")
        .and_then(|v| v.as_str())
        .ok_or_else(|| SpliceError::Other("Missing kind property".to_string()))?
        .to_string();

    // Extract language (optional)
    let language = node
        .data
        .get("language")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string());

    // Retrieve line/col from graph (stored by ingest modules)
    let line_start = node
        .data
        .get("line_start")
        .and_then(|v| v.as_u64())
        .unwrap_or(0) as usize;
    let line_end = node
        .data
        .get("line_end")
        .and_then(|v| v.as_u64())
        .unwrap_or(0) as usize;
    let col_start = node
        .data
        .get("col_start")
        .and_then(|v| v.as_u64())
        .unwrap_or(0) as usize;
    let col_end = node
        .data
        .get("col_end")
        .and_then(|v| v.as_u64())
        .unwrap_or(0) as usize;

    Ok(ResolvedSpan {
        node_id,
        match_id,
        name: name.to_string(),
        kind: kind_str,
        language,
        file_path: file_path_str,
        byte_start,
        byte_end,
        line_start,
        line_end,
        col_start,
        col_end,
    })
}

/// Resolve a symbol within a specific file.
fn resolve_symbol_in_file(
    graph: &CodeGraph,
    file_path: &Path,
    kind: Option<&str>,
    name: &str,
    match_id: &str,
) -> Result<ResolvedSpan> {
    let file_str = file_path
        .to_str()
        .ok_or_else(|| SpliceError::Other(format!("Invalid UTF-8 in path: {:?}", file_path)))?;

    // Use the cache-based lookup from CodeGraph
    let node_id = match graph.find_symbol_in_file(file_str, name) {
        Some(id) => id,
        None => {
            // Symbol not found - try to provide suggestions
            let all_symbols = graph.all_symbol_names();
            let suggestions = crate::suggestions::suggest_similar_symbols(name, &all_symbols, 3);

            let hint = if suggestions.is_empty() {
                format!(
                    "Symbol '{}' not found in {}. Run `splice ingest` to index the codebase.",
                    name,
                    file_str
                )
            } else {
                format!("Did you mean: {}?", suggestions.join(", "))
            };

            return Err(SpliceError::SymbolNotFound {
                message: format!("Symbol '{}' not found in {}", name, file_str),
                symbol: name.to_string(),
                file: Some(file_path.to_path_buf()),
                hint,
            });
        }
    };

    // Get node data from graph
    let node = graph.inner().get_node(node_id.as_i64())?;

    // Extract span data
    let byte_start = node
        .data
        .get("byte_start")
        .and_then(|v| v.as_u64())
        .ok_or_else(|| SpliceError::Other("Missing byte_start property".to_string()))?
        as usize;

    let byte_end = node
        .data
        .get("byte_end")
        .and_then(|v| v.as_u64())
        .ok_or_else(|| SpliceError::Other("Missing byte_end property".to_string()))?
        as usize;

    // Extract kind (language-agnostic string)
    let kind_str = node
        .data
        .get("kind")
        .and_then(|v| v.as_str())
        .ok_or_else(|| SpliceError::Other("Missing kind property".to_string()))?
        .to_string();

    // Extract language (optional)
    let language = node
        .data
        .get("language")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string());

    // Filter by kind if specified
    if let Some(k) = kind {
        if kind_str != k {
            return Err(SpliceError::SymbolNotFound {
                message: format!("Symbol '{}' of kind '{}' not found in {}", name, k, file_str),
                symbol: name.to_string(),
                file: Some(file_path.to_path_buf()),
                hint: format!("Symbol '{}' exists but is a '{}', not '{}'. Try adjusting the --kind flag.", name, kind_str, k),
            });
        }
    }

    // Get file path from node
    let node_file_path = node
        .data
        .get("file_path")
        .and_then(|v| v.as_str())
        .ok_or_else(|| SpliceError::Other("Missing file_path property".to_string()))?
        .to_string();

    // Retrieve line/col from graph (stored by ingest modules)
    let line_start = node
        .data
        .get("line_start")
        .and_then(|v| v.as_u64())
        .unwrap_or(0) as usize;
    let line_end = node
        .data
        .get("line_end")
        .and_then(|v| v.as_u64())
        .unwrap_or(0) as usize;
    let col_start = node
        .data
        .get("col_start")
        .and_then(|v| v.as_u64())
        .unwrap_or(0) as usize;
    let col_end = node
        .data
        .get("col_end")
        .and_then(|v| v.as_u64())
        .unwrap_or(0) as usize;

    Ok(ResolvedSpan {
        node_id,
        match_id: match_id.to_string(),
        name: name.to_string(),
        kind: kind_str,
        language,
        file_path: node_file_path,
        byte_start,
        byte_end,
        line_start,
        line_end,
        col_start,
        col_end,
    })
}

/// Backward compatibility: Resolve with Rust-specific symbol kind.
///
/// This function is provided for backward compatibility with existing code
/// that uses `RustSymbolKind`. New code should use `resolve_symbol` with
/// string kinds.
///
/// # Deprecated
/// Use `resolve_symbol` with `Option<&str>` for kind instead.
#[deprecated(note = "Use resolve_symbol with Option<&str> kind")]
pub fn resolve_symbol_with_rust_kind(
    graph: &CodeGraph,
    file: Option<&Path>,
    kind: Option<crate::ingest::rust::RustSymbolKind>,
    name: &str,
) -> Result<ResolvedSpan> {
    let kind_str = kind.map(|k| k.as_str().to_string());
    resolve_symbol(graph, file, kind_str.as_deref(), name)
}

/// Find a symbol, providing suggestions if not found.
///
/// This is the preferred method for symbol lookup in user-facing commands
/// because it provides helpful "did you mean" suggestions when a symbol
/// name is misspelled or doesn't exist.
///
/// # Arguments
/// * `graph` - The code graph to search
/// * `name` - The symbol name to find
/// * `file` - Optional file path to scope the search
///
/// # Returns
/// * `Ok(NodeId)` if symbol is found
/// * `Err(SpliceError::SymbolNotFound)` with suggestions if not found
///
/// # Examples
/// ```
/// let node_id = find_symbol_or_suggest(&graph, "my_function", None)?;
/// ```
pub fn find_symbol_or_suggest(
    graph: &CodeGraph,
    name: &str,
    file: Option<&Path>,
) -> Result<NodeId> {
    // Try to find the symbol in the specified file
    if let Some(file_path) = file {
        if let Some(file_str) = file_path.to_str() {
            if let Some(node_id) = graph.find_symbol_in_file(file_str, name) {
                return Ok(node_id);
            }
        }
    }

    // Check if symbol exists in any file
    let all_matches = graph.find_symbols_by_name(name);
    if !all_matches.is_empty() {
        if let Some((node_id, _)) = all_matches.first() {
            return Ok(*node_id);
        }
    }

    // Symbol not found - get suggestions
    let all_symbols = graph.all_symbol_names();
    let suggestions = crate::suggestions::suggest_similar_symbols(name, &all_symbols, 3);

    let hint = if suggestions.is_empty() {
        format!(
            "Symbol '{}' not found. Run `splice ingest` to index the codebase.",
            name
        )
    } else {
        format!("Did you mean: {}?", suggestions.join(", "))
    };

    Err(SpliceError::SymbolNotFound {
        message: format!("Symbol '{}' not found", name),
        symbol: name.to_string(),
        file: file.map(|p| p.to_path_buf()),
        hint,
    })
}
