# Phase 26: Integration Testing - Research

**Researched:** 2026-01-24
**Domain:** CLI end-to-end testing, command integration validation
**Confidence:** HIGH

## Summary

Phase 26 validates the unified CLI interface through end-to-end integration testing. The phase builds on Phases 23-25 which implemented Magellan query methods, CLI commands with --output flag, exit codes, and export functionality. Research confirms the existing test infrastructure (54 integration test files, 1,700+ LOC in CLI tests, established patterns using `std::process::Command`, `tempfile::TempDir`, and JSON extraction utilities) provides a solid foundation for comprehensive CLI testing.

The codebase already has extensive integration testing patterns including subprocess-based CLI invocation (`get_splice_binary()`), temporary workspace fixtures, JSON payload validation, and error code verification. The main areas needing integration coverage are: query commands (status, query, find, refs, files), export command (all three formats), error code mapping from Magellan errors, LLM consumption workflow validation, and performance benchmarking.

**Primary recommendation:** Extend existing test patterns in `tests/cli_tests.rs` and `tests/cli_output_tests.rs` with focused end-to-end test suites for each CLI command, using the established `get_splice_binary()` helper, `extract_json_from_stdout()` utility, and `TempDir` fixtures for isolated test environments.

## Standard Stack

### Core
| Library | Version | Purpose | Why Standard |
|---------|---------|---------|--------------|
| `std::process::Command` | builtin | Spawn subprocess for CLI testing | Rust standard for integration testing |
| `tempfile` | 3.10 | Temporary directories for isolation | Already in dependencies, proven pattern |
| `serde_json` | 1.0 | Validate JSON output structure | Already in dependencies, used in 15+ test files |
| `splice::graph::magellan_integration::MagellanIntegration` | internal | Create test databases | In-process DB setup for fixture data |

### Supporting
| Library | Version | Purpose | When to Use |
|---------|---------|---------|-------------|
| `sha2` | 0.10 | Verify file content changes | Already in dependencies for checksum validation |

### Alternatives Considered
| Instead of | Could Use | Tradeoff |
|------------|-----------|----------|
| Subprocess testing | In-library testing | Subprocess validates actual binary behavior, not internal API |

**Installation:** No new dependencies required.

## Architecture Patterns

### Recommended Test Structure

```
tests/
├── cli_tests.rs                    # Existing: 1,700+ LOC, subprocess CLI tests
├── cli_output_tests.rs             # Existing: export format tests, response types
├── magellan_integration_tests.rs   # Existing: Magellan wrapper tests
├── llm_consumption_tests.rs        # Existing: 16 LLM JSON validation tests
└── integration_e2e_tests.rs        # NEW: end-to-end workflow tests
```

### Pattern 1: Subprocess CLI Testing

**What:** Spawn the actual splice binary using `std::process::Command` and validate exit codes, stdout, stderr.

**When to use:** All CLI command integration tests requiring binary validation.

**Example:**
```rust
// Source: /home/feanor/Projects/splice/tests/cli_tests.rs:33-100
fn get_splice_binary() -> PathBuf {
    if let Ok(path) = std::env::var("SPLICE_TEST_BIN") {
        return PathBuf::from(path);
    }

    if let Ok(path) = std::env::var("CARGO_BIN_EXE_splice") {
        return PathBuf::from(path);
    }

    let mut path = std::env::current_exe().unwrap();
    path.pop(); // deps
    let deps_dir = path.clone();
    path.pop(); // debug
    let bin_path = path.join("splice");

    if bin_path.exists() {
        return bin_path;
    }

    // Fallback: search deps directory for large binaries (>50MB = CLI, not test harness)
    // ... (full implementation in source)
}

fn extract_json_from_stdout(stdout: &str) -> String {
    let start = stdout.find('{');
    let end = stdout.rfind('}');

    match (start, end) {
        (Some(start), Some(end)) if end >= start => stdout[start..=end].to_string(),
        _ => String::new(),
    }
}

#[test]
fn test_cli_query_returns_json() {
    let temp_dir = TempDir::new().unwrap();
    let db_path = temp_dir.path().join("test.db");
    let test_file = temp_dir.path().join("lib.rs");

    std::fs::write(&test_file, "fn test() {}").unwrap();

    let mut integration = MagellanIntegration::open(&db_path).unwrap();
    integration.index_file(&test_file).unwrap();

    let splice_binary = get_splice_binary();
    let output = Command::new(&splice_binary)
        .arg("query")
        .arg("--db")
        .arg(&db_path)
        .arg("--label")
        .arg("rust")
        .arg("--json")
        .output()
        .expect("Failed to run splice CLI");

    assert!(output.status.success());

    let stdout = String::from_utf8_lossy(&output.stdout);
    let json_payload = extract_json_from_stdout(&stdout);
    let payload: Value = serde_json::from_str(&json_payload)
        .expect("stdout should be valid JSON");

    assert_eq!(payload["status"], "ok");
}
```

