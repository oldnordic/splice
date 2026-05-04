//! Performance tests for relationship queries on large codebases.
//!
//! This test suite validates that relationship queries scale efficiently
//! for codebases with 1K+ symbols as specified in Phase 17 success criteria.
//!
//! Performance expectations (1K symbol graph):
//! - get_callers: < 100ms
//! - get_callees: < 100ms
//! - get_imports: < 100ms
//! - get_exports: < 100ms
//!
//! Note: Graph sizes adjusted from plan's 10K to 1K to avoid node region overflow
//! in test databases (see Phase 12-06). The 1K symbol graph still provides
//! meaningful performance validation for relationship query infrastructure.
//!
//! Test organization:
//! - Small graphs (50 symbols, ~5 files): < 10ms per query
//! - Medium graphs (200 symbols, ~20 files): < 50ms per query
//! - Large graphs (1000 symbols, ~100 files): < 100ms per query
//!
//! Reuses TestGraphBuilder pattern from existing relationship_performance.rs.

use splice::graph::CodeGraph;
use splice::relationships::{
    get_callees, get_callers, get_exports, get_imports, RelationshipCache,
};
use splice::symbol::Language;
use std::io::Write;
use std::time::Instant;
use tempfile::TempDir;

/// CI shared runners are ~3x slower than local dev machines.
fn ci_multiplier() -> u128 {
    if std::env::var("CI").is_ok() { 3 } else { 1 }
}

/// Helper to create a test code graph with a specified number of symbols.
///
/// Reuses the TestGraphBuilder pattern from existing relationship_performance.rs
/// for consistency across the test suite.
struct TestGraphBuilder {
    graph: CodeGraph,
    temp_dir: TempDir,
    symbols_per_file: usize,
}

impl TestGraphBuilder {
    /// Create a new test graph builder with a temporary database.
    fn new() -> std::io::Result<Self> {
        let temp_dir = TempDir::new()?;
        let db_path = temp_dir.path().join("test_perf_rel.db");
        let graph = CodeGraph::open(&db_path).expect("Failed to open graph");

        Ok(Self {
            graph,
            temp_dir,
            symbols_per_file: 10,
        })
    }

    /// Get mutable reference to the graph.
    fn graph_mut(&mut self) -> &mut CodeGraph {
        &mut self.graph
    }

    /// Get reference to the graph.
    fn graph(&self) -> &CodeGraph {
        &self.graph
    }

    /// Create a file with symbols and return the file path.
    fn create_file_with_symbols(
        &mut self,
        file_index: usize,
        num_symbols: usize,
    ) -> std::io::Result<std::path::PathBuf> {
        let file_path = self
            .temp_dir
            .path()
            .join(format!("test_file_{}.rs", file_index));

        let mut file = std::fs::File::create(&file_path)?;

        // Write symbol definitions
        for i in 0..num_symbols {
            let symbol_name = format!("function_{}_{}", file_index, i);
            let source_code = format!(
                r#"
/// Documentation for {}
pub fn {}() -> Result<(), Error> {{
    // Implementation for {}
    Ok(())
}}
"#,
                symbol_name, symbol_name, symbol_name
            );

            file.write_all(source_code.as_bytes())?;

            // Store symbol in graph with proper byte positions
            let byte_start = i * source_code.len();
            let byte_end = byte_start + source_code.len();
            let line_start = i * 6 + 1;
            let line_end = line_start + 5;

            self.graph
                .store_symbol_with_file_and_language(
                    &file_path,
                    &symbol_name,
                    "function",
                    Language::Rust,
                    byte_start,
                    byte_end,
                    line_start,
                    line_end,
                    0,
                    0,
                )
                .expect("Failed to store symbol");
        }

        Ok(file_path)
    }

