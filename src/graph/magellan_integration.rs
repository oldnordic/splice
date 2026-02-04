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

        // Convert anyhow::Error to SpliceError::Magellan for proper error mapping
        let inner = MagellanGraph::open(db_path_str).map_err(|e| {
            SpliceError::Magellan {
                context: format!("Failed to open Magellan graph at {}", db_path_str),
                source: e,
            }
        })?;

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
    pub fn find_symbol_by_name(
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
        use crate::symbol_id::{generate_v1, generate_v2};
        use rusqlite::Connection;

        let conn = Connection::open(&self.db_path).map_err(|e| {
            SpliceError::Other(format!("Failed to open database for symbol ID lookup: {}", e))
        })?;

        let mut stmt = conn
            .prepare(
                "SELECT id, name, file_path, data FROM graph_entities WHERE kind = 'Symbol'",
            )
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
            let data: serde_json::Value =
                serde_json::from_str(&data_json).map_err(|e| {
                    SpliceError::Other(format!("Failed to parse symbol data JSON: {}", e))
                })?;

            let byte_start = data
                .get("byte_start")
                .and_then(|v| v.as_u64())
                .ok_or_else(|| {
                    SpliceError::Other("Symbol data missing byte_start".to_string())
                })?;
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

                return Ok(Some(SymbolInfo {
                    entity_id,
                    name,
                    file_path,
                    kind,
                    byte_start,
                    byte_end,
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

                return Ok(Some(SymbolInfo {
                    entity_id,
                    name,
                    file_path,
                    kind,
                    byte_start,
                    byte_end,
                }));
            }
        }

        Ok(None)
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
        let path_str = file_path
            .to_str()
            .ok_or_else(|| SpliceError::Other(format!("Invalid UTF-8 in path: {:?}", file_path)))?;

        // Get the target symbol info first
        let symbol_facts = self
            .inner
            .symbol_extents(path_str, name)
            .map_err(|e| SpliceError::Other(format!("Failed to find symbol {} in {}: {}", name, path_str, e)))?;

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

        for fact in call_facts {
            // Resolve the referenced symbol (caller or callee depending on context)
            let ref_name = &fact.callee;
            let ref_path_str = fact.file_path.to_string_lossy();

            // Get symbol info for the referenced symbol
            let symbol_infos = self
                .inner
                .symbol_extents(&ref_path_str, ref_name)
                .map_err(|e| SpliceError::Other(format!("Failed to resolve symbol {}: {}", ref_name, e)))?;

            for (entity_id, symbol_fact) in symbol_infos {
                let symbol = SymbolInfo {
                    entity_id,
                    name: symbol_fact.name.clone().unwrap_or_else(|| ref_name.clone()),
                    file_path: symbol_fact.file_path.to_string_lossy().to_string(),
                    kind: symbol_fact.kind_normalized.clone(),
                    byte_start: symbol_fact.byte_start,
                    byte_end: symbol_fact.byte_end,
                };

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
    /// # Arguments
    /// * `with_symbol_counts` - If true, include symbol count per file
    ///
    /// # Returns
    /// Vector of file metadata for all indexed files.
    pub fn list_indexed_files(
        &mut self,
        with_symbol_counts: bool,
    ) -> Result<Vec<FileMetadata>> {
        let file_nodes = self.inner.all_file_nodes()
            .map_err(|e| SpliceError::Other(format!("Failed to get file nodes: {}", e)))?;

        file_nodes.into_iter().map(|(path, node)| {
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
        }).collect()
    }

    /// Count symbols for a specific file.
    fn count_symbols_in_file(&mut self, path: &str) -> Result<usize> {
        let symbols = self.inner.symbols_in_file(path)
            .map_err(|e| SpliceError::Other(format!("Failed to count symbols in {}: {}", path, e)))?;
        Ok(symbols.len())
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
                other => other
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
        let content_str = std::str::from_utf8(content).map_err(|_| SpliceError::Other(
            "File content is not valid UTF-8".to_string()
        ))?;

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
    pub fn get_all_references(&mut self, entity_id: i64) -> Result<Vec<magellan::references::ReferenceFact>> {
        self.inner
            .references_to_symbol(entity_id)
            .map_err(|e| SpliceError::Other(format!("Failed to get references for entity {}: {}", entity_id, e)))
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
            .map_err(|e| SpliceError::Other(format!("Failed to index references for {:?}: {}", file_path, e)))
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
        assert!(MagellanIntegration::validate_utf8_span(
            content.as_bytes(),
            0,
            5,
            file_path
        ).is_ok());

        // Full span
        assert!(MagellanIntegration::validate_utf8_span(
            content.as_bytes(),
            0,
            content.len(),
            file_path
        ).is_ok());
    }

    #[test]
    fn test_validate_utf8_span_out_of_bounds() {
        let content = "Hello";
        let file_path = Path::new("/test.rs");

        // Start beyond content length
        assert!(MagellanIntegration::validate_utf8_span(
            content.as_bytes(),
            10,
            15,
            file_path
        ).is_err());

        // End beyond content length
        assert!(MagellanIntegration::validate_utf8_span(
            content.as_bytes(),
            0,
            10,
            file_path
        ).is_err());
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
        ).is_err());

        // Valid span (full multibyte character)
        assert!(MagellanIntegration::validate_utf8_span(
            content.as_bytes(),
            6,
            9, // Exactly the first Chinese character
            file_path
        ).is_ok());
    }

    #[test]
    fn test_validate_utf8_span_invalid_utf8() {
        // Invalid UTF-8 sequence
        let content: &[u8] = &[0xFF, 0xFE, 0xFD];
        let file_path = Path::new("/test.rs");

        // Should fail because content is not valid UTF-8
        assert!(MagellanIntegration::validate_utf8_span(
            content,
            0,
            1,
            file_path
        ).is_err());
    }
}
