# Phase 17: Integration & Testing - Research

**Researched:** 2026-01-22
**Domain:** Multi-language refactoring tool integration testing
**Confidence:** MEDIUM

## Summary

Phase 17 focuses on comprehensive integration testing of all v2.2 features across 7 supported languages (Rust, Python, C, C++, Java, JavaScript, TypeScript). The codebase already has substantial test infrastructure (41 test files, ~350 tests) with well-established patterns for unit tests, integration tests, performance tests, and CLI tests. This phase needs to ensure all 334+ existing tests pass with the new JSON schema, add coverage for rich span extensions, verify performance on large files and codebases, test cross-tool compatibility, and validate LLM consumption patterns.

**Primary recommendation:** Use existing test infrastructure (TestGraphBuilder, tempfile fixtures, CLI subprocess testing) and extend with targeted performance tests and LLM consumption validators. Follow the established pattern of language-agnostic tests with language-specific fixtures.

## Standard Stack

### Core Testing Libraries
| Library | Version | Purpose | Why Standard |
|---------|---------|---------|--------------|
| `tempfile` | 3.10 | Temporary directories/files for isolated tests | Already in dependencies, industry standard |
| `std::process::Command` | builtin | CLI subprocess testing | No external dependency needed, already used in cli_tests.rs |
| `serde_json` | 1.0 | JSON schema validation and parsing | Already in dependencies for JSON output |
| `sha2` | 0.10 | Checksum validation for file integrity tests | Already in dependencies for checksum_before field |

### Test Infrastructure (Existing)
| Component | Purpose | When to Use |
|-----------|---------|-------------|
| `TestGraphBuilder` (tests/relationship_performance.rs) | Create test code graphs with configurable sizes (50, 200, 1000 symbols) | Performance tests, relationship query tests |
| `create_temp_magellan_db()` (tests/magellan_integration_tests.rs) | Isolated Magellan database testing | Cross-tool alignment tests |
| `extract_json_from_stdout()` (tests/cli_tests.rs) | Parse JSON from CLI output mixed with debug logs | LLM consumption tests, CLI tests |
| Language-specific fixtures | Pre-built source code samples for each language | Cross-language feature testing |

### Performance Testing
| Approach | Library | Purpose |
|----------|---------|---------|
| `std::time::Instant` | builtin | Microbenchmarking context extraction and queries |
| Manual file size fixtures | None | Testing with files >32KB for context extraction |
| TestGraphBuilder scaling | Custom | 50/200/1000 symbol graphs for relationship queries |

### Alternatives Considered
| Instead of | Could Use | Tradeoff |
|------------|-----------|----------|
| Manual `std::time::Instant` | `criterion` benchmarking framework | Criterion adds complexity and dependency for what can be simple timing assertions. Manual `Instant::now()` is sufficient for "< 100ms" assertions. |
| Subprocess CLI testing | `assert_cmd` crate | `assert_cmd` provides nicer API but adds dependency. Current `std::process::Command` approach works and is already established. |
| Manual JSON validation | `json_schema` crate | JSON schema validation would be thorough but adds dependency. Manual field checks with `serde_json::Value` are sufficient for this codebase. |

## Architecture Patterns

### Recommended Test Organization
```
tests/
├── integration_v2_tests.rs         # New: Comprehensive v2.2 integration tests
├── performance_context_tests.rs    # New: Context extraction performance (>32KB files)
├── performance_relationship_tests.rs # New: Relationship query scaling (1K symbols)
├── magellan_alignment_tests.rs     # New: Cross-tool format compatibility
├── llm_consumption_tests.rs        # New: JSON field structure validation
├── cross_language_rich_span_tests.rs # New: Rich span across all 7 languages
├── existing_tests/                 # Existing: 41 test files continue unchanged
```

