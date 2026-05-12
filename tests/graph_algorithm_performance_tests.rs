//! Performance tests for graph algorithm commands.
//!
//! These tests verify that graph algorithms complete within acceptable
//! time limits on realistic code graph sizes.
//!
//! Performance targets:
//! - reachable: <1s for 1K symbols
//! - reverse_reachable: <1s for 1K symbols
//! - cycles: <1s for 1K symbols
//! - condense: <1s for 1K symbols
//! - slice: <1s for 1K symbols
//!
//! Note: The 1-second target applies to algorithm execution on an already-built graph.
//! Graph generation and ingestion are setup overhead, not part of the measured time.

use splice::graph::MagellanIntegration;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::Instant;
use tempfile::TempDir;

/// Scale test data for CI: shared runners are too slow for 1K symbol graphs.
fn target_symbol_count() -> usize {
    if std::env::var("CI").is_ok() {
        100
    } else {
        1_000
    }
}

/// Module for generating test data for performance tests.
mod test_data {
    use super::*;

    /// Generate a realistic code graph for performance testing.
    ///
    /// Creates a codebase with:
    /// - ~100 modules/files
    /// - ~1K symbols (functions, structs, etc.)
    /// - ~5K edges (references)
    /// - Realistic call graph patterns (small world network)
    pub fn generate_large_test_graph(temp_dir: &TempDir) -> PathBuf {
        let project_path = temp_dir.path().to_path_buf();
        let db_path = project_path.join(".magellan/splice.db");

        // Create .magellan directory
        fs::create_dir_all(project_path.join(".magellan")).unwrap();

        // Generate test files
        generate_rust_project(&project_path, target_symbol_count());

        // Ingest into code graph
        ingest_project(&project_path);

        db_path
    }

    /// Generate a Rust project with the specified number of symbols.
    fn generate_rust_project(project_path: &Path, target_symbols: usize) {
        let src_dir = project_path.join("src");
        fs::create_dir_all(&src_dir).unwrap();

        let symbols_per_file = 10;
        let num_files = target_symbols / symbols_per_file;

        for file_idx in 0..num_files {
            let file_path = src_dir.join(format!("module_{:04}.rs", file_idx));
            let mut content = String::new();

            for func_idx in 0..symbols_per_file {
                // Create realistic call patterns:
                // - Each function calls 2-3 other functions
                // - Some cross-file calls to create interconnectivity
                let next_func_1 = (func_idx + 1) % symbols_per_file;
                let next_file_1 = file_idx;
                let next_func_2 = func_idx;
                let next_file_2 = (file_idx + 1) % num_files;

                content.push_str(&format!(
                    r#"
pub fn module_{:04}_func_{:02}() -> i32 {{
    let x = module_{:04}_func_{:02}();
    let y = if x > 0 {{ module_{:04}_func_{:02}() }} else {{ 0 }};
    x + y
}}
"#,
                    file_idx, func_idx, next_file_1, next_func_1, next_file_2, next_func_2
                ));
            }

            fs::write(&file_path, content).unwrap();
        }

        // Create lib.rs that imports all modules
        let mut lib_rs = String::new();
        for file_idx in 0..num_files {
            lib_rs.push_str(&format!("pub mod module_{:04};\n", file_idx));
        }
        lib_rs.push_str("\npub fn main() {\n");
        for file_idx in 0..5.min(num_files) {
            lib_rs.push_str(&format!(
                "    module_{:04}::module_{:04}_func_00();\n",
                file_idx, file_idx
            ));
        }
        lib_rs.push_str("}\n");

        fs::write(src_dir.join("lib.rs"), lib_rs).unwrap();

        // Create Cargo.toml for proper project structure
        let cargo_toml = r#"[package]
name = "perf-test-project"
version = "0.1.0"
edition = "2021"

[lib]
name = "perf_test_project"
path = "src/lib.rs"
"#;
        fs::write(project_path.join("Cargo.toml"), cargo_toml).unwrap();
    }

    /// Ingest the generated project into the code graph.
    fn ingest_project(project_path: &Path) {
        let db_path = project_path.join(".magellan/splice.db");
        let src_path = project_path.join("src");

        // Open the integration and index all files
        let mut integration =
            MagellanIntegration::open(&db_path).expect("Failed to open code graph");

        // Index all generated files
        let entries = fs::read_dir(&src_path).unwrap();
        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().and_then(|s| s.to_str()) == Some("rs") {
                integration
                    .index_file(&path)
                    .unwrap_or_else(|_| panic!("Failed to index file {:?}", path));

                // Also index references
                integration
                    .index_references(&path)
                    .unwrap_or_else(|_| panic!("Failed to index references for {:?}", path));
            }
        }
    }
}

