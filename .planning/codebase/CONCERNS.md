# Codebase Concerns

**Analysis Date:** 2026-01-23

## Tech Debt

**Incomplete Ingestor Implementation:**
- Issue: `Ingestor::ingest_file` and `Ingestor::ingest_dir` return hardcoded "Not implemented yet" errors
- Files: `src/ingest/mod.rs:55-67`
- Impact: High-level orchestration for file ingestion is non-functional despite having language-specific parsers
- Fix approach: Wire up existing language-specific extractors (`extract_rust_symbols`, `extract_python_symbols`, etc.) to the ingestor methods

**Deprecated API Usage:**
- Issue: `store_symbol_with_file` method is deprecated but still used in tests
- Files: `src/graph/mod.rs:443`, `src/graph/mod.rs:87-93`
- Impact: Clippy warnings, potential future breakage
- Fix approach: Replace all `store_symbol_with_file` calls with `store_symbol_with_file_and_language`

**Unimplemented Cross-File Resolution:**
- Issue: Python and C/C++ import resolution in `CrossFileResolver` returns `None` with TODO comments
- Files: `src/resolve/cross_file.rs:176-200`
- Impact: Cross-file reference finding only works for Rust/Java/JavaScript/TypeScript
- Fix approach: Implement Python-specific path resolution and C/C++ include resolution

**Lines Added/Removed Not Calculated:**
- Issue: Delete and patch operations report 0 for `lines_added` and `lines_removed` (TODO comments)
- Files: `src/main.rs:886`, `src/main.rs:1382-1383`
- Impact: Operation metrics are incomplete; users can't see line count changes
- Fix approach: Implement diff calculation using ropey or similar text diff library

**Stubbed Relationship Queries:**
- Issue: `get_imports` and `get_exports` return empty vectors with TODO comments
- Files: `src/relationships/mod.rs:455-509`
- Impact: Import/export relationship queries are non-functional despite being exposed in CLI
- Fix approach: Implement File->Symbol DEFINES edge traversal once Magellan provides that API

**Incomplete Strict/Skip Flag Wiring:**
- Issue: CLI flags for strict mode and skip mode are hardcoded (`strict = false`, `skip = true`)
- Files: `src/patch/mod.rs:152`
- Impact: Users cannot control verification strictness via CLI
- Fix approach: Wire `--strict` and `--skip` flags from `Cli` struct to validation calls

**Missing Disk Space Checking:**
- Issue: `get_disk_space` returns hardcoded 1TB instead of actually checking filesystem
- Files: `src/verify.rs:231-238`
- Impact: No real disk space validation before operations; insufficient disk space causes failures mid-operation
- Fix approach: Use `sysinfo` crate or similar to query actual filesystem statistics

## Known Bugs

**Line/Column Calculation Approximation:**
- Symptoms: Line and column calculations in `execute_patch` are documented as "approximate since span may have changed size"
- Files: `src/main.rs:1225`
- Trigger: Any patch operation that changes span size
- Workaround: None; accepts approximation
- Note: This is documented as current behavior, not a bug per se, but impacts accuracy

## Security Considerations

**Unsafe Code Blocks:**
- Risk: One explicit `panic!` macro in test code could cause crashes in production if test code path is reached
- Files: `src/patch/backup.rs:363`
- Current mitigation: Only present in test module `#[cfg(test)]`
- Recommendations: Replace `panic!` with `assert!` or `expect!` for proper test failures

**Extensive unwrap() Usage:**
- Risk: ~26 `unwrap()` calls across import modules (`src/ingest/imports/python.rs`, `src/ingest/imports/cpp.rs`, etc.) that could panic on unexpected None
- Files: Multiple import extraction modules
- Current mitigation: None; relies on tree-sitter parser always returning expected structure
- Recommendations: Replace `unwrap()` with `?` operator for proper error propagation

## Performance Bottlenecks

**Large Source Files:**
- Problem: `src/main.rs` is 3,668 lines (exceeds 300 LOC guideline)
- Files: `src/main.rs`, `src/patch/pattern.rs` (1,532 lines), `src/resolve/references/rust.rs` (1,395 lines)
- Cause: Monolithic entry point handling all CLI commands and operations
- Improvement path: Split `main.rs` into command-specific modules (delete.rs, patch.rs, query.rs, etc.)

**Excessive Cloning:**
- Problem: 786 `.clone()` calls detected across codebase; potential performance impact
- Files: Across 45+ source files using HashMap, Vec, String
- Cause: Heavy use of `clone()` for HashMap values, Vec elements, and String fields
- Improvement path: Use references (`&T`) where possible, implement `Copy` for small structs, consider `Arc<T>` for shared ownership

