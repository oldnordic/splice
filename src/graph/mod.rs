//! SQLiteGraph integration layer.
//!
//! This module provides a typed interface to the code graph stored
//! in SQLiteGraph. It handles symbol storage, span queries, and
//! relationship management for multi-language code analysis.

pub mod schema;

use crate::error::{Result, SpliceError};
use crate::symbol::Language;
use serde_json::json;
use sqlitegraph::{EdgeSpec, GraphBackend, NodeId, NodeSpec};
use std::collections::HashMap;
use std::path::Path;

/// Graph database handle.
///
/// Wraps SQLiteGraph and provides Splice-specific operations.
pub struct CodeGraph {
    /// The underlying graph backend.
    backend: Box<dyn GraphBackend>,

    /// Cache for symbol name → Vec<NodeId> mapping (multiple files can have same name).
    symbol_cache: HashMap<String, Vec<NodeId>>,

    /// Cache for file path → NodeId mapping.
    file_cache: HashMap<String, NodeId>,
}

impl CodeGraph {
    /// Open or create a code graph at the given path.
    pub fn open(path: &std::path::Path) -> Result<Self> {
        let cfg = sqlitegraph::GraphConfig::sqlite();
        let backend = sqlitegraph::open_graph(path, &cfg)?;
        Ok(Self {
            backend,
            symbol_cache: HashMap::new(),
            file_cache: HashMap::new(),
        })
    }

    /// Store a symbol with its byte span and metadata (legacy method for backward compatibility).
    ///
    /// Creates a node in the graph with:
    /// - Label: "symbol_function", "symbol_class", etc. (language-agnostic)
    /// - Properties: name, kind, byte_start, byte_end
    ///
    /// Returns the NodeId of the created node.
    ///
    /// # Deprecated
    /// This method is kept for backward compatibility. Use `store_symbol_with_file_and_language`
    /// for new code.
    #[deprecated(note = "Use store_symbol_with_file_and_language for multi-language support")]
    pub fn store_symbol(
        &mut self,
        name: &str,
        kind: &str,
        byte_start: usize,
        byte_end: usize,
    ) -> Result<NodeId> {
        // Determine label based on kind
        let label = schema::kind_to_label(kind);

        // Create node spec
        let node_spec = NodeSpec {
            kind: label.0,
            name: name.to_string(),
            file_path: None,
            data: json!({
                "kind": kind,
                "byte_start": byte_start,
                "byte_end": byte_end,
            }),
        };

        // Insert node
        let node_id_i64 = self.backend.insert_node(node_spec)?;
        let node_id = NodeId::from(node_id_i64);

        // Cache the symbol name → NodeId mapping
        self.symbol_cache
            .entry(name.to_string())
            .or_default()
            .push(node_id);

        Ok(node_id)
    }

    /// Store a symbol with file association, language, and complete metadata.
    ///
    /// This method:
    /// 1. Creates a File node if it doesn't exist
    /// 2. Creates a Symbol node with all metadata (byte spans + language)
    /// 3. Creates a DEFINES edge from File to Symbol
    ///
    /// Returns the NodeId of the created Symbol node.
    pub fn store_symbol_with_file_and_language(
        &mut self,
        file_path: &Path,
        name: &str,
        kind: &str,
        language: Language,
        byte_start: usize,
        byte_end: usize,
    ) -> Result<NodeId> {
        // Get or create File node
        let file_path_str = file_path
            .to_str()
            .ok_or_else(|| SpliceError::Other(format!("Invalid UTF-8 in path: {:?}", file_path)))?;
        let file_node_id = self.get_or_create_file_node(file_path_str)?;

        // Determine label based on kind (language-agnostic)
        let label = schema::kind_to_label(kind);

        // Create symbol node with file_path and language in spec
        let node_spec = NodeSpec {
            kind: label.0,
            name: name.to_string(),
            file_path: Some(file_path_str.to_string()),
            data: json!({
                "kind": kind,
                "language": language.as_str(),
                "byte_start": byte_start,
                "byte_end": byte_end,
                "file_path": file_path_str,
            }),
        };

        // Insert symbol node
        let symbol_id_i64 = self.backend.insert_node(node_spec)?;
        let symbol_id = NodeId::from(symbol_id_i64);

        // Create DEFINES edge: File ─[DEFINES]→ Symbol
        let edge_spec = EdgeSpec {
            from: file_node_id.as_i64(),
            to: symbol_id.as_i64(),
            edge_type: schema::EDGE_DEFINES.to_string(),
            data: json!({}),
        };
        self.backend.insert_edge(edge_spec)?;

        // Cache the symbol name → NodeId mapping (by file)
        let cache_key = format!("{}::{}", file_path_str, name);
        self.symbol_cache
            .entry(cache_key)
            .or_default()
            .push(symbol_id);

        Ok(symbol_id)
    }