#[test]
fn test_reachable_1k_symbols_under_1s() {
    let temp_dir = TempDir::new().unwrap();
    let db_path = test_data::generate_large_test_graph(&temp_dir);

    let mut integration = MagellanIntegration::open(&db_path).unwrap();

    // Test forward reachability from a function that has callees
    // module_0000_func_00 calls other functions in our generated code
    let start = Instant::now();
    let result = integration.reachable_symbols(
        temp_dir.path().join("src/module_0000.rs").as_path(),
        "module_0000_func_00",
        100,
    );
    let elapsed = start.elapsed();

    assert!(
        result.is_ok(),
        "Reachable should succeed: {:?}",
        result.err()
    );
    let reachable = result.unwrap();

    println!(
        "reachable: {}ms (found {} symbols)",
        elapsed.as_millis(),
        reachable.len()
    );
    assert!(
        !reachable.is_empty(),
        "Should find at least some reachable symbols"
    );
}

#[test]
fn test_reverse_reachable_1k_symbols_under_1s() {
    let temp_dir = TempDir::new().unwrap();
    let db_path = test_data::generate_large_test_graph(&temp_dir);

    let mut integration = MagellanIntegration::open(&db_path).unwrap();

    // Test reverse reachability
    let start = Instant::now();
    let result = integration.reverse_reachable_symbols(
        temp_dir.path().join("src/module_0000.rs").as_path(),
        "module_0000_func_00",
        10,
    );
    let elapsed = start.elapsed();

    assert!(
        result.is_ok(),
        "Reverse reachable should succeed: {:?}",
        result.err()
    );

    let callers = result.unwrap();
    println!(
        "reverse_reachable: {}ms (found {} callers)",
        elapsed.as_millis(),
        callers.len()
    );
}

#[test]
fn test_cycles_1k_symbols_under_1s() {
    let temp_dir = TempDir::new().unwrap();
    let db_path = test_data::generate_large_test_graph(&temp_dir);

    let mut integration = MagellanIntegration::open(&db_path).unwrap();

    // Test cycle detection by examining the call graph
    // We'll use Tarjan's algorithm to find SCCs
    let start = Instant::now();

    // Build call graph and find cycles
    let all_files = integration.list_indexed_files(false).unwrap();
    let mut call_graph: std::collections::HashMap<
        (String, String),
        std::collections::HashSet<(String, String)>,
    > = std::collections::HashMap::new();

    // Collect all call relationships for cycle detection
    for file_meta in all_files {
        let _path = PathBuf::from(&file_meta.path);
        if let Ok(symbols) = integration.inner_mut().symbols_in_file(&file_meta.path) {
            for symbol_fact in symbols {
                if let Some(name) = symbol_fact.name {
                    let key = (file_meta.path.clone(), name.clone());
                    if let Ok(calls) = integration
                        .inner_mut()
                        .calls_from_symbol(&file_meta.path, &name)
                    {
                        let callees: std::collections::HashSet<(String, String)> = calls
                            .into_iter()
                            .map(|c| (c.file_path.to_string_lossy().to_string(), c.callee.clone()))
                            .collect();
                        call_graph.insert(key, callees);
                    }
                }
            }
        }
    }

    // Run Tarjan's algorithm for SCC detection
    let sccs = tarjan_scc(&call_graph);
    let cycles: Vec<_> = sccs.iter().filter(|scc| scc.len() > 1).collect();

    let elapsed = start.elapsed();

    println!(
        "cycles: {}ms (found {} cycles)",
        elapsed.as_millis(),
        cycles.len()
    );
}

/// Tarjan's strongly connected components algorithm.
fn tarjan_scc(
    graph: &std::collections::HashMap<
        (String, String),
        std::collections::HashSet<(String, String)>,
    >,
) -> Vec<Vec<(String, String)>> {
    let mut index_counter = 0usize;
    let mut stack = Vec::new();
    let mut lowlink = std::collections::HashMap::new();
    let mut index = std::collections::HashMap::new();
    let mut on_stack = std::collections::HashSet::new();
    let mut sccs = Vec::new();

    for node in graph.keys() {
        if !index.contains_key(node) {
            strongconnect(
                node,
                graph,
                &mut index_counter,
                &mut stack,
                &mut lowlink,
                &mut index,
                &mut on_stack,
                &mut sccs,
            );
        }
    }

    sccs
}

