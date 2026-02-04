//! Criterion benchmarks for graph algorithm performance.
//!
//! Run with: cargo bench --bench graph_benchmarks
//!
//! Results are stored in target/criterion/ and can be compared
//! across commits to detect performance regressions.

use criterion::{black_box, BenchmarkId, Criterion, criterion_group, criterion_main};
use splice::graph::MagellanIntegration;
use std::fs;
use std::path::PathBuf;
use tempfile::TempDir;

/// Benchmark sizes to test (number of symbols)
const BENCHMARK_SIZES: &[usize] = &[1_000, 5_000, 10_000];

/// Generate a test graph with the specified number of symbols.
fn generate_test_graph(size: usize) -> (TempDir, PathBuf) {
    let temp_dir = TempDir::new().unwrap();
    let project_path = temp_dir.path().to_path_buf();
    let db_path = project_path.join(".codemcp/codegraph.db");

    // Create .codemcp directory
    fs::create_dir_all(project_path.join(".codemcp")).unwrap();

    // Generate test files
    generate_rust_project(&project_path, size);

    // Ingest into code graph
    ingest_project(&project_path);

    (temp_dir, db_path)
}

/// Generate a Rust project with the specified number of symbols.
fn generate_rust_project(project_path: &PathBuf, target_symbols: usize) {
    let src_dir = project_path.join("src");
    fs::create_dir_all(&src_dir).unwrap();

    let symbols_per_file = 10;
    let num_files = target_symbols / symbols_per_file;

    for file_idx in 0..num_files {
        let file_path = src_dir.join(format!("module_{:04}.rs", file_idx));
        let mut content = String::new();

        for func_idx in 0..symbols_per_file {
            // Create realistic call patterns
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

    // Create lib.rs
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

    // Create Cargo.toml
    let cargo_toml = r#"[package]
name = "bench-test-project"
version = "0.1.0"
edition = "2021"

[lib]
name = "bench_test_project"
path = "src/lib.rs"
"#;
    fs::write(project_path.join("Cargo.toml"), cargo_toml).unwrap();
}

/// Ingest the generated project into the code graph.
fn ingest_project(project_path: &PathBuf) {
    let src_path = project_path.join("src");

    let mut integration =
        MagellanIntegration::open(&project_path.join(".codemcp/codegraph.db"))
            .expect("Failed to open code graph");

    let entries = fs::read_dir(&src_path).unwrap();
    for entry in entries.flatten() {
        let path = entry.path();
        if path.extension().and_then(|s| s.to_str()) == Some("rs") {
            integration
                .index_file(&path)
                .expect(&format!("Failed to index file {:?}", path));

            integration
                .index_references(&path)
                .expect(&format!("Failed to index references for {:?}", path));
        }
    }
}

/// Benchmark forward reachability analysis.
fn bench_reachable(c: &mut Criterion) {
    let mut group = c.benchmark_group("reachable");

    for size in BENCHMARK_SIZES {
        let (_temp_dir, db_path) = generate_test_graph(*size);
        let mut integration = MagellanIntegration::open(&db_path).unwrap();

        group.bench_with_input(BenchmarkId::from_parameter(size), size, |b, _| {
            b.iter(|| {
                integration
                    .reachable_symbols(
                        black_box(PathBuf::from("src/lib.rs").as_path()),
                        black_box("main"),
                        black_box(100),
                    )
                    .unwrap()
            });
        });
    }

    group.finish();
}

/// Benchmark reverse reachability analysis.
fn bench_reverse_reachable(c: &mut Criterion) {
    let mut group = c.benchmark_group("reverse_reachable");

    for size in BENCHMARK_SIZES {
        let (_temp_dir, db_path) = generate_test_graph(*size);
        let mut integration = MagellanIntegration::open(&db_path).unwrap();

        group.bench_with_input(BenchmarkId::from_parameter(size), size, |b, _| {
            b.iter(|| {
                integration
                    .reverse_reachable_symbols(
                        black_box(PathBuf::from("src/module_0000.rs").as_path()),
                        black_box("module_0000_func_00"),
                        black_box(10),
                    )
                    .unwrap()
            });
        });
    }

    group.finish();
}

/// Benchmark cycle detection using Tarjan's algorithm.
fn bench_cycles(c: &mut Criterion) {
    let mut group = c.benchmark_group("cycles");

    for size in BENCHMARK_SIZES {
        let (_temp_dir, db_path) = generate_test_graph(*size);
        let mut integration = MagellanIntegration::open(&db_path).unwrap();

        group.bench_with_input(BenchmarkId::from_parameter(size), size, |b, _| {
            b.iter(|| {
                // Build call graph for cycle detection
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
                                if let Ok(calls) =
                                    integration.inner_mut().calls_from_symbol(&file_meta.path, &name)
                                {
                                    let callees: std::collections::HashSet<(String, String)> =
                                        calls
                                            .into_iter()
                                            .map(|c| {
                                                (
                                                    c.file_path.to_string_lossy().to_string(),
                                                    c.callee.clone(),
                                                )
                                            })
                                            .collect();
                                    call_graph.insert(key, callees);
                                }
                            }
                        }
                    }
                }

                // Run Tarjan's algorithm
                tarjan_scc(black_box(&call_graph))
            });
        });
    }

    group.finish();
}

