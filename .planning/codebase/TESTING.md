# Testing Patterns

**Analysis Date:** 2026-01-23

## Test Framework

**Runner:**
- Cargo test (built-in Rust test framework)
- Config: Standard Cargo.toml `[dev-dependencies]` section
- Parallel execution by default
- Test isolation through temporary directories

**Assertion Library:**
- Standard `assert!`, `assert_eq!`, `assert_ne!`
- Custom assertions for complex types like `Diagnostic`
- Result-based testing with `?` operator

**Run Commands:**
```bash
cargo test                    # Run all tests
cargo test --lib             # Run library tests only
cargo test integration        # Run integration tests (named)
cargo test -- --nocapture      # Show test output
cargo test --release          # Run tests in release mode
```

## Test File Organization

**Location:**
- Integration tests: `tests/` directory (46 test files)
- Unit tests: Co-located with source code (`#[cfg(test)]` mod)
- Performance tests: Dedicated module with benchmarks
- E2E tests: Mixed with integration tests

**Naming:**
- Integration: `*_tests.rs` (e.g., `patch_tests.rs`)
- Unit: `#[cfg(test)] mod tests` within source
- E2E: `*_tests.rs` with CLI commands
- Performance: `*_performance_tests.rs`

**Structure:**
```
tests/
├── patch_tests.rs          # Core patch validation
├── e2e_refactor_tests.rs   # Full workflow tests
├── integration_*.rs        # Cross-module integration
├── language_*.rs           # Language-specific tests
├── performance_*.rs       # Performance benchmarks
└── *_tests.rs              # Feature-specific tests
```

## Test Structure

**Suite Organization:**
```rust
#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;
    use std::process::Command;

    #[test]
    fn test_feature_x() {
        // Setup: Create temporary workspace
        let workspace = setup_test_workspace();

        // Exercise: Perform operation
        let result = operation_under_test(&workspace);

        // Verify: Check expected outcomes
        assert!(result.is_ok());
        verify_post_conditions(&workspace);
    }

    #[test]
    fn test_error_handling() {
        // Test error cases
        let result = operation_that_fails();
        assert!(matches!(result, Err(SpliceError::InvalidSpan { .. })));
    }
}
```

**Patterns:**
- Setup: Temporary directories with isolated workspaces
- Teardown: Automatic cleanup via `TempDir` drop
- Assertion: Clear error message matches for `Result<T, SpliceError>`
- Isolation: Each test creates fresh workspace/files

## Mocking

**Framework:** Minimal mocking, direct tool invocation
**Patterns:**
- External tools: Call real binaries (cargo check, rust-analyzer)
- File system: Use `tempfile` for isolated test environments
- Database: Use temporary SQLite files
- CLI: Invoke via `std::process::Command` with subprocess

**What to Mock:**
- Complex external dependencies (minimal)
- File I/O (via temporary files)
- Time-based operations (use fixed values)
- Random values (use seeded generators)

**What NOT to Mock:**
- Core library functionality
- Tree-sitter parsing (real parsing)
- Database operations (real SQLite)
- File hashing (real SHA-256)

## Fixtures and Factories

**Test Data:**
```rust
/// Create minimal Rust workspace for testing
fn create_rust_workspace() -> TempDir {
    let workspace_dir = TempDir::new().expect("Failed to create temp workspace");
    let workspace_path = workspace_dir.path();

    // Create Cargo.toml
    let cargo_toml = r#"[package]
name = "test-workspace"
version = "0.1.0"
edition = "2021"

[lib]
name = "test_workspace"
path = "src/lib.rs"
"#;
    fs::write(workspace_path.join("Cargo.toml"), cargo_toml)
        .expect("Failed to write Cargo.toml");

    // Create src directory and files
    let src_dir = workspace_path.join("src");
    fs::create_dir(&src_dir).expect("Failed to create src directory");

    workspace_dir
}

/// Create test source with specific symbol
fn create_test_file_with_symbol(path: &Path, symbol_name: &str, symbol_kind: &str) {
    let source = format!(
        r#"
pub fn {}() -> String {{
    format!("Hello, {{}}!", "world")
}}
"#,
        symbol_name
    );
    fs::write(path, source).expect("Failed to write test file");
}
```

**Location:**
- Test-specific helpers in each test module
- Shared utilities in `tests/mod.rs`
- Constants for common test data

## Coverage

**Requirements:** Not enforced at build time
**View Coverage:**
```bash
cargo install cargo-llvm-cov
cargo llvm-cov --lcov --output-path coverage.lcov
```

**Coverage Targets:**
- Core refactoring logic: High coverage
- Error handling: All paths tested
- Validation gates: Both success and failure paths
- Database operations: CRUD operations covered

## Test Types

**Unit Tests:**
- Scope: Individual functions/methods
- Focus: Happy path and error conditions
- Speed: Fast (microseconds to milliseconds)
- Example: Symbol parsing, hash computation

**Integration Tests:**
- Scope: Multi-module workflows
- Focus: API interactions and data flow
- Speed: Medium (milliseconds to seconds)
- Example: Patch → validate → commit flow

**E2E Tests:**
- Scope: Full refactoring workflows
- Focus: CLI behavior and external tool integration
- Speed: Slow (seconds to minutes)
- Example: Complete refactor with backup/restore

## Common Patterns

**Async Testing:**
- Minimal async in core library
- CLI tests use blocking subprocess calls
- No async/await in tests (simpler)

**Error Testing:**
```rust
#[test]
fn test_symbol_not_found() {
    let result = resolve_symbol(&empty_graph, None, None, "nonexistent");
    assert!(matches!(result, Err(SpliceError::SymbolNotFound { .. })));

    // Check error details
    if let Err(SpliceError::SymbolNotFound { symbol, hint, .. }) = result {
        assert_eq!(symbol, "nonexistent");
        assert!(hint.contains("Run `splice ingest`"));
    }
}
```

**Workspace Setup:**
```rust
#[test]
fn test_cross_file_refactor() {
    let workspace = create_multi_file_workspace();

    // Ingest symbols from multiple files
    let graph = create_test_graph(&workspace);

    // Perform cross-file operation
    let plan = create_cross_file_plan(&graph);

    // Execute and verify
    assert!(execute_plan(&plan, &workspace).is_ok());
    verify_cross_file_changes(&workspace);
}
```

**Performance Testing:**
```rust
#[test]
fn test_large_file_performance() {
    let workspace = create_large_test_file(100_000); // 100K lines

    let start = Instant::now();
    let result = ingest_large_file(&workspace);
    let duration = start.elapsed();

    assert!(result.is_ok());
    assert!(duration.as_millis() < 1000, "Too slow: {:?}", duration);
}
```

**Tool Validation:**
```rust
#[test]
fn test_compiler_validation_integration() {
    let workspace = create_rust_workspace();

    // Apply patch that should fail
    let result = apply_patch_with_validation(
        &workspace.join("src/lib.rs"),
        100, 200,
        "invalid rust code {",
        &workspace,
        Language::Rust,
        AnalyzerMode::Off,
    );

    // Should fail with compiler error
    assert!(matches!(result, Err(SpliceError::CompilerValidationFailed { .. })));
}
```

**Snapshot Testing:**
- Limited usage (file content diffs preferred)
- For complex outputs like generated patches
- Compare actual vs expected byte-by-byte

---

*Testing analysis: 2026-01-23*