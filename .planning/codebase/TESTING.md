# Testing Patterns

**Analysis Date:** 2026-01-22

## Test Framework

**Runner:**
- Rust built-in test runner (`cargo test`)
- No additional test framework configuration
- Dev dependencies: No explicit test framework

**Assertion Library:**
- Standard library `assert!`, `assert_eq!`, `assert_ne!`
- No additional assertion library

**Run Commands:**
```bash
cargo test              # Run all tests
cargo test --lib        # Run only library tests
cargo test --release    # Run tests in release mode
```

## Test File Organization

**Location:**
- Tests co-located with source code (`src/`)
- Each module contains `#[cfg(test)]` sections
- No separate `tests/` directory

**Naming:**
- Test files same as source files
- Test functions prefixed with `test_`
- Test modules named `tests`

**Structure:**
```
src/
├── module.rs
└── #[cfg(test)]          # Test code in same file
mod tests {
    #[test]
    fn test_feature() {
        // Test implementation
    }
}
```

## Test Structure

**Suite Organization:**
```rust
#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;
    use std::path::Path;

    #[test]
    fn test_extract_simple_import_basic() {
        let source = b"import os\n";
        let path = Path::new("test.py");
        let result = extract_python_imports(path, source);
        assert!(result.is_ok());
        let imports = result.unwrap();
        assert_eq!(imports.len(), 1);
    }
}
```

**Patterns:**
- Setup and teardown using `TempDir`
- Path-based testing with `Path::new`
- Error testing with `result.is_ok()`/`is_err()`
- Assertion methods for custom types

## Mocking

**Framework:** No mocking framework

**Patterns:**
- File system mocking with `tempfile`
- In-memory data structures for test data
- No external API mocking

**What to Mock:**
- File operations using `TempDir`
- Path operations with `Path::new`
- No network or external service calls in tests

**What NOT to Mock:**
- Core logic being tested
- Database operations (using test databases)
- Tree-sitter parsing (actual parsing used)

## Fixtures and Factories

**Test Data:**
```rust
// Inline test data
let source = b"import os\nfrom sys import path";
let path = Path::new("test.py");

// Complex test structures
let temp_dir = TempDir::new().unwrap();
let file_path = temp_dir.path().join("test.rs");
```

**Location:**
- Test data defined inline in tests
- No separate fixture files
- Reusable patterns in helper functions

## Coverage

**Requirements:** No explicit coverage target

**View Coverage:**
```bash
cargo test               # Built-in test runner
# No external coverage tool configured
```

## Test Types

**Unit Tests:**
- Scope: Individual functions and modules
- Approach: Isolated testing with mock data
- Coverage: Comprehensive for public APIs

**Integration Tests:**
- Scope: Cross-module interactions
- Approach: Real file system operations
- Database operations with test databases

**E2E Tests:**
- Framework: Not used
- CLI testing via command execution

## Common Patterns

**Async Testing:**
- No async patterns found in tests
- All tests are synchronous

**Error Testing:**
```rust
#[test]
fn test_error_conditions() {
    let result = risky_operation();
    assert!(result.is_err());
    if let Err(e) = result {
        assert_eq!(e.to_string(), "Expected error");
    }
}
```

**File System Testing:**
```rust
#[test]
fn test_file_operations() {
    let temp_dir = TempDir::new().unwrap();
    let file_path = temp_dir.path().join("test.txt");

    // Write test file
    fs::write(&file_path, "test content").unwrap();

    // Read and verify
    let content = fs::read_to_string(&file_path).unwrap();
    assert_eq!(content, "test content");
}
```

**Test Organization:**
- Related tests grouped in modules
- Setup/teardown per test (not shared)
- Independent test execution

## Test Results

**Current Status:**
- Total tests: 276 (estimated from multiple test runs)
- Success rate: 100% for library tests
- No flaky tests detected
- Fast execution (most tests < 0.01s)

**Test Distribution:**
- `verify.rs`: 17 tests
- `graph/`: Multiple test modules
- `ingest/`: Comprehensive test coverage for all language parsers
- `execution/`: Integration tests for logging