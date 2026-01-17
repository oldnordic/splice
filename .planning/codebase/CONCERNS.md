# Codebase Concerns

**Analysis Date:** 2026-01-17

## Tech Debt

**Excessive unwrap() calls:**
- Issue: Heavy use of `unwrap()` throughout codebase, especially in production paths
- Files: `src/plan/mod.rs:253,262`, `src/graph/magellan_integration.rs:68,187`, `src/validate/gates.rs:93,135,177`
- Why: Rapid development pattern, needs refactoring for production safety
- Impact: Potential panics in production, violates CLAUDE.md rules
- Fix approach: Replace with proper error handling using `?` operator and context

**Large source files:**
- Issue: Several files may exceed 300-line project standard
- Files: `src/validate/gates.rs`, `src/graph/mod.rs`, `src/patch/mod.rs`, `src/ingest/*.rs`
- Why: Complex implementations in single files
- Impact: Harder to maintain, violates project quality standards
- Fix approach: Extract submodules for related functionality

**TODO comments for unimplemented features:**
- Issue: Multiple TODOs indicate incomplete implementations
- Files:
  - `src/resolve/mod.rs:149,229` - TODO for storing line/col metadata
  - `src/resolve/cross_file.rs:176-177` - TODO for Python and C/C++ resolution
  - `src/ingest/javascript.rs:422` - TODO for TypeScript-specific tests
- Why: Features deferred during initial implementation
- Impact: Missing functionality for cross-file resolution in some languages
- Fix approach: Implement TODOs or convert to tracked issues

## Known Bugs

**No known bugs documented:**
- Codebase appears stable for implemented features
- Test suite passes for supported languages

## Security Considerations

**Input sanitization:**
- Risk: No apparent input sanitization for user-provided file paths
- Files: CLI argument handling in `src/main.rs`, `src/cli/mod.rs`
- Current mitigation: Rust's std::path provides some protection
- Recommendations: Add explicit path validation, prevent directory traversal

**Database file permissions:**
- Risk: Database files created without restrictive permissions
- Files: `src/graph/mod.rs`, database creation
- Current mitigation: None detected
- Recommendations: Set restrictive permissions (0600) on database files

**Temporary file cleanup:**
- Risk: Temporary files may not be cleaned up in all error scenarios
- Files: `src/patch/backup.rs`
- Current mitigation: Some cleanup implemented
- Recommendations: Ensure cleanup in all error paths via Drop or scopeguard

## Performance Bottlenecks

**Potential N+1 queries:**
- Problem: Symbol cache may cause repeated queries
- Files: `src/graph/mod.rs:144-244`
- Measurement: Not profiled
- Cause: Per-symbol queries vs batch operations
- Improvement path: Implement batch querying for symbol lookups

**Large file processing:**
- Problem: No apparent limits on file sizes being processed
- Files: `src/ingest/*.rs`
- Measurement: Not profiled
- Cause: Tree-sitter processes entire file
- Improvement path: Add file size limits, streaming for very large files

**Symbol cache memory:**
- Problem: In-memory symbol cache may grow unbounded
- Files: `src/graph/mod.rs`
- Measurement: Not profiled
- Cause: No cache eviction policy
- Improvement path: Implement LRU cache with size limits

## Fragile Areas

**Database schema changes:**
- Why fragile: No migration system detected
- Common failures: Manual database recreation required on schema changes
- Safe modification: Document schema versions, implement migrations
- Test coverage: Limited tests for schema compatibility

**Language tool dependencies:**
- Why fragile: External compiler availability varies by system
- Common failures: Missing language tools cause validation to fail
- Safe modification: Make validation optional, provide clear error messages
- Test coverage: Tests assume tools are available

**Backup/restore operations:**
- Why fragile: Critical for safety, complex error handling
- Files: `src/patch/backup.rs`
- Common failures: Partial backups, restore failures
- Safe modification: Comprehensive tests for all scenarios
- Test coverage: Needs more edge case coverage

## Scaling Limits

**Large codebase handling:**
- Current capacity: Not documented, tested on moderate projects
- Limit: Unknown
- Symptoms at limit: Memory growth, slow analysis
- Scaling path: Implement streaming, incremental analysis

**Multi-language projects:**
- Current capacity: Supports 7 languages
- Limit: Adding new language requires new module
- Scaling path: Pluggable language architecture exists

## Dependencies at Risk

**Tree-sitter parser versions:**
- Risk: Using 0.21 while current is 0.22
- Impact: May miss bug fixes and performance improvements
- Migration plan: Upgrade to 0.22 when stable

**Outdated dependencies:**
- Risk: Some dependencies may have security updates
- Impact: Potential security vulnerabilities
- Migration plan: Regular cargo audit and updates

## Missing Critical Features

**Cross-file resolution for some languages:**
- Problem: TODOs indicate incomplete cross-file resolution for Python and C/C++
- Files: `src/resolve/cross_file.rs:176-177`
- Current workaround: Single-file resolution only for affected languages
- Blocks: Full refactoring capability in Python and C/C++
- Implementation complexity: Medium (requires language-specific import resolution)

**Line/column metadata:**
- Problem: TODOs for storing line/col in graph
- Files: `src/resolve/mod.rs:149,229`
- Current workaround: Byte offsets only
- Blocks: Better error messages and user feedback
- Implementation complexity: Low (metadata tracking)

## Test Coverage Gaps

**Multi-language integration tests:**
- What's not tested: Projects mixing multiple languages
- Risk: Cross-language references may not work correctly
- Priority: Medium
- Difficulty to test: Requires complex test fixtures

**Error path coverage:**
- What's not tested: Some error paths may lack tests
- Risk: Errors may not provide useful information
- Priority: Low (Rust's type system provides coverage)
- Difficulty to test: Requires intentional error conditions

**Large codebase testing:**
- What's not tested: Performance with >1000 files
- Risk: Unknown scaling issues
- Priority: Medium
- Difficulty to test: Requires large test fixtures

---

*Concerns audit: 2026-01-17*
*Update as issues are fixed or new ones discovered*
