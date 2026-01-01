//! Magellan integration layer.
//!
//! This module provides integration with Magellan v0.5.0 for:
//! - Multi-language code indexing
//! - Label-based symbol queries
//! - Code chunk retrieval (no file re-reading)

use crate::error::{Result, SpliceError};
use magellan::{CodeGraph as MagellanGraph, SymbolQueryResult};
use std::path::Path;

/// Wrapper around Magellan's CodeGraph with Splice-specific extensions.
pub struct MagellanIntegration {
    inner: MagellanGraph,
}

impl MagellanIntegration {
    /// Open or create a Magellan code graph at the given path.
    pub fn open(db_path: &Path) -> Result<Self> {
        let db_path_str = db_path
            .to_str()
            .ok_or_else(|| SpliceError::Other(format!("Invalid UTF-8 in path: {:?}", db_path)))?;

        // Convert anyhow::Error to SpliceError
        let inner = MagellanGraph::open(db_path_str).map_err(|e| {
            SpliceError::Other(format!("Failed to open Magellan graph: {}", e))
        })?;

        Ok(Self { inner })
    }

    /// Index a file using Magellan's parsers.
    ///
    /// This extracts symbols, references, and calls from the file
    /// using Magellan's multi-language parsers (7 languages supported).
    ///
    /// Returns the number of symbols indexed.
    pub fn index_file(&mut self, file_path: &Path) -> Result<usize> {
        let file_path_str = file_path
            .to_str()
            .ok_or_else(|| SpliceError::Other(format!("Invalid UTF-8 in path: {:?}", file_path)))?;

        let source = std::fs::read(file_path).map_err(|e| {
            SpliceError::Other(format!("Failed to read file {:?}: {}", file_path, e))
        })?;

        self.inner.index_file(file_path_str, &source).map_err(|e| {
            SpliceError::Other(format!("Failed to index file {:?}: {}", file_path, e))
        })
    }

    /// Query symbols by labels (AND semantics).
    ///
    /// Labels are automatically assigned during indexing:
    /// - Language labels: "rust", "python", "javascript", "typescript", "c", "cpp", "java"
    /// - Symbol kind labels: "fn", "method", "struct", "class", "enum", "interface", "module", etc.
    ///
    /// Example: `query(&["rust", "fn"])` returns all Rust functions.
    pub fn query_by_labels(&self, labels: &[&str]) -> Result<Vec<SymbolInfo>> {
        let labels_ref: Vec<&str> = labels.to_vec();
        self.inner.get_symbols_by_labels(&labels_ref).map_err(|e| {
            SpliceError::Other(format!("Failed to query by labels {:?}: {}", labels, e))
        }).map(|results| results.into_iter().map(SymbolInfo::from).collect())
    }

    /// Get all available labels in the graph.
    pub fn get_all_labels(&self) -> Result<Vec<String>> {
        self.inner.get_all_labels().map_err(|e| {
            SpliceError::Other(format!("Failed to get labels: {}", e))
        })
    }

    /// Count entities with a specific label.
    pub fn count_by_label(&self, label: &str) -> Result<usize> {
        self.inner.count_entities_by_label(label).map_err(|e| {
            SpliceError::Other(format!("Failed to count label {}: {}", label, e))
        })
    }

    /// Get code chunk by exact byte span.
    ///
    /// This is the KEY feature for refactoring - it retrieves source code
    /// from the database without re-reading the file.
    ///
    /// Returns None if no code chunk exists at the given span.
    pub fn get_code_chunk(&self, file_path: &Path, start: usize, end: usize) -> Result<Option<String>> {
        let file_path_str = file_path
            .to_str()
            .ok_or_else(|| SpliceError::Other(format!("Invalid UTF-8 in path: {:?}", file_path)))?;

        self.inner.get_code_chunk_by_span(file_path_str, start, end).map_err(|e| {
            SpliceError::Other(format!("Failed to get code chunk: {}", e))
        }).map(|opt_chunk| opt_chunk.map(|chunk| chunk.content))
    }

    /// Get all code chunks for a symbol by name.
    ///
    /// Note: This retrieves chunks by symbol name, so if multiple symbols
    /// have the same name (e.g., struct + impl), you'll get all of them.
    /// Use `get_code_chunk` with exact spans for precision.
    pub fn get_code_chunks_for_symbol(&self, file_path: &Path, symbol_name: &str) -> Result<Vec<CodeChunk>> {
        let file_path_str = file_path
            .to_str()
            .ok_or_else(|| SpliceError::Other(format!("Invalid UTF-8 in path: {:?}", file_path)))?;

        self.inner.get_code_chunks_for_symbol(file_path_str, symbol_name).map_err(|e| {
            SpliceError::Other(format!("Failed to get code chunks for symbol {}: {}", symbol_name, e))
        }).map(|chunks| chunks.into_iter().map(CodeChunk::from).collect())
    }

    /// Access the underlying Magellan CodeGraph for advanced operations.
    pub fn inner(&self) -> &MagellanGraph {
        &self.inner
    }

    /// Access the underlying Magellan CodeGraph mutably for advanced operations.
    pub fn inner_mut(&mut self) -> &mut MagellanGraph {
        &mut self.inner
    }
}

/// Symbol information extracted from Magellan's SymbolQueryResult.
#[derive(Debug, Clone)]
pub struct SymbolInfo {
    /// Entity ID in the graph database.
    pub entity_id: i64,
    /// Symbol name.
    pub name: String,
    /// File path containing the symbol.
    pub file_path: String,
    /// Symbol kind (e.g., "fn", "struct", "class").
    pub kind: String,
    /// Byte offset where the symbol starts.
    pub byte_start: usize,
    /// Byte offset where the symbol ends.
    pub byte_end: usize,
}

impl From<SymbolQueryResult> for SymbolInfo {
    fn from(result: SymbolQueryResult) -> Self {
        Self {
            entity_id: result.entity_id,
            name: result.name,
            file_path: result.file_path,
            kind: result.kind,
            byte_start: result.byte_start,
            byte_end: result.byte_end,
        }
    }
}

/// Code chunk with content and metadata.
#[derive(Debug, Clone)]
pub struct CodeChunk {
    /// Source code content.
    pub content: String,
    /// Byte offset where the chunk starts.
    pub byte_start: usize,
    /// Byte offset where the chunk ends.
    pub byte_end: usize,
    /// Symbol name if this chunk belongs to a specific symbol.
    pub symbol_name: Option<String>,
}

impl From<magellan::CodeChunk> for CodeChunk {
    fn from(chunk: magellan::CodeChunk) -> Self {
        Self {
            content: chunk.content,
            byte_start: chunk.byte_start,
            byte_end: chunk.byte_end,
            symbol_name: chunk.symbol_name,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_open_and_query() {
        let temp_dir = TempDir::new().unwrap();
        let db_path = temp_dir.path().join("test.db");

        // Create integration
        let integration = MagellanIntegration::open(&db_path).unwrap();

        // Query with no data should return empty
        let results = integration.query_by_labels(&["rust"]).unwrap();
        assert!(results.is_empty());

        // Get all labels should be empty
        let labels = integration.get_all_labels().unwrap();
        assert!(labels.is_empty());
    }

    #[test]
    fn test_count_by_label() {
        let temp_dir = TempDir::new().unwrap();
        let db_path = temp_dir.path().join("test.db");

        let integration = MagellanIntegration::open(&db_path).unwrap();

        // Count should be 0 for empty graph
        let count = integration.count_by_label("rust").unwrap();
        assert_eq!(count, 0);
    }
}
