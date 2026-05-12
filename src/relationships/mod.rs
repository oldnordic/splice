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
use sqlitegraph::{BackendDirection, NeighborQuery, NodeId, SnapshotId};
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
pub fn get_callers(
    graph: &CodeGraph,
    symbol_node_id: NodeId,
    cache: &mut RelationshipCache,
) -> Result<Vec<Relationship>, Box<Relationships>> {
    const CALLER_THRESHOLD: usize = 100;

    // Check cache first
    let cache_key = format!("caller:{}", symbol_node_id.as_i64());
    if let Some(cached) = cache.get(&cache_key) {
        return Ok(cached.clone());
    }

    // Verify node exists - use SnapshotId(0) for latest state (sqlitegraph v1.2.7+ API)
    let backend = graph
        .inner()
        .map_err(|_| Relationships::error("BACKEND_UNAVAILABLE"))?;
    let _node = backend
        .get_node(SnapshotId(0), symbol_node_id.as_i64())
        .map_err(|_| Relationships::error("NODE_NOT_FOUND"))?;

    let call_node_ids = fetch_neighbor_ids(
        graph,
        symbol_node_id,
        BackendDirection::Incoming,
        &["CALLS", "calls"],
    )?;

    let mut callers = Vec::new();
    let mut seen: std::collections::HashSet<(String, String)> = std::collections::HashSet::new();
    for call_node_id in call_node_ids {
        if let Some(rel) = relationship_from_call_node(graph, call_node_id, "caller")? {
            let key = (rel.name.clone(), rel.file_path.clone());
            if seen.insert(key) {
                callers.push(rel);
            }
        } else if let Some(rel) = relationship_from_symbol_node(graph, call_node_id, "caller")? {
            let key = (rel.name.clone(), rel.file_path.clone());
            if seen.insert(key) {
                callers.push(rel);
            }
        }
    }

    if callers.len() > CALLER_THRESHOLD {
        callers.truncate(CALLER_THRESHOLD);
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
///
/// # Current Status
///
pub fn get_callees(
    graph: &CodeGraph,
    symbol_node_id: NodeId,
    cache: &mut RelationshipCache,
) -> Result<Vec<Relationship>, Box<Relationships>> {
    const CALLEE_THRESHOLD: usize = 100;

    // Check cache first
    let cache_key = format!("callee:{}", symbol_node_id.as_i64());
    if let Some(cached) = cache.get(&cache_key) {
        return Ok(cached.clone());
    }

    // Verify node exists - use SnapshotId(0) for latest state (sqlitegraph v1.2.7+ API)
    let backend = graph
        .inner()
        .map_err(|_| Relationships::error("BACKEND_UNAVAILABLE"))?;
    let _node = backend
        .get_node(SnapshotId(0), symbol_node_id.as_i64())
        .map_err(|_| Relationships::error("NODE_NOT_FOUND"))?;

    let call_node_ids = fetch_neighbor_ids(
        graph,
        symbol_node_id,
        BackendDirection::Outgoing,
        &["CALLER", "caller"],
    )?;

    let mut callees = Vec::new();
    let mut seen: std::collections::HashSet<(String, String)> = std::collections::HashSet::new();
    for call_node_id in call_node_ids {
        if let Some(rel) = relationship_from_call_node(graph, call_node_id, "callee")? {
            let key = (rel.name.clone(), rel.file_path.clone());
            if seen.insert(key) {
                callees.push(rel);
            }
        } else if let Some(rel) = relationship_from_symbol_node(graph, call_node_id, "callee")? {
            let key = (rel.name.clone(), rel.file_path.clone());
            if seen.insert(key) {
                callees.push(rel);
            }
        }
    }

    if callees.len() > CALLEE_THRESHOLD {
        callees.truncate(CALLEE_THRESHOLD);
    }

    // Cache before returning
    cache.set(cache_key, callees.clone());

    Ok(callees)
}

#[derive(Debug, Deserialize)]
struct CallNodePayload {
    file: String,
    caller: String,
    callee: String,
    byte_start: u64,
    byte_end: u64,
    start_line: u64,
}

fn fetch_neighbor_ids(
    graph: &CodeGraph,
    symbol_node_id: NodeId,
    direction: BackendDirection,
    edge_types: &[&str],
) -> Result<Vec<i64>, Box<Relationships>> {
    let mut ids = Vec::new();
    for edge_type in edge_types {
        // Use SnapshotId(0) for latest state (sqlitegraph v1.2.7+ API)
        let backend = graph
            .inner()
            .map_err(|_| Relationships::error("BACKEND_UNAVAILABLE"))?;
        let neighbors = backend
            .neighbors(
                SnapshotId(0),
                symbol_node_id.as_i64(),
                NeighborQuery {
                    direction,
                    edge_type: Some((*edge_type).to_string()),
                },
            )
            .map_err(|_| Relationships::error("REL_QUERY_FAILED"))?;
        ids.extend(neighbors);
    }

    ids.sort_unstable();
    ids.dedup();
    Ok(ids)
}

fn relationship_from_call_node(
    graph: &CodeGraph,
    call_node_id: i64,
    rel_type: &str,
) -> Result<Option<Relationship>, Box<Relationships>> {
    // Use SnapshotId(0) for latest state (sqlitegraph v1.2.7+ API)
    let backend = match graph.inner() {
        Ok(b) => b,
        Err(_) => return Ok(None),
    };
    let node = match backend.get_node(SnapshotId(0), call_node_id) {
        Ok(value) => value,
        Err(_) => return Ok(None),
    };

    if node.kind != "Call" {
        return Ok(None);
    }

    let call_node: CallNodePayload = match serde_json::from_value(node.data) {
        Ok(value) => value,
        Err(_) => return Ok(None),
    };

    let name = match rel_type {
        "caller" => call_node.caller,
        "callee" => call_node.callee,
        _ => return Ok(None),
    };

    Ok(Some(Relationship {
        rel_type: rel_type.to_string(),
        name,
        kind: "unknown".to_string(),
        file_path: call_node.file,
        line_start: call_node.start_line as usize,
        byte_start: call_node.byte_start as usize,
        byte_end: call_node.byte_end as usize,
    }))
}

fn relationship_from_symbol_node(
    graph: &CodeGraph,
    symbol_node_id: i64,
    rel_type: &str,
) -> Result<Option<Relationship>, Box<Relationships>> {
    // Use SnapshotId(0) for latest state (sqlitegraph v1.2.7+ API)
    let backend = match graph.inner() {
        Ok(b) => b,
        Err(_) => return Ok(None),
    };
    let node = match backend.get_node(SnapshotId(0), symbol_node_id) {
        Ok(value) => value,
        Err(_) => return Ok(None),
    };

    if node.kind != "Symbol" {
        return Ok(None);
    }

    let file_path = match node.file_path {
        Some(value) => value,
        None => return Ok(None),
    };

    let line_start = node
        .data
        .get("start_line")
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

    Ok(Some(Relationship {
        rel_type: rel_type.to_string(),
        name: node.name,
        kind: "unknown".to_string(),
        file_path,
        line_start,
        byte_start,
        byte_end,
    }))
}

/// Get all imports in a file.
///
/// Returns import statements and module-level imports for the given file.
/// Uses session caching to avoid redundant file parsing.
///
/// # Arguments
/// * `graph` - The code graph database (currently unused, reserved for future)
/// * `file_path` - Path to the file to query
/// * `cache` - Session cache for storing results
///
/// # Returns
/// - `Ok(Vec<Relationship>)` - List of imports with their locations
/// - `Err(Relationships)` - Error result with error_code set
///
/// # Error Codes
/// - `"FILE_NOT_FOUND"` - File not found on disk
/// - `"REL_QUERY_FAILED"` - Import extraction failed
///
/// # Implementation Notes
///
/// Import data is extracted by parsing the source file, as Magellan does not
/// track imports as graph nodes. The file must exist on disk to extract imports.
/// For unsupported file types, returns an empty vector (not an error).
pub fn get_imports(
    _graph: &CodeGraph,
    file_path: &Path,
    cache: &mut RelationshipCache,
) -> Result<Vec<Relationship>, Box<Relationships>> {
    use crate::ingest::detect::{detect_language, Language};
    use crate::ingest::imports::ImportFact;
    use std::fs;

    // Check cache first
    let cache_key = format!("import:{}", file_path.display());
    if let Some(cached) = cache.get(&cache_key) {
        return Ok(cached.clone());
    }

    // Check file exists
    if !file_path.exists() {
        return Err(Box::new(Relationships::error("FILE_NOT_FOUND")));
    }

    // Detect language from extension
    let language = match detect_language(file_path) {
        Some(lang) => lang,
        None => {
            // Unknown language - return empty result
            cache.set(cache_key, Vec::new());
            return Ok(Vec::new());
        }
    };

    // Read file content
    let source = fs::read(file_path).map_err(|_| Relationships::error("FILE_NOT_FOUND"))?;

    // Extract imports using language-specific extractor
    let imports: Result<Vec<ImportFact>, _> = match language {
        Language::Rust => {
            use crate::ingest::imports::extract_rust_imports;
            extract_rust_imports(file_path, &source)
        }
        Language::Python => {
            use crate::ingest::imports::extract_python_imports;
            extract_python_imports(file_path, &source)
        }
        Language::Cpp | Language::C => {
            use crate::ingest::imports::extract_cpp_imports;
            extract_cpp_imports(file_path, &source)
        }
        Language::JavaScript => {
            use crate::ingest::imports::extract_javascript_imports;
            extract_javascript_imports(file_path, &source)
        }
        Language::TypeScript => {
            use crate::ingest::imports::extract_typescript_imports;
            extract_typescript_imports(file_path, &source)
        }
        Language::Java => {
            use crate::ingest::imports::extract_java_imports;
            extract_java_imports(file_path, &source)
        }
    };

    let import_facts = imports.map_err(|_| Relationships::error("REL_QUERY_FAILED"))?;

    // Convert ImportFacts to Relationships
    let results: Vec<Relationship> = import_facts
        .into_iter()
        .map(|fact| {
            // Build display name from path and imported names
            let path_str = fact.path.join("::");
            let names_str = if fact.imported_names.is_empty() {
                path_str.clone()
            } else {
                format!("{}::{}", path_str, fact.imported_names.join("::"))
            };

            Relationship {
                rel_type: "import".to_string(),
                name: names_str,
                kind: fact.import_kind.as_str().to_string(),
                file_path: fact.file_path.to_string_lossy().to_string(),
                line_start: 0, // ImportFacts don't track line numbers
                byte_start: fact.byte_span.0,
                byte_end: fact.byte_span.1,
            }
        })
        .collect();

    // Cache before returning
    cache.set(cache_key, results.clone());

    Ok(results)
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
/// - `"FILE_NOT_FOUND"` - File not indexed in graph
/// - `"REL_QUERY_FAILED"` - Database query failed
///
/// # Implementation Notes
///
/// Public symbols are detected using heuristics:
/// - Rust: No leading underscore (pub items conventionally don't start with `_`)
/// - Python: No leading underscore
/// - JavaScript/TypeScript: No leading underscore
/// - C/C++: No leading underscore
///
/// This function queries DEFINES edges from the File node to find all symbols,
/// then filters for public symbols using language-specific heuristics.
///
/// # Implementation Notes
///
/// This is a placeholder implementation. The full implementation requires:
/// 1. A public API in CodeGraph to find file nodes
/// 2. A public API to query all symbols in a file
///
/// TODO: Implement once CodeGraph exposes file/symbol query APIs.
pub fn get_exports(
    _graph: &CodeGraph,
    _file_path: &Path,
    cache: &mut RelationshipCache,
) -> Result<Vec<Relationship>, Box<Relationships>> {
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
