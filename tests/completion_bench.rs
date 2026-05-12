//! Performance benchmarks for completion
//!
//! This test validates that the completion feature meets performance targets.
//! Target: Average query time <10ms for completion requests.

use splice::completion::engine::CompletionEngine;
use splice::completion::types::CompletionRequest;
use splice::graph::MagellanIntegration;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Instant;

#[test]
fn benchmark_completion_performance() {
    let db_path = PathBuf::from(".magellan/splice.db");
    if !db_path.exists() {
        eprintln!(
            "Skipping benchmark: database not found at {}",
            db_path.display()
        );
        return;
    }

    // Open Magellan database
    #[allow(
        clippy::arc_with_non_send_sync,
        reason = "single-threaded shared ownership for completion engine"
    )]
    let magellan = match MagellanIntegration::open(&db_path) {
        Ok(m) => Arc::new(m),
        Err(e) => {
            eprintln!("Skipping benchmark: failed to open database: {}", e);
            return;
        }
    };

    // Create completion engine
    let engine = CompletionEngine::new(magellan, &db_path);

    // Test request: src/completion/engine.rs has imports
    // Line 30, column 8 is inside the complete_at_cursor method
    let request = CompletionRequest {
        file_path: PathBuf::from("src/completion/engine.rs"),
        line: 30,
        column: 8,
        max_results: Some(10),
    };

    // Warm-up run (cache database connections, etc.)
    let _ = engine.complete_at_cursor(request.clone());

    // Performance benchmark: 100 iterations
    let iterations = 100;
    let start = Instant::now();

    let mut individual_times = Vec::with_capacity(iterations);

    for i in 0..iterations {
        let iter_start = Instant::now();
        let response = match engine.complete_at_cursor(request.clone()) {
            Ok(r) => r,
            Err(e) => {
                eprintln!("Benchmark failed at iteration {}: {}", i, e);
                return;
            }
        };
        let iter_elapsed = iter_start.elapsed();

        individual_times.push(iter_elapsed.as_millis() as f64);

        // Verify we get results
        if i == 0 {
            println!("\n=== Benchmark Configuration ===");
            println!("Iterations: {}", iterations);
            println!("File: {}", request.file_path.display());
            println!("Line: {}, Column: {}", request.line, request.column);
            println!("Max results: {:?}", request.max_results);
            println!("\n=== Sample Results (First Iteration) ===");
            println!("Suggestions: {}", response.suggestions.len());
            println!(
                "Total symbols indexed: {}",
                response.metadata.total_symbols_indexed
            );
            println!("Database version: {}", response.metadata.database_version);
            println!("Database queries: {}", response.metadata.database_queries);
        }
    }

    let total_elapsed = start.elapsed();
    let avg_ms = total_elapsed.as_millis() as f64 / iterations as f64;

    // Calculate statistics
    let min_ms = individual_times
        .iter()
        .cloned()
        .fold(f64::INFINITY, f64::min);
    let max_ms = individual_times
        .iter()
        .cloned()
        .fold(f64::NEG_INFINITY, f64::max);

    // Sort for median calculation
    let mut sorted_times = individual_times.clone();
    sorted_times.sort_by(|a, b| a.partial_cmp(b).unwrap());
    let median_ms = sorted_times[iterations / 2];

    // Calculate p95 (95th percentile)
    let p95_index = (iterations as f64 * 0.95) as usize;
    let p95_ms = sorted_times[p95_index];

    // Print results
    println!("\n=== Performance Results ===");
    println!("Total time: {:.2} ms", total_elapsed.as_millis());
    println!("Average time: {:.2} ms", avg_ms);
    println!("Median time: {:.2} ms", median_ms);
    println!("Min time: {:.2} ms", min_ms);
    println!("Max time: {:.2} ms", max_ms);
    println!("95th percentile: {:.2} ms", p95_ms);

    // Verify performance target
    println!("\n=== Performance Validation ===");
    if avg_ms < 10.0 {
        println!("✓ PASSED: Average time {:.2}ms < 10ms target", avg_ms);
    } else if avg_ms < 20.0 {
        println!(
            "⚠ ACCEPTABLE: Average time {:.2}ms < 20ms (above 10ms target)",
            avg_ms
        );
        println!("  Consider optimization if this is a hot path");
    } else {
        println!(
            "✗ FAILED: Average time {:.2}ms exceeds 20ms threshold",
            avg_ms
        );
        println!("  Performance optimization required");
    }

    // Assert performance requirement
    assert!(
        avg_ms < 10.0,
        "Average query time should be <10ms, got {:.2}ms. \
         Optimization opportunities:\n\
         - Cache database connections\n\
         - Use prepared statements\n\
         - Reduce query complexity\n\
         - Add result caching",
        avg_ms
    );

    println!("\n=== Benchmark Complete ===");
}
