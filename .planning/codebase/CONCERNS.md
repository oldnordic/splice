# Codebase Concerns

**Analysis Date:** 2026-01-23

## Tech Debt

**Large Monolithic Files:**
- Issue: `src/main.rs` is 3,395 lines - violates single responsibility principle
- Files: `[src/main.rs]`
- Impact: Hard to maintain, test, and reason about
- Fix approach: Extract command execution logic into separate modules

**Database File Management:**
- Issue: Multiple `.db` files scattered throughout the project (`splice_map.db`, `magellan.db`, `.codemcp/`, etc.)
- Files: `[splice_map.db]`, `[magellan.db]`, `[.codemcp/graph.db]`, `[operations.db]`, `[.splice/operations.db]`
- Impact: Confusing file management, potential conflicts
- Fix approach: Centralize database configuration and management

**Excessive `.unwrap()` Usage:**
- Issue: 70+ instances of `.unwrap()` throughout the codebase indicates panic-prone code
- Files: `[src/ingest/imports/*.rs]`, `[src/patch/backup.rs]`, `[src/graph/mod.rs]`
- Impact: Can cause unexpected crashes on invalid input
- Fix approach: Replace with proper error handling and validation

## Known Bugs

**Import Resolution Incomplete:**
- Issue: Multiple TODOs indicate incomplete cross-file resolution
- Files: `[src/resolve/cross_file.rs:176-177]`, `[src/relationships/mod.rs:451,498]`
- Symptoms: Incomplete symbol resolution for non-Rust languages
- Trigger: Cross-language import operations
- Workaround: Limited to Rust-only symbol resolution

**Diff Calculation Not Implemented:**
- Issue: `TODO: Calculate from diff` in main.rs
- Files: `[src/main.rs:829,1316,1317]`
- Symptoms: Incorrect statistics reported to user
- Trigger: Patch operations with dry run mode
- Workaround: Manual diff calculation in user code

## Security Considerations

**Database File Exposure:**
- Risk: Database files contain sensitive file and symbol information
- Files: `[src/patch/.splice_graph.db]`, `[.splice_graph.db]`
- Current mitigation: Files stored in project directory
- Recommendations: Add database encryption and access controls

**File System Access:**
- Risk: Direct file reading/writing without proper validation
- Files: `[src/graph/magellan_integration.rs:42-44]`
- Current mitigation: Basic path validation
- Recommendations: Implement file access restrictions and sandboxing

## Performance Bottlenecks

**Tree-sitter Parsing Overhead:**
- Problem: Multiple full file parses for validation
- Files: `[src/validate/mod.rs]`, `[src/patch/pattern.rs]`
- Cause: Tree-sitter reparses on every validation
- Improvement path: Caching and incremental parsing

**String Cloning Overhead:**
- Problem: Excessive `.clone()` calls on symbols and paths
- Files: `[src/ingest/python.rs, src/ingest/cpp.rs, src/ingest/java.rs]`
- Cause: Avoiding lifetime management complexity
- Improvement path: Use references and smart pointers

**Sequential File Processing:**
- Problem: Files processed one at a time
- Files: `[src/ingest/dispatch.rs]`
- Cause: Simple implementation, no parallelization
- Improvement path: Add concurrent file processing

## Fragile Areas

**Main CLI Module:**
- Files: `[src/main.rs, src/cli/mod.rs]`
- Why fragile: Large function with many parameters, tight coupling between commands
- Safe modification: Extract command handlers to separate modules
- Test coverage: Present but could be more comprehensive

**Patch Validation Gate:**
- Files: `[src/validate/gates.rs]`
- Why fragile: External compiler dependency, platform-specific behavior
- Safe modification: Add mock tests for compiler validation
- Test coverage: Limited integration tests with real compilers

## Dependencies at Risk

**Magellan Version Lock:**
- Risk: Using specific version 0.5.3 of Magellan
- Impact: Missed updates and potential security issues
- Migration plan: Update to latest stable version with compatibility tests

**Tree-sitter Parser Versions:**
- Risk: Using 0.21 versions for all parsers
- Impact: May miss language updates and bug fixes
- Migration plan: Update to current tree-sitter versions

## Missing Critical Features

**Automatic Update Detection:**
- Problem: No mechanism to detect when code has changed externally
- Files: `[src/verify.rs]`
- Blocks: Real-time synchronization and change detection

**Configuration Management:**
- Problem: No centralized configuration system
- Files: `[src/]`
- Blocks: Environment-specific settings and user preferences

## Test Coverage Gaps

**Integration Test Coverage:**
- What's not tested: Large file processing (>10MB files)
- Files: `[tests/performance_context_tests.rs, tests/relationship_performance.rs]`
- Risk: Performance degradation on large codebases
- Priority: High

**Error Handling Tests:**
- What's not tested: Database corruption scenarios
- Files: `[src/error.rs, src/graph/mod.rs]`
- Risk: Silent failures and data loss
- Priority: Medium

**Cross-Language Feature Tests:**
- What's not tested: Complete cross-language symbol resolution
- Files: `[src/resolve/cross_file.rs]`
- Risk: Inconsistent behavior across languages
- Priority: Medium

---

*Concerns audit: 2026-01-23*