**Key insight:** The `get_splice_binary()` helper handles multiple binary discovery strategies (env var, cargo test bin, path lookup) and includes a 50MB size heuristic to distinguish the CLI binary from test harnesses.

### Pattern 2: TempDir Isolation

**What:** Create isolated temporary directories for each test to ensure no state leakage.

**When to use:** All tests creating files, databases, or executing CLI commands.

**Example:**
```rust
#[test]
fn test_export_command_creates_output_file() {
    let temp_dir = TempDir::new().unwrap();
    let db_path = temp_dir.path().join("test.db");
    let output_path = temp_dir.path().join("export.json");

    // Create and index test file
    let test_file = temp_dir.path().join("test.rs");
    std::fs::write(&test_file, "fn demo() {}").unwrap();

    let mut integration = MagellanIntegration::open(&db_path).unwrap();
    integration.index_file(&test_file).unwrap();

    // Run export command
    let splice_binary = get_splice_binary();
    let result = Command::new(&splice_binary)
        .arg("export")
        .arg("--db")
        .arg(&db_path)
        .arg("--format")
        .arg("json")
        .arg("--file")
        .arg(&output_path)
        .output();

    assert!(result.is_ok());
    let output = result.unwrap();
    assert!(output.status.success());

    // Verify output file exists and is valid JSON
    let json_content = std::fs::read_to_string(&output_path).unwrap();
    let _value: Value = serde_json::from_str(&json_content)
        .expect("export should produce valid JSON");
}
```

### Pattern 3: Exit Code Validation

**What:** Verify that CLI commands return correct exit codes for success and error conditions.

**When to use:** All CLI command error handling validation.

**Example:**
```rust
#[test]
fn test_cli_database_error_returns_exit_code_3() {
    let temp_dir = TempDir::new().unwrap();
    let nonexistent_db = temp_dir.path().join("nonexistent.db");

    let splice_binary = get_splice_binary();
    let output = Command::new(&splice_binary)
        .arg("status")
        .arg("--db")
        .arg(&nonexistent_db)
        .output()
        .expect("Failed to run splice CLI");

    assert!(!output.status.success());
    assert_eq!(output.status.code(), Some(3), "Should exit with code 3 for database errors");
}
```

### Pattern 4: Error Code JSON Validation

**What:** Verify that error responses include structured error codes in SPL-E### format.

**When to use:** Error handling validation, LLM consumption tests.

**Example:**
```rust
#[test]
fn test_cli_symbol_not_found_includes_error_code() {
    let temp_dir = TempDir::new().unwrap();
    let db_path = temp_dir.path().join("test.db");
    let test_file = temp_dir.path().join("test.rs");

    std::fs::write(&test_file, "fn existing() {}").unwrap();

    let mut integration = MagellanIntegration::open(&db_path).unwrap();
    integration.index_file(&test_file).unwrap();

    let splice_binary = get_splice_binary();
    let output = Command::new(&splice_binary)
        .arg("find")
        .arg("--db")
        .arg(&db_path)
        .arg("--name")
        .arg("nonexistent_function")
        .output()
        .expect("Failed to run splice CLI");

    assert!(!output.status.success());

    let stderr = String::from_utf8_lossy(&output.stderr);
    let payload: Value = serde_json::from_str(&stderr)
        .expect("stderr should be JSON");

    assert_eq!(payload["status"], "error");
    assert!(payload["error"]["error_code"].is_some());
    assert_eq!(payload["error"]["error_code"]["code"], "SPL-E001");
}
```

### Anti-Patterns to Avoid

- **Testing internal APIs directly:** Integration tests should spawn the binary, not call library functions
- **Hardcoding binary paths:** Use `get_splice_binary()` helper with environment variable fallback
- **Not cleaning temp directories:** Always use `TempDir` which auto-cleans on drop
- **Ignoring stderr:** Many errors write to stderr, validate both stdout and stderr
- **Assuming JSON order:** Use serde_json `Value` for field-based assertions, not string comparison

## Don't Hand-Roll

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| Temp directory management | Manual fs::create_dir + cleanup | `tempfile::TempDir` | Auto-cleanup on drop, cross-platform |
| JSON parsing in tests | Manual string search | `serde_json::from_str` | Field-based assertions, handles whitespace |
| Binary path discovery | Hardcoded paths | `get_splice_binary()` helper | Handles cargo test env, multiple locations |
| Test database setup | Manual SQL | `MagellanIntegration::open()` + `index_file()` | In-process fixture creation |

