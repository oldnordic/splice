//! Dead code detection command handler.

use std::collections::HashMap;
use std::path::Path;

pub(crate) fn execute_dead_code(
    entry: &str,
    path: &Path,
    db_path: &Path,
    exclude_public: bool,
    group_by_file: bool,
    output: splice::cli::OutputFormat,
    json_output: bool,
) -> Result<splice::cli::CliSuccessPayload, splice::SpliceError> {
    use splice::graph::MagellanIntegration;
    use splice::output::{DeadCodeByFile, DeadCodeResult, SymbolInfo};

    let path_str = path
        .to_str()
        .ok_or_else(|| splice::SpliceError::Other("Invalid UTF-8 in path".to_string()))?;

    let mut integration = MagellanIntegration::open(db_path)?;

    let entry_symbol_info = match integration.find_symbol_by_path_and_name(path, entry)? {
        Some(info) => info,
        None => {
            return Err(splice::SpliceError::SymbolNotFound {
                message: format!("Entry point '{}' not found in '{}'", entry, path_str),
                symbol: entry.to_string(),
                file: Some(path.to_path_buf()),
                hint: "Ensure the entry point symbol exists in the specified file".to_string(),
            });
        }
    };

    let total_symbols = integration.get_statistics()?.symbols;

    let dead_symbols = integration.dead_symbols(path, entry, exclude_public)?;

    let reachable_count = total_symbols.saturating_sub(dead_symbols.len());
    let dead_count = dead_symbols.len();

    let dead_by_file = if group_by_file {
        let mut by_file: HashMap<String, Vec<splice::graph::magellan_integration::DeadSymbol>> =
            HashMap::new();
        for ds in dead_symbols {
            by_file
                .entry(ds.symbol.file_path.clone())
                .or_default()
                .push(ds);
        }

        by_file
            .into_iter()
            .map(|(path, symbols)| {
                let count = symbols.len();
                let output_symbols = symbols
                    .into_iter()
                    .map(|ds| splice::output::DeadSymbol {
                        symbol: SymbolInfo {
                            symbol_id: None,
                            id_format: None,
                            name: ds.symbol.name,
                            kind: ds.symbol.kind,
                            file_path: ds.symbol.file_path,
                            byte_start: ds.symbol.byte_start,
                            byte_end: ds.symbol.byte_end,
                        },
                        reason: ds.reason,
                    })
                    .collect();
                DeadCodeByFile {
                    path,
                    count,
                    symbols: output_symbols,
                }
            })
            .collect()
    } else {
        vec![DeadCodeByFile {
            path: "all".to_string(),
            count: dead_count,
            symbols: dead_symbols
                .into_iter()
                .map(|ds| splice::output::DeadSymbol {
                    symbol: SymbolInfo {
                        symbol_id: None,
                        id_format: None,
                        name: ds.symbol.name,
                        kind: ds.symbol.kind,
                        file_path: ds.symbol.file_path,
                        byte_start: ds.symbol.byte_start,
                        byte_end: ds.symbol.byte_end,
                    },
                    reason: ds.reason,
                })
                .collect(),
        }]
    };

    let result = DeadCodeResult {
        entry_point: SymbolInfo {
            symbol_id: None,
            id_format: None,
            name: entry_symbol_info.name.clone(),
            kind: entry_symbol_info.kind.clone(),
            file_path: entry_symbol_info.file_path.clone(),
            byte_start: entry_symbol_info.byte_start,
            byte_end: entry_symbol_info.byte_end,
        },
        total_symbols,
        reachable_count,
        dead_count,
        dead_by_file,
        excluded_public: exclude_public,
    };

    if output.is_json() || json_output {
        let json = output
            .format_json(&result)
            .map_err(|e| splice::SpliceError::Other(format!("JSON serialization error: {}", e)))?;
        println!("{}", json);
        Ok(
            splice::cli::CliSuccessPayload::message_only(
                "Dead code detection complete".to_string(),
            )
            .already_emitted(),
        )
    } else {
        println!("Dead Code Detection");
        println!("Entry Point: {} in {}", entry, path_str);
        println!();
        println!("Statistics:");
        println!("  Total symbols: {}", total_symbols);
        println!("  Reachable: {}", reachable_count);
        println!("  Dead (unreachable): {}", dead_count);
        println!();

        if dead_count == 0 {
            println!("No dead code found - all symbols are reachable from the entry point.");
        } else {
            for file_group in &result.dead_by_file {
                println!("{} ({} dead symbols):", file_group.path, file_group.count);
                for ds in &file_group.symbols {
                    println!("  - {} ({})", ds.symbol.name, ds.symbol.kind);
                }
                println!();
            }
        }

        Ok(splice::cli::CliSuccessPayload::message_only(
            "Dead code detection complete".to_string(),
        ))
    }
}
