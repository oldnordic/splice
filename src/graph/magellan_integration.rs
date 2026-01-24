//! Magellan integration layer.
//!
//! This module provides integration with Magellan v0.5.0 for:
//! - Multi-language code indexing
//! - Label-based symbol queries
//! - Code chunk retrieval (no file re-reading)

use crate::error::{Result, SpliceError};
use magellan::{CodeGraph as MagellanGraph, SymbolKind, SymbolQueryResult};
use std::path::{Path, PathBuf};

/// Parse a normalized symbol kind string into SymbolKind enum.
///
/// Maps user-facing kind strings (fn, struct, method, etc.) to SymbolKind variants.
fn parse_symbol_kind(kind: &str) -> SymbolKind {
    match kind {
        "fn" => SymbolKind::Function,
        "method" => SymbolKind::Method,
        "struct" => SymbolKind::Class,
        "class" => SymbolKind::Class,
        "trait" => SymbolKind::Interface,
        "interface" => SymbolKind::Interface,
        "enum" => SymbolKind::Enum,
        "mod" => SymbolKind::Module,
        "module" => SymbolKind::Module,
        "union" => SymbolKind::Union,
        "namespace" => SymbolKind::Namespace,
        "type_alias" => SymbolKind::TypeAlias,
        _ => SymbolKind::Unknown,
    }
}

/// Wrapper around Magellan's CodeGraph with Splice-specific extensions.
pub struct MagellanIntegration {
    inner: MagellanGraph,
    db_path: PathBuf,
}

impl MagellanIntegration {
    /// Open or create a Magellan code graph at the given path.
    pub fn open(db_path: &Path) -> Result<Self> {
        let db_path_str = db_path
            .to_str()
            .ok_or_else(|| SpliceError::Other(format!("Invalid UTF-8 in path: {:?}", db_path)))?;

        // Convert anyhow::Error to SpliceError
        let inner = MagellanGraph::open(db_path_str)
            .map_err(|e| SpliceError::Other(format!("Failed to open Magellan graph: {}", e)))?;

        Ok(Self {
            inner,
            db_path: db_path.to_path_buf(),
        })
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

        self.inner
            .index_file(file_path_str, &source)
            .map_err(|e| SpliceError::Other(format!("Failed to index file {:?}: {}", file_path, e)))
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
        self.inner
            .get_symbols_by_labels(&labels_ref)
            .map_err(|e| {
                SpliceError::Other(format!("Failed to query by labels {:?}: {}", labels, e))
            })
            .map(|results| results.into_iter().map(SymbolInfo::from).collect())
    }

    /// Get all available labels in the graph.
    pub fn get_all_labels(&self) -> Result<Vec<String>> {
        self.inner
            .get_all_labels()
            .map_err(|e| SpliceError::Other(format!("Failed to get labels: {}", e)))
    }

    /// Count entities with a specific label.
    pub fn count_by_label(&self, label: &str) -> Result<usize> {
        self.inner
            .count_entities_by_label(label)
            .map_err(|e| SpliceError::Other(format!("Failed to count label {}: {}", label, e)))
    }

    /// Get code chunk by exact byte span.
    ///
    /// This is the KEY feature for refactoring - it retrieves source code
    /// from the database without re-reading the file.
    ///
    /// Returns None if no code chunk exists at the given span.
    pub fn get_code_chunk(
        &self,
        file_path: &Path,
        start: usize,
        end: usize,
    ) -> Result<Option<CodeChunk>> {
        let file_path_str = file_path
            .to_str()
            .ok_or_else(|| SpliceError::Other(format!("Invalid UTF-8 in path: {:?}", file_path)))?;

        self.inner
            .get_code_chunk_by_span(file_path_str, start, end)
            .map_err(|e| SpliceError::Other(format!("Failed to get code chunk: {}", e)))
            .map(|opt_chunk| opt_chunk.map(CodeChunk::from))
    }

    /// Get all code chunks for a symbol by name.
    ///
    /// Note: This retrieves chunks by symbol name, so if multiple symbols
    /// have the same name (e.g., struct + impl), you'll get all of them.
    /// Use `get_code_chunk` with exact spans for precision.
    pub fn get_code_chunks_for_symbol(
        &self,
        file_path: &Path,
        symbol_name: &str,
    ) -> Result<Vec<CodeChunk>> {
        let file_path_str = file_path
            .to_str()
            .ok_or_else(|| SpliceError::Other(format!("Invalid UTF-8 in path: {:?}", file_path)))?;

        self.inner
            .get_code_chunks_for_symbol(file_path_str, symbol_name)
            .map_err(|e| {
                SpliceError::Other(format!(
                    "Failed to get code chunks for symbol {}: {}",
                    symbol_name, e
                ))
            })
            .map(|chunks| chunks.into_iter().map(CodeChunk::from).collect())
    }

