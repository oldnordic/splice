//! Performance tests for context extraction on large files.
//!
//! This test suite validates that context extraction using ropey scales efficiently
//! for large files as specified in Phase 17 success criteria.
//!
//! Performance expectations:
//! - 32KB files: < 100ms
//! - 64KB files: < 200ms
//! - 128KB files: < 400ms
//!
//! Context extraction uses ropey Rope for O(log n) line calculations,
//! which should scale efficiently even for very large files.
//!
//! Tests include:
//! - Basic context extraction on 32KB, 64KB, 128KB files
//! - Asymmetric context extraction (different before/after counts)
//! - Context extraction with symbol expansion (expand_to_body_with_docs)
//! - File boundary extraction (start and end of file)
//! - Linear scaling verification across different file sizes

use splice::context::{extract_context, extract_context_asymmetric};
use splice::expand::expand_to_body_with_docs;
use splice::symbol::Language;
use std::io::Write;
use std::path::Path;
use std::time::Instant;
use tempfile::TempDir;

/// CI shared runners are ~3x slower than local dev machines.
fn ci_multiplier() -> u128 {
    if std::env::var("CI").is_ok() { 3 } else { 1 }
}

/// Create a large Rust file with repeated function definitions.
///
/// Each function pair is approximately 100-110 bytes, so num_functions * 100 ≈ file size.
/// Functions include doc comments to enable expansion testing.
///
/// # Arguments
///
/// * `dir` - Temporary directory to create the file in
/// * `name` - Base name for the file
/// * `num_functions` - Number of function pairs to create
///
/// # Returns
///
/// The path to the created file
fn create_large_rust_file(dir: &Path, name: &str, num_functions: usize) -> std::path::PathBuf {
    let file_path = dir.join(name);
    let mut file = std::fs::File::create(&file_path).unwrap();

    for i in 0..num_functions {
        writeln!(
            file,
            r#"/// Documentation for function {}
/// This function adds two numbers together.
pub fn add_{}(x: i32, y: i32) -> i32 {{
    // Line 1 of function body
    // Line 2 of function body
    // Line 3 of function body
    x + y
}}

/// Another function for multiplication
pub fn multiply_{}(x: i32, y: i32) -> i32 {{
    x * y
}}
"#,
            i, i, i
        )
        .unwrap();
    }

    file_path
}

/// Test context extraction on 32KB file.
///
/// Verifies:
/// - Context extraction works on files >32KB
/// - Performance is < 100ms
/// - Before, selected, and after context are all extracted
#[test]
#[ignore = "performance benchmark: too slow on shared CI runners"]
fn test_context_extraction_32kb_file() {
    let dir = TempDir::new().unwrap();

    // Create file with ~350 functions (~35KB)
    let file_path = create_large_rust_file(dir.path(), "large_32kb.rs", 350);

    let file_size = std::fs::metadata(&file_path).unwrap().len();
    assert!(
        file_size > 32_000,
        "Test file should exceed 32KB, got {} bytes",
        file_size
    );

    let source = std::fs::read_to_string(&file_path).unwrap();

    // Find a symbol in the middle of the file
    let offset = source.find("add_175").expect("Symbol not found");

    let start = Instant::now();
    let ctx = extract_context(&file_path, offset, offset + 50, 3).unwrap();
    let duration = start.elapsed();

    // Verify context was extracted
    assert!(!ctx.before.is_empty(), "Should have context before");
    assert!(!ctx.selected.is_empty(), "Should have selected context");
    assert!(!ctx.after.is_empty(), "Should have context after");

    // Verify performance: < 100ms (300ms on CI)
    let max_ms = 100 * ci_multiplier();
    assert!(
        duration.as_millis() < max_ms,
        "Context extraction on 32KB file took {}ms, expected < {}ms",
        duration.as_millis(),
        max_ms
    );

    println!(
        "Context extraction on {}KB file took {}ms",
        file_size / 1024,
        duration.as_millis()
    );
}

