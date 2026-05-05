//! SQLite backend implementation.
//!
//! This module provides the CodeIntelBackend trait implementation
//! for SQLite databases via sqlitegraph.

use super::router::BackendType;
use crate::error::{Result, SpliceError};
use crate::symbol::Language;
use sqlitegraph::{GraphBackend, NodeId, NodeSpec, SnapshotId};
use std::collections::HashMap;
use std::path::Path;

/// SQLite code graph implementation.
///
/// Wraps sqlitegraph's GraphBackend and provides the CodeIntelBackend
/// trait implementation for Splice workflows.
pub struct CodeGraphSqlite {
    /// The underlying sqlitegraph backend
    backend: Box<dyn GraphBackend>,
    /// Cache for symbol name → Vec<NodeId> mapping
    symbol_cache: HashMap<String, Vec<NodeId>>,
    /// Cache for file path → NodeId mapping
    file_cache: HashMap<String, NodeId>,
    /// Database path
    db_path: std::path::PathBuf,
}

impl CodeGraphSqlite {
    /// Open or create a SQLite-backed code graph.
    pub fn open(path: &Path) -> Result<Self> {
        // Handle empty files
        if let Ok(metadata) = std::fs::metadata(path) {
            if metadata.len() == 0 {
                std::fs::remove_file(path).map_err(|e| {
                    SpliceError::Other(format!(
                        "Failed to remove empty graph database {:?}: {}",
                        path, e
                    ))
                })?;
            }
        }

        let config = sqlitegraph::GraphConfig::sqlite();

        let backend = sqlitegraph::open_graph(path, &config).map_err(|e| {
            SpliceError::Other(format!("Failed to open SQLite graph at {:?}: {}", path, e))
        })?;

        Ok(Self {
            backend,
            symbol_cache: HashMap::new(),
            file_cache: HashMap::new(),
            db_path: path.to_path_buf(),
        })
    }

    /// Access the underlying sqlitegraph backend.
    pub fn inner(&self) -> &dyn GraphBackend {
        self.backend.as_ref()
    }

    /// Access the underlying sqlitegraph backend mutably.
    pub fn inner_mut(&mut self) -> &mut dyn GraphBackend {
        self.backend.as_mut()
    }

    /// Get the database path.
    pub fn db_path(&self) -> &Path {
        &self.db_path
    }

    /// Store a file node in the graph.
    fn store_file_node(&mut self, file_path: &Path) -> Result<NodeId> {
        let file_path_str = file_path
            .to_str()
            .ok_or_else(|| SpliceError::Other(format!("Invalid UTF-8 in path: {:?}", file_path)))?;

        // Check cache first
        if let Some(&node_id) = self.file_cache.get(file_path_str) {
            return Ok(node_id);
        }

        use serde_json::json;
        let node = NodeSpec {
            kind: "File".to_string(),
            name: file_path_str.to_string(),
            file_path: Some(file_path_str.to_string()),
            data: json!({
                "file_path": file_path_str,
                "language": "unknown"
            }),
        };

        let node_id_i64 = self
            .backend
            .insert_node(node)
            .map_err(|e| SpliceError::Other(format!("Failed to store file node: {}", e)))?;
        let node_id = NodeId(node_id_i64);

        self.file_cache.insert(file_path_str.to_string(), node_id);
        Ok(node_id)
    }
}

// Inherent methods that match the CodeIntelBackend trait
impl CodeGraphSqlite {
    /// Find a symbol by name within a specific file.
    pub fn find_symbol_in_file(&self, file_path: &str, name: &str) -> Option<NodeId> {
        // Check cache first
        let cache_key = format!("{}::{}", file_path, name);
        if let Some(ids) = self.symbol_cache.get(&cache_key) {
            return ids.first().copied();
        }

        // Query database
        let snapshot = SnapshotId(0);
        let all_ids = match self.backend.entity_ids() {
            Ok(ids) => ids,
            Err(_) => return None,
        };

        for node_id in all_ids {
            if let Ok(node) = self.backend.get_node(snapshot, node_id) {
                if node.name == name {
                    // In Magellan 3.1+, file_path is in node.file_path, not node.data
                    if let Some(node_file) = node.file_path.as_deref() {
                        if node_file == file_path {
                            return Some(NodeId(node_id));
                        }
                    }
                }
            }
        }

        None
    }

    /// Find all symbols with the given name across all files.
    pub fn find_symbols_by_name(&self, name: &str) -> Vec<(NodeId, Option<String>)> {
        let mut results = Vec::new();
        let snapshot = SnapshotId(0);

        let all_ids = match self.backend.entity_ids() {
            Ok(ids) => ids,
            Err(_) => return results,
        };

        for node_id in all_ids {
            if let Ok(node) = self.backend.get_node(snapshot, node_id) {
                if node.name == name {
                    let file_path = node
                        .data
                        .get("file_path")
                        .and_then(|v| v.as_str())
                        .map(|s| s.to_string());
                    results.push((NodeId(node_id), file_path));
                }
            }
        }

        results
    }

    /// Return all unique symbol names in the graph.
    pub fn all_symbol_names(&self) -> Vec<String> {
        use std::collections::HashSet;
        let mut names = HashSet::new();

        // Collect from cache
        for key in self.symbol_cache.keys() {
            if let Some(name) = key.split("::").last() {
                names.insert(name.to_string());
            } else {
                names.insert(key.clone());
            }
        }

        // Collect from database
        if let Ok(all_ids) = self.backend.entity_ids() {
            let snapshot = SnapshotId(0);
            for node_id in all_ids {
                if let Ok(node) = self.backend.get_node(snapshot, node_id) {
                    if node.kind != "File" && node.kind != "file" {
                        names.insert(node.name);
                    }
                }
            }
        }

        names.into_iter().collect()
    }