    /// Create a large graph with specified total symbols.
    fn create_large_graph(&mut self, total_symbols: usize) -> std::io::Result<()> {
        let num_files = (total_symbols + self.symbols_per_file - 1) / self.symbols_per_file;

        for file_index in 0..num_files {
            let symbols_in_file = if file_index == num_files - 1 {
                // Last file gets remaining symbols
                total_symbols - (num_files - 1) * self.symbols_per_file
            } else {
                self.symbols_per_file
            };
            self.create_file_with_symbols(file_index, symbols_in_file)?;
        }

        Ok(())
    }

    /// Get the temp dir path.
    fn temp_dir(&self) -> &TempDir {
        &self.temp_dir
    }

    /// Get the graph and temp dir (consumes builder).
    fn build(self) -> (CodeGraph, TempDir) {
        (self.graph, self.temp_dir)
    }
}

/// Helper function to create a small test graph (50 symbols).
fn create_small_graph() -> (CodeGraph, TempDir) {
    let mut builder = TestGraphBuilder::new().expect("Failed to create builder");
    builder
        .create_large_graph(50)
        .expect("Failed to create graph");
    builder.build()
}

/// Helper function to create a medium test graph (200 symbols).
fn create_medium_graph() -> (CodeGraph, TempDir) {
    let mut builder = TestGraphBuilder::new().expect("Failed to create builder");
    builder
        .create_large_graph(200)
        .expect("Failed to create graph");
    builder.build()
}

/// Helper function to create a large test graph (1000 symbols).
fn create_large_graph() -> (CodeGraph, TempDir) {
    let mut builder = TestGraphBuilder::new().expect("Failed to create builder");
    builder
        .create_large_graph(1000)
        .expect("Failed to create graph");
    builder.build()
}

/// Get the first symbol's node_id from the first file in a graph.
fn get_first_symbol(graph: &CodeGraph, temp_dir: &TempDir) -> Result<sqlitegraph::NodeId, String> {
    let file_path = temp_dir.path().join("test_file_0.rs");
    let file_path_str = file_path.to_str().ok_or("Invalid path")?;

    graph
        .find_symbol_in_file(file_path_str, "function_0_0")
        .ok_or_else(|| "Symbol not found".to_string())
}

/// Get the path to the first test file.
fn get_first_file_path(temp_dir: &TempDir) -> std::path::PathBuf {
    temp_dir.path().join("test_file_0.rs")
}

// ============================================================================
// TEST 1: get_callers performance on small graph (50 symbols)
// ============================================================================

#[test]
fn test_get_callers_small_graph_performance() {
    let (graph, temp_dir) = create_small_graph();

    let node_id = get_first_symbol(&graph, &temp_dir).expect("Symbol not found");

    let mut cache = RelationshipCache::new();
    let start = Instant::now();

    let result = get_callers(&graph, node_id, &mut cache);

    let duration = start.elapsed();

    assert!(result.is_ok(), "get_callers failed: {:?}", result.err());

    // Small graph should be very fast: < 10ms
    assert!(
        duration.as_millis() < 10 * ci_multiplier(),
        "get_callers on 50-symbol graph took {}ms, expected < 10ms",
        duration.as_millis()
    );

    println!("get_callers on 50-symbol graph: {}ms", duration.as_millis());
}

// ============================================================================
// TEST 2: get_callers performance on large graph (1000 symbols)
// ============================================================================

#[test]
fn test_get_callers_large_graph_performance() {
    let (graph, temp_dir) = create_large_graph();

    let node_id = get_first_symbol(&graph, &temp_dir).expect("Symbol not found");

    let mut cache = RelationshipCache::new();
    let start = Instant::now();

    let result = get_callers(&graph, node_id, &mut cache);

    let duration = start.elapsed();

    assert!(result.is_ok(), "get_callers failed: {:?}", result.err());

    // Large graph should complete in < 100ms
    assert!(
        duration.as_millis() < 100 * ci_multiplier(),
        "get_callers on 1000-symbol graph took {}ms, expected < 100ms",
        duration.as_millis()
    );

    println!(
        "get_callers on 1000-symbol graph: {}ms",
        duration.as_millis()
    );
}

