//! Graph algorithm command handlers (reachable, cycles, condense, slice).

use std::path::{Path, PathBuf};

use super::impact::execute_impact_graph;

#[allow(
    clippy::too_many_arguments,
    reason = "CLI handler aggregates clap-parsed flags"
)]
pub(crate) fn execute_reachable(
    symbol: &str,
    semantic_query: Option<&str>,
    path: &Path,
    db_path: &Path,
    direction: &splice::cli::ReachabilityDirection,
    max_depth: usize,
    output: splice::cli::OutputFormat,
    impact_graph: bool,
    json_output: bool,
) -> Result<splice::cli::CliSuccessPayload, splice::SpliceError> {
    use splice::graph::MagellanIntegration;
    use splice::output::{AffectedFile, ReachabilityChain, ReachabilityResult, SymbolInfo};

    // Resolve semantic query upfront if provided
    let (symbol, path) = if let Some(query) = semantic_query {
        let resolved = splice::query::semantic::resolve_semantic_to_symbol(db_path, query)
            .map_err(|e| splice::SpliceError::Other(format!("Semantic search failed: {e}")))?;
        match resolved {
            Some(info) => {
                let sym = info.name;
                let p = PathBuf::from(info.file_path);
                (sym, p)
            }
            None => {
                return Err(splice::SpliceError::Other(
                    "No semantic matches found (ensure HNSW index exists and embeddings are generated)"
                        .to_string(),
                ));
            }
        }
    } else {
        (symbol.to_string(), path.to_path_buf())
    };

    // Get path string early
    let path_str = path
        .to_str()
        .ok_or_else(|| splice::SpliceError::Other("Invalid UTF-8 in path".to_string()))?;

    // Handle impact graph generation
    if impact_graph {
        return execute_impact_graph(
            db_path,
            &format!("{}:{}", path_str, symbol),
            direction,
            Some(max_depth),
        );
    }

    // Open database for operations
    let mut integration = MagellanIntegration::open(db_path)?;

    // Get root symbol info using backend-neutral method
    let root_symbol_info = match integration.find_symbol_by_path_and_name(&path, &symbol)? {
        Some(info) => info,
        None => {
            return Err(splice::SpliceError::SymbolNotFound {
                message: format!("Symbol '{}' not found in '{}'", symbol, path_str),
                symbol: symbol.to_string(),
                file: Some(path.to_path_buf()),
                hint: format!("Run 'splice find --name {}' to locate the symbol", symbol),
            });
        }
    };

    // Collect reachability based on direction
    let (forward_symbols, reverse_symbols) = match direction {
        splice::cli::ReachabilityDirection::Forward => {
            let symbols = integration.reachable_symbols(&path, &symbol, max_depth)?;
            (symbols, Vec::new())
        }
        splice::cli::ReachabilityDirection::Reverse => {
            let symbols = integration.reverse_reachable_symbols(&path, &symbol, max_depth)?;
            (Vec::new(), symbols)
        }
        splice::cli::ReachabilityDirection::Both => {
            let forward = integration.reachable_symbols(&path, &symbol, max_depth)?;
            let reverse = integration.reverse_reachable_symbols(&path, &symbol, max_depth)?;
            (forward, reverse)
        }
    };

    // Build affected files list
    let mut affected_files = std::collections::HashMap::new();
    affected_files.insert(root_symbol_info.file_path.clone(), (0, true)); // root file

    for reachable in &forward_symbols {
        let entry = affected_files
            .entry(reachable.symbol.file_path.clone())
            .or_insert((0, false));
        entry.0 += 1;
    }
    for reachable in &reverse_symbols {
        let entry = affected_files
            .entry(reachable.symbol.file_path.clone())
            .or_insert((0, false));
        entry.0 += 1;
    }

    let affected_files_vec: Vec<AffectedFile> = affected_files
        .into_iter()
        .map(|(path, (count, is_root))| AffectedFile {
            path,
            symbol_count: count,
            is_root,
        })
        .collect();

    // Build result
    let result = ReachabilityResult {
        symbol: SymbolInfo {
            symbol_id: None, // Can be added later if needed
            id_format: None,
            name: root_symbol_info.name.clone(),
            kind: root_symbol_info.kind.clone(),
            file_path: root_symbol_info.file_path.clone(),
            byte_start: root_symbol_info.byte_start,
            byte_end: root_symbol_info.byte_end,
        },
        direction: format!("{:?}", direction).to_lowercase(),
        max_depth,
        forward: if forward_symbols.is_empty() {
            None
        } else {
            Some(ReachabilityChain {
                count: forward_symbols.len(),
                depth: forward_symbols.iter().map(|s| s.depth).max().unwrap_or(0),
                symbols: forward_symbols
                    .into_iter()
                    .map(|s| splice::output::ReachableSymbol {
                        symbol: SymbolInfo {
                            symbol_id: None,
                            id_format: None,
                            name: s.symbol.name,
                            kind: s.symbol.kind,
                            file_path: s.symbol.file_path,
                            byte_start: s.symbol.byte_start,
                            byte_end: s.symbol.byte_end,
                        },
                        depth: s.depth,
                        path: s.path,
                    })
                    .collect(),
            })
        },
        reverse: if reverse_symbols.is_empty() {
            None
        } else {
            Some(ReachabilityChain {
                count: reverse_symbols.len(),
                depth: reverse_symbols.iter().map(|s| s.depth).max().unwrap_or(0),
                symbols: reverse_symbols
                    .into_iter()
                    .map(|s| splice::output::ReachableSymbol {
                        symbol: SymbolInfo {
                            symbol_id: None,
                            id_format: None,
                            name: s.symbol.name,
                            kind: s.symbol.kind,
                            file_path: s.symbol.file_path,
                            byte_start: s.symbol.byte_start,
                            byte_end: s.symbol.byte_end,
                        },
                        depth: s.depth,
                        path: s.path,
                    })
                    .collect(),
            })
        },
        affected_files: affected_files_vec,
    };

    // Format output
    if output.is_json() || json_output {
        let json = output
            .format_json(&result)
            .map_err(|e| splice::SpliceError::Other(format!("JSON serialization error: {}", e)))?;
        println!("{}", json);
        Ok(splice::cli::CliSuccessPayload::message_only(
            "Reachability analysis complete".to_string(),
        )
        .already_emitted())
    } else {
        // Human-readable output
        println!("Reachability Analysis for '{}' in {}", symbol, path_str);
        println!("Direction: {:?}", direction);
        println!("Max Depth: {}", max_depth);
        println!();

        if let Some(ref forward) = result.forward {
            println!(
                "Forward Reachability ({} callees, depth {}):",
                forward.count, forward.depth
            );
            for s in &forward.symbols {
                println!(
                    "  {} (depth {}): {}",
                    s.symbol.name, s.depth, s.symbol.file_path
                );
            }
            println!();
        }

        if let Some(ref reverse) = result.reverse {
            println!(
                "Reverse Reachability ({} callers, depth {}):",
                reverse.count, reverse.depth
            );
            for s in &reverse.symbols {
                println!(
                    "  {} (depth {}): {}",
                    s.symbol.name, s.depth, s.symbol.file_path
                );
            }
            println!();
        }

        println!("Affected Files ({}):", result.affected_files.len());
        for file in &result.affected_files {
            let marker = if file.is_root { " [root]" } else { "" };
            println!("  {}{} ({} symbols)", file.path, marker, file.symbol_count);
        }

        Ok(splice::cli::CliSuccessPayload::message_only(
            "Reachability analysis complete".to_string(),
        ))
    }
}