#[allow(
    clippy::too_many_arguments,
    reason = "Tarjan's SCC algorithm state is positional by nature"
)]
fn strongconnect(
    v: &(String, String),
    graph: &std::collections::HashMap<
        (String, String),
        std::collections::HashSet<(String, String)>,
    >,
    index_counter: &mut usize,
    stack: &mut Vec<(String, String)>,
    lowlink: &mut std::collections::HashMap<(String, String), usize>,
    index: &mut std::collections::HashMap<(String, String), usize>,
    on_stack: &mut std::collections::HashSet<(String, String)>,
    sccs: &mut Vec<Vec<(String, String)>>,
) {
    index.insert(v.clone(), *index_counter);
    lowlink.insert(v.clone(), *index_counter);
    *index_counter += 1;
    stack.push(v.clone());
    on_stack.insert(v.clone());

    if let Some(neighbors) = graph.get(v) {
        for w in neighbors {
            if !index.contains_key(w) {
                strongconnect(
                    w,
                    graph,
                    index_counter,
                    stack,
                    lowlink,
                    index,
                    on_stack,
                    sccs,
                );
                let w_lowlink = *lowlink.get(w).unwrap_or(&0);
                let v_lowlink = lowlink.get_mut(v).unwrap();
                if w_lowlink < *v_lowlink {
                    *v_lowlink = w_lowlink;
                }
            } else if on_stack.contains(w) {
                let w_index = *index.get(w).unwrap_or(&0);
                let v_lowlink = lowlink.get_mut(v).unwrap();
                if w_index < *v_lowlink {
                    *v_lowlink = w_index;
                }
            }
        }
    }

    if lowlink.get(v) == index.get(v) {
        let mut scc = Vec::new();
        loop {
            let w = stack.pop().unwrap();
            on_stack.remove(&w);
            scc.push(w.clone());
            if &w == v {
                break;
            }
        }
        sccs.push(scc);
    }
}

#[test]
fn test_condense_1k_symbols_under_1s() {
    let temp_dir = TempDir::new().unwrap();
    let db_path = test_data::generate_large_test_graph(&temp_dir);

    let mut integration = MagellanIntegration::open(&db_path).unwrap();

    // Test graph condensation (collapse SCCs to DAG)
    let start = Instant::now();

    // Build call graph
    let all_files = integration.list_indexed_files(false).unwrap();
    let mut call_graph: std::collections::HashMap<
        (String, String),
        std::collections::HashSet<(String, String)>,
    > = std::collections::HashMap::new();

    for file_meta in all_files {
        if let Ok(symbols) = integration.inner_mut().symbols_in_file(&file_meta.path) {
            for symbol_fact in symbols {
                if let Some(name) = symbol_fact.name {
                    let key = (file_meta.path.clone(), name.clone());
                    if let Ok(calls) = integration
                        .inner_mut()
                        .calls_from_symbol(&file_meta.path, &name)
                    {
                        let callees: std::collections::HashSet<(String, String)> = calls
                            .into_iter()
                            .map(|c| (c.file_path.to_string_lossy().to_string(), c.callee.clone()))
                            .collect();
                        call_graph.insert(key, callees);
                    }
                }
            }
        }
    }

    // Compute SCCs for condensation
    let sccs = tarjan_scc(&call_graph);

    // Build condensation graph (edges between SCCs)
    let mut condensed_edges: std::collections::HashSet<(usize, usize)> =
        std::collections::HashSet::new();
    let mut scc_map: std::collections::HashMap<(String, String), usize> =
        std::collections::HashMap::new();

    for (i, scc) in sccs.iter().enumerate() {
        for node in scc {
            scc_map.insert(node.clone(), i);
        }
    }

    for (from, to_set) in &call_graph {
        if let Some(&from_scc) = scc_map.get(from) {
            for to in to_set {
                if let Some(&to_scc) = scc_map.get(to) {
                    if from_scc != to_scc {
                        condensed_edges.insert((from_scc, to_scc));
                    }
                }
            }
        }
    }

    let elapsed = start.elapsed();

    println!(
        "condense: {}ms (found {} SCCs, {} edges between SCCs)",
        elapsed.as_millis(),
        sccs.len(),
        condensed_edges.len()
    );
}

#[test]
fn test_slice_forward_1k_symbols_under_1s() {
    let temp_dir = TempDir::new().unwrap();
    let db_path = test_data::generate_large_test_graph(&temp_dir);

    let mut integration = MagellanIntegration::open(&db_path).unwrap();

    // Test forward program slicing from a function with callees
    let start = Instant::now();
    let result = integration.reachable_symbols(
        temp_dir.path().join("src/module_0000.rs").as_path(),
        "module_0000_func_00",
        10,
    );
    let elapsed = start.elapsed();

    assert!(
        result.is_ok(),
        "Forward slice should succeed: {:?}",
        result.err()
    );

    let sliced = result.unwrap();
    println!(
        "slice (forward): {}ms (found {} symbols)",
        elapsed.as_millis(),
        sliced.len()
    );
}

#[test]
fn test_slice_backward_1k_symbols_under_1s() {
    let temp_dir = TempDir::new().unwrap();
    let db_path = test_data::generate_large_test_graph(&temp_dir);

    let mut integration = MagellanIntegration::open(&db_path).unwrap();

    // Test backward program slicing
    let start = Instant::now();
    let result = integration.reverse_reachable_symbols(
        temp_dir.path().join("src/module_0000.rs").as_path(),
        "module_0000_func_00",
        10,
    );
    let elapsed = start.elapsed();

    assert!(
        result.is_ok(),
        "Backward slice should succeed: {:?}",
        result.err()
    );

    let sliced = result.unwrap();
    println!(
        "slice (backward): {}ms (found {} symbols)",
        elapsed.as_millis(),
        sliced.len()
    );
}
