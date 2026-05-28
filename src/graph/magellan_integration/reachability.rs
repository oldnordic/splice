//! Reachability analysis methods for MagellanIntegration.

use crate::error::{Result, SpliceError};
use std::path::Path;

use super::normalize_lookup_path;
use super::types::*;
use super::MagellanIntegration;

impl MagellanIntegration {
    /// Get forward reachability (call graph traversal) from a symbol.
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
}