    /// Access the underlying Magellan CodeGraph for advanced operations.
    pub fn inner(&self) -> &MagellanGraph {
        &self.inner
    }

    /// Access the underlying Magellan CodeGraph mutably for advanced operations.
    pub fn inner_mut(&mut self) -> &mut MagellanGraph {
        &mut self.inner
    }

    /// Get comprehensive database statistics.
    ///
    /// Returns counts of all entity types in the graph database.
    pub fn get_statistics(&self) -> Result<DatabaseStats> {
        let files = self
            .inner
            .count_files()
            .map_err(|e| SpliceError::Other(format!("Failed to count files: {}", e)))?;
        let symbols = self
            .inner
            .count_symbols()
            .map_err(|e| SpliceError::Other(format!("Failed to count symbols: {}", e)))?;
        let references = self
            .inner
            .count_references()
            .map_err(|e| SpliceError::Other(format!("Failed to count references: {}", e)))?;
        let code_chunks = self
            .inner
            .count_chunks()
            .map_err(|e| SpliceError::Other(format!("Failed to count code chunks: {}", e)))?;

        // Magellan has no count_calls() method - count Call nodes explicitly
        let calls = self.count_call_nodes()?;

        Ok(DatabaseStats {
            files,
            symbols,
            references,
            calls,
            code_chunks,
        })
    }

    /// Count Call nodes by querying the graph database directly.
    ///
    /// Magellan's CodeGraph doesn't expose entity iteration APIs (entity_ids, get_node),
    /// so we query the database directly to count nodes with kind="Call".
    ///
    /// This is safe because the graph_entities table schema is stable in sqlitegraph.
    fn count_call_nodes(&self) -> Result<usize> {
        use rusqlite::Connection;

        let conn = Connection::open(&self.db_path).map_err(|e| {
            SpliceError::Other(format!("Failed to open database for Call counting: {}", e))
        })?;

        let count: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM graph_entities WHERE kind = 'Call'",
                [],
                |row| row.get(0),
            )
            .map_err(|e| SpliceError::Other(format!("Failed to count Call nodes: {}", e)))?;