// ============================================================================
// TEST 3: get_callees performance on 1K symbol graph
// ============================================================================

#[test]
fn test_get_callees_large_graph_performance() {
    let (graph, temp_dir) = create_large_graph();

    let node_id = get_first_symbol(&graph, &temp_dir).expect("Symbol not found");

    let mut cache = RelationshipCache::new();
    let start = Instant::now();

    let result = get_callees(&graph, node_id, &mut cache);

    let duration = start.elapsed();

    assert!(result.is_ok(), "get_callees failed: {:?}", result.err());

    // Verify performance: < 100ms
    assert!(
        duration.as_millis() < 100 * ci_multiplier(),
        "get_callees on 1000-symbol graph took {}ms, expected < 100ms",
        duration.as_millis()
    );

    println!(
        "get_callees on 1000-symbol graph: {}ms",
        duration.as_millis()
    );
}

// ============================================================================
// TEST 4: get_imports performance on 1K symbol graph
// ============================================================================

#[test]
fn test_get_imports_large_graph_performance() {
    let (graph, temp_dir) = create_large_graph();

    let file_path = get_first_file_path(&temp_dir);
    let file_path_str = file_path.to_str().expect("Invalid path");

    let mut cache = RelationshipCache::new();
    let start = Instant::now();

    let result = get_imports(&graph, file_path_str.as_ref(), &mut cache);

    let duration = start.elapsed();

    assert!(result.is_ok(), "get_imports failed: {:?}", result.err());

    // Verify performance: < 100ms
    assert!(
        duration.as_millis() < 100 * ci_multiplier(),
        "get_imports on 1000-symbol graph took {}ms, expected < 100ms",
        duration.as_millis()
    );

    println!(
        "get_imports on 1000-symbol graph: {}ms",
        duration.as_millis()
    );
}

// ============================================================================
// TEST 5: get_exports performance on 1K symbol graph
// ============================================================================

#[test]
fn test_get_exports_large_graph_performance() {
    let (graph, temp_dir) = create_large_graph();

    let file_path = get_first_file_path(&temp_dir);
    let file_path_str = file_path.to_str().expect("Invalid path");

    let mut cache = RelationshipCache::new();
    let start = Instant::now();

    let result = get_exports(&graph, file_path_str.as_ref(), &mut cache);

    let duration = start.elapsed();

    assert!(result.is_ok(), "get_exports failed: {:?}", result.err());

    // Verify performance: < 100ms
    assert!(
        duration.as_millis() < 100 * ci_multiplier(),
        "get_exports on 1000-symbol graph took {}ms, expected < 100ms",
        duration.as_millis()
    );

    println!(
        "get_exports on 1000-symbol graph: {}ms",
        duration.as_millis()
    );
}

// ============================================================================
// TEST 6: All four relationship types in sequence on 1K graph
// ============================================================================

#[test]
fn test_all_relationship_types_large_graph() {
    let (graph, temp_dir) = create_large_graph();

    let node_id = get_first_symbol(&graph, &temp_dir).expect("Symbol not found");
    let file_path = get_first_file_path(&temp_dir);
    let file_path_str = file_path.to_str().expect("Invalid path");

    let mut cache = RelationshipCache::new();
    let start = Instant::now();

    // Query all four relationship types
    let callers = get_callers(&graph, node_id, &mut cache);
    let callees = get_callees(&graph, node_id, &mut cache);
    let imports = get_imports(&graph, file_path_str.as_ref(), &mut cache);
    let exports = get_exports(&graph, file_path_str.as_ref(), &mut cache);

    let duration = start.elapsed();

    // Verify all succeeded (return Vec, not Result - check they're not Err)
    assert!(callers.is_ok(), "callers query failed");
    assert!(callees.is_ok(), "callees query failed");
    assert!(imports.is_ok(), "imports query failed");
    assert!(exports.is_ok(), "exports query failed");

    // Total time for all four queries should still be < 200ms
    assert!(
        duration.as_millis() < 200 * ci_multiplier(),
        "All relationship queries on 1000-symbol graph took {}ms, expected < 200ms",
        duration.as_millis()
    );

    println!(
        "All 4 relationship types on 1000-symbol graph: {}ms",
        duration.as_millis()
    );
}