**Unused Variables:**
- Problem: Multiple compilation warnings for unused variables (`block_span`, `file`, `output` etc.)
- Files: `src/expand/tree_walker.rs:888`, `src/execution/base.rs:384`, many test files
- Cause: Incomplete refactoring or dead code paths
- Improvement path: Remove unused variables or prefix with underscore (`_`) if intentionally unused

## Fragile Areas

**Complex Parsing Logic:**
- Files: `src/validate/mod.rs` (Rust and TypeScript error parsing with regex)
- Why fragile: Regex-based error parsing depends on compiler output format; any compiler output format change breaks parsing
- Safe modification: Add integration tests with actual compiler error samples
- Test coverage: Present but may not cover all error format variations

**Cross-File Reference Resolution:**
- Files: `src/resolve/cross_file.rs`, `src/resolve/references/rust.rs`
- Why fragile: Complex import resolution logic for multiple import types (use, super, self, glob, renamed)
- Safe modification: Add comprehensive unit tests for each import type before refactoring
- Test coverage: Good for Rust, minimal for other languages

**Tree-Sitter AST Traversal:**
- Files: `src/expand/tree_walker.rs` (1,154 lines)
- Why fragile: Deeply nested AST walking logic; easy to miss edge cases
- Safe modification: Use tree-sitter query API instead of manual node traversal
- Test coverage: Extensive but may miss AST structure variations

**Compiler Validation Gates:**
- Files: `src/validate/gates.rs` (774 lines)
- Why fragile: Depends on external compiler processes; subprocess failures can gate valid code
- Safe modification: Add timeouts to compiler subprocess calls; implement fallback validation
- Test coverage: Present but may not cover all compiler behaviors

## Scaling Limits

**Relationship Query Caching:**
- Current capacity: In-memory `RelationshipCache` with no size limits
- Limit: Unbounded memory growth for large codebases with many files
- Scaling path: Implement LRU cache with configurable size limits

**Symbol Graph Database:**
- Current capacity: SQLite-based graph with no known limits
- Limit: Unknown; should test with 10K+ files
- Scaling path: Benchmark with large monorepos; consider sharding if needed

## Dependencies at Risk

**Tree-Sitter Parsers:**
- Risk: Multiple tree-sitter language parsers at version 0.21; ecosystem moving to 0.22+
- Files: `Cargo.toml` lines 17-23
- Impact: Potential future incompatibility if parsers diverge
- Migration plan: Monitor tree-sitter releases; upgrade to 0.22 when ecosystem stabilizes

**Magellan/SQLiteGraph:**
- Risk: `magellan` at 0.5.3, `sqlitegraph` at 1.0
- Files: `Cargo.toml` lines 9-11
- Impact: Breaking changes possible in future versions
- Migration plan: Pin exact versions in lockfile; monitor release notes

## Missing Critical Features

**TypeScript Parser for Import Resolution:**
- Problem: TypeScript-specific tests commented out with TODO: "TypeScript-specific tests require tree-sitter-typescript parser"
- Files: `src/ingest/javascript.rs:422`
- Blocks: Complete TypeScript import extraction testing
- Note: tree-sitter-typescript is already a dependency (version 0.21)

**Batch Operation Diff Metrics:**
- Problem: Batch patch operations don't report line changes
- Blocks: Accurate operation metrics for batch operations
- Note: Single-operation patch also has this issue (lines_added/lines_removed hardcoded to 0)

## Test Coverage Gaps

**Cross-Language Integration Tests:**
- What's not tested: Full cross-file reference finding across language boundaries
- Files: Tests exist but limited to single-language or Rust-only cross-file
- Risk: Multi-language projects may have unresolved references
- Priority: High (Phase 4 per comment in `src/main.rs:400`)

**Disk Space Validation Tests:**
- What's not tested: Actual disk space checking (only tests hardcoded 1TB return value)
- Files: `src/verify.rs:231-238` has no integration tests
- Risk: Insufficient disk space causes mid-operation failures
- Priority: Medium (current warning-only behavior is safe)

**Deprecated API Migration Tests:**
- What's not tested: All code paths use `store_symbol_with_file_and_language` (no tests verify deprecated API removal)
- Files: `src/graph/mod.rs:443` still uses deprecated method in test
- Risk: Future removal of deprecated API breaks tests
- Priority: Low (clippy warnings provide ongoing detection)

---

*Concerns audit: 2026-01-23*