**Key insight:** The existing codebase has solved these problems—use the established utilities.

## Common Pitfalls

### Pitfall 1: Test Binary Not Built

**What goes wrong:** Integration tests fail because `splice` binary hasn't been built.

**Why it happens:** Running `cargo test` doesn't automatically build the binary crate.

**How to avoid:** Run `cargo build --bin splice` before integration tests, or use `cargo test --test cli_tests -- --test-threads=1` with build script.

**Warning signs:** Test fails with "No such file or directory" on binary path.

### Pitfall 2: Debug Output in JSON

**What goes wrong:** JSON extraction from stdout fails because SQLiteGraph debug output is mixed with JSON.

**Why it happens:** SQLiteGraph may write bracketed debug lines like `[SQLiteGraph] info` to stdout.

**How to avoid:** Use `extract_json_from_stdout()` helper which finds the first `{` and last `}`.

**Warning signs:** `serde_json::from_str` fails with "expected value" error.

### Pitfall 3: TempDir Cleanup Race

**What goes wrong:** Subprocess still holds file handle when TempDir drops.

**Why it happens:** Windows file locking or delayed subprocess cleanup.

**How to avoid:** Keep TempDir in scope until subprocess completes, use `drop(child)` before assertions.

**Warning signs:** Flaky tests that pass locally but fail in CI.

### Pitfall 4: Environment Variable Leaks

**What goes wrong:** Tests use environment variables from developer's machine.

**Why it happens:** `SPLICE_TEST_BIN` or `CARGO_BIN_EXE_splice` env vars point to stale binaries.

**How to avoid:** Tests should work without env vars; use them only for local testing speed.

**Warning signs:** Test passes for one developer but fails for another.

### Pitfall 5: Exit Code vs Success Confusion

**What goes wrong:** Using `output.status.success()` when specific exit code matters.

**Why it happens:** Success returns true for exit code 0 only; CLI uses 0-5 for different conditions.

**How to avoid:** Use `output.status.code()` and assert specific values per Magellan conventions.

**Warning signs:** Exit code 5 (validation error) test passes when it shouldn't.

## Code Examples

### Query Command Integration Test

```rust
#[test]
fn test_query_command_with_labels() {
    let temp_dir = TempDir::new().unwrap();
    let db_path = temp_dir.path().join("test.db");

    // Create multi-language test workspace
    let rust_file = temp_dir.path().join("lib.rs");
    std::fs::write(&rust_file, r#"
pub fn helper() {}
pub fn main() { helper(); }
"#).unwrap();

    let mut integration = MagellanIntegration::open(&db_path).unwrap();
    integration.index_file(&rust_file).unwrap();

    let splice_binary = get_splice_binary();
    let output = Command::new(&splice_binary)
        .arg("query")
        .arg("--db")
        .arg(&db_path)
        .arg("--label")
        .arg("rust")
        .arg("--label")
        .arg("fn")
        .arg("--json")
        .output()
        .expect("Failed to run splice query");

    assert!(output.status.success());

    let stdout = String::from_utf8_lossy(&output.stdout);
    let json_payload = extract_json_from_stdout(&stdout);
    let payload: Value = serde_json::from_str(&json_payload)
        .expect("stdout should be valid JSON");

    // Verify Magellan-compatible response structure
    let result = &payload["result"];
    assert!(result["symbols"].is_array());
    let symbols = result["symbols"].as_array().unwrap();
    assert!(!symbols.is_empty());

    // Verify Magellan field names (start_line, not line_start)
    let first_symbol = &symbols[0];
    assert!(first_symbol["start_line"].is_some());
}
```

### Export Format Tests

