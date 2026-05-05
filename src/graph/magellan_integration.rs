//! Magellan integration layer with backend split (Batch 1).
//!
//! This module provides integration with Magellan for:
//! - Multi-language code indexing
//! - Label-based symbol queries
//! - Code chunk retrieval (no file re-reading)
//!
//! # Backend Split
//! - SQLite backend: `magellan::CodeGraph` for `.db` files  
//! - Geometric backend: `magellan::graph::geometric_backend::GeometricBackend` for `.geo` files

use crate::error::{Result, SpliceError};
use magellan::{CodeGraph as MagellanGraph, SymbolKind, SymbolQueryResult};
use std::path::{Path, PathBuf};

/// Normalize a user-provided path for database lookup.
///
/// Magellan stores canonical absolute paths for existing files.
/// This helper converts relative paths to absolute so lookups succeed.
fn normalize_lookup_path(path: &Path) -> PathBuf {
    // Try canonicalize first (works for existing files, resolves symlinks)
    if let Ok(canonical) = std::fs::canonicalize(path) {
        canonical
    } else {
        // Fallback for non-existent paths: make absolute relative to current dir
        std::env::current_dir()
            .unwrap_or_else(|_| PathBuf::from("."))
            .join(path)
    }
}

/// Backend type identifier.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
/// Backend type for Magellan integration.
pub enum IntegrationBackend {
    /// SQLite database backend.
    Sqlite,
    /// Geometric spatial backend.
    #[cfg(feature = "geometric")]
    Geometric,
}

/// Parse a normalized symbol kind string into SymbolKind enum.
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

/// Wrapper around Magellan's backends with Splice-specific extensions.
pub struct MagellanIntegration {
    inner: MagellanGraph,
    #[cfg(feature = "geometric")]
    geo_inner: Option<magellan::graph::geometric_backend::GeometricBackend>,
    db_path: PathBuf,
    backend: IntegrationBackend,
}

impl MagellanIntegration {
    /// Get the backend type.
    pub fn backend_type(&self) -> IntegrationBackend {
        self.backend
    }

    /// Check if using geometric backend.
    pub fn is_geometric(&self) -> bool {
        #[cfg(feature = "geometric")]
        return matches!(self.backend, IntegrationBackend::Geometric);
        #[cfg(not(feature = "geometric"))]
        return false;
    }

    /// Open or create a Magellan code graph at the given path.
    ///
    /// Batch 1: Detects backend type from file extension:
    /// - `.db` → SQLite backend
    /// - `.geo` → Geometric backend (requires geometric feature)
    pub fn open(db_path: &Path) -> Result<Self> {
        #[cfg(feature = "geometric")]
        if Self::is_geometric_db(db_path) {
            return Self::open_geometric(db_path);
        }
        Self::open_sqlite(db_path)
    }

    /// Open SQLite backend.
    fn open_sqlite(db_path: &Path) -> Result<Self> {
        let db_path_str = db_path
            .to_str()
            .ok_or_else(|| SpliceError::Other(format!("Invalid UTF-8 in path: {:?}", db_path)))?;

        let inner = MagellanGraph::open(db_path_str).map_err(|e| SpliceError::Magellan {
            context: format!("Failed to open Magellan SQLite graph at {}", db_path_str),
            source: e,
        })?;

        Ok(Self {
            inner,
            #[cfg(feature = "geometric")]
            geo_inner: None,
            db_path: db_path.to_path_buf(),
            backend: IntegrationBackend::Sqlite,
        })
    }

    /// Open Geometric backend.
    #[cfg(feature = "geometric")]
    fn open_geometric(db_path: &Path) -> Result<Self> {
        use magellan::graph::geometric_backend::GeometricBackend;

        let geo = GeometricBackend::open(db_path).map_err(|e| SpliceError::Magellan {
            context: format!("Failed to open Geometric backend at {:?}", db_path),
            source: e,
        })?;

        // Create a dummy in-memory SQLite connection for methods that require it
        // This is a transitional approach for Batch 1
        let inner = MagellanGraph::open(":memory:").map_err(|e| SpliceError::Magellan {
            context: "Failed to create in-memory SQLite for geometric backend".to_string(),
            source: e,
        })?;

        Ok(Self {
            inner,
            geo_inner: Some(geo),
            db_path: db_path.to_path_buf(),
            backend: IntegrationBackend::Geometric,
        })
    }

    /// Access the underlying GeometricBackend (if using Geometric).
    #[cfg(feature = "geometric")]
    pub fn geo_inner(&self) -> Option<&magellan::graph::geometric_backend::GeometricBackend> {
        self.geo_inner.as_ref()
    }