/// Execute the cycles command.
///
/// Detects cycles in the call graph using Tarjan's SCC algorithm.
pub(crate) fn execute_cycles(
    db_path: &Path,
    symbol: Option<&str>,
    path: Option<&PathBuf>,
    max_cycles: usize,
    show_members: bool,
    output: splice::cli::OutputFormat,
    json_output: bool,
) -> Result<splice::cli::CliSuccessPayload, splice::SpliceError> {
    use splice::graph::MagellanIntegration;
    use splice::output::{CycleDetectionResult, CycleInfo, SymbolInfo};

    let mut integration = MagellanIntegration::open(db_path)?;

    let (cycles, queried_symbol) = if let (Some(sym_name), Some(sym_path)) = (symbol, path) {
        // Find cycles containing specific symbol
        let cycles = integration.find_cycles_containing(sym_path, sym_name, max_cycles)?;

        // Get queried symbol info using backend-neutral method
        let queried_symbol = match integration.find_symbol_by_path_and_name(sym_path, sym_name)? {
            Some(info) => Some(SymbolInfo {
                symbol_id: None,
                id_format: None,
                name: info.name,
                kind: info.kind,
                file_path: info.file_path,
                byte_start: info.byte_start,
                byte_end: info.byte_end,
            }),
            None => None,
        };

        (cycles, queried_symbol)
    } else {
        // Find all cycles
        (integration.detect_cycles(max_cycles)?, None)
    };

    let total_cycles = cycles.len();
    let truncated = total_cycles >= max_cycles;

    let result_cycles: Vec<CycleInfo> = cycles
        .into_iter()
        .map(|c| CycleInfo {
            id: c.id,
            size: c.size,
            members: c
                .members
                .into_iter()
                .map(|s| SymbolInfo {
                    symbol_id: None,
                    id_format: None,
                    name: s.name,
                    kind: s.kind,
                    file_path: s.file_path,
                    byte_start: s.byte_start,
                    byte_end: s.byte_end,
                })
                .collect(),
            representative: SymbolInfo {
                symbol_id: None,
                id_format: None,
                name: c.representative.name,
                kind: c.representative.kind,
                file_path: c.representative.file_path,
                byte_start: c.representative.byte_start,
                byte_end: c.representative.byte_end,
            },
            is_self_loop: c.is_self_loop,
        })
        .collect();

    let result = CycleDetectionResult {
        total_cycles,
        max_cycles,
        truncated,
        cycles: result_cycles,
        queried_symbol,
    };

    // Format output
    if output.is_json() || json_output {
        let json = output
            .format_json(&result)
            .map_err(|e| splice::SpliceError::Other(format!("JSON serialization error: {}", e)))?;
        println!("{}", json);
        Ok(
            splice::cli::CliSuccessPayload::message_only("Cycle detection complete".to_string())
                .already_emitted(),
        )
    } else {
        // Human-readable output
        if let Some(ref qs) = result.queried_symbol {
            println!("Cycles containing '{}' in {}", qs.name, qs.file_path);
        } else {
            println!("Call Graph Cycle Detection");
        }
        println!();

        if result.total_cycles == 0 {
            println!("No cycles detected in the call graph.");
        } else {
            println!("Found {} cycle(s):", result.total_cycles);
            if result.truncated {
                println!("(showing first {} cycles)", result.max_cycles);
            }
            println!();

            for cycle in &result.cycles {
                println!("Cycle {} [{}]:", cycle.id, cycle.size);
                println!(
                    "  Representative: {} ({})",
                    cycle.representative.name, cycle.representative.kind
                );
                if cycle.is_self_loop {
                    println!("  Type: Self-loop");
                }
                if show_members {
                    println!("  Members:");
                    for member in &cycle.members {
                        println!("    - {} in {}", member.name, member.file_path);
                    }
                }
                println!();
            }
        }

        Ok(splice::cli::CliSuccessPayload::message_only(
            "Cycle detection complete".to_string(),
        ))
    }
}

