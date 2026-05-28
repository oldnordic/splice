//! Cycle detection and condensation methods for MagellanIntegration.

use crate::error::{Result, SpliceError};
use std::path::Path;

use super::types::*;
use super::MagellanIntegration;

impl MagellanIntegration {
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
    ///
    /// Uses a single bulk SQL query (`all_call_edges_sqlite`) to build the call
    /// graph, replacing the previous N+1 per-symbol loop. This also fixes a
    /// cross-file cycle detection bug where the old code keyed callee nodes by
    /// the *caller's* file path instead of the callee's actual file path.
    fn detect_cycles_sqlite(&mut self, max_cycles: usize) -> Result<Vec<CycleInfo>> {
        use std::collections::{HashMap, HashSet};

        let call_edges = self.all_call_edges_sqlite()?;

        // Build call graph: (file, name) → set of (callee_file, callee_name)
        let mut call_graph: HashMap<(String, String), HashSet<(String, String)>> = HashMap::new();

        for (caller_file, caller_name, callee_file, callee_name) in call_edges {
            let caller_key = (caller_file, caller_name);
            let callee_key = (callee_file, callee_name);
            call_graph
                .entry(caller_key)
                .or_default()
                .insert(callee_key.clone());
            call_graph.entry(callee_key).or_default();
        }

        let sccs = self.find_sccs(&call_graph)?;

        let mut cycles = Vec::new();
        for (index, scc) in sccs.iter().enumerate() {
            if cycles.len() >= max_cycles {
                break;
            }
            let is_self_loop = scc.len() == 1 && self.has_self_loop(&scc[0], &call_graph);
            if scc.len() > 1 || is_self_loop {
                cycles.push(self.scc_to_cycle_info(scc, index, is_self_loop)?);
            }
        }

        Ok(cycles)
    }

    /// Retrieve all call edges from the database in a single query.
    ///
    /// Returns `(caller_file, caller_name, callee_file, callee_name)` tuples by
    /// joining Call entities with their CALLS edges and the target Symbol entities.
    /// This replaces the previous N+1 per-symbol `calls_from_symbol` loop and
    /// correctly carries the callee's own file path rather than the caller's.
    pub(crate) fn all_call_edges_sqlite(&self) -> Result<Vec<(String, String, String, String)>> {
        use rusqlite::Connection;

        let conn = Connection::open(&self.db_path).map_err(|e| {
            SpliceError::Other(format!("Failed to open db for call-edge query: {}", e))
        })?;

        let mut stmt = conn
            .prepare(
                "SELECT call_node.file_path,
                        json_extract(call_node.data, '$.caller'),
                        callee_sym.file_path,
                        callee_sym.name
                 FROM graph_entities call_node
                 JOIN graph_edges ge
                   ON ge.from_id = call_node.id AND ge.edge_type = 'CALLS'
                 JOIN graph_entities callee_sym
                   ON ge.to_id = callee_sym.id
                 WHERE call_node.kind = 'Call'
                   AND json_extract(call_node.data, '$.caller') IS NOT NULL
                   AND call_node.file_path IS NOT NULL
                   AND callee_sym.file_path IS NOT NULL",
            )
            .map_err(|e| SpliceError::Other(format!("Failed to prepare call-edge query: {}", e)))?;

        let edges: Vec<(String, String, String, String)> = stmt
            .query_map([], |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                    row.get::<_, String>(3)?,
                ))
            })
            .map_err(|e| SpliceError::Other(format!("Failed to execute call-edge query: {}", e)))?
            .flatten()
            .collect();

        Ok(edges)
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
    #[allow(
        clippy::too_many_arguments,
        reason = "Tarjan's SCC algorithm state is positional by nature"
    )]
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
                    let current_low = lowlink
                        .get_mut(node)
                        .expect("invariant: node index inserted at DFS entry");
                    *current_low = (*current_low).min(neighbor_low);
                } else if on_stack.contains(neighbor) {
                    let neighbor_idx = *indices.get(neighbor).unwrap_or(&0);
                    let current_low = lowlink
                        .get_mut(node)
                        .expect("invariant: node index inserted at DFS entry");
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
                let w = stack
                    .pop()
                    .expect("invariant: SCC root pops at least itself");
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
    ///
    /// Falls back to a synthetic SymbolInfo derived from the call-edge key when
    /// the symbol is not indexed in the graph (e.g. in unit tests with minimal
    /// DB setup, or when the indexer hasn't run yet).
    fn scc_to_cycle_info(
        &mut self,
        scc: &[(String, String)],
        index: usize,
        is_self_loop: bool,
    ) -> Result<CycleInfo> {
        let mut members = Vec::new();

        for (file_path, symbol_name) in scc {
            let info = match self.inner_mut().symbol_extents(file_path, symbol_name) {
                Ok(facts) if !facts.is_empty() => {
                    let (entity_id, fact) = &facts[0];
                    SymbolInfo {
                        entity_id: *entity_id,
                        name: fact.name.clone().unwrap_or_else(|| symbol_name.clone()),
                        file_path: fact.file_path.to_string_lossy().to_string(),
                        kind: fact.kind_normalized.clone(),
                        byte_start: fact.byte_start,
                        byte_end: fact.byte_end,
                        start_line: None,
                        end_line: None,
                    }
                }
                _ => SymbolInfo {
                    entity_id: 0,
                    name: symbol_name.clone(),
                    file_path: file_path.clone(),
                    kind: String::new(),
                    byte_start: 0,
                    byte_end: 0,
                    start_line: None,
                    end_line: None,
                },
            };
            members.push(info);
        }

        members.sort_by(|a, b| a.name.cmp(&b.name));

        let representative = members
            .first()
            .ok_or_else(|| SpliceError::Other("Empty cycle".to_string()))?
            .clone();

        Ok(CycleInfo {
            id: format!("cycle-{}", index),
            size: scc.len(),
            members,
            representative,
            is_self_loop,
        })
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

        for (from, to) in scc_edges.keys() {
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
                        let deg = in_degree
                            .get_mut(&neighbor)
                            .expect("invariant: neighbor in_degree initialized from out_neighbors");
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
}
