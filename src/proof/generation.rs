//! Proof generation for refactoring operations.

use crate::error::{Result, SpliceError};
use crate::graph::MagellanIntegration;
use crate::proof::data_structures::{
    GraphSnapshot, GraphStats, ProofMetadata, RefactoringProof, SymbolInfo,
};
use std::collections::HashMap;
use std::path::{Path, PathBuf};

/// Generate a graph snapshot from the current database state.
pub fn generate_snapshot(db_path: &Path) -> Result<GraphSnapshot> {
    let mut integration = MagellanIntegration::open(db_path)?;
    let mut symbols: HashMap<String, SymbolInfo> = HashMap::new();
    let mut edges: HashMap<String, Vec<String>> = HashMap::new();
    let mut entry_points: Vec<String> = Vec::new();

    // Build a unique ID for each symbol
    // Format: "file_path:name" for uniqueness
    let mut name_to_id: HashMap<(String, String), String> = HashMap::new();
    let mut id_counter = 0u64;

    // Get all file nodes from the database
    let file_nodes = integration
        .inner_mut()
        .all_file_nodes()
        .map_err(|e| SpliceError::Other(format!("Failed to get file nodes: {}", e)))?;

    // For each file, get all symbols and their call relationships
    for file_path in file_nodes.keys() {
        let file_path_str = file_path.clone();

        // Get all symbols in this file
        let symbols_in_file = integration
            .inner_mut()
            .symbols_in_file(file_path)
            .map_err(|e| {
                SpliceError::Other(format!(
                    "Failed to get symbols for {}: {}",
                    file_path_str, e
                ))
            })?;

        for symbol_fact in symbols_in_file {
            // Skip symbols without names (e.g., anonymous items)
            let name: String = match &symbol_fact.name {
                Some(n) => n.clone(),
                None => continue,
            };

            // Generate a unique ID: incremental counter for snapshot consistency
            let id = format!("{:016x}", id_counter);
            id_counter += 1;

            let kind = symbol_fact.kind_normalized.to_string();
            let file_path_str = symbol_fact.file_path.to_string_lossy().to_string();

            // Build name -> ID mapping for edge resolution
            name_to_id.insert((file_path_str.clone(), name.clone()), id.clone());

            // Get caller and callee information
            let callers = integration
                .inner_mut()
                .callers_of_symbol(&file_path_str, &name)
                .unwrap_or_default();
            let callees = integration
                .inner_mut()
                .calls_from_symbol(&file_path_str, &name)
                .unwrap_or_default();

            let fan_in = callers.len();
            let fan_out = callees.len();

            // Detect entry points (no callers, public visibility)
            if callers.is_empty() && is_public_symbol(&name, &kind) {
                entry_points.push(id.clone());
            }

            let symbol_info = SymbolInfo {
                id: id.clone(),
                name: name.clone(),
                file_path: file_path_str,
                kind,
                byte_span: (symbol_fact.byte_start, symbol_fact.byte_end),
                fan_in,
                fan_out,
            };

            symbols.insert(id, symbol_info);
        }
    }

    // Build edges (caller ID -> list of callee IDs)
    // We need to iterate again to build edges since we need the complete name_to_id map
    for file_path in file_nodes.keys() {
        let file_path_str = file_path.clone();

        let symbols_in_file = integration
            .inner_mut()
            .symbols_in_file(file_path)
            .map_err(|e| {
                SpliceError::Other(format!(
                    "Failed to get symbols for {}: {}",
                    file_path_str, e
                ))
            })?;

        for symbol_fact in symbols_in_file {
            let name: String = match &symbol_fact.name {
                Some(n) => n.clone(),
                None => continue,
            };

            // Find the ID for this symbol
            let key = (file_path_str.clone(), name.clone());
            let id = match name_to_id.get(&key) {
                Some(id) => id.clone(),
                None => continue,
            };

            // Get callees and resolve their IDs
            let callees = integration
                .inner_mut()
                .calls_from_symbol(&file_path_str, &name)
                .unwrap_or_default();

            let callee_ids: Vec<String> = callees
                .iter()
                .filter_map(|c| {
                    let callee_path_str = c.file_path.to_string_lossy().to_string();
                    let callee_key = (callee_path_str, c.callee.clone());
                    name_to_id.get(&callee_key).cloned()
                })
                .collect();

            edges.insert(id, callee_ids);
        }
    }

    // Compute statistics
    let total_symbols = symbols.len();
    let total_edges = edges.values().map(|v| v.len()).sum();
    let entry_point_count = entry_points.len();

    // Compute maximum cyclomatic complexity
    // For a call graph, complexity of a symbol = fan_out + 1 (McCabe's metric)
    // We take the maximum across all symbols
    let max_complexity = symbols
        .values()
        .map(|s| s.fan_out + 1)
        .max()
        .filter(|&x| x > 1) // Only meaningful if there's at least one edge
        .or({
            // If no edges, complexity is 1 (trivial case) or 0 (empty graph)
            if total_symbols > 0 {
                Some(1)
            } else {
                Some(0)
            }
        });

    let stats = GraphStats {
        total_symbols,
        total_edges,
        entry_point_count,
        max_complexity,
    };

    Ok(GraphSnapshot {
        timestamp: chrono::Utc::now().timestamp(),
        symbols,
        edges,
        entry_points,
        stats,
    })
}