### Pattern 1: Language-Agnostic Test with Language Fixtures
**What:** Write test logic once, parameterize with language-specific source fixtures
**When to use:** Testing features that work identically across all languages (context extraction, expansion, checksums)
**Example:**
```rust
// Source: tests/context_expansion_integration_tests.rs
fn test_context_with_expand_all_languages() {
    let test_cases = vec![
        ("rust", RUST_FUNCTION_WITH_CONTEXT, Language::Rust),
        ("python", PYTHON_CLASS_WITH_CONTEXT, Language::Python),
        ("typescript", TYPESCRIPT_INTERFACE_WITH_CONTEXT, Language::TypeScript),
        // ... all 7 languages
    ];

    for (lang_name, content, lang) in test_cases {
        let dir = TempDir::new().unwrap();
        let file = create_test_file(&dir, &format!("test.{}", lang_name), content);

        let offset = find_symbol_offset(&file, content);
        let (expanded_start, expanded_end) =
            expand_to_body_with_docs(&file, offset, lang).unwrap();

        let ctx = extract_context_asymmetric(&file, expanded_start, expanded_end, 1, 1).unwrap();

        // Verify context works for this language
        assert!(!ctx.before.is_empty() || !ctx.after.is_empty(),
                "Context extraction failed for {}", lang_name);
    }
}
```

### Pattern 2: TestGraphBuilder for Scalable Performance Tests
**What:** Use existing TestGraphBuilder pattern to create graphs with specific symbol counts
**When to use:** Performance testing relationship queries, testing with large codebases
**Example:**
```rust
// Source: tests/relationship_performance.rs (adapted)
fn test_context_extraction_large_file() {
    // Create large file (>32KB threshold)
    let mut builder = TestGraphBuilder::new().expect("Failed to create builder");

    // Create 100 symbols (file will be >32KB)
    let file_path = builder.create_file_with_symbols(0, 100)
        .expect("Failed to create large file");

    let source = std::fs::read_to_string(&file_path).unwrap();
    assert!(source.len() > 32_000, "Test file should exceed 32KB threshold");

    // Test context extraction performance
    let start = Instant::now();
    let ctx = extract_context(&file_path, 1000, 2000, 3).unwrap();
    let duration = start.elapsed();

    // Should complete in reasonable time
    assert!(duration.as_millis() < 100,
            "Context extraction on large file took {}ms, expected < 100ms",
            duration.as_millis());
}
```

### Pattern 3: LLM Consumption Validation
**What:** Verify JSON structure meets LLM requirements (nested objects, type consistency, no null pollution)
**When to use:** Validating output format for agent consumption
**Example:**
```rust
fn test_llm_json_structure() {
    let workspace_dir = TempDir::new().expect("Failed to create temp workspace");
    // ... create test file, run CLI ...

    let output = Command::new(&splice_binary)
        .arg("query")
        .arg("--db")
        .arg(&db_path)
        .arg("--label")
        .arg("rust")
        .output()
        .expect("Failed to run splice CLI");

    let json_output = extract_json_from_stdout(&String::from_utf8_lossy(&output.stdout));
    let payload: Value = serde_json::from_str(&json_output)
        .expect("Output should be valid JSON");

    // LLM consumption requirements:
    // 1. Top-level has "status" field
    assert!(payload.get("status").is_some(), "Missing status field");

    // 2. Optional fields use skip_serializing_if (not present when None)
    let results = payload.get("results").and_then(|v| v.as_array())
        .expect("Should have results array");

    if !results.is_empty() {
        let first = &results[0];

        // 3. Rich span fields are properly structured
        if let Some(context) = first.get("context") {
            assert!(context.is_object(), "context should be object, not null");
            let ctx_obj = context.as_object().unwrap();
            assert!(ctx_obj.contains_key("before"), "context has 'before' array");
            assert!(ctx_obj.contains_key("selected"), "context has 'selected' array");
            assert!(ctx_obj.contains_key("after"), "context has 'after' array");

            // Arrays should not be null (use empty arrays instead)
            if let Some(before) = ctx_obj.get("before").and_then(|v| v.as_array()) {
                // OK - array is present (may be empty)
            } else {
                panic!("context.before should be array, not null or wrong type");
            }
        }

        // 4. Error codes have required fields
        if let Some(error_code) = first.get("error_code") {
            assert!(error_code.is_object(), "error_code should be object");
            let ec = error_code.as_object().unwrap();
            assert!(ec.contains_key("code"), "error_code has 'code' field");
            assert!(ec.contains_key("severity"), "error_code has 'severity' field");
            assert!(ec.contains_key("location"), "error_code has 'location' field");
            assert!(ec.contains_key("hint"), "error_code has 'hint' field");
        }
    }
}
```