    /// Get the byte span (start, end) for a node.
    pub fn get_span(&self, node_id: NodeId) -> Result<(usize, usize)> {
        let node = self
            .backend
            .get_node(SnapshotId(0), node_id.as_i64())
            .map_err(|e| SpliceError::Other(format!("Failed to get node: {}", e)))?;

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

        Ok((byte_start, byte_end))
    }

    /// Store a symbol node in the graph.
    pub fn store_symbol(
        &mut self,
        name: &str,
        kind: &str,
        language: Language,
        byte_start: usize,
        byte_end: usize,
        line_start: usize,
        line_end: usize,
        col_start: usize,
        col_end: usize,
    ) -> Result<NodeId> {
        use serde_json::json;
        let node = NodeSpec {
            kind: kind.to_string(),
            name: name.to_string(),
            file_path: None,
            data: json!({
                "byte_start": byte_start,
                "byte_end": byte_end,
                "line_start": line_start,
                "line_end": line_end,
                "col_start": col_start,
                "col_end": col_end,
                "language": language.as_str(),
                "kind": kind
            }),
        };

        let node_id_i64 = self
            .backend
            .insert_node(node)
            .map_err(|e| SpliceError::Other(format!("Failed to store symbol: {}", e)))?;
        let node_id = NodeId(node_id_i64);

        // Update cache
        self.symbol_cache
            .entry(name.to_string())
            .or_default()
            .push(node_id);

        Ok(node_id)
    }

    /// Store a symbol node with associated file and language.
    pub fn store_symbol_with_file_and_language(
        &mut self,
        file_path: &Path,
        name: &str,
        kind: &str,
        language: Language,
        byte_start: usize,
        byte_end: usize,
        line_start: usize,
        line_end: usize,
        col_start: usize,
        col_end: usize,
    ) -> Result<NodeId> {
        // Ensure file node exists
        let _file_node_id = self.store_file_node(file_path)?;

        let file_path_str = file_path
            .to_str()
            .ok_or_else(|| SpliceError::Other(format!("Invalid UTF-8 in path: {:?}", file_path)))?;

        use serde_json::json;
        let node = NodeSpec {
            kind: kind.to_string(),
            name: name.to_string(),
            file_path: Some(file_path_str.to_string()),
            data: json!({
                "byte_start": byte_start,
                "byte_end": byte_end,
                "line_start": line_start,
                "line_end": line_end,
                "col_start": col_start,
                "col_end": col_end,
                "file_path": file_path_str,
                "language": language.as_str(),
                "kind": kind
            }),
        };

        let node_id_i64 = self
            .backend
            .insert_node(node)
            .map_err(|e| SpliceError::Other(format!("Failed to store symbol: {}", e)))?;
        let node_id = NodeId(node_id_i64);

        // Update caches
        let cache_key = format!("{}::{}", file_path_str, name);
        self.symbol_cache
            .entry(cache_key)
            .or_default()
            .push(node_id);
        self.symbol_cache
            .entry(name.to_string())
            .or_default()
            .push(node_id);

        Ok(node_id)
    }

    /// Return the backend type identifier.
    pub fn backend_type(&self) -> BackendType {
        BackendType::SQLite
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::NamedTempFile;

    #[test]
    fn test_sqlite_backend_open() {
        let temp_file = NamedTempFile::new().unwrap();
        let path = temp_file.path();

        let graph = CodeGraphSqlite::open(path).unwrap();
        assert!(matches!(graph.backend_type(), BackendType::SQLite));
    }

    #[test]
    fn test_sqlite_store_and_find_symbol() {
        let temp_file = NamedTempFile::new().unwrap();
        let mut graph = CodeGraphSqlite::open(temp_file.path()).unwrap();

        let file_path = Path::new("src/test.rs");
        let node_id = graph
            .store_symbol_with_file_and_language(
                file_path,
                "test_function",
                "function",
                Language::Rust,
                100, // byte_start
                200, // byte_end
                10,  // line_start
                20,  // line_end
                4,   // col_start
                0,   // col_end
            )
            .unwrap();

        // Find by name in file
        let found = graph.find_symbol_in_file("src/test.rs", "test_function");
        assert_eq!(found, Some(node_id));

        // Get span
        let (start, end) = graph.get_span(node_id).unwrap();
        assert_eq!(start, 100);
        assert_eq!(end, 200);
    }

    #[test]
    fn test_sqlite_find_symbols_by_name() {
        let temp_file = NamedTempFile::new().unwrap();
        let mut graph = CodeGraphSqlite::open(temp_file.path()).unwrap();

        // Store same name in different files
        graph
            .store_symbol_with_file_and_language(
                Path::new("src/a.rs"),
                "common",
                "function",
                Language::Rust,
                0,
                10,
                1,
                1,
                0,
                0,
            )
            .unwrap();

        graph
            .store_symbol_with_file_and_language(
                Path::new("src/b.rs"),
                "common",
                "function",
                Language::Rust,
                0,
                10,
                1,
                1,
                0,
                0,
            )
            .unwrap();

        let matches = graph.find_symbols_by_name("common");
        assert_eq!(matches.len(), 2);
    }
}