/// Benchmark graph condensation (SCC collapse to DAG).
fn bench_condense(c: &mut Criterion) {
    let mut group = c.benchmark_group("condense");

    for size in BENCHMARK_SIZES {
        let (_temp_dir, db_path) = generate_test_graph(*size);
        let mut integration = MagellanIntegration::open(&db_path).unwrap();

        group.bench_with_input(BenchmarkId::from_parameter(size), size, |b, _| {
            b.iter(|| {
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
                                if let Ok(calls) =
                                    integration.inner_mut().calls_from_symbol(&file_meta.path, &name)
                                {
                                    let callees: std::collections::HashSet<(String, String)> =
                                        calls
                                            .into_iter()
                                            .map(|c| {
                                                (
                                                    c.file_path.to_string_lossy().to_string(),
                                                    c.callee.clone(),
                                                )
                                            })
                                            .collect();
                                    call_graph.insert(key, callees);
                                }
                            }
                        }
                    }
                }

                // Compute SCCs and build condensation
                let sccs = tarjan_scc(black_box(&call_graph));
                let mut scc_map: std::collections::HashMap<(String, String), usize> =
                    std::collections::HashMap::new();

                for (i, scc) in sccs.iter().enumerate() {
                    for node in scc {
                        scc_map.insert(node.clone(), i);
                    }
                }

                let mut condensed_edges: std::collections::HashSet<(usize, usize)> =
                    std::collections::HashSet::new();

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

                (sccs.len(), condensed_edges.len())
            });
        });
    }

    group.finish();
}

/// Benchmark forward program slicing.
fn bench_slice_forward(c: &mut Criterion) {
    let mut group = c.benchmark_group("slice_forward");

    for size in BENCHMARK_SIZES {
        let (_temp_dir, db_path) = generate_test_graph(*size);
        let mut integration = MagellanIntegration::open(&db_path).unwrap();

        group.bench_with_input(BenchmarkId::from_parameter(size), size, |b, _| {
            b.iter(|| {
                integration
                    .reachable_symbols(
                        black_box(PathBuf::from("src/lib.rs").as_path()),
                        black_box("main"),
                        black_box(10),
                    )
                    .unwrap()
            });
        });
    }

    group.finish();
}

/// Benchmark backward program slicing.
fn bench_slice_backward(c: &mut Criterion) {
    let mut group = c.benchmark_group("slice_backward");

    for size in BENCHMARK_SIZES {
        let (_temp_dir, db_path) = generate_test_graph(*size);
        let mut integration = MagellanIntegration::open(&db_path).unwrap();

        group.bench_with_input(BenchmarkId::from_parameter(size), size, |b, _| {
            b.iter(|| {
                integration
                    .reverse_reachable_symbols(
                        black_box(PathBuf::from("src/module_0000.rs").as_path()),
                        black_box("module_0000_func_00"),
                        black_box(10),
                    )
                    .unwrap()
            });
        });
    }

    group.finish();
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

criterion_group!(
    benches,
    bench_reachable,
    bench_reverse_reachable,
    bench_cycles,
    bench_condense,
    bench_slice_forward,
    bench_slice_backward
);
criterion_main!(benches);