    /// Store a symbol with file association using Rust symbol kind.
    ///
    /// This is a backward-compatible method that internally converts
    /// RustSymbolKind to the string representation.
    ///
    /// # Deprecated
    /// Use `store_symbol_with_file_and_language` for new code.
    #[deprecated(note = "Use store_symbol_with_file_and_language for multi-language support")]
    pub fn store_symbol_with_file(
        &mut self,
        file_path: &Path,
        name: &str,
        kind: &str,
        byte_start: usize,
        byte_end: usize,
    ) -> Result<NodeId> {
        // For backward compatibility, assume Rust language
        self.store_symbol_with_file_and_language(
            file_path,
            name,
            kind,
            Language::Rust,
            byte_start,
            byte_end,
        )
    }

    /// Get or create a File node for the given path.
    fn get_or_create_file_node(&mut self, file_path: &str) -> Result<NodeId> {
        // Check cache first
        if let Some(&node_id) = self.file_cache.get(file_path) {
            return Ok(node_id);
        }

        // Create new File node
        let node_spec = NodeSpec {
            kind: schema::label_file().0,
            name: file_path.to_string(),
            file_path: Some(file_path.to_string()),
            data: json!({
                "path": file_path,
            }),
        };

        let node_id_i64 = self.backend.insert_node(node_spec)?;
        let node_id = NodeId::from(node_id_i64);

        // Cache it
        self.file_cache.insert(file_path.to_string(), node_id);

        Ok(node_id)
    }

    /// Resolve a symbol name to its NodeId (legacy method).
    ///
    /// Returns the NodeId if the symbol exists in the graph.
    /// NOTE: This uses the old cache format and is kept for backward compatibility.
    pub fn resolve_symbol(&self, name: &str) -> Result<NodeId> {
        self.symbol_cache
            .get(name)
            .and_then(|ids| ids.first())
            .copied()
            .ok_or_else(|| SpliceError::SymbolNotFound(name.to_string()))
    }

    /// Get all symbol nodes with a given name across all files.
    ///
    /// Returns a Vec of (node_id, file_path) tuples for all symbols with the given name.
    pub fn find_symbols_by_name(&self, name: &str) -> Vec<(NodeId, Option<String>)> {
        let mut results = Vec::new();

        // Search through cache keys that end with "::name"
        for (key, ids) in &self.symbol_cache {
            if key.ends_with(&format!("::{}", name)) || key == name {
                for &node_id in ids {
                    // Try to get file_path from the node
                    if let Ok(node) = self.backend.get_node(node_id.as_i64()) {
                        let file_path = node.data.get("file_path").and_then(|v| v.as_str());
                        results.push((node_id, file_path.map(|s| s.to_string())));
                    }
                }
            }
        }

        results
    }

    /// Get symbol by file and name from cache.
    pub fn find_symbol_in_file(&self, file_path: &str, name: &str) -> Option<NodeId> {
        let cache_key = format!("{}::{}", file_path, name);
        self.symbol_cache
            .get(&cache_key)
            .and_then(|ids| ids.first())
            .copied()
    }

    /// Get the byte span for a NodeId.
    ///
    /// Returns (byte_start, byte_end) from the node's properties.
    pub fn get_span(&self, node_id: NodeId) -> Result<(usize, usize)> {
        // Get node from graph
        let node = self.backend.get_node(node_id.as_i64())?;

        // Extract byte span from data
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

    /// Access the underlying graph backend for advanced operations.
    pub fn inner(&self) -> &dyn GraphBackend {
        self.backend.as_ref()
    }

    /// Access the underlying graph backend mutably for advanced operations.
    pub fn inner_mut(&mut self) -> &mut dyn GraphBackend {
        self.backend.as_mut()
    }
}