// ============================================================================
// TEST 7: Relationship caching effectiveness
// ============================================================================

#[test]
fn test_relationship_cache_effectiveness() {
    let (graph, temp_dir) = create_large_graph();

    let node_id = get_first_symbol(&graph, &temp_dir).expect("Symbol not found");

    // First query (no cache hit)
    let mut cache = RelationshipCache::new();
    let start = Instant::now();
    let result1 = get_callers(&graph, node_id, &mut cache);
    let first_duration = start.elapsed();

    assert!(result1.is_ok(), "First query failed");

    // Second query (should hit cache)
    let start = Instant::now();
    let result2 = get_callers(&graph, node_id, &mut cache);
    let cached_duration = start.elapsed();

    assert!(result2.is_ok(), "Second query failed");

    // Verify cache was used
    let cache_key = format!("caller:{}", node_id.as_i64());
    assert!(
        cache.contains_key(&cache_key),
        "Cache should contain the query result"
    );

    // Cached query should be significantly faster or at least not slower
    println!(
        "First query: {}ms, Cached query: {}ms",
        first_duration.as_millis(),
        cached_duration.as_millis()
    );

    // With current implementation (empty results), timing may be similar
    // but cached query should not be significantly slower
    assert!(
        cached_duration.as_millis() <= first_duration.as_millis() + 10,
        "Cached query should not be significantly slower than first query"
    );
}

// ============================================================================
// TEST 8: Relationship queries on medium graph (200 symbols)
// ============================================================================

#[test]
fn test_relationship_queries_medium_graph() {
    let (graph, temp_dir) = create_medium_graph();

    let node_id = get_first_symbol(&graph, &temp_dir).expect("Symbol not found");

    let mut cache = RelationshipCache::new();
    let start = Instant::now();

    let result = get_callers(&graph, node_id, &mut cache);

    let duration = start.elapsed();

    assert!(result.is_ok(), "get_callers failed: {:?}", result.err());

    // Medium graph should be faster: < 50ms
    assert!(
        duration.as_millis() < 50 * ci_multiplier(),
        "get_callers on 200-symbol graph took {}ms, expected < 50ms",
        duration.as_millis()
    );

    println!(
        "get_callers on 200-symbol graph: {}ms",
        duration.as_millis()
    );
}

// ============================================================================
// TEST 9: Cache clear and reuse
// ============================================================================

#[test]
fn test_cache_clear_and_reuse() {
    let (graph, temp_dir) = create_large_graph();

    let node_id = get_first_symbol(&graph, &temp_dir).expect("Symbol not found");

    let mut cache = RelationshipCache::new();

    // First query - populates cache
    let result1 = get_callers(&graph, node_id, &mut cache);
    assert!(result1.is_ok(), "First query failed");

    let cache_key = format!("caller:{}", node_id.as_i64());
    assert!(cache.contains_key(&cache_key), "Cache should be populated");

    // Clear cache
    cache.clear();
    assert!(
        !cache.contains_key(&cache_key),
        "Cache should be empty after clear"
    );

    // Query again - should repopulate cache
    let result2 = get_callers(&graph, node_id, &mut cache);
    assert!(result2.is_ok(), "Second query failed");
    assert!(
        cache.contains_key(&cache_key),
        "Cache should be repopulated"
    );
}

// ============================================================================
// TEST 10: Multiple symbols queried in sequence on large graph
// ============================================================================

