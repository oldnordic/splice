//! Symbol lookup and query methods for MagellanIntegration.

use crate::error::{Result, SpliceError};
use std::path::Path;

use super::types::*;
use super::MagellanIntegration;
use super::{normalize_lookup_path, parse_symbol_kind};

impl MagellanIntegration {
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
    ///
    /// Uses magellan's `SymbolNavigator` for O(1) resolution, falling back to
    /// O(N) file scan if the navigator cannot resolve the name.
    fn find_symbol_by_name_sqlite(
        &mut self,
        name: &str,
        ambiguous: bool,
    ) -> Result<Vec<SymbolInfo>> {
        use std::collections::HashSet;

        // Fast path: navigator gives candidate (file, name) pairs. We then resolve
        // full extents via symbol_extents to obtain correct byte_end and the
        // canonical kind_normalized, avoiding the zero-width navigator records.
        let nav = self.inner.navigator();
        let mut results = Vec::new();
        let mut seen = HashSet::new();
        if let Ok(resolved) = nav.resolve(name) {
            for si in resolved {
                let path = si.file_path.unwrap_or_default();
                if let Ok(matches) = self.inner.symbol_extents(&path, &si.name) {
                    for (entity_id, fact) in matches {
                        if seen.insert(entity_id) {
                            results.push(SymbolInfo {
                                entity_id,
                                name: fact.name.clone().unwrap_or_else(|| si.name.clone()),
                                file_path: fact.file_path.to_string_lossy().to_string(),
                                kind: fact.kind_normalized.clone(),
                                byte_start: fact.byte_start,
                                byte_end: fact.byte_end,
                                start_line: Some(fact.start_line),
                                end_line: Some(fact.end_line),
                            });

                            if !ambiguous {
                                return Ok(results);
                            }
                        }
                    }
                }
            }
        }

        // Navigator found results: skip the expensive O(N) file scan.
        if !results.is_empty() {
            return Ok(results);
        }

        // Fallback: scan every file for the name.
        let file_nodes = self
            .inner
            .all_file_nodes()
            .map_err(|e| SpliceError::Other(format!("Failed to get file nodes: {}", e)))?;

        for file_path in file_nodes.keys() {
            if let Ok(matches) = self.inner.symbol_extents(file_path, name) {
                for (entity_id, fact) in matches {
                    if seen.insert(entity_id) {
                        let symbol = SymbolInfo {
                            entity_id,
                            name: fact.name.clone().unwrap_or_default(),
                            file_path: fact.file_path.to_string_lossy().to_string(),
                            kind: fact.kind_normalized.clone(),
                            byte_start: fact.byte_start,
                            byte_end: fact.byte_end,
                            start_line: Some(fact.start_line),
                            end_line: Some(fact.end_line),
                        };
                        results.push(symbol);

                        if !ambiguous && !results.is_empty() {
                            return Ok(results);
                        }
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

    /// Find symbol by ID.
    ///
    /// Accepts:
    /// - magellan's raw entity ID (integer / 16-char hex)
    /// - splice V2 BLAKE3 symbol ID (32-char hex)
    /// - splice V1 SHA-256 symbol ID (16-char hex)
    ///
    /// # Arguments
    /// * `symbol_id` - symbol identifier
    ///
    /// # Returns
    /// Some(SymbolInfo) if found, None if not found.
    pub fn find_symbol_by_id(&mut self, symbol_id: &str) -> Result<Option<SymbolInfo>> {
        match self.backend {
            IntegrationBackend::Sqlite => self.find_symbol_by_id_sqlite(symbol_id),
            #[cfg(feature = "geometric")]
            IntegrationBackend::Geometric => self.find_symbol_by_id_geo(symbol_id),
        }
    }

    /// Find symbol by ID (SQLite implementation).
    ///
    /// Accepts:
    /// - magellan's stable `symbol_id` (16-char hex stored in symbol data)
    /// - fully-qualified symbol name (FQN / display_fqn / canonical_fqn)
    /// - raw SQLite entity ID (integer)
    /// - splice V2 BLAKE3 symbol ID (32-char hex)
    /// - splice V1 SHA-256 symbol ID (16-char hex)
    fn find_symbol_by_id_sqlite(&mut self, symbol_id: &str) -> Result<Option<SymbolInfo>> {
        use crate::symbol_id::{generate_v1, generate_v2};
        use rusqlite::Connection;

        // Try 1: magellan's resolver (symbol_id, FQN, entity ID)
        match self.inner.resolve_symbol_entity(symbol_id) {
            Ok(entity_id) => {
                let conn = Connection::open(&self.db_path).map_err(|e| {
                    SpliceError::Other(format!(
                        "Failed to open database for symbol ID lookup: {}",
                        e
                    ))
                })?;

                let mut stmt = conn
                    .prepare(
                        "SELECT id, name, file_path, data FROM graph_entities WHERE id = ? AND kind = 'Symbol'",
                    )
                    .map_err(|e| SpliceError::Other(format!("Failed to prepare query: {}", e)))?;

                let mut rows = stmt
                    .query_map([entity_id], |row| {
                        Ok((
                            row.get::<_, i64>(0)?,
                            row.get::<_, String>(1)?,
                            row.get::<_, String>(2)?,
                            row.get::<_, String>(3)?,
                        ))
                    })
                    .map_err(|e| SpliceError::Other(format!("Failed to query symbols: {}", e)))?;

                if let Some(row_result) = rows.next() {
                    let (entity_id, name, file_path, data_json) = row_result
                        .map_err(|e| SpliceError::Other(format!("Failed to read row: {}", e)))?;
                    return Ok(Some(Self::build_symbol_info_from_data(
                        entity_id, &name, &file_path, &data_json,
                    )?));
                }
            }
            Err(_) => {
                // Not found via magellan resolver; fall through to legacy IDs.
            }
        }

        // Try 2: generated splice V2 / V1 IDs (backward compatibility)
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

            let data: serde_json::Value = serde_json::from_str(&data_json).map_err(|e| {
                SpliceError::Other(format!("Failed to parse symbol data JSON: {}", e))
            })?;

            let byte_start = data
                .get("byte_start")
                .and_then(|v| v.as_u64())
                .ok_or_else(|| SpliceError::Other("Symbol data missing byte_start".to_string()))?
                as usize;

            let generated_v2 = generate_v2(&name, &file_path, byte_start);
            if generated_v2.as_str() == symbol_id {
                return Ok(Some(Self::build_symbol_info_from_data(
                    entity_id, &name, &file_path, &data_json,
                )?));
            }

            let generated_v1 = generate_v1(&name, &file_path, byte_start);
            if generated_v1.as_str() == symbol_id {
                return Ok(Some(Self::build_symbol_info_from_data(
                    entity_id, &name, &file_path, &data_json,
                )?));
            }
        }

        Ok(None)
    }

    /// Build SymbolInfo from the JSON data blob stored in graph_entities.
    fn build_symbol_info_from_data(
        entity_id: i64,
        name: &str,
        file_path: &str,
        data_json: &str,
    ) -> Result<SymbolInfo> {
        let data: serde_json::Value = serde_json::from_str(data_json)
            .map_err(|e| SpliceError::Other(format!("Failed to parse symbol data JSON: {}", e)))?;

        let byte_start = data
            .get("byte_start")
            .and_then(|v| v.as_u64())
            .ok_or_else(|| SpliceError::Other("Symbol data missing byte_start".to_string()))?
            as usize;
        let byte_end = data
            .get("byte_end")
            .and_then(|v| v.as_u64())
            .ok_or_else(|| SpliceError::Other("Symbol data missing byte_end".to_string()))?
            as usize;
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

        Ok(SymbolInfo {
            entity_id,
            name: name.to_string(),
            file_path: file_path.to_string(),
            kind,
            byte_start,
            byte_end,
            start_line,
            end_line,
        })
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
}