/// Test context extraction on 64KB file.
///
/// Verifies:
/// - Context extraction works on files >64KB
/// - Performance is < 200ms
/// - All context components are extracted
#[test]
#[ignore = "performance benchmark: too slow on shared CI runners"]
fn test_context_extraction_64kb_file() {
    let dir = TempDir::new().unwrap();

    // Create file with ~700 functions (~70KB)
    let file_path = create_large_rust_file(dir.path(), "large_64kb.rs", 700);

    let file_size = std::fs::metadata(&file_path).unwrap().len();
    assert!(
        file_size > 64_000,
        "Test file should exceed 64KB, got {} bytes",
        file_size
    );

    let source = std::fs::read_to_string(&file_path).unwrap();

    // Find a symbol in the middle of the file
    let offset = source.find("add_350").expect("Symbol not found");

    let start = Instant::now();
    let ctx = extract_context(&file_path, offset, offset + 50, 3).unwrap();
    let duration = start.elapsed();

    // Verify context was extracted
    assert!(!ctx.before.is_empty(), "Should have context before");
    assert!(!ctx.selected.is_empty(), "Should have selected context");
    assert!(!ctx.after.is_empty(), "Should have context after");

    // Verify performance: < 200ms (600ms on CI)
    let max_ms = 200 * ci_multiplier();
    assert!(
        duration.as_millis() < max_ms,
        "Context extraction on 64KB file took {}ms, expected < {}ms",
        duration.as_millis(),
        max_ms
    );

    println!(
        "Context extraction on {}KB file took {}ms",
        file_size / 1024,
        duration.as_millis()
    );
}

/// Test context extraction on 128KB file.
///
/// Verifies:
/// - Context extraction works on files >128KB
/// - Performance is < 400ms
/// - Scaling remains acceptable at larger file sizes
#[test]
#[ignore = "performance benchmark: too slow on shared CI runners"]
fn test_context_extraction_128kb_file() {
    let dir = TempDir::new().unwrap();

    // Create file with ~1400 functions (~140KB)
    let file_path = create_large_rust_file(dir.path(), "large_128kb.rs", 1400);

    let file_size = std::fs::metadata(&file_path).unwrap().len();
    assert!(
        file_size > 128_000,
        "Test file should exceed 128KB, got {} bytes",
        file_size
    );

    let source = std::fs::read_to_string(&file_path).unwrap();

    // Find a symbol in the middle of the file
    let offset = source.find("add_700").expect("Symbol not found");

    let start = Instant::now();
    let ctx = extract_context(&file_path, offset, offset + 50, 3).unwrap();
    let duration = start.elapsed();

    // Verify context was extracted
    assert!(!ctx.before.is_empty(), "Should have context before");
    assert!(!ctx.selected.is_empty(), "Should have selected context");
    assert!(!ctx.after.is_empty(), "Should have context after");

    // Verify performance: < 400ms (1200ms on CI)
    let max_ms = 400 * ci_multiplier();
    assert!(
        duration.as_millis() < max_ms,
        "Context extraction on 128KB file took {}ms, expected < {}ms",
        duration.as_millis(),
        max_ms
    );

    println!(
        "Context extraction on {}KB file took {}ms",
        file_size / 1024,
        duration.as_millis()
    );
}

/// Test context extraction with expansion on large file.
///
/// Verifies:
/// - expand_to_body_with_docs works on 64KB files
/// - Combined expansion + context extraction performance
/// - Performance is < 300ms for the combined operation
#[test]
#[ignore = "performance benchmark: too slow on shared CI runners"]
fn test_context_extraction_with_expansion_64kb() {
    let dir = TempDir::new().unwrap();

    // Create file with ~700 functions (~70KB)
    let file_path = create_large_rust_file(dir.path(), "large_expand.rs", 700);

    let file_size = std::fs::metadata(&file_path).unwrap().len();
    assert!(
        file_size > 64_000,
        "Test file should exceed 64KB, got {} bytes",
        file_size
    );

    let source = std::fs::read_to_string(&file_path).unwrap();
    let offset = source.find("add_350").expect("Symbol not found");

    let start = Instant::now();

    // First expand to body (with docs)
    let (expanded_start, expanded_end) =
        expand_to_body_with_docs(&file_path, offset, Language::Rust).unwrap();

    // Then extract context from expanded span
    let ctx = extract_context(&file_path, expanded_start, expanded_end, 3).unwrap();

    let duration = start.elapsed();

    // Verify expansion worked
    assert!(
        expanded_end > expanded_start,
        "Should expand to larger span"
    );
    assert!(!ctx.before.is_empty(), "Should have context before");
    assert!(
        !ctx.selected.is_empty(),
        "Should have selected content from expanded span"
    );

    // Verify performance: < 300ms (expansion + context, 900ms on CI)
    let max_ms = 300 * ci_multiplier();
    assert!(
        duration.as_millis() < max_ms,
        "Expansion + context extraction on 64KB file took {}ms, expected < {}ms",
        duration.as_millis(),
        max_ms
    );

    println!(
        "Expansion + context extraction on {}KB file took {}ms",
        file_size / 1024,
        duration.as_millis()
    );
}

