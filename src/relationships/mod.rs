//! Relationship query module for traversing code graphs.
//!
//! This module provides functions for querying code relationships:
//! - Callers: functions that call a given symbol
//! - Callees: functions called by a given symbol
//! - Imports: module/file imports in a file
//! - Exports: public symbols exported by a file
//!
//! All queries use session caching to avoid redundant database lookups.

use crate::graph::CodeGraph;
use serde::{Deserialize, Serialize};
use sqlitegraph::NodeId;
use std::collections::HashMap;
use std::path::Path;

/// A single relationship between code elements.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Relationship {
    /// Relationship type ("caller", "callee", "import", "export").
    pub rel_type: String,

    /// Symbol or file name.
    pub name: String,

    /// Symbol kind (function, struct, etc.).
    pub kind: String,

    /// File path containing this relationship.
    pub file_path: String,

    /// Start line (1-based).
    pub line_start: usize,

    /// Start byte offset.
    pub byte_start: usize,

    /// End byte offset.
    pub byte_end: usize,
}

/// Aggregate result for relationship queries.
///
/// This struct holds the results of relationship queries with optional
/// error information for Phase 11 error code integration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Relationships {
    /// Callers of the queried symbol.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub callers: Vec<Relationship>,

    /// Callees of the queried symbol.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub callees: Vec<Relationship>,

    /// Imports in the queried file.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub imports: Vec<Relationship>,

    /// Exports from the queried file.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub exports: Vec<Relationship>,

    /// Whether a cycle was detected during traversal.
    #[serde(skip_serializing_if = "is_false")]
    pub cycle_detected: bool,

    /// Error code for failed queries (Phase 11 format).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error_code: Option<String>,
}

impl Relationships {
    /// Create an empty Relationships result.
    pub fn new() -> Self {
        Self {
            callers: Vec::new(),
            callees: Vec::new(),
            imports: Vec::new(),
            exports: Vec::new(),
            cycle_detected: false,
            error_code: None,
        }
    }

    /// Create an error Relationships result with an error code.
    pub fn error(code: impl Into<String>) -> Self {
        Self {
            callers: Vec::new(),
            callees: Vec::new(),
            imports: Vec::new(),
            exports: Vec::new(),
            cycle_detected: false,
            error_code: Some(code.into()),
        }
    }
}

impl Default for Relationships {
    fn default() -> Self {
        Self::new()
    }
}

/// Helper function for serde skip_serializing_if.
fn is_false(value: &bool) -> bool {
    !value
}

/// Session-based cache for relationship queries.
///
/// Caches query results within a single invocation to avoid redundant
/// database lookups. Cache keys follow the format "{rel_type}:{node_id_or_path}".
#[derive(Debug, Clone)]
pub struct RelationshipCache {
    /// Internal cache storage.
    cache: HashMap<String, Vec<Relationship>>,
}

impl RelationshipCache {
    /// Create a new empty cache.
    pub fn new() -> Self {
        Self {
            cache: HashMap::new(),
        }
    }

    /// Get cached results for a key.
    pub fn get(&self, key: &str) -> Option<&Vec<Relationship>> {
        self.cache.get(key)
    }

    /// Set cached results for a key.
    pub fn set(&mut self, key: String, value: Vec<Relationship>) {
        self.cache.insert(key, value);
    }

    /// Clear all cached results.
    pub fn clear(&mut self) {
        self.cache.clear();
    }

    /// Check if a key exists in the cache.
    pub fn contains_key(&self, key: &str) -> bool {
        self.cache.contains_key(key)
    }
}

impl Default for RelationshipCache {
    fn default() -> Self {
        Self::new()
    }
}