### Pattern 4: CLI Subprocess Testing with Exit Code Validation
**What:** Test CLI via subprocess to validate exit codes, stdout/stderr
**When to use:** End-to-end CLI testing, error code validation
**Example:**
```rust
// Source: tests/cli_tests.rs (adapted)
fn test_cli_structured_error_output() {
    let workspace_dir = TempDir::new().expect("Failed to create temp workspace");
    // ... create workspace with symbol ...

    let splice_binary = get_splice_binary();
    let output = Command::new(&splice_binary)
        .arg("patch")
        .arg("--file")
        .arg(&lib_rs_path)
        .arg("--symbol")
        .arg("missing_symbol")
        .arg("--with")
        .arg(&patch_path)
        .current_dir(workspace_path)
        .output()
        .expect("Failed to run splice CLI");

    // Exit code validation
    assert!(!output.status.success(),
            "CLI should fail when symbol cannot be resolved");

    // Structured error JSON in stderr
    let stderr = String::from_utf8_lossy(&output.stderr);
    let payload: Value = serde_json::from_str(&stderr)
        .expect("stderr should contain JSON payload");

    assert_eq!(payload.get("status").and_then(|v| v.as_str()),
               Some("error"),
               "status should be error");

    let error = payload.get("error")
        .and_then(|v| v.as_object())
        .expect("error object missing");

    assert_eq!(error.get("kind").and_then(|v| v.as_str()),
               Some("SymbolNotFound"),
               "kind should be SymbolNotFound");
}
```

### Anti-Patterns to Avoid
- **Hard-coded absolute paths:** Use `TempDir` and workspace-relative paths for test isolation
- **Testing implementation details:** Test observable behavior (CLI output, file changes), not internal function calls
- **Ignoring language-specific quirks:** Each language has different parsing behavior - test with real language fixtures, not generic code
- **Assuming test execution order:** Tests must pass individually and in any order - use `TempDir` for isolation
- **Sleeping for timing:** Don't use `std::thread::sleep()` for synchronization - use synchronous APIs or proper async waiting

## Don't Hand-Roll

Problems that look simple but have existing solutions:

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| Test file isolation | Manual temp file cleanup | `tempfile::TempDir` | Automatic cleanup on drop, cross-platform, handles edge cases |
| JSON parsing in tests | Custom string manipulation | `serde_json::from_str::<Value>()` | Handles whitespace, escapes, nested structures correctly |
| CLI output parsing | Manual string splitting | `extract_json_from_stdout()` pattern | Already filters debug logs, handles multi-line JSON |
| Process spawning | Manual fork/exec | `std::process::Command` | Cross-platform, handles stdout/stderr/stdin, exit codes |
| Temporary workspace setup | Manual directory creation | `TempDir` + `write!()` | Idiomatic Rust, automatic cleanup, clearer intent |

**Key insight:** The test infrastructure already exists. Don't invent new patterns - extend what works (TestGraphBuilder, tempfile fixtures, subprocess CLI testing).

## Common Pitfalls

### Pitfall 1: Test Database Leaks
**What goes wrong:** Tests create databases in `/tmp` but don't clean up, filling disk
**Why it happens:** Forgetting to use `TempDir`, or keeping references to temp paths after drop
**How to avoid:**
```rust
// CORRECT: TempDir auto-cleans when dropped
let temp_dir = TempDir::new().expect("Failed to create temp dir");
let db_path = temp_dir.path().join("test.db");
// ... use db_path ...
// TempDir automatically cleans up when it goes out of scope

// WRONG: Manual paths leak
let db_path = std::env::temp_dir().join("my-test.db");
// ... must manually remove later ...
```
**Warning signs:** Disk full errors, "database locked" errors in parallel tests

