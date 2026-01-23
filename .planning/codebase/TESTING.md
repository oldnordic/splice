# Testing Patterns

**Analysis Date:** 2026-01-23

## Test Framework

**Runner:**
- Rust built-in `cargo test`
- Config: No explicit config file (uses cargo defaults)
- Test modules in `tests/` directory and in-source `#[cfg(test)]` modules

**Assertion Library:**
- Rust built-in `assert!`, `assert_eq!`, `assert!` macros

**Run Commands:**
```bash
cargo test                 # Run all tests
cargo test test_name      # Run specific test
cargo test --lib          # Run library unit tests only
cargo test --tests        # Run integration tests only
```

## Test File Organization

**Location:**
- Integration tests: `tests/*.rs` directory (co-located with source)
- Unit tests: In-source `#[cfg(test)] mod tests` blocks
- Language-specific tests: Separate files per language (e.g., `python_import_tests.rs`)

**Naming:**
- Integration tests: `*_tests.rs` or `test_*.rs` pattern
- Domain-specific: `ingest_tests.rs`, `patch_tests.rs`, `validation_tests.rs`, `resolve_tests.rs`
- Language-specific: `{language}_import_tests.rs`, `{language}_symbol_tests.rs`, `{language}_patch_tests.rs`

**Structure:**
```
tests/
├── ingest_tests.rs                    # Rust ingest tests
├── resolve_tests.rs                   # Reference finding tests
├── patch_tests.rs                    # Patch operations tests
├── validation_tests.rs                # Validation gate tests
├── validation_gates_tests.rs          # Per-language validation
├── cli_tests.rs                      # CLI wiring tests
├── python_import_tests.rs            # Python import extraction
├── python_symbol_tests.rs            # Python symbol extraction
├── python_patch_tests.rs             # Python patch operations
├── cpp_import_tests.rs              # C++ import extraction
├── cpp_symbol_tests.rs              # C++ symbol extraction
├── cpp_patch_tests.rs               # C++ patch operations
├── java_import_tests.rs             # Java import extraction
├── java_symbol_tests.rs             # Java symbol extraction
├── java_patch_tests.rs              # Java patch operations
├── javascript_import_tests.rs        # JavaScript import extraction
├── javascript_patch_tests.rs         # JavaScript patch operations
├── typescript_import_tests.rs       # TypeScript import extraction
├── typescript_symbol_tests.rs       # TypeScript symbol extraction
├── typescript_patch_tests.rs        # TypeScript patch operations
└── mod.rs                          # Test module declarations
```

## Test Structure

**Suite Organization:**
```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_descriptive_name() {
        // Setup
        let source = r#"source code"#;

        // Action
        let result = function_under_test(&source);

        // Assert
        assert!(result.is_ok(), "Expected success, got {:?}", result);
    }

    #[test]
    fn test_specific_scenario() {
        // Test implementation
    }
}
```

**Patterns:**
- Setup phase: Create test data using raw string literals (`r#"..."#`)
- Action phase: Call function/operation being tested
- Assertion phase: Use `assert!` macros with descriptive failure messages
- Temp file setup: `let mut temp_file = NamedTempFile::new().expect("Failed to create temp file");`

**Setup Pattern:**
```rust
let source = r#"
fn hello_world() {
    println!("Hello, world!");
}
"#;

let mut temp_file = NamedTempFile::new().expect("Failed to create temp file");
temp_file.write_all(source.as_bytes()).expect("Failed to write to temp file");
let temp_path = temp_file.path();
```

## Mocking

**Framework:** No explicit mocking framework used

**Patterns:**
- Tests use `tempfile` crate for file system isolation
- Integration tests invoke CLI via `std::process::Command`
- Language-specific tests create temp files with source code samples
- No service mocks found (tests are integration-style with real tools)

**CLI Integration Testing:**
```rust
use std::process::Command;

let output = Command::new("splice")
    .arg("patch")
    .arg("--file")
    .arg(&file_path)
    .arg("--start")
    .arg("10")
    .arg("--end")
    .arg("20")
    .arg("replacement")
    .current_dir(&workspace_path)
    .output()
    .expect("Failed to run splice command");

assert!(output.status.success(), "Command failed: {:?}", output.stderr);
```