        Ok(count as usize)
    }

    /// Query symbols in a file, with optional filters and relationship context.
    ///
    /// # Arguments
    /// * `file_path` - Path to the file to query
    /// * `kind_filter` - Optional symbol kind filter (e.g., "fn", "struct", "class")
    /// * `with_callers` - If true, include symbols that call each returned symbol
    /// * `with_callees` - If true, include symbols that each returned symbol calls
    ///
    /// # Returns
    /// Vector of symbols with their relationships (if requested).
    pub fn query_symbols_by_file(
        &mut self,
        file_path: &Path,
        kind_filter: Option<&str>,
        with_callers: bool,
        with_callees: bool,
    ) -> Result<Vec<SymbolWithRelations>> {
        let path_str = file_path
            .to_str()
            .ok_or_else(|| SpliceError::Other(format!("Invalid UTF-8 in path: {:?}", file_path)))?;

        // Query symbols with optional kind filter
        let symbol_facts = if let Some(kind) = kind_filter {
            let symbol_kind = parse_symbol_kind(kind);
            self.inner
                .symbols_in_file_with_kind(path_str, Some(symbol_kind))
        } else {
            self.inner.symbols_in_file(path_str)
        }
        .map_err(|e| {
            SpliceError::Other(format!("Failed to query symbols in file {}: {}", path_str, e))
        })?;

        // Convert to SymbolWithRelations, optionally fetching relationships
        let mut results = Vec::new();
        for fact in symbol_facts {
            // Skip symbols without names (e.g., impl blocks)
            let name = match fact.name {
                Some(n) => n,
                None => continue,
            };

            let symbol = SymbolInfo {
                entity_id: 0, // SymbolFact doesn't include entity_id
                name: name.clone(),
                file_path: fact.file_path.to_string_lossy().to_string(),
                kind: fact.kind_normalized,
                byte_start: fact.byte_start,
                byte_end: fact.byte_end,
            };

            let (callers, callees) = if with_callers || with_callees {
                self.fetch_call_relationships_for_symbol(path_str, &name, with_callers, with_callees)?
            } else {
                (Vec::new(), Vec::new())
            };

            results.push(SymbolWithRelations { symbol, callers, callees });
        }

        Ok(results)
    }

    /// Fetch call relationships for a symbol by name.
    fn fetch_call_relationships_for_symbol(
        &mut self,
        file_path: &str,
        symbol_name: &str,
        fetch_callers: bool,
        fetch_callees: bool,
    ) -> Result<(Vec<SymbolInfo>, Vec<SymbolInfo>)> {
        let mut callers = Vec::new();
        let mut callees = Vec::new();

        if fetch_callers {
            let call_facts = self
                .inner
                .callers_of_symbol(file_path, symbol_name)
                .map_err(|e| SpliceError::Other(format!("Failed to get callers: {}", e)))?;
            for fact in call_facts {
                // Resolve caller name to SymbolInfo
                // CallFact contains the caller's file_path and name
                if let Ok(caller_symbols) =
                    self.inner.symbol_extents(&fact.file_path.to_string_lossy(), &fact.caller)
                {
                    for (_id, caller_fact) in caller_symbols {
                        callers.push(SymbolInfo {
                            entity_id: _id,
                            name: caller_fact.name.unwrap_or_else(|| fact.caller.clone()),
                            file_path: caller_fact.file_path.to_string_lossy().to_string(),
                            kind: caller_fact.kind_normalized,
                            byte_start: caller_fact.byte_start,
                            byte_end: caller_fact.byte_end,
                        });
                    }
                }
            }
        }

        if fetch_callees {
            let call_facts = self
                .inner
                .calls_from_symbol(file_path, symbol_name)
                .map_err(|e| SpliceError::Other(format!("Failed to get callees: {}", e)))?;
            for fact in call_facts {
                // Resolve callee name to SymbolInfo
                // CallFact contains the callee's file_path and name
                if let Ok(callee_symbols) =
                    self.inner.symbol_extents(&fact.file_path.to_string_lossy(), &fact.callee)
                {
                    for (_id, callee_fact) in callee_symbols {
                        callees.push(SymbolInfo {
                            entity_id: _id,
                            name: callee_fact.name.unwrap_or_else(|| fact.callee.clone()),
                            file_path: callee_fact.file_path.to_string_lossy().to_string(),
                            kind: callee_fact.kind_normalized,
                            byte_start: callee_fact.byte_start,
                            byte_end: callee_fact.byte_end,
                        });
                    }
                }
            }
        }

        Ok((callers, callees))
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

/// Symbol with optional call relationship context.
#[derive(Debug, Clone)]
pub struct SymbolWithRelations {
    /// The symbol's basic information.
    pub symbol: SymbolInfo,
    /// Symbols that call this symbol (if --with-callers flag).
    pub callers: Vec<SymbolInfo>,
    /// Symbols that this symbol calls (if --with-callees flag).
    pub callees: Vec<SymbolInfo>,
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
    /// File path containing this chunk.
    pub file_path: String,
    /// Byte offset where the chunk starts.
    pub byte_start: usize,
    /// Byte offset where the chunk ends.
    pub byte_end: usize,
    /// Symbol name if this chunk belongs to a specific symbol.
    pub symbol_name: Option<String>,
    /// Symbol kind if available.
    pub symbol_kind: Option<String>,
}

impl CodeChunk {
    /// Return the length of the chunk content in bytes.
    pub fn len(&self) -> usize {
        self.content.len()
    }

    /// Check if the chunk content is empty.
    pub fn is_empty(&self) -> bool {
        self.content.is_empty()
    }

    /// Return the chunk content as bytes.
    pub fn as_bytes(&self) -> &[u8] {
        self.content.as_bytes()
    }

    /// Iterate over lines in the chunk content.
    pub fn lines(&self) -> std::str::Lines<'_> {
        self.content.lines()
    }
}

impl From<magellan::CodeChunk> for CodeChunk {
    fn from(chunk: magellan::CodeChunk) -> Self {
        Self {
            content: chunk.content,
            file_path: chunk.file_path,
            byte_start: chunk.byte_start,
            byte_end: chunk.byte_end,
            symbol_name: chunk.symbol_name,
            symbol_kind: chunk.symbol_kind,
        }
    }
}

/// Database statistics for Magellan graph.
#[derive(Debug, Clone)]
pub struct DatabaseStats {
    /// Number of indexed files.
    pub files: usize,
    /// Number of indexed symbols.
    pub symbols: usize,
    /// Number of indexed references.
    pub references: usize,
    /// Number of indexed function calls.
    pub calls: usize,
    /// Number of stored code chunks.
    pub code_chunks: usize,
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