```rust
#[test]
fn test_export_json_format() {
    let temp_dir = TempDir::new().unwrap();
    let db_path = temp_dir.path().join("test.db");
    let output_path = temp_dir.path().join("export.json");

    let test_file = temp_dir.path().join("test.rs");
    std::fs::write(&test_file, "fn test() {}").unwrap();

    let mut integration = MagellanIntegration::open(&db_path).unwrap();
    integration.index_file(&test_file).unwrap();

    let splice_binary = get_splice_binary();
    let output = Command::new(&splice_binary)
        .arg("export")
        .arg("--db")
        .arg(&db_path)
        .arg("--format")
        .arg("json")
        .arg("--file")
        .arg(&output_path)
        .output()
        .expect("Failed to run splice export");

    assert!(output.status.success());

    let json_content = std::fs::read_to_string(&output_path).unwrap();
    let value: Value = serde_json::from_str(&json_content)
        .expect("export should produce valid JSON");

    // Verify required fields
    assert!(value["schema_version"].is_some());
    assert!(value["timestamp"].is_some());
    assert!(value["db_path"].is_some());
    assert!(value["data"]["files"].is_array());
    assert!(value["data"]["symbols"].is_array());
}

#[test]
fn test_export_jsonl_format() {
    let temp_dir = TempDir::new().unwrap();
    let db_path = temp_dir.path().join("test.db");
    let output_path = temp_dir.path().join("export.jsonl");

    let test_file = temp_dir.path().join("test.rs");
    std::fs::write(&test_file, "fn test() {}").unwrap();

    let mut integration = MagellanIntegration::open(&db_path).unwrap();
    integration.index_file(&test_file).unwrap();

    let splice_binary = get_splice_binary();
    let output = Command::new(&splice_binary)
        .arg("export")
        .arg("--db")
        .arg(&db_path)
        .arg("--format")
        .arg("jsonl")
        .arg("--file")
        .arg(&output_path)
        .output()
        .expect("Failed to run splice export");

    assert!(output.status.success());

    let jsonl_content = std::fs::read_to_string(&output_path).unwrap();

    // JSONL: one JSON object per line
    for line in jsonl_content.lines() {
        let value: Value = serde_json::from_str(line)
            .expect("each line should be valid JSON");

        if let Some(obj) = value.as_object() {
            if let Some(record_type) = obj.get("type") {
                let type_str = record_type.as_str().unwrap();
                assert!(
                    type_str == "header" || type_str == "file" || type_str == "symbol",
                    "type should be header, file, or symbol"
                );
            }
        }
    }
}

#[test]
fn test_export_csv_format() {
    let temp_dir = TempDir::new().unwrap();
    let db_path = temp_dir.path().join("test.db");
    let output_path = temp_dir.path().join("export.csv");

    let test_file = temp_dir.path().join("test.rs");
    std::fs::write(&test_file, "fn test() {}").unwrap();

    let mut integration = MagellanIntegration::open(&db_path).unwrap();
    integration.index_file(&test_file).unwrap();

    let splice_binary = get_splice_binary();
    let output = Command::new(&splice_binary)
        .arg("export")
        .arg("--db")
        .arg(&db_path)
        .arg("--format")
        .arg("csv")
        .arg("--file")
        .arg(&output_path)
        .output()
        .expect("Failed to run splice export");

    assert!(output.status.success());

    let csv_content = std::fs::read_to_string(&output_path).unwrap();

    // CSV should have section headers
    assert!(csv_content.contains("# Files"));
    assert!(csv_content.contains("# Symbols"));
    assert!(csv_content.contains("path"));
    assert!(csv_content.contains("hash"));
}
```

### LLM Consumption Workflow Test

```rust
#[test]
fn test_llm_single_tool_workflow_discovery() {
    let temp_dir = TempDir::new().unwrap();
    let db_path = temp_dir.path().join("test.db");

    let test_file = temp_dir.path().join("calculator.rs");
    std::fs::write(&test_file, r#"
pub fn add(a: i32, b: i32) -> i32 { a + b }
pub fn multiply(x: i32, y: i32) -> i32 { x * y }
pub fn main() {
    let result = add(5, multiply(3, 4));
    println!("{}", result);
}
"#).unwrap();

    let mut integration = MagellanIntegration::open(&db_path).unwrap();
    integration.index_file(&test_file).unwrap();

    let splice_binary = get_splice_binary();

    // Step 1: Discover functions via query
    let query_output = Command::new(&splice_binary)
        .arg("query")
        .arg("--db")
        .arg(&db_path)
        .arg("--label")
        .arg("rust")
        .arg("--label")
        .arg("fn")
        .arg("--json")
        .output()
        .expect("Failed to run splice query");

    assert!(query_output.status.success());

    let query_json = extract_json_from_stdout(
        &String::from_utf8_lossy(&query_output.stdout)
    );
    let query_payload: Value = serde_json::from_str(&query_json)
        .expect("query response should be valid JSON");

    let symbols = &query_payload["result"]["symbols"];
    let functions: Vec<&str> = symbols.as_array()
        .unwrap()
        .iter()
        .filter_map(|s| s["name"].as_str())
        .collect();

    assert!(functions.contains(&"add"));
    assert!(functions.contains(&"multiply"));

    // Step 2: Get details for specific function
    let get_output = Command::new(&splice_binary)
        .arg("find")
        .arg("--db")
        .arg(&db_path)
        .arg("--name")
        .arg("add")
        .arg("--output")
        .arg("json")
        .output()
        .expect("Failed to run splice find");

    assert!(get_output.status.success());

    let find_json = extract_json_from_stdout(
        &String::from_utf8_lossy(&get_output.stdout)
    );
    let find_payload: Value = serde_json::from_str(&find_json)
        .expect("find response should be valid JSON");

    assert_eq!(find_payload["result"]["symbols"][0]["name"], "add");
}
```

