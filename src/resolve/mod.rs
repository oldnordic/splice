//! Deterministic symbol resolution with ambiguity detection.
//!
//! This module provides file-aware, deterministic symbol resolution.
//! Name-only resolution is forbidden unless uniquely provable.
//! Supports multi-language code analysis.

pub mod cross_file;
pub mod module_resolver;
pub mod references;

use crate::error::{Result, SpliceError};
use crate::graph::CodeGraph;
use sqlitegraph::NodeId;
use std::path::Path;

/// A resolved symbol with complete location information.
#[derive(Debug, Clone, PartialEq)]
pub struct ResolvedSpan {
    /// Graph node ID for this symbol.
    pub node_id: NodeId,

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
        return resolve_symbol_in_file(graph, file_path, kind, name);
    }

    // Name-only resolution: check for ambiguity
    let all_matches = graph.find_symbols_by_name(name);

    if all_matches.is_empty() {
        return Err(SpliceError::symbol_not_found(name, None));
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

    // For now, we don't have line/col stored yet, use 0 as placeholders
    // TODO: Store line/col in graph during ingest
    Ok(ResolvedSpan {
        node_id,
        name: name.to_string(),
        kind: kind_str,
        language,
        file_path: file_path_str,
        byte_start,
        byte_end,
        line_start: 0,
        line_end: 0,
        col_start: 0,
        col_end: 0,
    })
}

/// Resolve a symbol within a specific file.
fn resolve_symbol_in_file(
    graph: &CodeGraph,
    file_path: &Path,
    kind: Option<&str>,
    name: &str,
) -> Result<ResolvedSpan> {
    let file_str = file_path
        .to_str()
        .ok_or_else(|| SpliceError::Other(format!("Invalid UTF-8 in path: {:?}", file_path)))?;

    // Use the cache-based lookup from CodeGraph
    let node_id = graph
        .find_symbol_in_file(file_str, name)
        .ok_or_else(|| SpliceError::symbol_not_found(name, Some(file_path)))?;

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
            return Err(SpliceError::symbol_not_found(name, Some(file_path)));
        }
    }

    // Get file path from node
    let node_file_path = node
        .data
        .get("file_path")
        .and_then(|v| v.as_str())
        .ok_or_else(|| SpliceError::Other("Missing file_path property".to_string()))?
        .to_string();

    // TODO: Return actual line/col when we store it
    Ok(ResolvedSpan {
        node_id,
        name: name.to_string(),
        kind: kind_str,
        language,
        file_path: node_file_path,
        byte_start,
        byte_end,
        line_start: 0,
        line_end: 0,
        col_start: 0,
        col_end: 0,
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