/// Execute the condense command.
///
/// Analyzes the condensation graph (SCCs collapsed to DAG).
pub(crate) fn execute_condense(
    db_path: &Path,
    show_members: bool,
    show_levels: bool,
    output: splice::cli::OutputFormat,
    json_output: bool,
) -> Result<splice::cli::CliSuccessPayload, splice::SpliceError> {
    use splice::graph::MagellanIntegration;
    use splice::output::{CondensationResult, CondensedScc, LevelInfo, SccEdge, SymbolInfo};

    let mut integration = MagellanIntegration::open(db_path)?;
    let graph = integration.condense_graph()?;

    // Build SCC result with optional members
    let sccs: Vec<CondensedScc> = graph
        .sccs
        .into_iter()
        .map(|scc| {
            let members = if show_members {
                // Members would need to be tracked during condensation
                // For now, leave as None - could be populated from scc_members
                None
            } else {
                None
            };

            CondensedScc {
                id: scc.id,
                size: scc.size,
                is_cycle: scc.is_cycle,
                members,
                representative: SymbolInfo {
                    symbol_id: None,
                    id_format: None,
                    name: scc.representative.name,
                    kind: scc.representative.kind,
                    file_path: scc.representative.file_path,
                    byte_start: scc.representative.byte_start,
                    byte_end: scc.representative.byte_end,
                },
            }
        })
        .collect();

    let edges: Vec<SccEdge> = graph
        .edges
        .into_iter()
        .map(|e| SccEdge {
            from: e.from,
            to: e.to,
            weight: e.weight,
        })
        .collect();

    let levels: Option<Vec<LevelInfo>> = if show_levels {
        Some(
            graph
                .levels
                .into_iter()
                .map(|l| LevelInfo {
                    level: l.level,
                    scc_ids: l.scc_ids,
                    count: l.count,
                })
                .collect(),
        )
    } else {
        None
    };

    let result = CondensationResult {
        scc_count: graph.scc_count,
        cycle_scc_count: graph.cycle_scc_count,
        singleton_count: graph.singleton_count,
        sccs,
        edges,
        levels,
    };

    // Format output
    if output.is_json() || json_output {
        let json = output
            .format_json(&result)
            .map_err(|e| splice::SpliceError::Other(format!("JSON serialization error: {}", e)))?;
        println!("{}", json);
        Ok(splice::cli::CliSuccessPayload::message_only(
            "Condensation analysis complete".to_string(),
        )
        .already_emitted())
    } else {
        // Human-readable output
        println!("Condensation Graph Analysis");
        println!();
        println!("Summary:");
        println!("  Total SCCs: {}", result.scc_count);
        println!("  Cycle SCCs (size > 1): {}", result.cycle_scc_count);
        println!("  Singleton SCCs: {}", result.singleton_count);
        println!();

        if show_levels {
            if let Some(ref levels) = result.levels {
                println!("Topological Levels:");
                for level in levels {
                    println!("  Level {} ({} SCCs):", level.level, level.count);
                    for scc_id in &level.scc_ids {
                        if let Some(scc) = result.sccs.iter().find(|s| &s.id == scc_id) {
                            println!(
                                "    {} - {} (size: {}, cycle: {})",
                                scc.id, scc.representative.name, scc.size, scc.is_cycle
                            );
                        }
                    }
                }
                println!();
            }
        }

        println!("Cycle SCCs:");
        let cycle_sccs: Vec<_> = result.sccs.iter().filter(|s| s.is_cycle).collect();
        if cycle_sccs.is_empty() {
            println!("  (none)");
        } else {
            for scc in cycle_sccs {
                println!(
                    "  {} - {} (size: {})",
                    scc.id, scc.representative.name, scc.size
                );
            }
        }
        println!();

        println!("Edges in Condensed Graph: {}", result.edges.len());

        Ok(splice::cli::CliSuccessPayload::message_only(
            "Condensation analysis complete".to_string(),
        ))
    }
}