### Error Code Mapping Test

```rust
#[test]
fn test_magellan_error_maps_to_spl_e091() {
    let temp_dir = TempDir::new().unwrap();
    let nonexistent_db = temp_dir.path().join("does_not_exist.db");

    let splice_binary = get_splice_binary();
    let output = Command::new(&splice_binary)
        .arg("status")
        .arg("--db")
        .arg(&nonexistent_db)
        .output()
        .expect("Failed to run splice status");

    assert!(!output.status.success());

    let stderr = String::from_utf8_lossy(&output.stderr);
    let payload: Value = serde_json::from_str(&stderr)
        .expect("error response should be valid JSON");

    // Should have Magellan error code SPL-E091
    if let Some(error_code) = payload["error"]["error_code"].as_object() {
        assert_eq!(error_code["code"], "SPL-E091");
        assert_eq!(error_code["severity"], "error");
    }
}
```

## State of the Art

| Old Approach | Current Approach | When Changed | Impact |
|--------------|------------------|--------------|--------|
| No CLI integration tests | 54 test files, subprocess testing | Throughout v2.0 development | Established patterns to extend |
| No export validation | cli_output_tests.rs with JSON/JSONL/CSV | Phase 25 | Export format testing patterns exist |
| No error code testing | llm_consumption_tests.rs with error codes | v2.1 | Error code validation patterns exist |

**Current capabilities:**
- CLI subprocess testing via `get_splice_binary()`
- JSON extraction from stdout via `extract_json_from_stdout()`
- TempDir isolation for clean test environments
- MagellanIntegration for in-process fixture databases
- LLM consumption JSON validation (16 tests)
- Export format validation (JSON, JSONL, CSV)

## Open Questions

1. **Performance benchmarks thresholds**
   - What we know: Performance testing is a success criterion
   - What's unclear: Acceptable query latency limits, expected symbol throughput
   - Recommendation: Focus on correctness first, add benchmarking as follow-up; use `criterion` crate if needed

2. **Single-tool workflow definition**
   - What we know: LLM consumption tests validate single-tool workflow
   - What's unclear: Exact workflow steps to validate
   - Recommendation: Use existing `llm_consumption_tests.rs` pattern; test query -> find -> refs sequence

## Sources

### Primary (HIGH confidence)

**Codebase analysis (verified directly):**
- `/home/feanor/Projects/splice/tests/cli_tests.rs` — 1,700+ LOC of CLI subprocess testing patterns
- `/home/feanor/Projects/splice/tests/cli_output_tests.rs` — Export format tests (JSON/JSONL/CSV)
- `/home/feanor/Projects/splice/tests/magellan_integration_tests.rs` — Magellan wrapper tests with TempDir fixtures
- `/home/feanor/Projects/splice/tests/llm_consumption_tests.rs` — 16 LLM JSON validation tests
- `/home/feanor/Projects/splice/src/main.rs` — CLI entry point with exit code mapping
- `/home/feanor/Projects/splice/src/cli/mod.rs` — CLI command definitions and response types
- `/home/feanor/Projects/splice/src/error_codes.rs` — SPL-E### error code registry
- `/home/feanor/Projects/splice/src/output.rs` — Magellan-compatible response types

**Project documentation:**
- `/home/feanor/Projects/splice/.planning/phases/22-symbol-id-and-format-foundation/22-RESEARCH.md` — Integration testing patterns reference
- `/home/feanor/Projects/splice/.planning/ROADMAP.md` — Phase 26 specification

### Secondary (MEDIUM confidence)

**Rust testing patterns (verified knowledge):**
- `std::process::Command` — Standard Rust subprocess spawning
- `tempfile::TempDir` — Standard for temporary directory management in tests
- Integration test organization in `tests/` directory

## Metadata

**Confidence breakdown:**
- Standard stack: HIGH — All dependencies verified in Cargo.toml
- Architecture: HIGH — Test patterns verified in existing codebase
- Pitfalls: HIGH — Based on observed test failures and CI patterns
- Performance thresholds: LOW — Not specified, requires benchmarking setup

**Research date:** 2026-01-24
**Valid until:** 30 days (stable domain—CLI patterns and test infrastructure don't change frequently)
