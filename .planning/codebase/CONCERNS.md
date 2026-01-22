# Codebase Concerns

**Analysis Date:** 2026-01-22

## Tech Debt

**Missing line counting in results:**
- Issue: CLI results report 0 lines added/removed instead of actual values
- Files: `src/main.rs:492,779,780`
- Impact: Users cannot see actual code changes
- Fix approach: Implement diff calculation using ropey's change tracking

**Unimplemented cross-language resolution:**
- Issue: Import resolution only works for Rust, others return None
- Files: `src/resolve/cross_file.rs:176-216`
- Impact: Cross-language refactoring fails for Python, C++, JS/TS, Java
- Fix approach: Implement language-specific resolvers for each supported language

**Missing strict/skip CLI flags:**
- Issue: Patch validation flags are hardcoded to false/true
- Files: `src/patch/mod.rs:152`
- Impact: Users cannot customize validation behavior
- Fix approach: Wire CLI arguments to validation parameters

**Unimplemented TypeScript tests:**
- Issue: TypeScript-specific functionality lacks dedicated tests
- Files: `src/ingest/javascript.rs:422`
- Impact: TypeScript support may have bugs not caught by tests
- Fix approach: Add tree-sitter-typescript dependency and implement TS-specific tests

**Disabled reference tracking for glob imports:**
- Issue: Glob imports may miss references due to warning-based approach
- Files: `src/main.rs:365`
- Impact: Incomplete refactoring when glob imports are present
- Fix approach: Implement proper glob import resolution or warn more explicitly

**Missing disk space checking:**
- Issue: No verification that sufficient disk space exists for operations
- Files: `src/verify.rs:236`
- Impact: Operations may fail mid-way due to disk space exhaustion
- Fix approach: Implement disk space check before large operations

## Known Bugs

**JSON serialization panics:**
- Symptoms: Ungraceful failure when serialization fails
- Files: `src/main.rs:524,820,1055,1211`
- Trigger: Invalid JSON structure or non-serializable data
- Workaround: Not reproducible since panic terminates process

**Unnecessary string cloning:**
- Symptoms: Performance overhead from excessive cloning
- Files: `src/main.rs:285,423,446,501,503,517,684,720,767,768,777,778,795,797,813`
- Trigger: Code pattern where &str could be used instead of owned String
- Workaround: Manual optimization in hot paths

**Missing fallback for single import:**
- Symptoms: May not capture import references in complex patterns
- Files: `src/resolve/cross_file.rs:198-214`
- Trigger: Single imports without explicit modules
- Workaround: Manual regex-based pattern matching as fallback

## Security Considerations

**Directory traversal risk:**
- Risk: Potential for accessing sensitive files via path manipulation
- Files: `src/main.rs` (file paths from user input)
- Current mitigation: Path validation in operation functions
- Recommendations: Add path canonicalization checks

**Database file permissions:**
- Risk: `.splice/operations.db` may contain sensitive operation data
- Files: `src/execution/log.rs`
- Current mitigation: Default filesystem permissions
- Recommendations: Explicit permission setting and encryption for sensitive data

## Performance Bottlenecks

**Large single files:**
- Problem: `src/main.rs` (2140 lines) violates code size limits
- Files: `src/main.rs`, `src/resolve/references/rust.rs` (1395 lines)
- Cause: CLI grew to handle all commands in one place
- Improvement path: Split into separate modules by command type

**Inefficient symbol cloning:**
- Problem: Excessive String cloning in hot paths
- Files: `src/main.rs`
- Cause: Early API design patterns
- Improvement path: Use &str andCow<'_, str> where possible

**Database write performance:**
- Problem: Synchronous logging may block operations
- Files: `src/execution/log.rs`
- Cause:rusqlite operations are blocking
- Improvement path: Consider async logging with SQLite WAL mode

## Fragile Areas

**Cross-language support:**
- Files: `src/ingest/` modules for each language
- Why fragile: Heavy reliance on tree-sitter parsers and magellan integration
- Safe modification: Test each language independently before changes
- Test coverage: Moderate (has test files for each language)

**Patch validation:**
- Files: `src/patch/mod.rs`, `src/validate/gates.rs`
- Why fragile: Complex validation logic with multiple language-specific rules
- Safe modification: Make validation functions pure and test in isolation
- Test coverage: Good (has comprehensive test coverage)

**Reference resolution:**
- Files: `src/resolve/references/rust.rs`, `src/resolve/cross_file.rs`
- Why fragile: Complex graph traversal with edge cases
- Safe modification: Add more unit tests for edge cases before changes
- Test coverage: Moderate (has tests but may miss edge cases)

## Scaling Limits

**Memory usage:**
- Current capacity: Moderate (limited by ropey and SQLite)
- Limit: Large files (>1MB) may cause memory pressure
- Scaling path: Stream processing for large files

**Database concurrency:**
- Current capacity: Single-writer, multi-reader
- Limit: Concurrent operations may lock
- Scaling path: Consider connection pooling or WAL mode

## Dependencies at Risk

**SQLiteGraph:**
- Risk: External dependency with potential breaking changes
- Impact: Core graph functionality may break
- Migration plan: Fork or create abstraction layer

**Magellan:**
- Risk: Version 0.5.3 is pinned, may have security updates
- Impact: Code indexing functionality
- Migration plan: Regular dependency updates with testing

**Tree-sitter parsers:**
- Risk: Language updates may break parser
- Impact: Code parsing functionality
- Migration plan: Monitor parser updates and update tests

## Missing Critical Features

**Incremental analysis:**
- Problem: Full code analysis on every operation
- Blocks: Large codebases become slow
- Priority: High

**Parallel processing:**
- Problem: Single-threaded execution
- Blocks: Large refactoring operations
- Priority: Medium

**Error recovery:**
- Problem: No rollback on partial failure
- Blocks: Data safety during complex operations
- Priority: High

## Test Coverage Gaps

**Edge case testing:**
- What's not tested: Exceptional paths in error handling
- Files: `src/error.rs` error types
- Risk: Ungraceful failures on unexpected input
- Priority: Medium

**Concurrency testing:**
- What's not tested: Parallel execution scenarios
- Files: `src/execution/log.rs`
- Risk: Database corruption or race conditions
- Priority: Low (currently single-threaded)

---

*Concerns audit: 2026-01-22*
```