### Pitfall 2: Platform-Specific Path Handling
**What goes wrong:** Tests pass on Linux but fail on Windows due to path separators
**Why it happens:** Hard-coding `/` separators or using string concatenation
**How to avoid:**
```rust
// CORRECT: Use PathBuf methods
let file_path = workspace_dir.join("src").join("lib.rs");

// WRONG: String concatenation
let file_path = format!("{}/src/lib.rs", workspace_dir_str);
```
**Warning signs:** Tests fail on CI (different OS), path-related assertion errors

### Pitfall 3: Flaky Performance Tests
**What goes wrong:** Performance tests intermittently fail due to system load
**Why it happens:** Asserting exact timing instead of reasonable ranges
**How to avoid:**
```rust
// CORRECT: Use generous thresholds
assert!(duration.as_millis() < 500,
        "Query took {}ms, expected < 500ms", duration.as_millis());

// WRONG: Exact timing
assert_eq!(duration.as_millis(), 42);  // Will flake
```
**Warning signs:** Intermittent CI failures, tests pass locally but fail on CI

### Pitfall 4: JSON Parsing of Mixed Output
**What goes wrong:** CLI tests fail because stdout contains debug logs mixed with JSON
**Why it happens:** SQLiteGraph logs to stdout, tests assume pure JSON output
**How to avoid:** Use the existing `extract_json_from_stdout()` helper from cli_tests.rs
```rust
// From tests/cli_tests.rs:22
fn extract_json_from_stdout(stdout: &str) -> String {
    let json_lines: Vec<&str> = stdout
        .lines()
        .filter(|line| {
            let trimmed = line.trim();
            !trimmed.is_empty()
                && !trimmed.starts_with('[')
                && !trimmed.starts_with("DEBUG:")
                && trimmed.starts_with('{')
        })
        .collect();
    json_lines.join("\n")
}
```
**Warning signs:** JSON parse errors in tests, "expected '{' but found '['"

### Pitfall 5: Not Testing Backward Compatibility
**What goes wrong:** New JSON schema breaks existing consumers
**Why it happens:** Only testing new fields, not that old JSON still parses
**How to avoid:**
```rust
// From tests/rich_span_tests.rs:65
#[test]
fn test_backward_compatibility_old_json() {
    let old_json = r#"{
        "file_path": "src/main.rs",
        "byte_start": 0,
        "byte_end": 10,
        // No new fields
    }"#;

    let span: SpanResult = serde_json::from_str(old_json).unwrap();
    assert_eq!(span.file_path, "src/main.rs");

    // New fields should be None
    assert!(span.context.is_none());
    assert!(span.semantic_kind.is_none());
    assert!(span.checksum_before.is_none());
}
```
**Warning signs:** Version bump breaks external tools, LLM agents fail on new format

## Code Examples

Verified patterns from existing tests:

### Creating Large File Fixtures (>32KB)
```rust
// Generate source code with repeated function definitions
fn create_large_rust_file(dir: &Path, num_functions: usize) -> PathBuf {
    let file_path = dir.join("large.rs");
    let mut file = std::fs::File::create(&file_path).unwrap();

    for i in 0..num_functions {
        writeln!(file, r#"
/// Documentation for function {}
pub fn function_{}(x: i32) -> i32 {{
    x + {}
}}
"#, i, i, i).unwrap();
    }

    file_path
}

// Use in performance test
#[test]
fn test_context_extraction_32kb_file() {
    let dir = TempDir::new().unwrap();
    let file = create_large_rust_file(dir.path(), 200);  // ~40KB

    let file_size = std::fs::metadata(&file).unwrap().len();
    assert!(file_size > 32_000, "Test file should exceed 32KB");

    // Test context extraction performance
    let source = std::fs::read_to_string(&file).unwrap();
    let offset = source.find("function_100").unwrap();

    let start = Instant::now();
    let (expanded_start, expanded_end) =
        expand_to_body_with_docs(&file, offset, Language::Rust).unwrap();
    let ctx = extract_context(&file, expanded_start, expanded_end, 3).unwrap();
    let duration = start.elapsed();

    assert!(duration.as_millis() < 100, "Context extraction should be fast on large files");
}
```