/// Execute the slice command.
///
/// Performs forward or backward program slicing for impact analysis.
#[allow(
    clippy::too_many_arguments,
    reason = "CLI handler aggregates clap-parsed flags"
)]
pub(crate) fn execute_slice(
    target: &str,
    semantic_query: Option<&str>,
    path: &Path,
    db_path: &Path,
    direction: &splice::cli::SliceDirection,
    max_depth: Option<usize>,
    output: splice::cli::OutputFormat,
    json_output: bool,
) -> Result<splice::cli::CliSuccessPayload, splice::SpliceError> {
    use splice::graph::MagellanIntegration;
    use splice::output::{AffectedFile, SliceResult, SliceStats, SlicedSymbol, SymbolInfo};
    use std::collections::HashMap;

    // Resolve semantic query upfront if provided
    let (target, path) = if let Some(query) = semantic_query {
        let resolved = splice::query::semantic::resolve_semantic_to_symbol(db_path, query)
            .map_err(|e| splice::SpliceError::Other(format!("Semantic search failed: {e}")))?;
        match resolved {
            Some(info) => {
                let sym = info.name;
                let p = PathBuf::from(info.file_path);
                (sym, p)
            }
            None => {
                return Err(splice::SpliceError::Other(
                    "No semantic matches found (ensure HNSW index exists and embeddings are generated)"
                        .to_string(),
                ));
            }
        }
    } else {
        (target.to_string(), path.to_path_buf())
    };

    let mut integration = MagellanIntegration::open(db_path)?;

    // Perform slice
    let sliced_symbols = match direction {
        splice::cli::SliceDirection::Forward => {
            integration.forward_slice(&path, &target, max_depth)?
        }
        splice::cli::SliceDirection::Backward => {
            integration.backward_slice(&path, &target, max_depth)?
        }
    };

    // Get target symbol info using backend-neutral method
    let target_symbol_info = match integration.find_symbol_by_path_and_name(&path, &target)? {
        Some(info) => info,
        None => {
            return Err(splice::SpliceError::Other(
                "Target symbol not found".to_string(),
            ));
        }
    };

    let target_symbol = SymbolInfo {
        symbol_id: None,
        id_format: None,
        name: target_symbol_info.name,
        kind: target_symbol_info.kind,
        file_path: target_symbol_info.file_path,
        byte_start: target_symbol_info.byte_start,
        byte_end: target_symbol_info.byte_end,
    };

    // Compute affected files
    let mut affected_files: HashMap<String, usize> = HashMap::new();
    for ss in &sliced_symbols {
        *affected_files
            .entry(ss.symbol.file_path.clone())
            .or_insert(0) += 1;
    }

    let target_file_path = target_symbol.file_path.clone();
    let affected_files_result: Vec<AffectedFile> = affected_files
        .into_iter()
        .map(|(path, count)| AffectedFile {
            is_root: path == target_file_path,
            path,
            symbol_count: count,
        })
        .collect();

    // Compute stats
    let max_distance = sliced_symbols.iter().map(|s| s.distance).max().unwrap_or(0);

    let stats = SliceStats {
        total_symbols: sliced_symbols.len(),
        max_distance,
        affected_file_count: affected_files_result.len(),
    };

    let symbols_result: Vec<SlicedSymbol> = sliced_symbols
        .into_iter()
        .map(|ss| SlicedSymbol {
            symbol: SymbolInfo {
                symbol_id: None,
                id_format: None,
                name: ss.symbol.name,
                kind: ss.symbol.kind,
                file_path: ss.symbol.file_path,
                byte_start: ss.symbol.byte_start,
                byte_end: ss.symbol.byte_end,
            },
            distance: ss.distance,
            is_target: ss.is_target,
            relationship: ss.relationship,
        })
        .collect();

    let result = SliceResult {
        target: target_symbol,
        direction: format!("{:?}", direction).to_lowercase(),
        max_depth,
        symbols: symbols_result,
        affected_files: affected_files_result,
        stats,
    };

    // Format output
    if output.is_json() || json_output {
        let json = output
            .format_json(&result)
            .map_err(|e| splice::SpliceError::Other(format!("JSON serialization error: {}", e)))?;
        println!("{}", json);
        Ok(
            splice::cli::CliSuccessPayload::message_only("Program slice complete".to_string())
                .already_emitted(),
        )
    } else {
        // Human-readable output
        println!(
            "Program Slice: {} {} from '{}'",
            result.direction,
            result
                .max_depth
                .map_or("(unlimited)".to_string(), |d| format!("(max depth {})", d)),
            result.target.name
        );
        println!();
        println!("Statistics:");
        println!("  Total symbols in slice: {}", result.stats.total_symbols);
        println!("  Max distance: {}", result.stats.max_distance);
        println!("  Affected files: {}", result.stats.affected_file_count);
        println!();

        println!("Affected Files:");
        for file in &result.affected_files {
            let marker = if file.is_root { " [target]" } else { "" };
            println!("  {}{} ({} symbols)", file.path, marker, file.symbol_count);
        }
        println!();

        println!("Symbols in Slice:");
        for ss in &result.symbols {
            let target_marker = if ss.is_target { " [TARGET]" } else { "" };
            println!(
                "  [d={:2}] {}{} in {} ({})",
                ss.distance, ss.symbol.name, target_marker, ss.symbol.file_path, ss.relationship
            );
        }

        Ok(splice::cli::CliSuccessPayload::message_only(
            "Program slice complete".to_string(),
        ))
    }
}
