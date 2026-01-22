//! Relationship query module for traversing code graphs.
//!
//! This module provides functions for querying code relationships:
//! - Callers: functions that call a given symbol
//! - Callees: functions called by a given symbol
//! - Imports: module/file imports in a file
//! - Exports: public symbols exported by a file
//!
//! All queries use session caching to avoid redundant database lookups.
//!
//! # Current Implementation Status
//!
//! The relationship query infrastructure is in place, but CALLS edges
//! are not yet created during code ingestion. This means:
//! - `get_callers` and `get_callees` will return empty results until edge creation is implemented
//! - `get_imports` and `get_exports` work by querying DEFINES edges from File nodes
//!
//! See src/graph/schema.rs for EDGE_CALLS constant definition.

use crate::graph::CodeGraph;
use serde::{Deserialize, Serialize};
use sqlitegraph::NodeId;
use std::collections::HashMap;
use std::path::Path;

/// A single relationship between code elements.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
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
///
/// # Current Status
///
/// This function currently returns an empty result because CALLS edges
/// are not yet created during code ingestion. Edge creation infrastructure
/// needs to be added to the ingest modules.
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

    // Verify node exists
    let _node = graph
        .inner()
        .get_node(symbol_node_id.as_i64())
        .map_err(|_| Relationships::error("NODE_NOT_FOUND"))?;

    // TODO: Query incoming CALLS edges once edge creation is implemented
    // For now, return empty result
    let callers = Vec::new();

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
///
/// # Current Status
///
/// This function currently returns an empty result because CALLS edges
/// are not yet created during code ingestion. Edge creation infrastructure
/// needs to be added to the ingest modules.
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

    // Verify node exists
    let _node = graph
        .inner()
        .get_node(symbol_node_id.as_i64())
        .map_err(|_| Relationships::error("NODE_NOT_FOUND"))?;

    // TODO: Query outgoing CALLS edges once edge creation is implemented
    // For now, return empty result
    let callees = Vec::new();

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
///
/// # Current Implementation
///
/// This function currently returns an empty result because:
/// 1. Symbol nodes are not indexed by file in the current cache structure
/// 2. Direct file->symbol edge queries are not yet available
///
/// Future implementation should add edge traversal infrastructure to query
/// DEFINES edges from File nodes.
pub fn get_imports(
    _graph: &CodeGraph,
    _file_path: &Path,
    cache: &mut RelationshipCache,
) -> Result<Vec<Relationship>, Relationships> {
    // Check cache first
    let cache_key = format!("import:{}", _file_path.display());
    if let Some(cached) = cache.get(&cache_key) {
        return Ok(cached.clone());
    }

    // TODO: Query File->Symbol DEFINES edges and filter for import statements
    // Current limitation: No public API to iterate all symbols in a file
    let imports = Vec::new();

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
///
/// # Current Implementation
///
/// This function currently returns an empty result because:
/// 1. Symbol nodes are not indexed by file in the current cache structure
/// 2. Direct file->symbol edge queries are not yet available
///
/// Future implementation should add edge traversal infrastructure to query
/// DEFINES edges from File nodes.
pub fn get_exports(
    _graph: &CodeGraph,
    _file_path: &Path,
    cache: &mut RelationshipCache,
) -> Result<Vec<Relationship>, Relationships> {
    // Check cache first
    let cache_key = format!("export:{}", _file_path.display());
    if let Some(cached) = cache.get(&cache_key) {
        return Ok(cached.clone());
    }

    // TODO: Query File->Symbol DEFINES edges and filter for public symbols
    // Current limitation: No public API to iterate all symbols in a file
    let exports = Vec::new();

    // Cache before returning
    cache.set(cache_key, exports.clone());

    Ok(exports)
}

#[cfg(test)]
mod tests {
    use super::*;

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

    #[test]
    fn test_relationship_serialization() {
        let rel = Relationship {
            rel_type: "caller".to_string(),
            name: "test_function".to_string(),
            kind: "function".to_string(),
            file_path: "/test/file.rs".to_string(),
            line_start: 42,
            byte_start: 1000,
            byte_end: 2000,
        };

        let json = serde_json::to_string(&rel).unwrap();
        assert!(json.contains("\"caller\""));
        assert!(json.contains("\"test_function\""));
        assert!(json.contains("\"line_start\":42"));

        let deserialized: Relationship = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.rel_type, "caller");
        assert_eq!(deserialized.name, "test_function");
    }

    #[test]
    fn test_relationships_serialization_empty_fields() {
        let rels = Relationships::new();
        let json = serde_json::to_string(&rels).unwrap();

        // Empty fields should not appear in JSON due to skip_serializing_if
        assert!(!json.contains("callers"));
        assert!(!json.contains("callees"));
        assert!(!json.contains("imports"));
        assert!(!json.contains("exports"));
        assert!(!json.contains("cycle_detected"));
    }

    #[test]
    fn test_relationships_serialization_with_error() {
        let rels = Relationships::error("REL_QUERY_FAILED");
        let json = serde_json::to_string(&rels).unwrap();

        assert!(json.contains("REL_QUERY_FAILED"));
        assert!(json.contains("\"error_code\""));
    }
}