/// Get all callers of a symbol.
///
/// Returns functions that call the symbol with the given `symbol_node_id`.
/// Uses session caching to avoid redundant database queries.
///
/// # Arguments
/// * `graph` - The code graph database
/// * `symbol_node_id` - Node ID of the symbol to query
/// * `cache` - Session cache for storing results
///
/// # Returns
/// - `Ok(Vec<Relationship>)` - List of callers with their locations
/// - `Err(Relationships)` - Error result with error_code set
///
/// # Error Codes
/// - `"REL_QUERY_FAILED"` - Database query failed
/// - `"REL_THRESHOLD_EXCEEDED"` - More than 100 callers (result truncated)
/// - `"NODE_NOT_FOUND"` - Symbol node not found in graph
pub fn get_callers(
    graph: &CodeGraph,
    symbol_node_id: NodeId,
    cache: &mut RelationshipCache,
) -> Result<Vec<Relationship>, Relationships> {
    const CALLER_THRESHOLD: usize = 100;

    // Check cache first
    let cache_key = format!("caller:{}", symbol_node_id.as_i64());
    if let Some(cached) = cache.get(&cache_key) {
        return Ok(cached.clone());
    }

    // Query CodeGraph for incoming CALLS edges
    let incoming_edges = match graph
        .inner()
        .query_edges(
            symbol_node_id.as_i64(),
            crate::graph::schema::EDGE_CALLS,
            true, // incoming
        ) {
        Ok(edges) => edges,
        Err(e) => {
            return Err(Relationships::error("REL_QUERY_FAILED"));
        }
    };

    // Check threshold
    if incoming_edges.len() > CALLER_THRESHOLD {
        let mut error_result = Relationships::error("REL_THRESHOLD_EXCEEDED");
        // Populate partial results (first 100)
        let mut callers = Vec::new();
        for edge in incoming_edges.iter().take(CALLER_THRESHOLD) {
            if let Ok(caller) = edge_to_relationship(graph, edge, "caller") {
                callers.push(caller);
            }
        }
        error_result.callers = callers;
        return Err(error_result);
    }

    // Convert edges to relationships
    let mut callers = Vec::new();
    for edge in &incoming_edges {
        match edge_to_relationship(graph, edge, "caller") {
            Ok(rel) => callers.push(rel),
            Err(_) => continue, // Skip malformed edges
        }
    }

    // Cache before returning
    cache.set(cache_key, callers.clone());

    Ok(callers)
}

/// Get all callees of a symbol.
///
/// Returns functions called by the symbol with the given `symbol_node_id`.
/// Uses session caching to avoid redundant database queries.
///
/// # Arguments
/// * `graph` - The code graph database
/// * `symbol_node_id` - Node ID of the symbol to query
/// * `cache` - Session cache for storing results
///
/// # Returns
/// - `Ok(Vec<Relationship>)` - List of callees with their locations
/// - `Err(Relationships)` - Error result with error_code set
///
/// # Error Codes
/// - `"REL_QUERY_FAILED"` - Database query failed
/// - `"REL_THRESHOLD_EXCEEDED"` - More than 100 callees (result truncated)
/// - `"NODE_NOT_FOUND"` - Symbol node not found in graph
pub fn get_callees(
    graph: &CodeGraph,
    symbol_node_id: NodeId,
    cache: &mut RelationshipCache,
) -> Result<Vec<Relationship>, Relationships> {
    const CALLEE_THRESHOLD: usize = 100;

    // Check cache first
    let cache_key = format!("callee:{}", symbol_node_id.as_i64());
    if let Some(cached) = cache.get(&cache_key) {
        return Ok(cached.clone());
    }

    // Query CodeGraph for outgoing CALLS edges
    let outgoing_edges = match graph
        .inner()
        .query_edges(
            symbol_node_id.as_i64(),
            crate::graph::schema::EDGE_CALLS,
            false, // outgoing
        ) {
        Ok(edges) => edges,
        Err(e) => {
            return Err(Relationships::error("REL_QUERY_FAILED"));
        }
    };

    // Check threshold
    if outgoing_edges.len() > CALLEE_THRESHOLD {
        let mut error_result = Relationships::error("REL_THRESHOLD_EXCEEDED");
        // Populate partial results (first 100)
        let mut callees = Vec::new();
        for edge in outgoing_edges.iter().take(CALLEE_THRESHOLD) {
            if let Ok(callee) = edge_to_relationship(graph, edge, "callee") {
                callees.push(callee);
            }
        }
        error_result.callees = callees;
        return Err(error_result);
    }

    // Convert edges to relationships
    let mut callees = Vec::new();
    for edge in &outgoing_edges {
        match edge_to_relationship(graph, edge, "callee") {
            Ok(rel) => callees.push(rel),
            Err(_) => continue, // Skip malformed edges
        }
    }

    // Cache before returning
    cache.set(cache_key, callees.clone());

    Ok(callees)
}