**What to Mock:**
- File system operations (using `tempfile`)
- CLI invocation (using `std::process::Command`)
- Language compilers (integrated via validation gates)

**What NOT to Mock:**
- Tree-sitter parsing (uses real parsers)
- Byte span calculations (tests exact behavior)
- Validation gates (use real compilers where available)

## Fixtures and Factories

**Test Data:**
- Inline test data using raw string literals (`r#"..."#`)
- Per-language source samples in individual test files
- Workspace fixtures created via `TempDir` for CLI tests

**Location:**
- Test data inline in test functions
- No separate fixtures directory found
- Each test file contains its own sample code

**Example:**
```rust
let source = r#"
struct MyStruct {
    pub value: i32
}

impl MyStruct {
    pub fn new() -> Self { Self { value: 42 } }
}
"#;
```

## Coverage

**Requirements:** No explicit coverage target enforced (per AGENTS.md, but `cargo test --all` must pass)

**View Coverage:**
```bash
cargo test --all       # Runs 300+ tests
cargo test -- --nocapture  # See test output
```

**Test Count:**
- 40+ test files in `tests/` directory
- In-source unit tests in `src/ingest/imports/*.rs` files
- Each language has import, symbol, and patch tests
- Overall: 300+ tests covering CLI, validation, ingest, resolve, and patch operations

## Test Types

**Unit Tests:**
- Scope: Individual functions and data structures
- Location: In-source `#[cfg(test)]` modules
- Examples: `src/error.rs:821-854` (byte offset conversion test)
- Pattern: Test small, focused functionality in isolation

**Integration Tests:**
- Scope: End-to-end operations across modules
- Location: `tests/*.rs` directory
- Examples: `tests/cli_tests.rs`, `tests/patch_tests.rs`
- Pattern: Create temp workspace → invoke operation → verify results

**E2E Tests:**
- Framework: Not used (no E2E framework detected)
- CLI integration tests serve as end-to-end validation
- Example: `tests/cli_tests.rs` creates full workspace, calls splice, verifies results

## Common Patterns

**Byte Span Testing:**
```rust
// Assert exact byte spans
assert_eq!(symbol.byte_start, 1, "Function should start at byte 1");
assert_eq!(symbol.byte_end, 40, "Function should end at byte 40");
assert_eq!(symbol.line_start, 2, "Function starts on line 2");
assert_eq!(symbol.line_end, 4, "Function ends on line 4");

// Assert column ranges are reasonable (0-based)
assert_eq!(symbol.col_start, 0, "Function starts at column 0");
assert_eq!(symbol.col_end, 1, "Function ends at column 1");
```

**Error Testing:**
```rust
let result = extract_rust_symbols(temp_path, source.as_bytes());

assert!(result.is_ok(), "extract_rust_symbols failed: {:?}", result.err());

let symbols = result.unwrap();

assert!(
    !symbols.is_empty(),
    "Expected at least 1 symbol, got {}",
    symbols.len()
);

// Find specific symbol and verify properties
let hello_world = symbols
    .iter()
    .find(|s| s.name == "hello_world" && s.kind == RustSymbolKind::Function)
    .expect("Should find hello_world function");
```

**Multi-Language Testing:**
- Separate test files per language: `python_*.rs`, `cpp_*.rs`, `java_*.rs`, `javascript_*.rs`, `typescript_*.rs`
- Each language has: import extraction tests, symbol extraction tests, patch tests
- Tests verify language-specific behavior (e.g., impl names, import formats)

**Table-Driven Testing (Limited):**
- Some test files contain similar tests for different scenarios
- Not heavily used; prefer descriptive test names

**CLI Output Testing:**
```rust
// Extract JSON from stdout that may contain debug output
fn extract_json_from_stdout(stdout: &str) -> String {
    let start = stdout.find('{');
    let end = stdout.rfind('}');

    match (start, end) {
        (Some(start), Some(end)) if end >= start => stdout[start..=end].to_string(),
        _ => String::new(),
    }
}

let output = Command::new("splice")
    .arg("query")
    .arg("--json")
    .output()
    .expect("Failed to run splice");

let json_str = extract_json_from_stdout(&String::from_utf8_lossy(&output.stdout));
let value: Value = serde_json::from_str(&json_str).expect("Failed to parse JSON");
```

---

*Testing analysis: 2026-01-23*