/// Test asymmetric context extraction on large file.
///
/// Verifies:
/// - extract_context_asymmetric works on 64KB files
/// - Different before/after context counts are respected
/// - Performance is < 150ms for asymmetric extraction
#[test]
#[ignore = "performance benchmark: too slow on shared CI runners"]
fn test_asymmetric_context_extraction_64kb() {
    let dir = TempDir::new().unwrap();

    // Create file with ~700 functions (~70KB)
    let file_path = create_large_rust_file(dir.path(), "large_asymmetric.rs", 700);

    let file_size = std::fs::metadata(&file_path).unwrap().len();
    assert!(
        file_size > 64_000,
        "Test file should exceed 64KB, got {} bytes",
        file_size
    );

    let source = std::fs::read_to_string(&file_path).unwrap();
    let offset = source.find("add_350").expect("Symbol not found");

    let start = Instant::now();

    // Extract asymmetric context (5 before, 2 after)
    let ctx = extract_context_asymmetric(&file_path, offset, offset + 50, 5, 2).unwrap();

    let duration = start.elapsed();

    // Verify asymmetric counts (exact count may vary based on file position)
    assert!(
        ctx.before.len() <= 5,
        "Should have at most 5 lines before, got {}",
        ctx.before.len()
    );
    assert!(
        ctx.after.len() <= 2,
        "Should have at most 2 lines after, got {}",
        ctx.after.len()
    );
    assert!(!ctx.selected.is_empty(), "Should have selected content");

    // Verify performance: < 150ms (450ms on CI)
    let max_ms = 150 * ci_multiplier();
    assert!(
        duration.as_millis() < max_ms,
        "Asymmetric context extraction on 64KB file took {}ms, expected < {}ms",
        duration.as_millis(),
        max_ms
    );

    println!(
        "Asymmetric context extraction ({} before, {} after) on {}KB file took {}ms",
        ctx.before.len(),
        ctx.after.len(),
        file_size / 1024,
        duration.as_millis()
    );
}

/// Test context extraction at file boundaries.
///
/// Verifies:
/// - Context extraction works at the start of a file
/// - Context extraction works at the end of a file
/// - Boundary conditions don't cause errors
/// - Performance at boundaries is acceptable (< 50ms)
#[test]
#[ignore = "performance benchmark: too slow on shared CI runners"]
fn test_context_extraction_at_file_boundaries() {
    let dir = TempDir::new().unwrap();

    // Create file with ~350 functions (~35KB)
    let file_path = create_large_rust_file(dir.path(), "large_boundary.rs", 350);

    let source = std::fs::read_to_string(&file_path).unwrap();

    // Test at beginning of file (first function)
    let first_offset = source.find("add_0").expect("Symbol not found");
    let start = Instant::now();
    let ctx_start = extract_context(&file_path, first_offset, first_offset + 50, 3).unwrap();
    let duration_start = start.elapsed();

    assert!(
        !ctx_start.selected.is_empty(),
        "Should extract at file start"
    );
    // At file start, before should be empty or minimal
    assert!(
        ctx_start.before.len() <= 3,
        "At file start, before context should be minimal"
    );

    // Test at end of file (last function)
    let last_offset = source.rfind("multiply_").expect("Symbol not found");
    let start = Instant::now();
    let ctx_end = extract_context(&file_path, last_offset, last_offset + 50, 3).unwrap();
    let duration_end = start.elapsed();

    assert!(!ctx_end.selected.is_empty(), "Should extract at file end");
    // At file end, after should be empty or minimal
    assert!(
        ctx_end.after.len() <= 1,
        "At file end, after context should be minimal"
    );

    // Both should be fast (< 50ms, 150ms on CI)
    let max_ms = 50 * ci_multiplier();
    assert!(
        duration_start.as_millis() < max_ms,
        "Context extraction at file start took {}ms, expected < {}ms",
        duration_start.as_millis(),
        max_ms
    );
    assert!(
        duration_end.as_millis() < max_ms,
        "Context extraction at file end took {}ms, expected < {}ms",
        duration_end.as_millis(),
        max_ms
    );

    println!(
        "Boundary extraction: start {}ms, end {}ms",
        duration_start.as_millis(),
        duration_end.as_millis()
    );
}