/// Detect if a symbol is public (entry point candidate).
fn is_public_symbol(name: &str, kind: &str) -> bool {
    // Heuristics for public symbols:
    // 1. Rust: Uppercase first char (types, traits) or no underscore prefix (functions)
    // 2. Python: No leading underscore
    // 3. JS/TS: Not detected as private
    match kind {
        "fn" | "method" => {
            // Functions starting with _ are typically private
            !name.starts_with('_')
        }
        "struct" | "interface" | "class" | "trait" | "enum" => {
            // Types are typically public if they don't start with lowercase
            name.chars()
                .next()
                .map(|c| c.is_uppercase())
                .unwrap_or(false)
        }
        _ => true,
    }
}

/// Create proof metadata for an operation.
pub fn create_metadata(operation: &str, db_path: &Path) -> ProofMetadata {
    ProofMetadata {
        operation: operation.to_string(),
        user: std::env::var("USER").ok(),
        timestamp: chrono::Utc::now().timestamp(),
        git_commit: get_git_commit().ok(),
        splice_version: env!("CARGO_PKG_VERSION").to_string(),
        database_path: db_path.to_path_buf(),
    }
}

/// Get the current git commit hash.
fn get_git_commit() -> Result<String> {
    use std::process::Command;

    let output = Command::new("git")
        .args(["rev-parse", "HEAD"])
        .output()
        .map_err(|_| SpliceError::Other("git not found".to_string()))?;

    if output.status.success() {
        let hash = String::from_utf8_lossy(&output.stdout).trim().to_string();
        Ok(hash)
    } else {
        Err(SpliceError::Other("not in git repo".to_string()))
    }
}

/// Write a proof to a JSON file.
pub fn write_proof(proof: &RefactoringProof, output_dir: &Path) -> Result<PathBuf> {
    use std::fs;

    fs::create_dir_all(output_dir)
        .map_err(|e| SpliceError::Other(format!("Failed to create proof dir: {}", e)))?;

    let filename = format!(
        "{}-{}.json",
        proof.metadata.operation, proof.metadata.timestamp
    );
    let path = output_dir.join(&filename);

    let json = serde_json::to_string_pretty(proof)
        .map_err(|e| SpliceError::Other(format!("Failed to serialize proof: {}", e)))?;

    fs::write(&path, json)
        .map_err(|e| SpliceError::Other(format!("Failed to write proof: {}", e)))?;

    Ok(path)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_public_symbol_functions() {
        // Public functions
        assert!(is_public_symbol("main", "fn"));
        assert!(is_public_symbol("public_func", "fn"));

        // Private functions (underscore prefix)
        assert!(!is_public_symbol("_private_func", "fn"));
        assert!(!is_public_symbol("__internal", "fn"));
    }

    #[test]
    fn test_is_public_symbol_types() {
        // Public types (uppercase first letter)
        assert!(is_public_symbol("MyStruct", "struct"));
        assert!(is_public_symbol("MyTrait", "trait"));
        assert!(is_public_symbol("MyClass", "class"));
        assert!(is_public_symbol("MyEnum", "enum"));

        // Private types (lowercase first letter - convention)
        assert!(!is_public_symbol("privateStruct", "struct"));
        assert!(!is_public_symbol("helperClass", "class"));
    }

    #[test]
    fn test_is_public_symbol_methods() {
        // Public methods
        assert!(is_public_symbol("run", "method"));
        assert!(is_public_symbol("execute", "method"));

        // Private methods
        assert!(!is_public_symbol("_helper", "method"));
        assert!(!is_public_symbol("_internal_method", "method"));
    }
}