### Cross-Language Rich Span Testing
```rust
// Parameterized test for all 7 languages
#[test]
fn test_rich_span_fields_all_languages() {
    let test_cases = vec![
        ("test.rs", r#"pub fn test() { 0 }"#, Language::Rust, "function"),
        ("test.py", r#"def test(): pass"#, Language::Python, "function"),
        ("test.ts", r#"function test() { return 0; }"#, Language::TypeScript, "function"),
        ("test.js", r#"function test() { return 0; }"#, Language::JavaScript, "function"),
        ("test.java", r#"void test() {}"#, Language::Java, "function"),
        ("test.c", r#"void test() {}"#, Language::C, "function"),
        ("test.cpp", r#"void test() {}"#, Language::Cpp, "function"),
    ];

    for (filename, content, lang, expected_kind) in test_cases {
        let dir = TempDir::new().unwrap();
        let file_path = dir.path().join(filename);
        std::fs::write(&file_path, content).unwrap();

        let mut span = SpanResult::from_byte_span(
            file_path.to_string_lossy().to_string(),
            0,
            content.len()
        );

        // Test language detection
        span = span.with_language(lang.as_str());
        assert_eq!(span.language, Some(lang.as_str().to_string()),
                   "Language detection failed for {}", filename);

        // Test semantic kind
        span = span.with_semantic_kind(expected_kind);
        assert_eq!(span.semantic_kind, Some(expected_kind.to_string()),
                   "Semantic kind mismatch for {}", filename);

        // Test context extraction
        let ctx = extract_context(&file_path, 0, content.len(), 1).unwrap();
        span = span.with_context(ctx);
        assert!(span.context.is_some(),
                "Context extraction failed for {}", filename);

        // Test checksums
        let span_checksum = checksum_span(&file_path, 0, content.len()).unwrap();
        let file_checksum = checksum_file(&file_path).unwrap();
        span = span.with_both_checksums(span_checksum.as_hex(), file_checksum.as_hex());
        assert!(span.checksum_before.is_some(),
                "Span checksum missing for {}", filename);
        assert!(span.file_checksum_before.is_some(),
                "File checksum missing for {}", filename);
    }
}
```

### Relationship Query Performance with 1K Graph
```rust
// Adapted from tests/relationship_performance.rs:273
#[test]
fn test_get_callers_1k_graph_performance() {
    let (graph, temp_dir) = large_graph();  // 1000 symbols, 100 files

    let file_path = temp_dir.path().join("test_file_0.rs");
    let file_path_str = file_path.to_str().expect("Invalid path");
    let node_id = graph
        .find_symbol_in_file(file_path_str, "function_0_0")
        .expect("Symbol not found");

    let mut cache = RelationshipCache::new();
    let start = Instant::now();

    let result = get_callers(&graph, node_id, &mut cache);

    let duration = start.elapsed();

    assert!(result.is_ok(), "get_callers failed: {:?}", result.err());
    assert!(duration.as_millis() < 100,
            "get_callers on 1K graph took {}ms, expected < 100ms",
            duration.as_millis());
}
```

## State of the Art

| Old Approach | Current Approach | When Changed | Impact |
|--------------|------------------|--------------|--------|
| Unit tests only | Integration + Performance + CLI tests | Phase 1-10 (v2.0) | Comprehensive coverage across 7 languages |
| Hard-coded test data | Parameterized language fixtures | Phase 11-16 | Test all languages with same logic |
| No performance validation | TestGraphBuilder with 50/200/1000 symbols | Phase 12 | Validate scaling behavior |
| Manual JSON parsing | serde_json with structured validation | Phase 11 | Type-safe JSON testing |
| No LLM consumption testing | New: llm_consumption_tests.rs | Phase 17 | Validate AI agent requirements |

**Deprecated/outdated:**
- `store_symbol()` deprecated - Use `store_symbol_with_file_and_language()` for multi-language support
- Manual test database cleanup - Use `TempDir` for automatic cleanup
- Testing implementation details - Test observable behavior instead

## Open Questions

1. **What constitutes "LLM consumption testing"?**
   - What we know: JSON structure validation, nested object presence, type consistency, optional field handling
   - What's unclear: Should we simulate actual LLM ingestion, or just validate JSON schema?
   - Recommendation: Validate JSON schema and structure only. Don't test with actual LLMs - that's out of scope for a refactoring tool. Test that JSON is parseable, has correct types, and includes required fields for agent consumption.