/// Test performance scaling is linear.
///
/// Verifies:
/// - Context extraction scales roughly linearly with file size
/// - Small files (100 functions) complete quickly
/// - Medium files (200 functions) complete in reasonable time
/// - Larger files (400 functions) still perform well
///
/// This test ensures the O(log n) behavior of ropey is providing
/// efficient line calculations without unexpected overhead.
#[test]
#[ignore = "performance benchmark: too slow on shared CI runners"]
fn test_context_extraction_linear_scaling() {
    let dir = TempDir::new().unwrap();

    // CI shared runners are ~3x slower than local dev machines.
    let ci_multiplier = if std::env::var("CI").is_ok() { 3 } else { 1 };

    // Create files of different sizes and measure performance
    let test_cases = vec![
        (100, 50 * ci_multiplier),  // 100 functions, max 50ms
        (200, 100 * ci_multiplier), // 200 functions, max 100ms
        (400, 200 * ci_multiplier), // 400 functions, max 200ms
    ];

    let mut timings = Vec::new();

    for (num_functions, expected_max_ms) in test_cases {
        let file_path = create_large_rust_file(
            dir.path(),
            &format!("scale_{}.rs", num_functions),
            num_functions,
        );

        let source = std::fs::read_to_string(&file_path).unwrap();
        let offset = source.find("add_").expect("Symbol not found");

        let start = Instant::now();
        let ctx = extract_context(&file_path, offset, offset + 50, 3).unwrap();
        let duration = start.elapsed();

        assert!(
            !ctx.before.is_empty(),
            "Should extract context for {} functions",
            num_functions
        );

        let duration_ms = duration.as_millis();
        timings.push((num_functions, duration_ms));

        assert!(
            duration_ms < expected_max_ms,
            "Context extraction on {}-function file took {}ms, expected < {}ms",
            num_functions,
            duration_ms,
            expected_max_ms
        );

        println!("{} functions: {}ms", num_functions, duration_ms);
    }

    // Verify linear-ish scaling: each doubling should not more than triple the time
    // This is a loose check to catch严重的 performance regressions
    if timings.len() >= 2 {
        let (_, time1) = timings[0];
        let (count2, time2) = timings[1];

        // Going from ~100 to ~200 functions should not take more than 3x the time
        let ratio = time2 as f64 / time1.max(1) as f64;
        assert!(
            ratio < 3.0,
            "Performance scaling seems non-linear: {} functions took {:.2}x the time of {} functions",
            count2,
            ratio,
            timings[0].0
        );
    }
}

/// Test context extraction with zero context lines.
///
/// Verifies the edge case where no context is requested.
#[test]
#[ignore = "performance benchmark: too slow on shared CI runners"]
fn test_context_extraction_zero_context_large_file() {
    let dir = TempDir::new().unwrap();

    let file_path = create_large_rust_file(dir.path(), "large_zero.rs", 350);

    let source = std::fs::read_to_string(&file_path).unwrap();
    let offset = source.find("add_175").expect("Symbol not found");

    let start = Instant::now();
    let ctx = extract_context(&file_path, offset, offset + 50, 0).unwrap();
    let duration = start.elapsed();

    // With zero context, before and after should be empty
    assert_eq!(ctx.before.len(), 0, "Should have no context before");
    assert_eq!(ctx.after.len(), 0, "Should have no context after");
    assert!(!ctx.selected.is_empty(), "Should have selected content");

    // Should still be very fast with no context (< 50ms, 150ms on CI)
    let max_ms = 50 * ci_multiplier();
    assert!(
        duration.as_millis() < max_ms,
        "Zero-context extraction took {}ms, expected < {}ms",
        duration.as_millis(),
        max_ms
    );
}

/// Test context extraction with large context window.
///
/// Verifies performance when requesting many lines of context.
#[test]
#[ignore = "performance benchmark: too slow on shared CI runners"]
fn test_context_extraction_large_context_window() {
    let dir = TempDir::new().unwrap();

    let file_path = create_large_rust_file(dir.path(), "large_window.rs", 700);

    let source = std::fs::read_to_string(&file_path).unwrap();
    let offset = source.find("add_350").expect("Symbol not found");

    let start = Instant::now();
    let ctx = extract_context(&file_path, offset, offset + 50, 20).unwrap();
    let duration = start.elapsed();

    // Should extract up to 20 lines before and after
    assert!(
        ctx.before.len() <= 20,
        "Should have at most 20 lines before, got {}",
        ctx.before.len()
    );
    assert!(
        ctx.after.len() <= 20,
        "Should have at most 20 lines after, got {}",
        ctx.after.len()
    );

    // Even with large context window, should be reasonably fast (< 200ms, 600ms on CI)
    let max_ms = 200 * ci_multiplier();
    assert!(
        duration.as_millis() < max_ms,
        "Large context window extraction took {}ms, expected < {}ms",
        duration.as_millis(),
        max_ms
    );

    println!(
        "Large context window (up to 20 lines): {}ms",
        duration.as_millis()
    );
}