    /// Access the underlying GeometricBackend mutably (if using Geometric).
    #[cfg(feature = "geometric")]
    pub fn geo_inner_mut(
        &mut self,
    ) -> Option<&mut magellan::graph::geometric_backend::GeometricBackend> {
        self.geo_inner.as_mut()
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
    #[cfg(feature = "sqlite")]
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
    #[cfg(feature = "sqlite")]
    pub fn get_all_labels(&self) -> Result<Vec<String>> {
        self.inner
            .get_all_labels()
            .map_err(|e| SpliceError::Other(format!("Failed to get labels: {}", e)))
    }

    /// Count entities with a specific label.
    #[cfg(feature = "sqlite")]
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

    /// Get the database file path for direct database access.
    pub fn db_path(&self) -> &Path {
        &self.db_path
    }

    /// Get comprehensive database statistics.
    ///
    /// Returns counts of all entity types in the graph database.
    pub fn get_statistics(&self) -> Result<DatabaseStats> {
        match self.backend {
            IntegrationBackend::Sqlite => self.get_statistics_sqlite(),
            #[cfg(feature = "geometric")]
            IntegrationBackend::Geometric => self.get_statistics_geometric(),
        }
    }

    /// SQLite implementation of get_statistics.
    fn get_statistics_sqlite(&self) -> Result<DatabaseStats> {
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

    /// Geometric backend implementation of get_statistics.
    #[cfg(feature = "geometric")]
    fn get_statistics_geometric(&self) -> Result<DatabaseStats> {
        if let Some(ref geo) = self.geo_inner {
            let stats = geo.get_geometric_stats();
            Ok(DatabaseStats {
                files: stats.file_count,
                symbols: stats.symbol_count,
                references: 0, // Not tracked separately in geometric backend
                calls: 0,      // Not tracked separately in geometric backend
                code_chunks: stats.cfg_block_count,
            })
        } else {
            Err(SpliceError::Other(
                "Geometric backend not initialized".to_string(),
            ))
        }
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
            SpliceError::Other(format!(
                "Failed to query symbols in file {}: {}",
                path_str, e
            ))
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
                start_line: None,
                end_line: None,
            };

            let (callers, callees) = if with_callers || with_callees {
                self.fetch_call_relationships_for_symbol(
                    path_str,
                    &name,
                    with_callers,
                    with_callees,
                )?
            } else {
                (Vec::new(), Vec::new())
            };

            results.push(SymbolWithRelations {
                symbol,
                callers,
                callees,
            });
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
                if let Ok(caller_symbols) = self
                    .inner
                    .symbol_extents(&fact.file_path.to_string_lossy(), &fact.caller)
                {
                    for (_id, caller_fact) in caller_symbols {
                        callers.push(SymbolInfo {
                            entity_id: _id,
                            name: caller_fact.name.unwrap_or_else(|| fact.caller.clone()),
                            file_path: caller_fact.file_path.to_string_lossy().to_string(),
                            kind: caller_fact.kind_normalized,
                            byte_start: caller_fact.byte_start,
                            byte_end: caller_fact.byte_end,
                            start_line: None,
                            end_line: None,
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
                if let Ok(callee_symbols) = self
                    .inner
                    .symbol_extents(&fact.file_path.to_string_lossy(), &fact.callee)
                {
                    for (_id, callee_fact) in callee_symbols {
                        callees.push(SymbolInfo {
                            entity_id: _id,
                            name: callee_fact.name.unwrap_or_else(|| fact.callee.clone()),
                            file_path: callee_fact.file_path.to_string_lossy().to_string(),
                            kind: callee_fact.kind_normalized,
                            byte_start: callee_fact.byte_start,
                            byte_end: callee_fact.byte_end,
                            start_line: None,
                            end_line: None,
                        });
                    }
                }
            }
        }

        Ok((callers, callees))
    }

    /// Find symbol by name across ALL indexed files.
    ///
    /// # Arguments
    /// * `name` - Symbol name to search for
    /// * `ambiguous` - If true, return all matches. If false, return first match only.
    ///
    /// # Returns
    /// Vector of matching symbols (empty if none found).
    ///
    /// # Performance
    /// This requires O(N) file queries where N = number of indexed files.
    /// Magellan has no global symbol name index.
    ///
    /// Batch 1: Backend-neutral implementation.
    pub fn find_symbol_by_name(&mut self, name: &str, ambiguous: bool) -> Result<Vec<SymbolInfo>> {
        match self.backend {
            IntegrationBackend::Sqlite => self.find_symbol_by_name_sqlite(name, ambiguous),
            #[cfg(feature = "geometric")]
            IntegrationBackend::Geometric => self.find_symbol_by_name_geometric(name, ambiguous),
        }
    }

    /// SQLite implementation of find_symbol_by_name.
    fn find_symbol_by_name_sqlite(
        &mut self,
        name: &str,
        ambiguous: bool,
    ) -> Result<Vec<SymbolInfo>> {
        let mut results = Vec::new();

        // Get all indexed files
        let file_nodes = self
            .inner
            .all_file_nodes()
            .map_err(|e| SpliceError::Other(format!("Failed to get file nodes: {}", e)))?;

        for file_path in file_nodes.keys() {
            // Search for symbol in this file
            if let Ok(matches) = self.inner.symbol_extents(file_path, name) {
                for (entity_id, fact) in matches {
                    let symbol = SymbolInfo {
                        entity_id,
                        name: fact.name.clone().unwrap_or_default(),
                        file_path: fact.file_path.to_string_lossy().to_string(),
                        kind: fact.kind_normalized,
                        byte_start: fact.byte_start,
                        byte_end: fact.byte_end,
                        start_line: Some(fact.start_line),
                        end_line: Some(fact.end_line),
                    };
                    results.push(symbol);

                    // Early exit if not looking for all matches
                    if !ambiguous && !results.is_empty() {
                        return Ok(results);
                    }
                }
            }
        }

        Ok(results)
    }

    /// Geometric backend implementation of find_symbol_by_name.
    #[cfg(feature = "geometric")]
    fn find_symbol_by_name_geometric(
        &self,
        name: &str,
        ambiguous: bool,
    ) -> Result<Vec<SymbolInfo>> {
        if let Some(ref geo) = self.geo_inner {
            let matches = geo.find_symbols_by_name_info(name);
            let results: Vec<SymbolInfo> = matches
                .into_iter()
                .map(|info| SymbolInfo {
                    entity_id: info.id as i64,
                    name: info.name,
                    file_path: info.file_path,
                    kind: format!("{:?}", info.kind),
                    byte_start: info.byte_start as usize,
                    byte_end: info.byte_end as usize,
                    start_line: Some(info.start_line as usize),
                    end_line: Some(info.end_line as usize),
                })
                .collect();

            if !ambiguous && !results.is_empty() {
                Ok(results.into_iter().take(1).collect())
            } else {
                Ok(results)
            }
        } else {
            Err(SpliceError::Other(
                "Geometric backend not initialized".to_string(),
            ))
        }
    }

    /// Find symbol by file path and name.
    ///
    /// Batch 1: Backend-neutral symbol lookup.
    ///
    /// # Arguments
    /// * `file_path` - Path to the file containing the symbol
    /// * `name` - Symbol name to search for
    ///
    /// # Returns
    /// Some(SymbolInfo) if found, None if not found.
    pub fn find_symbol_by_path_and_name(
        &mut self,
        file_path: &Path,
        name: &str,
    ) -> Result<Option<SymbolInfo>> {
        let normalized = normalize_lookup_path(file_path);
        match self.backend {
            IntegrationBackend::Sqlite => {
                let path_str = normalized.to_str().ok_or_else(|| {
                    SpliceError::Other(format!("Invalid UTF-8 in path: {:?}", normalized))
                })?;
                let matches = self
                    .inner
                    .symbol_extents(path_str, name)
                    .map_err(|e| SpliceError::Other(format!("Failed to find symbol: {}", e)))?;

                if let Some((entity_id, fact)) = matches.first() {
                    Ok(Some(SymbolInfo {
                        entity_id: *entity_id,
                        name: fact.name.clone().unwrap_or_else(|| name.to_string()),
                        file_path: fact.file_path.to_string_lossy().to_string(),
                        kind: fact.kind_normalized.clone(),
                        byte_start: fact.byte_start,
                        byte_end: fact.byte_end,
                        start_line: None,
                        end_line: None,
                    }))
                } else {
                    Ok(None)
                }
            }
            #[cfg(feature = "geometric")]
            IntegrationBackend::Geometric => {
                if let Some(ref geo) = self.geo_inner {
                    let path_str = normalized.to_str().ok_or_else(|| {
                        SpliceError::Other(format!("Invalid UTF-8 in path: {:?}", normalized))
                    })?;

                    // Use geometric backend's method to find symbol by name and path
                    if let Some(id) = geo.find_symbol_id_by_name_and_path(name, path_str) {
                        if let Some(info) = geo.find_symbol_by_id_info(id) {
                            return Ok(Some(SymbolInfo {
                                entity_id: id as i64,
                                name: info.name,
                                file_path: info.file_path,
                                kind: format!("{:?}", info.kind),
                                byte_start: info.byte_start as usize,
                                byte_end: info.byte_end as usize,
                                start_line: None,
                                end_line: None,
                            }));
                        }
                    }
                    Ok(None)
                } else {
                    Err(SpliceError::Other(
                        "Geometric backend not initialized".to_string(),
                    ))
                }
            }
        }
    }

    /// Find symbol by 16-char SHA-256 or 32-char BLAKE3 symbol ID.
    ///
    /// # Arguments
    /// * `symbol_id` - 16-char (V1 SHA-256) or 32-char (V2 BLAKE3) lowercase hex symbol ID
    ///
    /// # Returns
    /// Some(SymbolInfo) if found, None if not found.
    ///
    /// # Performance
    /// This requires O(N) entity iteration where N = total symbols.
    /// Magellan does not store symbol_id or provide reverse lookup.
    /// Consider building a symbol_id index in future if performance is inadequate.
    ///
    /// # Note
    /// Symbol IDs are generated as:
    /// - V1: SHA-256(name:path:byte_start)[0..8] -> 16 hex chars
    /// - V2: BLAKE3(name:path:byte_start)[0..16] -> 32 hex chars
    /// We regenerate IDs during iteration to find matches, trying V2 first.
    pub fn find_symbol_by_id(&mut self, symbol_id: &str) -> Result<Option<SymbolInfo>> {
        match self.backend {
            IntegrationBackend::Sqlite => self.find_symbol_by_id_sqlite(symbol_id),
            #[cfg(feature = "geometric")]
            IntegrationBackend::Geometric => self.find_symbol_by_id_geo(symbol_id),
        }
    }

    /// Find symbol by ID (SQLite implementation).
    fn find_symbol_by_id_sqlite(&mut self, symbol_id: &str) -> Result<Option<SymbolInfo>> {
        use crate::symbol_id::{generate_v1, generate_v2};
        use rusqlite::Connection;

        let conn = Connection::open(&self.db_path).map_err(|e| {
            SpliceError::Other(format!(
                "Failed to open database for symbol ID lookup: {}",
                e
            ))
        })?;

        let mut stmt = conn
            .prepare("SELECT id, name, file_path, data FROM graph_entities WHERE kind = 'Symbol'")
            .map_err(|e| SpliceError::Other(format!("Failed to prepare query: {}", e)))?;

        let symbol_rows = stmt
            .query_map([], |row| {
                Ok((
                    row.get::<_, i64>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                    row.get::<_, String>(3)?,
                ))
            })
            .map_err(|e| SpliceError::Other(format!("Failed to query symbols: {}", e)))?;

        for row_result in symbol_rows {
            let (entity_id, name, file_path, data_json) =
                row_result.map_err(|e| SpliceError::Other(format!("Failed to read row: {}", e)))?;

            // Parse the JSON data to get byte_start
            let data: serde_json::Value = serde_json::from_str(&data_json).map_err(|e| {
                SpliceError::Other(format!("Failed to parse symbol data JSON: {}", e))
            })?;

            let byte_start = data
                .get("byte_start")
                .and_then(|v| v.as_u64())
                .ok_or_else(|| SpliceError::Other("Symbol data missing byte_start".to_string()))?;
            let byte_start = byte_start as usize;

            // Try V2 (32-char BLAKE3) first, then V1 (16-char SHA-256) for backward compatibility
            let generated_v2 = generate_v2(&name, &file_path, byte_start);
            if generated_v2.as_str() == symbol_id {
                // Found V2 match - extract remaining fields
                let byte_end = data
                    .get("byte_end")
                    .and_then(|v| v.as_u64())
                    .ok_or_else(|| {
                        SpliceError::Other("Symbol data missing byte_end".to_string())
                    })?;
                let byte_end = byte_end as usize;

                let kind = data
                    .get("kind")
                    .and_then(|v| v.as_str())
                    .unwrap_or("Unknown")
                    .to_string();

                let start_line = data
                    .get("start_line")
                    .and_then(|v| v.as_u64())
                    .map(|l| l as usize);
                let end_line = data
                    .get("end_line")
                    .and_then(|v| v.as_u64())
                    .map(|l| l as usize);

                return Ok(Some(SymbolInfo {
                    entity_id,
                    name,
                    file_path,
                    kind,
                    byte_start,
                    byte_end,
                    start_line,
                    end_line,
                }));
            }

            // Try V1 (16-char SHA-256) for backward compatibility
            let generated_v1 = generate_v1(&name, &file_path, byte_start);
            if generated_v1.as_str() == symbol_id {
                // Found V1 match - extract remaining fields
                let byte_end = data
                    .get("byte_end")
                    .and_then(|v| v.as_u64())
                    .ok_or_else(|| {
                        SpliceError::Other("Symbol data missing byte_end".to_string())
                    })?;
                let byte_end = byte_end as usize;

                let kind = data
                    .get("kind")
                    .and_then(|v| v.as_str())
                    .unwrap_or("Unknown")
                    .to_string();

                let start_line = data
                    .get("start_line")
                    .and_then(|v| v.as_u64())
                    .map(|l| l as usize);
                let end_line = data
                    .get("end_line")
                    .and_then(|v| v.as_u64())
                    .map(|l| l as usize);

                return Ok(Some(SymbolInfo {
                    entity_id,
                    name,
                    file_path,
                    kind,
                    byte_start,
                    byte_end,
                    start_line,
                    end_line,
                }));
            }
        }

        Ok(None)
    }

    /// Find symbol by ID (Geometric implementation).
    #[cfg(feature = "geometric")]
    fn find_symbol_by_id_geo(&mut self, symbol_id: &str) -> Result<Option<SymbolInfo>> {
        if let Some(ref geo) = self.geo_inner {
            // Parse symbol_id as u64 for geometric backend
            let id = symbol_id.parse::<u64>().map_err(|_| {
                SpliceError::Other(format!(
                    "Invalid symbol ID for geometric backend: {}. Expected u64.",
                    symbol_id
                ))
            })?;

            if let Some(info) = geo.find_symbol_by_id_info(id) {
                Ok(Some(SymbolInfo {
                    entity_id: info.id as i64,
                    name: info.name,
                    file_path: info.file_path,
                    kind: format!("{:?}", info.kind),
                    byte_start: info.byte_start as usize,
                    byte_end: info.byte_end as usize,
                    start_line: None,
                    end_line: None,
                }))
            } else {
                Ok(None)
            }
        } else {
            Err(SpliceError::Other(
                "Geometric backend not initialized".to_string(),
            ))
        }
    }

    /// Get call relationships for a symbol.
    ///
    /// # Arguments
    /// * `file_path` - Path to file containing the symbol
    /// * `name` - Symbol name
    /// * `direction` - Which relationships to fetch (In/Out/Both)
    ///
    /// # Returns
    /// CallRelationships containing the symbol and its relationships.
    pub fn get_call_relationships(
        &mut self,
        file_path: &Path,
        name: &str,
        direction: CallDirection,
    ) -> Result<CallRelationships> {
        let normalized = normalize_lookup_path(file_path);
        let path_str = normalized.to_str().ok_or_else(|| {
            SpliceError::Other(format!("Invalid UTF-8 in path: {:?}", normalized))
        })?;

        // Get the target symbol info first
        let symbol_facts = self.inner.symbol_extents(path_str, name).map_err(|e| {
            SpliceError::Other(format!(
                "Failed to find symbol {} in {}: {}",
                name, path_str, e
            ))
        })?;

        if symbol_facts.is_empty() {
            return Err(SpliceError::Other(format!(
                "Symbol '{}' not found in file '{}'",
                name, path_str
            )));
        }

        let (entity_id, fact) = &symbol_facts[0];
        let target_symbol = SymbolInfo {
            entity_id: *entity_id,
            name: fact.name.clone().unwrap_or_else(|| name.to_string()),
            file_path: fact.file_path.to_string_lossy().to_string(),
            kind: fact.kind_normalized.clone(),
            byte_start: fact.byte_start,
            byte_end: fact.byte_end,
            start_line: None,
            end_line: None,
        };

        let (callers, callees) = match direction {
            CallDirection::In => {
                let calls = self
                    .inner
                    .callers_of_symbol(path_str, name)
                    .map_err(|e| SpliceError::Other(format!("Failed to get callers: {}", e)))?;
                (self.resolve_call_facts_to_references(calls)?, Vec::new())
            }
            CallDirection::Out => {
                let calls = self
                    .inner
                    .calls_from_symbol(path_str, name)
                    .map_err(|e| SpliceError::Other(format!("Failed to get callees: {}", e)))?;
                (Vec::new(), self.resolve_call_facts_to_references(calls)?)
            }
            CallDirection::Both => {
                let callers_facts = self
                    .inner
                    .callers_of_symbol(path_str, name)
                    .map_err(|e| SpliceError::Other(format!("Failed to get callers: {}", e)))?;
                let callees_facts = self
                    .inner
                    .calls_from_symbol(path_str, name)
                    .map_err(|e| SpliceError::Other(format!("Failed to get callees: {}", e)))?;
                (
                    self.resolve_call_facts_to_references(callers_facts)?,
                    self.resolve_call_facts_to_references(callees_facts)?,
                )
            }
        };

        Ok(CallRelationships {
            symbol: target_symbol,
            callers,
            callees,
        })
    }

    /// Resolve CallFact vectors to CallReference vectors with symbol info.
    fn resolve_call_facts_to_references(
        &mut self,
        call_facts: Vec<magellan::references::CallFact>,
    ) -> Result<Vec<CallReference>> {
        let mut references = Vec::new();
        let mut seen: std::collections::HashSet<(String, String)> =
            std::collections::HashSet::new();

        for fact in call_facts {
            // Resolve the referenced symbol (caller or callee depending on context)
            let ref_name = &fact.callee;
            let ref_path_str = fact.file_path.to_string_lossy();

            // Get symbol info for the referenced symbol
            let symbol_infos = self
                .inner
                .symbol_extents(&ref_path_str, ref_name)
                .map_err(|e| {
                    SpliceError::Other(format!("Failed to resolve symbol {}: {}", ref_name, e))
                })?;

            for (entity_id, symbol_fact) in symbol_infos {
                let symbol = SymbolInfo {
                    entity_id,
                    name: symbol_fact.name.clone().unwrap_or_else(|| ref_name.clone()),
                    file_path: symbol_fact.file_path.to_string_lossy().to_string(),
                    kind: symbol_fact.kind_normalized.clone(),
                    byte_start: symbol_fact.byte_start,
                    byte_end: symbol_fact.byte_end,
                    start_line: None,
                    end_line: None,
                };

                let key = (symbol.name.clone(), symbol.file_path.clone());
                if !seen.insert(key) {
                    continue;
                }

                let call_site = CallSite {
                    file_path: fact.file_path.to_string_lossy().to_string(),
                    byte_start: fact.byte_start,
                    byte_end: fact.byte_end,
                    start_line: fact.start_line,
                    start_col: fact.start_col,
                    end_line: fact.end_line,
                    end_col: fact.end_col,
                };

                references.push(CallReference { symbol, call_site });
            }
        }

        Ok(references)
    }

    /// List all indexed files, with optional symbol counts.
    ///
    /// Batch 2: Supports both SQLite and Geometric backends.
    pub fn list_indexed_files(&mut self, with_symbol_counts: bool) -> Result<Vec<FileMetadata>> {
        match self.backend {
            IntegrationBackend::Sqlite => {
                let file_nodes = self
                    .inner
                    .all_file_nodes()
                    .map_err(|e| SpliceError::Other(format!("Failed to get file nodes: {}", e)))?;

                file_nodes
                    .into_iter()
                    .map(|(path, node)| {
                        let symbol_count = if with_symbol_counts {
                            Some(self.count_symbols_in_file(&path)?)
                        } else {
                            None
                        };

                        Ok(FileMetadata {
                            path,
                            hash: node.hash,
                            last_indexed_at: node.last_indexed_at,
                            last_modified: node.last_modified,
                            symbol_count,
                        })
                    })
                    .collect()
            }
            #[cfg(feature = "geometric")]
            IntegrationBackend::Geometric => {
                if let Some(ref geo) = self.geo_inner {
                    let files = geo.get_all_files();
                    files
                        .into_iter()
                        .map(|(path, hash, last_indexed)| {
                            let symbol_count = if with_symbol_counts {
                                let symbols = geo.symbols_in_file(&path).map_err(|e| {
                                    SpliceError::Other(format!(
                                        "Failed to count symbols in {}: {}",
                                        path, e
                                    ))
                                })?;
                                Some(symbols.len())
                            } else {
                                None
                            };

                            Ok(FileMetadata {
                                path,
                                hash: hash.unwrap_or_default(),
                                last_indexed_at: last_indexed,
                                last_modified: 0, // Not stored in geometric backend
                                symbol_count,
                            })
                        })
                        .collect()
                } else {
                    Err(SpliceError::Other(
                        "Geometric backend not initialized".to_string(),
                    ))
                }
            }
        }
    }

    /// Count symbols for a specific file.
    fn count_symbols_in_file(&mut self, path: &str) -> Result<usize> {
        match self.backend {
            IntegrationBackend::Sqlite => {
                let symbols = self.inner.symbols_in_file(path).map_err(|e| {
                    SpliceError::Other(format!("Failed to count symbols in {}: {}", path, e))
                })?;
                Ok(symbols.len())
            }
            #[cfg(feature = "geometric")]
            IntegrationBackend::Geometric => {
                if let Some(ref geo) = self.geo_inner {
                    let symbols = geo.symbols_in_file(path).map_err(|e| {
                        SpliceError::Other(format!("Failed to count symbols in {}: {}", path, e))
                    })?;
                    Ok(symbols.len())
                } else {
                    Err(SpliceError::Other(
                        "Geometric backend not initialized".to_string(),
                    ))
                }
            }
        }
    }

    /// Sort references for safe in-order replacement.
    ///
    /// References are sorted by (file_path, byte_start) with descending
    /// byte_start within each file. This ensures that replacing earlier
    /// references doesn't affect byte offsets of later ones.
    ///
    /// This is critical when the new name has different byte length.
    ///
    /// # Arguments
    /// * `references` - References to sort in-place
    pub fn sort_references_for_replacement(references: &mut [magellan::references::ReferenceFact]) {
        references.sort_by(|a, b| {
            // First by file path (ascending) for logical grouping
            match a.file_path.cmp(&b.file_path) {
                std::cmp::Ordering::Equal => {
                    // Then by byte_start within file (descending)
                    // Descending order prevents offset shifts from affecting later replacements
                    b.byte_start.cmp(&a.byte_start)
                }
                other => other,
            }
        });
    }

    /// Validate that a byte span is on UTF-8 character boundaries.
    ///
    /// # Arguments
    /// * `content` - File content as bytes
    /// * `byte_start` - Start offset
    /// * `byte_end` - End offset
    /// * `file_path` - Path to file (for error reporting)
    ///
    /// # Returns
    /// Ok(()) if span is valid, Err with description if invalid
    pub fn validate_utf8_span(
        content: &[u8],
        byte_start: usize,
        byte_end: usize,
        file_path: &Path,
    ) -> Result<()> {
        if byte_start >= content.len() || byte_end > content.len() {
            return Err(SpliceError::InvalidSpan {
                file: file_path.to_path_buf(),
                start: byte_start,
                end: byte_end,
                file_size: content.len(),
            });
        }

        // Convert to str for char_boundary checking
        // SAFETY: We only validate boundaries, not content validity
        let content_str = std::str::from_utf8(content)
            .map_err(|_| SpliceError::Other("File content is not valid UTF-8".to_string()))?;

        // Check start is on character boundary
        if !content_str.is_char_boundary(byte_start) {
            return Err(SpliceError::Other(format!(
                "Byte start {} is not on UTF-8 character boundary",
                byte_start
            )));
        }

        // Check end is on character boundary
        if !content_str.is_char_boundary(byte_end) {
            return Err(SpliceError::Other(format!(
                "Byte end {} is not on UTF-8 character boundary",
                byte_end
            )));
        }

        Ok(())
    }

    /// Get all references for a symbol by entity ID.
    ///
    /// Returns ReferenceFact entries for all references to this symbol
    /// across all indexed files. This is the core data for cross-file rename.
    ///
    /// # Arguments
    /// * `entity_id` - Entity ID of the symbol definition
    ///
    /// # Returns
    /// Vector of ReferenceFact entries with byte spans
    pub fn get_all_references(
        &mut self,
        entity_id: i64,
    ) -> Result<Vec<magellan::references::ReferenceFact>> {
        self.inner.references_to_symbol(entity_id).map_err(|e| {
            SpliceError::Other(format!(
                "Failed to get references for entity {}: {}",
                entity_id, e
            ))
        })
    }

    /// Index references for a file into the graph.
    ///
    /// This extracts all references to known symbols in the file and creates
    /// Reference nodes with REFERENCES edges to the corresponding symbols.
    /// This must be called after `index_file` for proper cross-file reference tracking.
    ///
    /// # Arguments
    /// * `file_path` - Path to the file to index references for
    ///
    /// # Returns
    /// Number of references indexed
    ///
    /// # Errors
    /// Returns Other error if indexing fails
    pub fn index_references(&mut self, file_path: &Path) -> Result<usize> {
        let file_path_str = file_path
            .to_str()
            .ok_or_else(|| SpliceError::Other(format!("Invalid UTF-8 in path: {:?}", file_path)))?;

        let source = std::fs::read(file_path).map_err(|e| {
            SpliceError::Other(format!("Failed to read file {:?}: {}", file_path, e))
        })?;

        self.inner
            .index_references(file_path_str, &source)
            .map_err(|e| {
                SpliceError::Other(format!(
                    "Failed to index references for {:?}: {}",
                    file_path, e
                ))
            })
    }

    /// Get forward reachability (callees) from a symbol.
    ///
    /// # Arguments
    /// * `file_path` - Path to file containing the symbol
    /// * `name` - Symbol name
    /// * `max_depth` - Maximum depth to traverse
    ///
    /// # Returns
    /// Vec<ReachableSymbol> with depth and path information
    ///
    /// Batch 2: Supports both SQLite and Geometric backends.
    pub fn reachable_symbols(
        &mut self,
        file_path: &Path,
        name: &str,
        max_depth: usize,
    ) -> Result<Vec<ReachableSymbol>> {
        let normalized = normalize_lookup_path(file_path);
        match self.backend {
            IntegrationBackend::Sqlite => {
                self.reachable_symbols_sqlite(&normalized, name, max_depth)
            }
            #[cfg(feature = "geometric")]
            IntegrationBackend::Geometric => {
                self.reachable_symbols_geometric(&normalized, name, max_depth)
            }
        }
    }

    /// SQLite implementation of reachable_symbols.
    fn reachable_symbols_sqlite(
        &mut self,
        file_path: &Path,
        name: &str,
        max_depth: usize,
    ) -> Result<Vec<ReachableSymbol>> {
        let path_str = file_path
            .to_str()
            .ok_or_else(|| SpliceError::Other(format!("Invalid UTF-8 in path: {:?}", file_path)))?;

        // Use BFS traversal using calls_from_symbol
        let mut result = Vec::new();
        let mut visited = std::collections::HashSet::new();
        let mut queue = std::collections::VecDeque::new();

        // Start with direct callees at depth 1
        let calls = self
            .inner
            .calls_from_symbol(path_str, name)
            .map_err(|e| SpliceError::Other(format!("Failed to get callees: {}", e)))?;

        for call in calls {
            let key = (
                call.file_path.to_string_lossy().to_string(),
                call.callee.clone(),
            );
            if visited.insert(key.clone()) {
                queue.push_back((
                    call.callee.clone(),
                    call.file_path.to_string_lossy().to_string(),
                    1,
                    vec![name.to_string()],
                ));
            }
        }

        // BFS traversal
        while let Some((symbol_name, symbol_path, depth, path)) = queue.pop_front() {
            if depth > max_depth {
                continue;
            }

            // Get symbol info
            if let Ok(symbol_facts) = self.inner.symbol_extents(&symbol_path, &symbol_name) {
                if let Some((entity_id, fact)) = symbol_facts.first() {
                    let symbol = ReachableSymbol {
                        symbol: SymbolInfo {
                            entity_id: *entity_id,
                            name: fact.name.clone().unwrap_or_else(|| symbol_name.clone()),
                            file_path: fact.file_path.to_string_lossy().to_string(),
                            kind: fact.kind_normalized.clone(),
                            byte_start: fact.byte_start,
                            byte_end: fact.byte_end,
                            start_line: None,
                            end_line: None,
                        },
                        depth,
                        path: path.clone(),
                    };
                    result.push(symbol);

                    // Continue traversal if not at max depth
                    if depth < max_depth {
                        let next_calls = self
                            .inner
                            .calls_from_symbol(&symbol_path, &symbol_name)
                            .unwrap_or_default();
                        for call in next_calls {
                            let key = (
                                call.file_path.to_string_lossy().to_string(),
                                call.callee.clone(),
                            );
                            if visited.insert(key.clone()) {
                                let mut new_path = path.clone();
                                new_path.push(symbol_name.clone());
                                queue.push_back((
                                    call.callee.clone(),
                                    call.file_path.to_string_lossy().to_string(),
                                    depth + 1,
                                    new_path,
                                ));
                            }
                        }
                    }
                }
            }
        }

        Ok(result)
    }

    /// Geometric backend implementation of reachable_symbols.
    #[cfg(feature = "geometric")]
    fn reachable_symbols_geometric(
        &mut self,
        file_path: &Path,
        name: &str,
        max_depth: usize,
    ) -> Result<Vec<ReachableSymbol>> {
        let geo = self
            .geo_inner
            .as_ref()
            .ok_or_else(|| SpliceError::Other("Geometric backend not initialized".to_string()))?;

        let path_str = file_path
            .to_str()
            .ok_or_else(|| SpliceError::Other(format!("Invalid UTF-8 in path: {:?}", file_path)))?;

        // Find the symbol ID by name and path
        let symbol_id = geo
            .find_symbol_id_by_name_and_path(name, path_str)
            .ok_or_else(|| {
                SpliceError::Other(format!(
                    "Symbol '{}' not found in file '{}'",
                    name, path_str
                ))
            })?;

        // Use BFS traversal
        let mut result = Vec::new();
        let mut visited = std::collections::HashSet::new();
        let mut queue = std::collections::VecDeque::new();

        // Start with the target symbol
        visited.insert(symbol_id);
        queue.push_back((symbol_id, 0, vec![name.to_string()]));

        while let Some((current_id, depth, path)) = queue.pop_front() {
            if depth > max_depth {
                continue;
            }

            // Get symbol info
            if let Some(info) = geo.find_symbol_by_id_info(current_id) {
                // Skip the root symbol itself (depth 0)
                if depth > 0 {
                    let symbol = ReachableSymbol {
                        symbol: SymbolInfo {
                            entity_id: current_id as i64,
                            name: info.name.clone(),
                            file_path: info.file_path.clone(),
                            kind: format!("{:?}", info.kind),
                            byte_start: info.byte_start as usize,
                            byte_end: info.byte_end as usize,
                            start_line: None,
                            end_line: None,
                        },
                        depth,
                        path: path.clone(),
                    };
                    result.push(symbol);
                }

                // Continue traversal if not at max depth
                if depth < max_depth {
                    let callees = geo.get_callees(current_id);
                    for callee_id in callees {
                        if visited.insert(callee_id) {
                            let mut new_path = path.clone();
                            new_path.push(info.name.clone());
                            queue.push_back((callee_id, depth + 1, new_path));
                        }
                    }
                }
            }
        }

        Ok(result)
    }

    /// Get reverse reachability (callers) to a symbol.
    ///
    /// # Arguments
    /// * `file_path` - Path to file containing the symbol
    /// * `name` - Symbol name
    /// * `max_depth` - Maximum depth to traverse
    ///
    /// # Returns
    /// Vec<ReachableSymbol> with depth and path information
    ///
    /// Batch 2: Supports both SQLite and Geometric backends.
    pub fn reverse_reachable_symbols(
        &mut self,
        file_path: &Path,
        name: &str,
        max_depth: usize,
    ) -> Result<Vec<ReachableSymbol>> {
        let normalized = normalize_lookup_path(file_path);
        match self.backend {
            IntegrationBackend::Sqlite => {
                self.reverse_reachable_symbols_sqlite(&normalized, name, max_depth)
            }
            #[cfg(feature = "geometric")]
            IntegrationBackend::Geometric => {
                self.reverse_reachable_symbols_geometric(&normalized, name, max_depth)
            }
        }
    }

    /// SQLite implementation of reverse_reachable_symbols.
    fn reverse_reachable_symbols_sqlite(
        &mut self,
        file_path: &Path,
        name: &str,
        max_depth: usize,
    ) -> Result<Vec<ReachableSymbol>> {
        let path_str = file_path
            .to_str()
            .ok_or_else(|| SpliceError::Other(format!("Invalid UTF-8 in path: {:?}", file_path)))?;

        // Similar to reachable_symbols but uses callers_of_symbol
        let mut result = Vec::new();
        let mut visited = std::collections::HashSet::new();
        let mut queue = std::collections::VecDeque::new();

        let callers = self
            .inner
            .callers_of_symbol(path_str, name)
            .map_err(|e| SpliceError::Other(format!("Failed to get callers: {}", e)))?;

        for call in callers {
            let key = (
                call.file_path.to_string_lossy().to_string(),
                call.caller.clone(),
            );
            if visited.insert(key.clone()) {
                queue.push_back((
                    call.caller.clone(),
                    call.file_path.to_string_lossy().to_string(),
                    1,
                    vec![name.to_string()],
                ));
            }
        }

        while let Some((symbol_name, symbol_path, depth, path)) = queue.pop_front() {
            if depth > max_depth {
                continue;
            }

            if let Ok(symbol_facts) = self.inner.symbol_extents(&symbol_path, &symbol_name) {
                if let Some((entity_id, fact)) = symbol_facts.first() {
                    let symbol = ReachableSymbol {
                        symbol: SymbolInfo {
                            entity_id: *entity_id,
                            name: fact.name.clone().unwrap_or_else(|| symbol_name.clone()),
                            file_path: fact.file_path.to_string_lossy().to_string(),
                            kind: fact.kind_normalized.clone(),
                            byte_start: fact.byte_start,
                            byte_end: fact.byte_end,
                            start_line: None,
                            end_line: None,
                        },
                        depth,
                        path: path.clone(),
                    };
                    result.push(symbol);

                    if depth < max_depth {
                        let next_callers = self
                            .inner
                            .callers_of_symbol(&symbol_path, &symbol_name)
                            .unwrap_or_default();
                        for call in next_callers {
                            let key = (
                                call.file_path.to_string_lossy().to_string(),
                                call.caller.clone(),
                            );
                            if visited.insert(key.clone()) {
                                let mut new_path = path.clone();
                                new_path.push(symbol_name.clone());
                                queue.push_back((
                                    call.caller.clone(),
                                    call.file_path.to_string_lossy().to_string(),
                                    depth + 1,
                                    new_path,
                                ));
                            }
                        }
                    }
                }
            }
        }

        Ok(result)
    }

    /// Geometric backend implementation of reverse_reachable_symbols.
    #[cfg(feature = "geometric")]
    fn reverse_reachable_symbols_geometric(
        &mut self,
        file_path: &Path,
        name: &str,
        max_depth: usize,
    ) -> Result<Vec<ReachableSymbol>> {
        let geo = self
            .geo_inner
            .as_ref()
            .ok_or_else(|| SpliceError::Other("Geometric backend not initialized".to_string()))?;

        let path_str = file_path
            .to_str()
            .ok_or_else(|| SpliceError::Other(format!("Invalid UTF-8 in path: {:?}", file_path)))?;

        // Find the symbol ID by name and path
        let symbol_id = geo
            .find_symbol_id_by_name_and_path(name, path_str)
            .ok_or_else(|| {
                SpliceError::Other(format!(
                    "Symbol '{}' not found in file '{}'",
                    name, path_str
                ))
            })?;

        // Use BFS traversal with callers
        let mut result = Vec::new();
        let mut visited = std::collections::HashSet::new();
        let mut queue = std::collections::VecDeque::new();

        // Start with the target symbol
        visited.insert(symbol_id);
        queue.push_back((symbol_id, 0, vec![name.to_string()]));

        while let Some((current_id, depth, path)) = queue.pop_front() {
            if depth > max_depth {
                continue;
            }

            // Get symbol info
            if let Some(info) = geo.find_symbol_by_id_info(current_id) {
                // Skip the root symbol itself (depth 0)
                if depth > 0 {
                    let symbol = ReachableSymbol {
                        symbol: SymbolInfo {
                            entity_id: current_id as i64,
                            name: info.name.clone(),
                            file_path: info.file_path.clone(),
                            kind: format!("{:?}", info.kind),
                            byte_start: info.byte_start as usize,
                            byte_end: info.byte_end as usize,
                            start_line: None,
                            end_line: None,
                        },
                        depth,
                        path: path.clone(),
                    };
                    result.push(symbol);
                }

                // Continue traversal if not at max depth
                if depth < max_depth {
                    let callers = geo.get_callers(current_id);
                    for caller_id in callers {
                        if visited.insert(caller_id) {
                            let mut new_path = path.clone();
                            new_path.push(info.name.clone());
                            queue.push_back((caller_id, depth + 1, new_path));
                        }
                    }
                }
            }
        }

        Ok(result)
    }

    /// Detect all cycles in the call graph.
    ///
    /// Uses Tarjan's SCC algorithm to find strongly connected components
    /// with more than one node (cycles) or self-loops.
    ///
    /// # Arguments
    /// * `max_cycles` - Maximum number of cycles to return
    ///
    /// # Returns
    /// Vec<CycleInfo> describing detected cycles
    ///
    /// Batch 2: Supports both SQLite and Geometric backends.
    pub fn detect_cycles(&mut self, max_cycles: usize) -> Result<Vec<CycleInfo>> {
        match self.backend {
            IntegrationBackend::Sqlite => self.detect_cycles_sqlite(max_cycles),
            #[cfg(feature = "geometric")]
            IntegrationBackend::Geometric => self.detect_cycles_geometric(max_cycles),
        }
    }

    /// SQLite implementation of detect_cycles.
    fn detect_cycles_sqlite(&mut self, max_cycles: usize) -> Result<Vec<CycleInfo>> {
        use std::collections::{HashMap, HashSet};

        // Build call graph: symbol -> set of callees
        let mut call_graph: HashMap<(String, String), HashSet<(String, String)>> = HashMap::new();
        let mut all_symbols: HashSet<(String, String)> = HashSet::new();

        let file_nodes = self
            .inner
            .all_file_nodes()
            .map_err(|e| SpliceError::Other(format!("Failed to get file nodes: {}", e)))?;

        for file_path in file_nodes.keys() {
            let symbols = self
                .inner
                .symbols_in_file(file_path)
                .map_err(|e| SpliceError::Other(format!("Failed to get symbols: {}", e)))?;

            for fact in symbols {
                if let Some(ref name) = fact.name {
                    let key = (fact.file_path.to_string_lossy().to_string(), name.clone());
                    all_symbols.insert(key.clone());
                    call_graph.entry(key).or_default();
                }
            }
        }

        // Add edges
        for (caller, callees) in call_graph.iter_mut() {
            let calls = self
                .inner
                .calls_from_symbol(&caller.0, &caller.1)
                .unwrap_or_default();

            for call in calls {
                let callee_key = (
                    call.file_path.to_string_lossy().to_string(),
                    call.callee.clone(),
                );
                if all_symbols.contains(&callee_key) {
                    callees.insert(callee_key);
                }
            }
        }

        // Find SCCs using iterative DFS (Tarjan's algorithm simplified)
        let sccs = self.find_sccs(&call_graph)?;

        // Convert SCCs to cycles (only SCCs with size > 1 or self-loops)
        let mut cycles = Vec::new();

        for (index, scc) in sccs.iter().enumerate() {
            if cycles.len() >= max_cycles {
                break;
            }

            // Filter: cycles have size > 1 OR are self-loops
            let is_self_loop = scc.len() == 1 && self.has_self_loop(&scc[0], &call_graph);
            let is_cycle = scc.len() > 1 || is_self_loop;

            if is_cycle {
                cycles.push(self.scc_to_cycle_info(scc, index, is_self_loop)?);
            }
        }

        Ok(cycles)
    }

    /// Geometric backend implementation of detect_cycles.
    #[cfg(feature = "geometric")]
    fn detect_cycles_geometric(&mut self, max_cycles: usize) -> Result<Vec<CycleInfo>> {
        let geo = self
            .geo_inner
            .as_ref()
            .ok_or_else(|| SpliceError::Other("Geometric backend not initialized".to_string()))?;

        // Use geometric backend's cycle detection
        let cycles_ids = geo.find_call_graph_cycles();

        let mut cycles = Vec::new();
        for (index, cycle_ids) in cycles_ids.iter().enumerate() {
            if cycles.len() >= max_cycles {
                break;
            }

            let is_self_loop = cycle_ids.len() == 1;
            let mut members = Vec::new();

            for &id in cycle_ids {
                if let Some(info) = geo.find_symbol_by_id_info(id) {
                    members.push(SymbolInfo {
                        entity_id: id as i64,
                        name: info.name,
                        file_path: info.file_path,
                        kind: format!("{:?}", info.kind),
                        byte_start: info.byte_start as usize,
                        byte_end: info.byte_end as usize,
                        start_line: None,
                        end_line: None,
                    });
                }
            }

            if !members.is_empty() {
                // Sort for representative selection
                members.sort_by(|a, b| a.name.cmp(&b.name));
                let representative = members[0].clone();

                cycles.push(CycleInfo {
                    id: format!("cycle-{}", index),
                    size: cycle_ids.len(),
                    members,
                    representative,
                    is_self_loop,
                });
            }
        }

        Ok(cycles)
    }

    /// Find cycles containing a specific symbol.
    ///
    /// # Arguments
    /// * `file_path` - Path to file containing the symbol
    /// * `symbol_name` - Name of the symbol
    /// * `max_cycles` - Maximum number of cycles to return
    ///
    /// # Returns
    /// Vec<CycleInfo> for cycles containing the specified symbol
    pub fn find_cycles_containing(
        &mut self,
        file_path: &Path,
        symbol_name: &str,
        max_cycles: usize,
    ) -> Result<Vec<CycleInfo>> {
        let path_str = file_path
            .to_str()
            .ok_or_else(|| SpliceError::Other(format!("Invalid UTF-8 in path: {:?}", file_path)))?;

        // Get all cycles first
        let all_cycles = self.detect_cycles(usize::MAX)?;

        // Filter cycles containing the target symbol
        let _target_key = (path_str.to_string(), symbol_name.to_string());

        let mut matching_cycles = Vec::new();
        for cycle in all_cycles {
            if matching_cycles.len() >= max_cycles {
                break;
            }

            let contains_target = cycle
                .members
                .iter()
                .any(|m| m.file_path == path_str && m.name == symbol_name);

            if contains_target {
                matching_cycles.push(cycle);
            }
        }

        Ok(matching_cycles)
    }

    /// Find strongly connected components using iterative DFS.
    fn find_sccs(
        &self,
        graph: &std::collections::HashMap<
            (String, String),
            std::collections::HashSet<(String, String)>,
        >,
    ) -> Result<Vec<Vec<(String, String)>>> {
        use std::collections::{HashMap, HashSet};

        let mut index = 0i32;
        let mut indices: HashMap<(String, String), i32> = HashMap::new();
        let mut lowlink: HashMap<(String, String), i32> = HashMap::new();
        let mut on_stack: HashSet<(String, String)> = HashSet::new();
        let mut stack: Vec<(String, String)> = Vec::new();
        let mut sccs: Vec<Vec<(String, String)>> = Vec::new();

        for node in graph.keys() {
            if !indices.contains_key(node) {
                self.scc_dfs(
                    node,
                    graph,
                    &mut index,
                    &mut indices,
                    &mut lowlink,
                    &mut on_stack,
                    &mut stack,
                    &mut sccs,
                )?;
            }
        }

        Ok(sccs)
    }

    /// Recursive DFS helper for SCC detection (iterative to avoid stack overflow).
    fn scc_dfs(
        &self,
        node: &(String, String),
        graph: &std::collections::HashMap<
            (String, String),
            std::collections::HashSet<(String, String)>,
        >,
        index: &mut i32,
        indices: &mut std::collections::HashMap<(String, String), i32>,
        lowlink: &mut std::collections::HashMap<(String, String), i32>,
        on_stack: &mut std::collections::HashSet<(String, String)>,
        stack: &mut Vec<(String, String)>,
        sccs: &mut Vec<Vec<(String, String)>>,
    ) -> Result<()> {
        indices.insert(node.clone(), *index);
        lowlink.insert(node.clone(), *index);
        *index += 1;
        stack.push(node.clone());
        on_stack.insert(node.clone());

        if let Some(neighbors) = graph.get(node) {
            for neighbor in neighbors {
                if !indices.contains_key(neighbor) {
                    self.scc_dfs(
                        neighbor, graph, index, indices, lowlink, on_stack, stack, sccs,
                    )?;
                    let neighbor_low = *lowlink.get(neighbor).unwrap_or(&0);
                    let current_low = lowlink.get_mut(node).unwrap();
                    *current_low = (*current_low).min(neighbor_low);
                } else if on_stack.contains(neighbor) {
                    let neighbor_idx = *indices.get(neighbor).unwrap_or(&0);
                    let current_low = lowlink.get_mut(node).unwrap();
                    *current_low = (*current_low).min(neighbor_idx);
                }
            }
        }

        // If node is a root node, pop the stack and generate an SCC
        let node_low = *lowlink.get(node).unwrap_or(&0);
        let node_idx = *indices.get(node).unwrap_or(&0);
        if node_low == node_idx {
            let mut scc = Vec::new();
            loop {
                let w = stack.pop().unwrap();
                on_stack.remove(&w);
                if &w == node {
                    scc.push(w);
                    break;
                }
                scc.push(w);
            }
            sccs.push(scc);
        }

        Ok(())
    }

    /// Check if a symbol has a self-loop (calls itself).
    fn has_self_loop(
        &self,
        node: &(String, String),
        graph: &std::collections::HashMap<
            (String, String),
            std::collections::HashSet<(String, String)>,
        >,
    ) -> bool {
        if let Some(callees) = graph.get(node) {
            callees.contains(node)
        } else {
            false
        }
    }

    /// Convert an SCC to CycleInfo.
    fn scc_to_cycle_info(
        &mut self,
        scc: &[(String, String)],
        index: usize,
        is_self_loop: bool,
    ) -> Result<CycleInfo> {
        let mut members = Vec::new();

        for (file_path, symbol_name) in scc {
            if let Ok(symbol_facts) = self.inner_mut().symbol_extents(file_path, symbol_name) {
                if let Some((entity_id, fact)) = symbol_facts.first() {
                    members.push(SymbolInfo {
                        entity_id: *entity_id,
                        name: fact.name.clone().unwrap_or_else(|| symbol_name.clone()),
                        file_path: fact.file_path.to_string_lossy().to_string(),
                        kind: fact.kind_normalized.clone(),
                        byte_start: fact.byte_start,
                        byte_end: fact.byte_end,
                        start_line: None,
                        end_line: None,
                    });
                }
            }
        }

        // Sort for representative selection
        members.sort_by(|a, b| a.name.cmp(&b.name));

        let representative = members
            .first()
            .ok_or_else(|| SpliceError::Other("Empty cycle".to_string()))?
            .clone();

        let id = format!("cycle-{}", index);

        Ok(CycleInfo {
            id,
            size: scc.len(),
            members,
            representative,
            is_self_loop,
        })
    }

    /// Detect dead code (unreachable symbols) from an entry point.
    ///
    /// # Arguments
    /// * `entry_file` - Path to file containing entry point
    /// * `entry_symbol` - Name of entry point symbol
    /// * `exclude_public` - Whether to exclude public symbols from results
    ///
    /// # Returns
    /// Vec<DeadSymbol> of unreachable symbols
    ///
    /// Batch 2: Supports both SQLite and Geometric backends.
    pub fn dead_symbols(
        &mut self,
        entry_file: &Path,
        entry_symbol: &str,
        exclude_public: bool,
    ) -> Result<Vec<DeadSymbol>> {
        let normalized = normalize_lookup_path(entry_file);
        match self.backend {
            IntegrationBackend::Sqlite => {
                self.dead_symbols_sqlite(&normalized, entry_symbol, exclude_public)
            }
            #[cfg(feature = "geometric")]
            IntegrationBackend::Geometric => {
                self.dead_symbols_geometric(&normalized, entry_symbol, exclude_public)
            }
        }
    }

    /// SQLite implementation of dead_symbols.
    fn dead_symbols_sqlite(
        &mut self,
        entry_file: &Path,
        entry_symbol: &str,
        exclude_public: bool,
    ) -> Result<Vec<DeadSymbol>> {
        use std::collections::{HashMap, HashSet};

        let entry_path_str = entry_file.to_str().ok_or_else(|| {
            SpliceError::Other(format!("Invalid UTF-8 in path: {:?}", entry_file))
        })?;

        // Step 1: Get all symbols in the database
        let mut all_symbols = HashMap::new();
        let file_nodes = self
            .inner
            .all_file_nodes()
            .map_err(|e| SpliceError::Other(format!("Failed to get file nodes: {}", e)))?;

        for file_path in file_nodes.keys() {
            let symbols = self.inner.symbols_in_file(file_path).map_err(|e| {
                SpliceError::Other(format!("Failed to get symbols in {}: {}", file_path, e))
            })?;

            for fact in symbols {
                if let Some(ref name) = fact.name {
                    let key = (fact.file_path.to_string_lossy().to_string(), name.clone());
                    all_symbols.insert(key, (fact, false)); // false = not yet visited
                }
            }
        }

        // Step 2: BFS from entry point to mark reachable symbols
        let mut visited = HashSet::new();
        let mut queue = std::collections::VecDeque::new();

        // Start with entry point
        let entry_key = (entry_path_str.to_string(), entry_symbol.to_string());
        queue.push_back(entry_key.clone());
        visited.insert(entry_key);

        while let Some((file_path, symbol_name)) = queue.pop_front() {
            // Get all callees of this symbol
            let callees = self
                .inner
                .calls_from_symbol(&file_path, &symbol_name)
                .unwrap_or_default();

            for call in callees {
                let callee_key = (
                    call.file_path.to_string_lossy().to_string(),
                    call.callee.clone(),
                );
                if visited.insert(callee_key.clone()) {
                    queue.push_back(callee_key);
                }
            }
        }

        // Step 3: Collect unvisited symbols as dead code
        let mut dead_symbols = Vec::new();

        for ((file_path, symbol_name), (fact, _)) in all_symbols {
            if !visited.contains(&(file_path.clone(), symbol_name.clone())) {
                // Skip if excluding public symbols
                if exclude_public && is_public_symbol(&fact) {
                    continue;
                }

                let dead = DeadSymbol {
                    symbol: SymbolInfo {
                        entity_id: 0, // entity_id not available from SymbolFact
                        name: symbol_name.clone(),
                        file_path: fact.file_path.to_string_lossy().to_string(),
                        kind: fact.kind_normalized,
                        byte_start: fact.byte_start,
                        byte_end: fact.byte_end,
                        start_line: None,
                        end_line: None,
                    },
                    reason: "Not reachable from entry point".to_string(),
                };
                dead_symbols.push(dead);
            }
        }

        Ok(dead_symbols)
    }

    /// Geometric backend implementation of dead_symbols.
    #[cfg(feature = "geometric")]
    fn dead_symbols_geometric(
        &mut self,
        entry_file: &Path,
        entry_symbol: &str,
        _exclude_public: bool,
    ) -> Result<Vec<DeadSymbol>> {
        let geo = self
            .geo_inner
            .as_ref()
            .ok_or_else(|| SpliceError::Other("Geometric backend not initialized".to_string()))?;

        let entry_path_str = entry_file.to_str().ok_or_else(|| {
            SpliceError::Other(format!("Invalid UTF-8 in path: {:?}", entry_file))
        })?;

        // Find the entry symbol ID
        let entry_id = geo
            .find_symbol_id_by_name_and_path(entry_symbol, entry_path_str)
            .ok_or_else(|| {
                SpliceError::Other(format!(
                    "Entry symbol '{}' not found in file '{}'",
                    entry_symbol, entry_path_str
                ))
            })?;

        // Get all reachable symbols from the entry point
        let reachable_ids: std::collections::HashSet<u64> =
            geo.reachable_from(entry_id).into_iter().collect();

        // Get all symbols in the database
        let all_symbols = geo
            .get_all_symbols()
            .map_err(|e| SpliceError::Other(format!("Failed to get all symbols: {}", e)))?;

        // Collect unreachable symbols
        let mut dead_symbols = Vec::new();
        for info in all_symbols {
            if !reachable_ids.contains(&(info.id as u64)) {
                // Note: exclude_public is not implemented for geometric backend
                // as SymbolInfo doesn't have visibility information
                let dead = DeadSymbol {
                    symbol: SymbolInfo {
                        entity_id: info.id as i64,
                        name: info.name,
                        file_path: info.file_path,
                        kind: format!("{:?}", info.kind),
                        byte_start: info.byte_start as usize,
                        byte_end: info.byte_end as usize,
                        start_line: None,
                        end_line: None,
                    },
                    reason: "Not reachable from entry point".to_string(),
                };
                dead_symbols.push(dead);
            }
        }

        Ok(dead_symbols)
    }

    /// Forward slice: find all symbols affected by changes to the target.
    ///
    /// This computes the transitive closure of callees from the target symbol.
    ///
    /// # Arguments
    /// * `file_path` - Path to file containing target symbol
    /// * `symbol_name` - Name of target symbol
    /// * `max_depth` - Optional maximum depth to traverse
    ///
    /// # Returns
    /// Vec<SlicedSymbol> with distance and relationship information
    pub fn forward_slice(
        &mut self,
        file_path: &Path,
        symbol_name: &str,
        max_depth: Option<usize>,
    ) -> Result<Vec<SlicedSymbol>> {
        let path_str = file_path
            .to_str()
            .ok_or_else(|| SpliceError::Other(format!("Invalid UTF-8 in path: {:?}", file_path)))?;

        let mut result = Vec::new();
        let mut visited: std::collections::HashSet<(String, String)> =
            std::collections::HashSet::new();
        let mut queue: std::collections::VecDeque<(String, String, usize)> =
            std::collections::VecDeque::new();

        // Get target symbol info
        let target_facts = self
            .inner
            .symbol_extents(path_str, symbol_name)
            .map_err(|e| SpliceError::Other(format!("Failed to find target: {}", e)))?;

        if target_facts.is_empty() {
            return Err(SpliceError::SymbolNotFound {
                message: format!("Target '{}' not found in '{}'", symbol_name, path_str),
                symbol: symbol_name.to_string(),
                file: Some(file_path.to_path_buf()),
                hint: String::new(),
            });
        }

        let (target_id, target_fact) = &target_facts[0];
        let target_key = (
            target_fact.file_path.to_string_lossy().to_string(),
            symbol_name.to_string(),
        );
        visited.insert(target_key.clone());

        // Add target symbol at distance 0
        result.push(SlicedSymbol {
            symbol: SymbolInfo {
                entity_id: *target_id,
                name: target_fact
                    .name
                    .clone()
                    .unwrap_or_else(|| symbol_name.to_string()),
                file_path: target_fact.file_path.to_string_lossy().to_string(),
                kind: target_fact.kind_normalized.clone(),
                byte_start: target_fact.byte_start,
                byte_end: target_fact.byte_end,
                start_line: None,
                end_line: None,
            },
            distance: 0,
            is_target: true,
            relationship: "target".to_string(),
        });

        // BFS for forward slice
        let calls = self
            .inner
            .calls_from_symbol(path_str, symbol_name)
            .unwrap_or_default();

        for call in calls {
            let key = (
                call.file_path.to_string_lossy().to_string(),
                call.callee.clone(),
            );
            if visited.insert(key) {
                queue.push_back((call.file_path.to_string_lossy().to_string(), call.callee, 1));
            }
        }

        while let Some((file, name, dist)) = queue.pop_front() {
            if let Some(max_d) = max_depth {
                if dist > max_d {
                    continue;
                }
            }

            // Get symbol info
            if let Ok(symbol_facts) = self.inner.symbol_extents(&file, &name) {
                if let Some((entity_id, fact)) = symbol_facts.first() {
                    result.push(SlicedSymbol {
                        symbol: SymbolInfo {
                            entity_id: *entity_id,
                            name: fact.name.clone().unwrap_or_else(|| name.clone()),
                            file_path: fact.file_path.to_string_lossy().to_string(),
                            kind: fact.kind_normalized.clone(),
                            byte_start: fact.byte_start,
                            byte_end: fact.byte_end,
                            start_line: None,
                            end_line: None,
                        },
                        distance: dist,
                        is_target: false,
                        relationship: "calls".to_string(),
                    });

                    // Continue BFS
                    let next_calls = self
                        .inner
                        .calls_from_symbol(&file, &name)
                        .unwrap_or_default();

                    for call in next_calls {
                        let key = (
                            call.file_path.to_string_lossy().to_string(),
                            call.callee.clone(),
                        );
                        if visited.insert(key) {
                            queue.push_back((
                                call.file_path.to_string_lossy().to_string(),
                                call.callee,
                                dist + 1,
                            ));
                        }
                    }
                }
            }
        }

        Ok(result)
    }

    /// Backward slice: find all symbols that affect the target.
    ///
    /// This computes the transitive closure of callers to the target symbol.
    ///
    /// # Arguments
    /// * `file_path` - Path to file containing target symbol
    /// * `symbol_name` - Name of target symbol
    /// * `max_depth` - Optional maximum depth to traverse
    ///
    /// # Returns
    /// Vec<SlicedSymbol> with distance and relationship information
    pub fn backward_slice(
        &mut self,
        file_path: &Path,
        symbol_name: &str,
        max_depth: Option<usize>,
    ) -> Result<Vec<SlicedSymbol>> {
        let path_str = file_path
            .to_str()
            .ok_or_else(|| SpliceError::Other(format!("Invalid UTF-8 in path: {:?}", file_path)))?;

        let mut result = Vec::new();
        let mut visited: std::collections::HashSet<(String, String)> =
            std::collections::HashSet::new();
        let mut queue: std::collections::VecDeque<(String, String, usize)> =
            std::collections::VecDeque::new();

        // Get target symbol info
        let target_facts = self
            .inner
            .symbol_extents(path_str, symbol_name)
            .map_err(|e| SpliceError::Other(format!("Failed to find target: {}", e)))?;

        if target_facts.is_empty() {
            return Err(SpliceError::SymbolNotFound {
                message: format!("Target '{}' not found in '{}'", symbol_name, path_str),
                symbol: symbol_name.to_string(),
                file: Some(file_path.to_path_buf()),
                hint: String::new(),
            });
        }

        let (target_id, target_fact) = &target_facts[0];
        let target_key = (
            target_fact.file_path.to_string_lossy().to_string(),
            symbol_name.to_string(),
        );
        visited.insert(target_key.clone());

        // Add target symbol at distance 0
        result.push(SlicedSymbol {
            symbol: SymbolInfo {
                entity_id: *target_id,
                name: target_fact
                    .name
                    .clone()
                    .unwrap_or_else(|| symbol_name.to_string()),
                file_path: target_fact.file_path.to_string_lossy().to_string(),
                kind: target_fact.kind_normalized.clone(),
                byte_start: target_fact.byte_start,
                byte_end: target_fact.byte_end,
                start_line: None,
                end_line: None,
            },
            distance: 0,
            is_target: true,
            relationship: "target".to_string(),
        });

        // BFS for backward slice
        let callers = self
            .inner
            .callers_of_symbol(path_str, symbol_name)
            .unwrap_or_default();

        for call in callers {
            let key = (
                call.file_path.to_string_lossy().to_string(),
                call.caller.clone(),
            );
            if visited.insert(key) {
                queue.push_back((call.file_path.to_string_lossy().to_string(), call.caller, 1));
            }
        }

        while let Some((file, name, dist)) = queue.pop_front() {
            if let Some(max_d) = max_depth {
                if dist > max_d {
                    continue;
                }
            }

            // Get symbol info
            if let Ok(symbol_facts) = self.inner.symbol_extents(&file, &name) {
                if let Some((entity_id, fact)) = symbol_facts.first() {
                    result.push(SlicedSymbol {
                        symbol: SymbolInfo {
                            entity_id: *entity_id,
                            name: fact.name.clone().unwrap_or_else(|| name.clone()),
                            file_path: fact.file_path.to_string_lossy().to_string(),
                            kind: fact.kind_normalized.clone(),
                            byte_start: fact.byte_start,
                            byte_end: fact.byte_end,
                            start_line: None,
                            end_line: None,
                        },
                        distance: dist,
                        is_target: false,
                        relationship: "called_by".to_string(),
                    });

                    // Continue BFS
                    let next_callers = self
                        .inner
                        .callers_of_symbol(&file, &name)
                        .unwrap_or_default();

                    for call in next_callers {
                        let key = (
                            call.file_path.to_string_lossy().to_string(),
                            call.caller.clone(),
                        );
                        if visited.insert(key) {
                            queue.push_back((
                                call.file_path.to_string_lossy().to_string(),
                                call.caller,
                                dist + 1,
                            ));
                        }
                    }
                }
            }
        }

        Ok(result)
    }

    /// Compute the condensation graph (SCCs collapsed to DAG).
    ///
    /// Returns SCCs, edges between them, and topological levels.
    pub fn condense_graph(&mut self) -> Result<CondensationGraph> {
        use std::collections::{HashMap, HashSet};

        // Build call graph
        let mut call_graph: HashMap<(String, String), HashSet<(String, String)>> = HashMap::new();
        let mut all_symbols: Vec<(String, String)> = Vec::new();

        let file_nodes = self
            .inner
            .all_file_nodes()
            .map_err(|e| SpliceError::Other(format!("Failed to get file nodes: {}", e)))?;

        for file_path in file_nodes.keys() {
            let symbols = self
                .inner
                .symbols_in_file(file_path)
                .map_err(|e| SpliceError::Other(format!("Failed to get symbols: {}", e)))?;

            for fact in symbols {
                if let Some(name) = &fact.name {
                    let key = (fact.file_path.to_string_lossy().to_string(), name.clone());
                    call_graph.entry(key.clone()).or_default();
                    all_symbols.push(key);
                }
            }
        }

        // Add edges
        // First collect all valid callees, then add them to avoid borrow issues
        let mut edges_to_add: Vec<((String, String), (String, String))> = Vec::new();
        for caller in call_graph.keys() {
            let calls = self
                .inner
                .calls_from_symbol(&caller.0, &caller.1)
                .unwrap_or_default();

            for call in calls {
                let callee_key = (
                    call.file_path.to_string_lossy().to_string(),
                    call.callee.clone(),
                );
                if call_graph.contains_key(&callee_key) {
                    edges_to_add.push((caller.clone(), callee_key));
                }
            }
        }

        // Now add the collected edges
        for (caller, callee) in edges_to_add {
            if let Some(callees) = call_graph.get_mut(&caller) {
                callees.insert(callee);
            }
        }

        // Find SCCs
        let sccs = self.find_sccs(&call_graph)?;

        // Assign SCC IDs
        let mut symbol_to_scc: HashMap<(String, String), usize> = HashMap::new();
        let mut scc_members: Vec<Vec<(String, String)>> = vec![Vec::new(); sccs.len()];

        for (scc_id, scc) in sccs.into_iter().enumerate() {
            for symbol in &scc {
                symbol_to_scc.insert(symbol.clone(), scc_id);
                scc_members[scc_id].push(symbol.clone());
            }
        }

        // Build edges between SCCs
        let mut scc_edges: HashMap<(usize, usize), usize> = HashMap::new();

        for (caller, callees) in &call_graph {
            let caller_scc = symbol_to_scc.get(caller).copied().unwrap_or(usize::MAX);

            for callee in callees {
                let callee_scc = symbol_to_scc.get(callee).copied().unwrap_or(usize::MAX);

                // Only edges between different SCCs
                if caller_scc != callee_scc && caller_scc != usize::MAX && callee_scc != usize::MAX
                {
                    *scc_edges.entry((caller_scc, callee_scc)).or_insert(0) += 1;
                }
            }
        }

        // Compute topological levels
        let mut in_degree: HashMap<usize, usize> = HashMap::new();
        let mut out_neighbors: HashMap<usize, HashSet<usize>> = HashMap::new();

        for ((from, to), _weight) in &scc_edges {
            *in_degree.entry(*to).or_insert(0) += 1;
            out_neighbors.entry(*from).or_default().insert(*to);
            in_degree.entry(*from).or_insert(0); // ensure all SCCs have entry
        }

        // Initialize in_degree for SCCs with no edges
        for scc_id in 0..scc_members.len() {
            in_degree.entry(scc_id).or_insert(0);
        }

        // Topological sort to assign levels
        let mut levels: Vec<Vec<usize>> = Vec::new();
        let mut queue: Vec<usize> = in_degree
            .iter()
            .filter(|(_, &deg)| deg == 0)
            .map(|(&id, _)| id)
            .collect();

        while !queue.is_empty() {
            queue.sort_unstable();
            levels.push(queue.clone());

            let mut next_queue = Vec::new();
            for scc_id in queue {
                if let Some(neighbors) = out_neighbors.get(&scc_id) {
                    for &neighbor in neighbors {
                        let deg = in_degree.get_mut(&neighbor).unwrap();
                        *deg -= 1;
                        if *deg == 0 {
                            next_queue.push(neighbor);
                        }
                    }
                }
            }
            queue = next_queue;
        }

        // Build result
        let sccs_result: Vec<CondensedScc> = scc_members
            .iter()
            .enumerate()
            .map(|(id, members)| {
                let is_cycle = members.len() > 1;
                let representative = if let Some((path, name)) = members.first() {
                    match self.inner.symbol_extents(path, name) {
                        Ok(facts) => {
                            if let Some((eid, fact)) = facts.first() {
                                SymbolInfo {
                                    entity_id: *eid,
                                    name: fact.name.clone().unwrap_or_else(|| name.clone()),
                                    file_path: fact.file_path.to_string_lossy().to_string(),
                                    kind: fact.kind_normalized.clone(),
                                    byte_start: fact.byte_start,
                                    byte_end: fact.byte_end,
                                    start_line: None,
                                    end_line: None,
                                }
                            } else {
                                // Fallback if no facts found
                                SymbolInfo {
                                    entity_id: 0,
                                    name: name.clone(),
                                    file_path: path.clone(),
                                    kind: "Unknown".to_string(),
                                    byte_start: 0,
                                    byte_end: 0,
                                    start_line: None,
                                    end_line: None,
                                }
                            }
                        }
                        Err(_) => {
                            // Fallback on error
                            SymbolInfo {
                                entity_id: 0,
                                name: name.clone(),
                                file_path: path.clone(),
                                kind: "Unknown".to_string(),
                                byte_start: 0,
                                byte_end: 0,
                                start_line: None,
                                end_line: None,
                            }
                        }
                    }
                } else {
                    // Empty SCC - shouldn't happen but handle gracefully
                    SymbolInfo {
                        entity_id: 0,
                        name: "unknown".to_string(),
                        file_path: "unknown".to_string(),
                        kind: "Unknown".to_string(),
                        byte_start: 0,
                        byte_end: 0,
                        start_line: None,
                        end_line: None,
                    }
                };

                CondensedScc {
                    id: format!("scc-{}", id),
                    size: members.len(),
                    is_cycle,
                    members: None, // populated separately if needed
                    representative,
                }
            })
            .collect();

        let edges_result: Vec<SccEdge> = scc_edges
            .into_iter()
            .map(|((from, to), weight)| SccEdge {
                from: format!("scc-{}", from),
                to: format!("scc-{}", to),
                weight,
            })
            .collect();

        let levels_result: Vec<LevelInfo> = levels
            .iter()
            .enumerate()
            .map(|(level, sccs)| LevelInfo {
                level,
                scc_ids: sccs.iter().map(|id| format!("scc-{}", id)).collect(),
                count: sccs.len(),
            })
            .collect();

        Ok(CondensationGraph {
            scc_count: scc_members.len(),
            cycle_scc_count: scc_members.iter().filter(|scc| scc.len() > 1).count(),
            singleton_count: scc_members.iter().filter(|scc| scc.len() == 1).count(),
            sccs: sccs_result,
            edges: edges_result,
            levels: levels_result,
        })
    }

    /// Generate DOT graph output for impact visualization.
    ///
    /// # Arguments
    /// * `symbol_id` - Entity ID of the root symbol
    /// * `direction` - Direction of traversal (Forward/Reverse/Both)
    /// * `config` - Configuration for DOT output
    ///
    /// # Returns
    /// DOT format string suitable for Graphviz rendering
    pub fn generate_impact_dot(
        &mut self,
        symbol_id: &str,
        direction: &crate::cli::ReachabilityDirection,
        config: &ImpactDotConfig,
    ) -> Result<String> {
        use crate::cli::ReachabilityDirection;

        // Parse symbol_id as "file_path:symbol_name" format
        let (file_path, symbol_name) = symbol_id.split_once(':').ok_or_else(|| {
            SpliceError::Other(format!(
                "Invalid symbol_id format: '{}'. Expected 'file_path:symbol_name'",
                symbol_id
            ))
        })?;

        let file_path_obj = Path::new(file_path);

        // Collect reachable symbols based on direction
        let (forward_symbols, reverse_symbols) = match direction {
            ReachabilityDirection::Forward => {
                let symbols = self.reachable_symbols(
                    file_path_obj,
                    symbol_name,
                    config.max_depth.unwrap_or(10),
                )?;
                (symbols, Vec::new())
            }
            ReachabilityDirection::Reverse => {
                let symbols = self.reverse_reachable_symbols(
                    file_path_obj,
                    symbol_name,
                    config.max_depth.unwrap_or(10),
                )?;
                (Vec::new(), symbols)
            }
            ReachabilityDirection::Both => {
                let max_depth = config.max_depth.unwrap_or(10);
                let forward = self.reachable_symbols(file_path_obj, symbol_name, max_depth)?;
                let reverse =
                    self.reverse_reachable_symbols(file_path_obj, symbol_name, max_depth)?;
                (forward, reverse)
            }
        };

        // Generate DOT output
        let mut dot = String::from("digraph Impact {\n");
        dot.push_str("  rankdir=LR;\n");
        dot.push_str("  node [shape=box, style=rounded];\n\n");

        // Track all edges to avoid duplicates
        let mut edges = std::collections::HashSet::new();
        let mut nodes = std::collections::HashSet::new();

        // Add root node
        let root_label = if config.show_symbol_kinds {
            format!(
                "{} ({})",
                symbol_name,
                _get_root_kind(self, file_path, symbol_name)
            )
        } else {
            symbol_name.to_string()
        };

        let root_attrs = if config
            .highlight_symbol
            .as_ref()
            .map_or(false, |h| *h == symbol_name)
        {
            " [style=filled, fillcolor=lightblue]"
        } else {
            ""
        };

        dot.push_str(&format!(
            "  \"{}\"{} [label=\"{}\"];\n",
            _sanitize_id(symbol_id),
            root_attrs,
            _escape_label(&root_label)
        ));
        nodes.insert(symbol_id.to_string());

        // Add forward edges (callees)
        for reachable in &forward_symbols {
            let caller_id = format!("{}:{}", reachable.symbol.file_path, reachable.symbol.name);
            let label = if config.show_symbol_kinds {
                format!("{} ({})", reachable.symbol.name, reachable.symbol.kind)
            } else {
                reachable.symbol.name.clone()
            };

            // Add node if not already added
            if nodes.insert(caller_id.clone()) {
                let attrs = if config
                    .highlight_symbol
                    .as_ref()
                    .map_or(false, |h| *h == reachable.symbol.name)
                {
                    " [style=filled, fillcolor=lightblue]"
                } else {
                    ""
                };
                dot.push_str(&format!(
                    "  \"{}\"{} [label=\"{}\"];\n",
                    _sanitize_id(&caller_id),
                    attrs,
                    _escape_label(&label)
                ));
            }

            // Add edge: root -> callee
            let edge = (symbol_id.to_string(), caller_id.clone());
            if edges.insert(edge) {
                dot.push_str(&format!(
                    "  \"{}\" -> \"{}\";\n",
                    _sanitize_id(symbol_id),
                    _sanitize_id(&caller_id)
                ));
            }

            // Add edges along the path
            for i in 0..reachable.path.len() {
                let from = if i == 0 {
                    symbol_id.to_string()
                } else {
                    format!("{}:{}", reachable.symbol.file_path, reachable.path[i - 1])
                };
                let to = format!("{}:{}", reachable.symbol.file_path, reachable.path[i]);
                let edge = (from.clone(), to.clone());
                if edges.insert(edge) {
                    dot.push_str(&format!(
                        "  \"{}\" -> \"{}\";\n",
                        _sanitize_id(&from),
                        _sanitize_id(&to)
                    ));
                }
            }
        }

        // Add reverse edges (callers)
        for reachable in &reverse_symbols {
            let caller_id = format!("{}:{}", reachable.symbol.file_path, reachable.symbol.name);
            let label = if config.show_symbol_kinds {
                format!("{} ({})", reachable.symbol.name, reachable.symbol.kind)
            } else {
                reachable.symbol.name.clone()
            };

            if nodes.insert(caller_id.clone()) {
                let attrs = if config
                    .highlight_symbol
                    .as_ref()
                    .map_or(false, |h| *h == reachable.symbol.name)
                {
                    " [style=filled, fillcolor=lightblue]"
                } else {
                    ""
                };
                dot.push_str(&format!(
                    "  \"{}\"{} [label=\"{}\"];\n",
                    _sanitize_id(&caller_id),
                    attrs,
                    _escape_label(&label)
                ));
            }

            // Add edge: caller -> root
            let edge = (caller_id.clone(), symbol_id.to_string());
            if edges.insert(edge) {
                dot.push_str(&format!(
                    "  \"{}\" -> \"{}\";\n",
                    _sanitize_id(&caller_id),
                    _sanitize_id(symbol_id)
                ));
            }

            // Add edges along the path
            for i in 0..reachable.path.len() {
                let from = format!("{}:{}", reachable.symbol.file_path, reachable.path[i]);
                let to = if i == reachable.path.len() - 1 {
                    symbol_id.to_string()
                } else {
                    format!("{}:{}", reachable.symbol.file_path, reachable.path[i + 1])
                };
                let edge = (from.clone(), to.clone());
                if edges.insert(edge) {
                    dot.push_str(&format!(
                        "  \"{}\" -> \"{}\";\n",
                        _sanitize_id(&from),
                        _sanitize_id(&to)
                    ));
                }
            }
        }

        dot.push_str("}\n");
        Ok(dot)
    }

    /// Generate DOT graph output for refs command.
    ///
    /// # Arguments
    /// * `symbol_name` - Symbol name
    /// * `file_path` - Path to file containing the symbol
    /// * `config` - Configuration for DOT output
    ///
    /// # Returns
    /// DOT format string suitable for Graphviz rendering
    pub fn generate_refs_dot(
        &mut self,
        symbol_name: &str,
        file_path: &Path,
        config: &ImpactDotConfig,
    ) -> Result<String> {
        let path_str = file_path
            .to_str()
            .ok_or_else(|| SpliceError::Other(format!("Invalid UTF-8 in path: {:?}", file_path)))?;

        let symbol_id = format!("{}:{}", path_str, symbol_name);

        // Get callers and callees
        let callers = self
            .inner
            .callers_of_symbol(path_str, symbol_name)
            .map_err(|e| SpliceError::Other(format!("Failed to get callers: {}", e)))?;

        let callees = self
            .inner
            .calls_from_symbol(path_str, symbol_name)
            .map_err(|e| SpliceError::Other(format!("Failed to get callees: {}", e)))?;

        // Generate DOT output
        let mut dot = String::from("digraph Impact {\n");
        dot.push_str("  rankdir=LR;\n");
        dot.push_str("  node [shape=box, style=rounded];\n\n");

        // Add root node
        let root_label = if config.show_symbol_kinds {
            format!(
                "{} ({})",
                symbol_name,
                _get_root_kind(self, path_str, symbol_name)
            )
        } else {
            symbol_name.to_string()
        };

        let root_attrs = if config
            .highlight_symbol
            .as_ref()
            .map_or(false, |h| *h == symbol_name)
        {
            " [style=filled, fillcolor=lightblue]"
        } else {
            ""
        };

        dot.push_str(&format!(
            "  \"{}\"{} [label=\"{}\"];\n",
            _sanitize_id(&symbol_id),
            root_attrs,
            _escape_label(&root_label)
        ));

        // Add caller nodes and edges
        for call in &callers {
            let caller_id = format!("{}:{}", call.file_path.to_string_lossy(), call.caller);
            let label = if config.show_symbol_kinds {
                // Try to get kind info
                let kind = _get_symbol_kind(self, &call.file_path.to_string_lossy(), &call.caller);
                format!("{} ({})", call.caller, kind)
            } else {
                call.caller.clone()
            };

            let attrs = if config
                .highlight_symbol
                .as_ref()
                .map_or(false, |h| *h == call.caller)
            {
                " [style=filled, fillcolor=lightblue]"
            } else {
                ""
            };

            dot.push_str(&format!(
                "  \"{}\"{} [label=\"{}\"];\n",
                _sanitize_id(&caller_id),
                attrs,
                _escape_label(&label)
            ));
            dot.push_str(&format!(
                "  \"{}\" -> \"{}\";\n",
                _sanitize_id(&caller_id),
                _sanitize_id(&symbol_id)
            ));
        }

        // Add callee nodes and edges
        for call in &callees {
            let callee_id = format!("{}:{}", call.file_path.to_string_lossy(), call.callee);
            let label = if config.show_symbol_kinds {
                let kind = _get_symbol_kind(self, &call.file_path.to_string_lossy(), &call.callee);
                format!("{} ({})", call.callee, kind)
            } else {
                call.callee.clone()
            };

            let attrs = if config
                .highlight_symbol
                .as_ref()
                .map_or(false, |h| *h == call.callee)
            {
                " [style=filled, fillcolor=lightblue]"
            } else {
                ""
            };

            dot.push_str(&format!(
                "  \"{}\"{} [label=\"{}\"];\n",
                _sanitize_id(&callee_id),
                attrs,
                _escape_label(&label)
            ));
            dot.push_str(&format!(
                "  \"{}\" -> \"{}\";\n",
                _sanitize_id(&symbol_id),
                _sanitize_id(&callee_id)
            ));
        }

        dot.push_str("}\n");
        Ok(dot)
    }
}

/// Check if a symbol is public (exported).
///
/// This is a heuristic check based on symbol kind and naming conventions.
/// For more accurate results, language-specific analysis would be needed.
fn is_public_symbol(fact: &magellan::SymbolFact) -> bool {
    // Functions starting with uppercase are typically public in Rust
    if fact.kind_normalized == "fn" {
        if let Some(name) = &fact.name {
            if let Some(first_char) = name.chars().next() {
                return first_char.is_uppercase();
            }
        }
    }

    // Structs, enums, traits, impls are typically public
    matches!(
        fact.kind_normalized.as_str(),
        "struct" | "enum" | "trait" | "interface" | "class"
    )
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
    /// Line number where the symbol starts (1-indexed).
    pub start_line: Option<usize>,
    /// Line number where the symbol ends (1-indexed).
    pub end_line: Option<usize>,
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

/// Direction for call relationship traversal.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CallDirection {
    /// Get callers only (symbols that call this symbol).
    In,
    /// Get callees only (symbols that this symbol calls).
    Out,
    /// Get both callers and callees.
    Both,
}

/// Location of a call in source code.
#[derive(Debug, Clone)]
pub struct CallSite {
    /// File path containing the call.
    pub file_path: String,
    /// Byte offset where call starts.
    pub byte_start: usize,
    /// Byte offset where call ends.
    pub byte_end: usize,
    /// Line number where call starts (1-indexed).
    pub start_line: usize,
    /// Column number where call starts (0-indexed).
    pub start_col: usize,
    /// Line number where call ends (1-indexed).
    pub end_line: usize,
    /// Column number where call ends (0-indexed).
    pub end_col: usize,
}

/// A call relationship reference with symbol and call site.
#[derive(Debug, Clone)]
pub struct CallReference {
    /// The symbol being referenced (caller or callee).
    pub symbol: SymbolInfo,
    /// Location of the call site.
    pub call_site: CallSite,
}

/// Call relationships for a symbol.
#[derive(Debug, Clone)]
pub struct CallRelationships {
    /// The symbol whose relationships are being queried.
    pub symbol: SymbolInfo,
    /// Symbols that call this symbol (if direction is In or Both).
    pub callers: Vec<CallReference>,
    /// Symbols that this symbol calls (if direction is Out or Both).
    pub callees: Vec<CallReference>,
}

/// A symbol in reachability analysis with depth and path.
#[derive(Debug, Clone)]
pub struct ReachableSymbol {
    /// The symbol's basic information.
    pub symbol: SymbolInfo,
    /// Depth from root (0 = root, 1 = direct relationship, etc.).
    pub depth: usize,
    /// Call path from root to this symbol.
    pub path: Vec<String>,
}

/// A dead (unreachable) symbol.
#[derive(Debug, Clone)]
pub struct DeadSymbol {
    /// The symbol's basic information.
    pub symbol: SymbolInfo,
    /// Reason why this symbol is considered dead.
    pub reason: String,
}

/// Information about a detected cycle.
#[derive(Debug, Clone)]
pub struct CycleInfo {
    /// Unique cycle identifier.
    pub id: String,
    /// Number of symbols in the cycle.
    pub size: usize,
    /// Symbols in the cycle.
    pub members: Vec<SymbolInfo>,
    /// Representative symbol (e.g., alphabetically first).
    pub representative: SymbolInfo,
    /// Whether this is a self-loop (single symbol calling itself).
    pub is_self_loop: bool,
}

/// Condensation graph result (SCCs collapsed to DAG).
#[derive(Debug, Clone)]
pub struct CondensationGraph {
    /// Total number of SCCs.
    pub scc_count: usize,
    /// Number of SCCs that are cycles.
    pub cycle_scc_count: usize,
    /// Number of singleton SCCs.
    pub singleton_count: usize,
    /// SCCs in the graph.
    pub sccs: Vec<CondensedScc>,
    /// Edges between SCCs.
    pub edges: Vec<SccEdge>,
    /// Topological levels.
    pub levels: Vec<LevelInfo>,
}

/// A condensed SCC.
#[derive(Debug, Clone)]
/// A condensed strongly connected component.
pub struct CondensedScc {
    /// Unique identifier for the SCC.
    pub id: String,
    /// Number of symbols in the SCC.
    pub size: usize,
    /// Whether the SCC contains a cycle.
    pub is_cycle: bool,
    /// Member symbols, if expanded.
    pub members: Option<Vec<SymbolInfo>>,
    /// Representative symbol for the SCC.
    pub representative: SymbolInfo,
}

/// Edge between SCCs.
#[derive(Debug, Clone)]
/// Edge between two strongly connected components.
pub struct SccEdge {
    /// Source SCC identifier.
    pub from: String,
    /// Target SCC identifier.
    pub to: String,
    /// Number of edges between the SCCs.
    pub weight: usize,
}

/// Topological level.
#[derive(Debug, Clone)]
/// Topological level in the condensation graph.
pub struct LevelInfo {
    /// Level number in topological order.
    pub level: usize,
    /// SCC identifiers at this level.
    pub scc_ids: Vec<String>,
    /// Number of SCCs at this level.
    pub count: usize,
}

/// Configuration for DOT graph generation.
#[derive(Debug, Clone)]
pub struct ImpactDotConfig {
    /// Show symbol kinds in node labels (e.g., "main (fn)").
    pub show_symbol_kinds: bool,
    /// Maximum depth for traversal (None = unlimited).
    pub max_depth: Option<usize>,
    /// Symbol to highlight in graph (fillcolor=lightblue).
    pub highlight_symbol: Option<String>,
}

impl Default for ImpactDotConfig {
    fn default() -> Self {
        Self {
            show_symbol_kinds: true,
            max_depth: Some(10),
            highlight_symbol: None,
        }
    }
}

/// A symbol in a program slice.
#[derive(Debug, Clone)]
pub struct SlicedSymbol {
    /// The symbol.
    pub symbol: SymbolInfo,
    /// Distance from target.
    pub distance: usize,
    /// Whether this is the target symbol.
    pub is_target: bool,
    /// Relationship type.
    pub relationship: String,
}

/// File metadata with optional symbol count.
#[derive(Debug, Clone)]
pub struct FileMetadata {
    /// Path to the file.
    pub path: String,
    /// Content hash of the file.
    pub hash: String,
    /// Unix timestamp when file was last indexed.
    pub last_indexed_at: i64,
    /// Unix timestamp when file was last modified.
    pub last_modified: i64,
    /// Symbol count if requested (None if --symbols flag not provided).
    pub symbol_count: Option<usize>,
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
            start_line: None,
            end_line: None,
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

/// Helper to get the kind of a root symbol for DOT labels.
fn _get_root_kind(
    integration: &mut MagellanIntegration,
    file_path: &str,
    symbol_name: &str,
) -> String {
    integration
        .inner
        .symbol_extents(file_path, symbol_name)
        .ok()
        .and_then(|facts| facts.first().map(|(_, fact)| fact.kind_normalized.clone()))
        .unwrap_or_else(|| "unknown".to_string())
}

/// Helper to get the kind of a symbol for DOT labels.
fn _get_symbol_kind(
    integration: &mut MagellanIntegration,
    file_path: &str,
    symbol_name: &str,
) -> String {
    integration
        .inner
        .symbol_extents(file_path, symbol_name)
        .ok()
        .and_then(|facts| facts.first().map(|(_, fact)| fact.kind_normalized.clone()))
        .unwrap_or_else(|| "unknown".to_string())
}

/// Escape special DOT characters in labels.
fn _escape_label(label: &str) -> String {
    label
        .replace('\\', "\\\\")
        .replace('"', "\\\"")
        .replace('{', "\\{")
        .replace('}', "\\}")
        .replace('<', "\\<")
        .replace('>', "\\>")
        .replace('|', "\\|")
}

/// Sanitize a string for use as a DOT node ID.
fn _sanitize_id(id: &str) -> String {
    // Replace invalid characters with underscores
    id.chars()
        .map(|c| {
            if c.is_alphanumeric() || c == '_' || c == '-' || c == '.' {
                c
            } else {
                '_'
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[cfg(feature = "sqlite")]
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

    #[cfg(feature = "sqlite")]
    #[test]
    fn test_count_by_label() {
        let temp_dir = TempDir::new().unwrap();
        let db_path = temp_dir.path().join("test.db");

        let integration = MagellanIntegration::open(&db_path).unwrap();

        // Count should be 0 for empty graph
        let count = integration.count_by_label("rust").unwrap();
        assert_eq!(count, 0);
    }

    #[test]
    fn test_sort_references_for_replacement() {
        use std::path::PathBuf;

        // Create test references in unsorted order
        let mut references = vec![
            magellan::references::ReferenceFact {
                file_path: PathBuf::from("/src/b.rs"),
                referenced_symbol: "foo".to_string(),
                byte_start: 50,
                byte_end: 53,
                start_line: 2,
                start_col: 0,
                end_line: 2,
                end_col: 3,
            },
            magellan::references::ReferenceFact {
                file_path: PathBuf::from("/src/a.rs"),
                referenced_symbol: "foo".to_string(),
                byte_start: 100,
                byte_end: 103,
                start_line: 5,
                start_col: 0,
                end_line: 5,
                end_col: 3,
            },
            magellan::references::ReferenceFact {
                file_path: PathBuf::from("/src/a.rs"),
                referenced_symbol: "foo".to_string(),
                byte_start: 20,
                byte_end: 23,
                start_line: 1,
                start_col: 0,
                end_line: 1,
                end_col: 3,
            },
            magellan::references::ReferenceFact {
                file_path: PathBuf::from("/src/b.rs"),
                referenced_symbol: "foo".to_string(),
                byte_start: 10,
                byte_end: 13,
                start_line: 1,
                start_col: 0,
                end_line: 1,
                end_col: 3,
            },
        ];

        MagellanIntegration::sort_references_for_replacement(&mut references);

        // Should be sorted by file (ascending), then byte_start (descending)
        assert_eq!(references[0].file_path, PathBuf::from("/src/a.rs"));
        assert_eq!(references[0].byte_start, 100); // First in a.rs (highest offset)

        assert_eq!(references[1].file_path, PathBuf::from("/src/a.rs"));
        assert_eq!(references[1].byte_start, 20); // Second in a.rs

        assert_eq!(references[2].file_path, PathBuf::from("/src/b.rs"));
        assert_eq!(references[2].byte_start, 50); // First in b.rs (highest offset)

        assert_eq!(references[3].file_path, PathBuf::from("/src/b.rs"));
        assert_eq!(references[3].byte_start, 10); // Second in b.rs
    }

    #[test]
    fn test_validate_utf8_span_valid() {
        let content = "Hello, world!";
        let file_path = Path::new("/test.rs");

        // Valid span
        assert!(
            MagellanIntegration::validate_utf8_span(content.as_bytes(), 0, 5, file_path).is_ok()
        );

        // Full span
        assert!(MagellanIntegration::validate_utf8_span(
            content.as_bytes(),
            0,
            content.len(),
            file_path
        )
        .is_ok());
    }

    #[test]
    fn test_validate_utf8_span_out_of_bounds() {
        let content = "Hello";
        let file_path = Path::new("/test.rs");

        // Start beyond content length
        assert!(
            MagellanIntegration::validate_utf8_span(content.as_bytes(), 10, 15, file_path).is_err()
        );

        // End beyond content length
        assert!(
            MagellanIntegration::validate_utf8_span(content.as_bytes(), 0, 10, file_path).is_err()
        );
    }

    #[test]
    fn test_validate_utf8_span_multibyte_boundary() {
        let content = "Hello 世界"; // "世界" is 6 bytes (3 each)
        let file_path = Path::new("/test.rs");

        // Span ends in middle of multibyte character (invalid)
        // "世界" starts at byte 6, first char ends at byte 9
        assert!(MagellanIntegration::validate_utf8_span(
            content.as_bytes(),
            0,
            8, // Ends in middle of first Chinese character
            file_path
        )
        .is_err());

        // Valid span (full multibyte character)
        assert!(MagellanIntegration::validate_utf8_span(
            content.as_bytes(),
            6,
            9, // Exactly the first Chinese character
            file_path
        )
        .is_ok());
    }

    #[test]
    fn test_validate_utf8_span_invalid_utf8() {
        // Invalid UTF-8 sequence
        let content: &[u8] = &[0xFF, 0xFE, 0xFD];
        let file_path = Path::new("/test.rs");

        // Should fail because content is not valid UTF-8
        assert!(MagellanIntegration::validate_utf8_span(content, 0, 1, file_path).is_err());
    }
}