2. **Should we test Magellan compatibility with real Magellan databases?**
   - What we know: MagellanIntegration wraps Magellan v0.5.3, we create test databases
   - What's unclear: Should we test compatibility with externally-created Magellan databases?
   - Recommendation: Test with internally-created test databases only. External database compatibility is Magellan's responsibility, not Splice's. Our tests verify we correctly use the Magellan API.

3. **What is the actual >32KB threshold for context extraction performance?**
   - What we know: Success criteria mentions "large files (>32KB)"
   - What's unclear: Is 32KB a hard threshold or just an example?
   - Recommendation: Treat 32KB as minimum. Test with 32KB, 64KB, 128KB files to verify performance scales. Use 100 functions (~40KB) as standard large file fixture.

4. **How comprehensive should cross-tool alignment testing be?**
   - What we know: MagellanIntegration exists, we can query by labels, get code chunks
   - What's unclear: Are we testing Magellan integration, or full interoperability with other tools?
   - Recommendation: Test MagellanIntegration API surface only (index_file, query_by_labels, get_code_chunk). Don't test external tool compatibility - that's integration testing beyond Splice's scope.

## Sources

### Primary (HIGH confidence)
- **Existing test infrastructure** - Verified patterns from:
  - `/home/feanor/Projects/splice/tests/relationship_performance.rs` - TestGraphBuilder pattern
  - `/home/feanor/Projects/splice/tests/magellan_integration_tests.rs` - MagellanIntegration testing
  - `/home/feanor/Projects/splice/tests/cli_tests.rs` - Subprocess CLI testing, JSON extraction
  - `/home/feanor/Projects/splice/tests/rich_span_tests.rs` - Rich span field validation
  - `/home/feanor/Projects/splice/tests/context_expansion_integration_tests.rs` - Context + expansion integration
  - `/home/feanor/Projects/splice/tests/integration_refactor.rs` - Round-trip integration testing
  - `/home/feanor/Projects/splice/tests/error_integration_tests.rs` - Error code validation

- **Cargo.toml dependencies** - Verified available testing libraries:
  - tempfile 3.10 - Temporary file/directory creation
  - serde_json 1.0 - JSON parsing and validation
  - sha2 0.10 - Checksum validation
  - No additional benchmarking libraries (use std::time::Instant)

### Secondary (MEDIUM confidence)
- **Phase requirements** - From `.planning/phases/17-integration-and-testing`:
  - TEST-01 through TEST-06 define integration and testing requirements
  - CLI-01 through CLI-33 must be satisfied
  - RICHSPAN-01 through RICHSPAN-21 must be tested

- **ROADMAP.md** - Phase 17 context:
  - All 334+ existing tests must pass with new JSON schema
  - New tests verify rich span extensions across 7 languages
  - Performance tests for context extraction (>32KB files)
  - Performance tests for relationship queries (>10K LOC, adjusted to 1K symbols)
  - Cross-tool alignment tests for Magellan format
  - LLM consumption tests for JSON structure

### Tertiary (LOW confidence)
- **Web search attempted** - Rate limit reached, unable to research:
  - Integration testing patterns for multi-language refactoring tools
  - Rust performance testing best practices for 2026
  - LLM JSON consumption testing approaches
- **Mitigation:** Rely on existing codebase patterns and established Rust testing conventions. The existing test infrastructure (41 test files, 350+ tests) demonstrates proven patterns for this domain.

## Metadata

**Confidence breakdown:**
- Standard stack: HIGH - Based on verified Cargo.toml dependencies and existing test code
- Architecture: HIGH - Patterns verified from existing test files in the codebase
- Pitfalls: HIGH - All pitfalls observed in actual test code or documented in existing tests
- Performance testing: MEDIUM - TestGraphBuilder pattern exists, but large file testing (>32KB) needs new fixtures
- LLM consumption: LOW - No existing tests, defining scope based on requirements interpretation

**Research date:** 2026-01-22
**Valid until:** 2026-02-22 (30 days - testing infrastructure is stable, but new patterns may emerge as Phase 17 progresses)
