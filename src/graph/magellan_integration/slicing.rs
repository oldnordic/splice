//! Dead code detection and program slicing methods for MagellanIntegration.

use crate::error::{Result, SpliceError};
use std::path::Path;

use super::normalize_lookup_path;
use super::types::*;
use super::MagellanIntegration;

impl MagellanIntegration {
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
        use std::collections::{HashMap, HashSet, VecDeque};

        let entry_path_str = entry_file.to_str().ok_or_else(|| {
            SpliceError::Other(format!("Invalid UTF-8 in path: {:?}", entry_file))
        })?;

        // Step 1: Enumerate all symbols by entity ID so the BFS is unambiguous
        // across files (the previous string-key BFS missed cross-file calls
        // because callee names can collide or omit module prefixes).
        let mut entity_to_fact: HashMap<i64, magellan::ingest::SymbolFact> = HashMap::new();
        let mut symbol_id_to_entity: HashMap<String, i64> = HashMap::new();
        let mut path_name_to_entity: HashMap<(String, String), i64> = HashMap::new();

        let file_nodes = self
            .inner
            .all_file_nodes()
            .map_err(|e| SpliceError::Other(format!("Failed to get file nodes: {}", e)))?;

        for file_path in file_nodes.keys() {
            let symbols = self.inner.symbols_in_file(file_path).map_err(|e| {
                SpliceError::Other(format!("Failed to get symbols in {}: {}", file_path, e))
            })?;

            for fact in symbols {
                let name = match fact.name.as_deref() {
                    Some(n) => n,
                    None => continue,
                };

                let entity_id = match self.inner.symbol_id_by_name(file_path, name) {
                    Ok(Some(id)) => id,
                    _ => continue,
                };

                entity_to_fact.insert(entity_id, fact.clone());
                path_name_to_entity.insert((file_path.to_string(), name.to_string()), entity_id);

                if let Ok(info) = self.inner.symbol_by_entity_id(entity_id) {
                    if let Some(symbol_id) = info.symbol_id {
                        symbol_id_to_entity.insert(symbol_id, entity_id);
                    }
                }
            }
        }

        // Step 2: Locate the entry point entity ID
        let entry_entity = self
            .inner
            .symbol_id_by_name(entry_path_str, entry_symbol)
            .map_err(|e| SpliceError::Other(format!("Failed to resolve entry symbol: {}", e)))?
            .ok_or_else(|| SpliceError::SymbolNotFound {
                message: format!(
                    "Entry point '{}' not found in '{}'",
                    entry_symbol, entry_path_str
                ),
                symbol: entry_symbol.to_string(),
                file: Some(entry_file.to_path_buf()),
                hint: "Ensure the entry point symbol exists in the specified file".to_string(),
            })?;

        // Step 3: BFS over entity IDs using the call graph
        let mut visited = HashSet::new();
        let mut queue = VecDeque::new();
        visited.insert(entry_entity);
        queue.push_back(entry_entity);

        while let Some(entity_id) = queue.pop_front() {
            let fact = match entity_to_fact.get(&entity_id) {
                Some(f) => f,
                None => continue,
            };

            let path = fact.file_path.to_string_lossy().to_string();
            let name = fact.name.as_deref().unwrap_or("");

            let calls = self
                .inner
                .calls_from_symbol(&path, name)
                .unwrap_or_default();

            for call in calls {
                let mut next_entity = None;

                // Prefer the stable callee symbol ID, which is accurate for
                // cross-file calls.
                if let Some(ref callee_symbol_id) = call.callee_symbol_id {
                    if let Some(&eid) = symbol_id_to_entity.get(callee_symbol_id) {
                        next_entity = Some(eid);
                    }
                }

                // Fall back to name-based lookup in the callee's file.
                if next_entity.is_none() {
                    let callee_path = call.file_path.to_string_lossy().to_string();
                    if let Some(&eid) = path_name_to_entity.get(&(callee_path, call.callee.clone()))
                    {
                        next_entity = Some(eid);
                    }
                }

                if let Some(eid) = next_entity {
                    if visited.insert(eid) {
                        queue.push_back(eid);
                    }
                }
            }
        }

        // Step 4: Collect unvisited symbols as dead code
        let mut dead_symbols = Vec::new();

        for (entity_id, fact) in entity_to_fact {
            if !visited.contains(&entity_id) {
                // Skip if excluding public symbols
                if exclude_public && is_public_symbol(&fact) {
                    continue;
                }

                let dead = DeadSymbol {
                    symbol: SymbolInfo {
                        entity_id,
                        name: fact.name.unwrap_or_default(),
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