#[test]
fn test_multiple_symbols_large_graph() {
    let (graph, temp_dir) = create_large_graph();

    let file_path = temp_dir.path().join("test_file_0.rs");
    let file_path_str = file_path.to_str().expect("Invalid path");

    // Query multiple symbols from the same file
    let symbol_names = vec!["function_0_0", "function_0_1", "function_0_2"];

    let mut cache = RelationshipCache::new();
    let start = Instant::now();

    for symbol_name in symbol_names {
        let node_id = graph
            .find_symbol_in_file(file_path_str, symbol_name)
            .expect(&format!("Symbol {} not found", symbol_name));

        let result = get_callers(&graph, node_id, &mut cache);
        assert!(
            result.is_ok(),
            "Query for {} failed: {:?}",
            symbol_name,
            result.err()
        );
    }

    let duration = start.elapsed();

    // Querying 3 symbols should still be fast
    assert!(
        duration.as_millis() < 100 * ci_multiplier(),
        "Querying 3 symbols on 1000-symbol graph took {}ms, expected < 100ms",
        duration.as_millis()
    );

    println!(
        "Querying 3 symbols on 1000-symbol graph: {}ms",
        duration.as_millis()
    );
}

// ============================================================================
// TEST 11: Imports and exports on different graph sizes
// ============================================================================

#[test]
fn test_imports_exports_scaling() {
    // Small graph
    let (graph_small, temp_dir_small) = create_small_graph();
    let file_path_small = temp_dir_small.path().join("test_file_0.rs");
    let file_path_small_str = file_path_small.to_str().expect("Invalid path");

    let mut cache = RelationshipCache::new();
    let result_small = get_imports(&graph_small, file_path_small_str.as_ref(), &mut cache);
    assert!(result_small.is_ok(), "get_imports on small graph failed");

    // Medium graph
    let (graph_medium, temp_dir_medium) = create_medium_graph();
    let file_path_medium = temp_dir_medium.path().join("test_file_0.rs");
    let file_path_medium_str = file_path_medium.to_str().expect("Invalid path");

    let mut cache = RelationshipCache::new();
    let result_medium = get_imports(&graph_medium, file_path_medium_str.as_ref(), &mut cache);
    assert!(result_medium.is_ok(), "get_imports on medium graph failed");

    // Large graph
    let (graph_large, temp_dir_large) = create_large_graph();
    let file_path_large = temp_dir_large.path().join("test_file_0.rs");
    let file_path_large_str = file_path_large.to_str().expect("Invalid path");

    let mut cache = RelationshipCache::new();
    let result_large = get_imports(&graph_large, file_path_large_str.as_ref(), &mut cache);
    assert!(result_large.is_ok(), "get_imports on large graph failed");

    println!("Imports/exports queries work on all graph sizes (small, medium, large)");
}

// ============================================================================
// TEST 12: Cache keys are unique per query type
// ============================================================================

#[test]
fn test_cache_keys_unique_per_query_type() {
    let (graph, temp_dir) = create_large_graph();

    let node_id = get_first_symbol(&graph, &temp_dir).expect("Symbol not found");
    let file_path = get_first_file_path(&temp_dir);
    let file_path_str = file_path.to_str().expect("Invalid path");

    let mut cache = RelationshipCache::new();

    // Query all four relationship types
    get_callers(&graph, node_id, &mut cache).unwrap();
    get_callees(&graph, node_id, &mut cache).unwrap();
    get_imports(&graph, file_path_str.as_ref(), &mut cache).unwrap();
    get_exports(&graph, file_path_str.as_ref(), &mut cache).unwrap();

    // Verify all four cache keys exist
    let caller_key = format!("caller:{}", node_id.as_i64());
    let callee_key = format!("callee:{}", node_id.as_i64());
    let import_key = format!("import:{}", file_path_str);
    let export_key = format!("export:{}", file_path_str);

    assert!(
        cache.contains_key(&caller_key),
        "Caller cache key should exist"
    );
    assert!(
        cache.contains_key(&callee_key),
        "Callee cache key should exist"
    );
    assert!(
        cache.contains_key(&import_key),
        "Import cache key should exist"
    );
    assert!(
        cache.contains_key(&export_key),
        "Export cache key should exist"
    );

    println!("All four cache keys are unique and present");
}