/// Get all imports in a file.
///
/// Returns import statements and module-level imports for the given file.
/// Uses session caching to avoid redundant database queries.
///
/// # Arguments
/// * `graph` - The code graph database
/// * `file_path` - Path to the file to query
/// * `cache` - Session cache for storing results
///
/// # Returns
/// - `Ok(Vec<Relationship>)` - List of imports with their locations
/// - `Err(Relationships)` - Error result with error_code set
///
/// # Error Codes
/// - `"FILE_NOT_FOUND"` - File node not found in graph
/// - `"REL_QUERY_FAILED"` - Database query failed
pub fn get_imports(
    graph: &CodeGraph,
    file_path: &Path,
    cache: &mut RelationshipCache,
) -> Result<Vec<Relationship>, Relationships> {
    // Check cache first
    let cache_key = format!("import:{}", file_path.display());
    if let Some(cached) = cache.get(&cache_key) {
        return Ok(cached.clone());
    }

    let file_path_str = file_path
        .to_str()
        .ok_or_else(|| Relationships::error("REL_QUERY_FAILED"))?;

    // Find File node in cache
    let file_node_id = match graph.file_cache.get(file_path_str) {
        Some(&id) => id,
        None => return Err(Relationships::error("FILE_NOT_FOUND")),
    };

    // Query for all Symbol nodes with DEFINES edge from File
    let defines_edges = match graph
        .inner()
        .query_edges(
            file_node_id.as_i64(),
            crate::graph::schema::EDGE_DEFINES,
            false, // outgoing from File
        ) {
        Ok(edges) => edges,
        Err(e) => {
            return Err(Relationships::error("REL_QUERY_FAILED"));
        }
    };

    // Filter for import statements (kind contains "import")
    let mut imports = Vec::new();
    for edge in &defines_edges {
        // Get the symbol node
        let symbol_id = match edge.to {
            Some(id) => NodeId::from(id),
            None => continue,
        };

        let node = match graph.inner().get_node(symbol_id.as_i64()) {
            Ok(n) => n,
            Err(_) => continue,
        };

        // Check if kind contains "import"
        let kind = node
            .data
            .get("kind")
            .and_then(|v| v.as_str())
            .unwrap_or("");

        if kind.contains("import") || kind.contains("Import") {
            // Extract relationship data
            let name = node.name.clone();
            let file_path = node
                .data
                .get("file_path")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();

            let line_start = node
                .data
                .get("line_start")
                .and_then(|v| v.as_u64())
                .unwrap_or(0) as usize;

            let byte_start = node
                .data
                .get("byte_start")
                .and_then(|v| v.as_u64())
                .unwrap_or(0) as usize;

            let byte_end = node
                .data
                .get("byte_end")
                .and_then(|v| v.as_u64())
                .unwrap_or(0) as usize;

            imports.push(Relationship {
                rel_type: "import".to_string(),
                name,
                kind: kind.to_string(),
                file_path,
                line_start,
                byte_start,
                byte_end,
            });
        }
    }

    // Cache before returning
    cache.set(cache_key, imports.clone());

    Ok(imports)
}

/// Get all exports from a file.
///
/// Returns public symbols (pub fn, pub struct, exports) defined in the given file.
/// Uses session caching to avoid redundant database queries.
///
/// # Arguments
/// * `graph` - The code graph database
/// * `file_path` - Path to the file to query
/// * `cache` - Session cache for storing results
///
/// # Returns
/// - `Ok(Vec<Relationship>)` - List of exports with their locations
/// - `Err(Relationships)` - Error result with error_code set
///
/// # Error Codes
/// - `"FILE_NOT_FOUND"` - File node not found in graph
/// - `"REL_QUERY_FAILED"` - Database query failed
pub fn get_exports(
    graph: &CodeGraph,
    file_path: &Path,
    cache: &mut RelationshipCache,
) -> Result<Vec<Relationship>, Relationships> {
    // Check cache first
    let cache_key = format!("export:{}", file_path.display());
    if let Some(cached) = cache.get(&cache_key) {
        return Ok(cached.clone());
    }

    let file_path_str = file_path
        .to_str()
        .ok_or_else(|| Relationships::error("REL_QUERY_FAILED"))?;

    // Find File node in cache
    let file_node_id = match graph.file_cache.get(file_path_str) {
        Some(&id) => id,
        None => return Err(Relationships::error("FILE_NOT_FOUND")),
    };

    // Query for all Symbol nodes with DEFINES edge from File
    let defines_edges = match graph
        .inner()
        .query_edges(
            file_node_id.as_i64(),
            crate::graph::schema::EDGE_DEFINES,
            false, // outgoing from File
        ) {
        Ok(edges) => edges,
        Err(e) => {
            return Err(Relationships::error("REL_QUERY_FAILED"));
        }
    };

    // Filter for public symbols (pub fn, pub struct, exports)
    let mut exports = Vec::new();
    for edge in &defines_edges {
        // Get the symbol node
        let symbol_id = match edge.to {
            Some(id) => NodeId::from(id),
            None => continue,
        };

        let node = match graph.inner().get_node(symbol_id.as_i64()) {
            Ok(n) => n,
            Err(_) => continue,
        };

        // Check if name indicates public symbol (starts with "pub" or is export)
        let name = node.name.clone();
        let is_public = name.starts_with("pub ") || name.contains("export");

        if is_public {
            // Extract relationship data
            let kind = node
                .data
                .get("kind")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();

            let file_path = node
                .data
                .get("file_path")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();

            let line_start = node
                .data
                .get("line_start")
                .and_then(|v| v.as_u64())
                .unwrap_or(0) as usize;

            let byte_start = node
                .data
                .get("byte_start")
                .and_then(|v| v.as_u64())
                .unwrap_or(0) as usize;

            let byte_end = node
                .data
                .get("byte_end")
                .and_then(|v| v.as_u64())
                .unwrap_or(0) as usize;

            exports.push(Relationship {
                rel_type: "export".to_string(),
                name,
                kind,
                file_path,
                line_start,
                byte_start,
                byte_end,
            });
        }
    }

    // Cache before returning
    cache.set(cache_key, exports.clone());

    Ok(exports)
}

/// Helper: Convert an edge to a Relationship.
///
/// Extracts node data from the edge's target (for callers) or source (for callees).
fn edge_to_relationship(
    graph: &CodeGraph,
    edge: &sqlitegraph::EdgeRecord,
    rel_type: &str,
) -> Result<Relationship, Relationships> {
    use sqlitegraph::NodeId;

    // Determine which node to extract (from or to based on rel_type)
    let node_id = match rel_type {
        "caller" => edge.from.map(NodeId::from), // Caller is the source of the edge
        "callee" => edge.to.map(NodeId::from),   // Callee is the target
        _ => return Err(Relationships::error("REL_QUERY_FAILED")),
    };

    let node_id = node_id.ok_or_else(|| Relationships::error("REL_QUERY_FAILED"))?;

    // Get node from graph
    let node = graph
        .inner()
        .get_node(node_id.as_i64())
        .map_err(|_| Relationships::error("REL_QUERY_FAILED"))?;

    // Extract relationship data
    let name = node.name;
    let kind = node
        .data
        .get("kind")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();

    let file_path = node
        .data
        .get("file_path")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();

    let line_start = node
        .data
        .get("line_start")
        .and_then(|v| v.as_u64())
        .unwrap_or(0) as usize;

    let byte_start = node
        .data
        .get("byte_start")
        .and_then(|v| v.as_u64())
        .unwrap_or(0) as usize;

    let byte_end = node
        .data
        .get("byte_end")
        .and_then(|v| v.as_u64())
        .unwrap_or(0) as usize;

    Ok(Relationship {
        rel_type: rel_type.to_string(),
        name,
        kind,
        file_path,
        line_start,
        byte_start,
        byte_end,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn test_relationship_cache_new() {
        let cache = RelationshipCache::new();
        assert!(!cache.contains_key("test:key"));
    }

    #[test]
    fn test_relationship_cache_set_get() {
        let mut cache = RelationshipCache::new();
        let rel = Relationship {
            rel_type: "caller".to_string(),
            name: "foo".to_string(),
            kind: "function".to_string(),
            file_path: "/test/path.rs".to_string(),
            line_start: 10,
            byte_start: 100,
            byte_end: 200,
        };

        cache.set("test:key".to_string(), vec![rel.clone()]);
        assert!(cache.contains_key("test:key"));

        let retrieved = cache.get("test:key");
        assert_eq!(retrieved, Some(&vec![rel]));
    }

    #[test]
    fn test_relationship_cache_clear() {
        let mut cache = RelationshipCache::new();
        let rel = Relationship {
            rel_type: "caller".to_string(),
            name: "foo".to_string(),
            kind: "function".to_string(),
            file_path: "/test/path.rs".to_string(),
            line_start: 10,
            byte_start: 100,
            byte_end: 200,
        };

        cache.set("test:key".to_string(), vec![rel]);
        assert!(cache.contains_key("test:key"));

        cache.clear();
        assert!(!cache.contains_key("test:key"));
    }

    #[test]
    fn test_relationships_new() {
        let rels = Relationships::new();
        assert!(rels.callers.is_empty());
        assert!(rels.callees.is_empty());
        assert!(rels.imports.is_empty());
        assert!(rels.exports.is_empty());
        assert!(!rels.cycle_detected);
        assert!(rels.error_code.is_none());
    }

    #[test]
    fn test_relationships_error() {
        let rels = Relationships::error("TEST_ERROR");
        assert!(rels.error_code == Some("TEST_ERROR".to_string()));
    }

    #[test]
    fn test_relationships_default() {
        let rels = Relationships::default();
        assert!(rels.callers.is_empty());
        assert!(rels.error_code.is_none());
    }
}
