# Requirements: Splice v2.2.1 - Code Quality & Bug Fixes

**Defined:** 2026-01-23
**Core Value:** Span-safe refactoring with validation
**Milestone:** v2.2.1 - Code Quality & Bug Fixes

## v2.2.1 Requirements

Requirements for code quality improvements and bug fixes based on comprehensive bug analysis.

### Error Handling (ERROR-01 through ERROR-12)

- [ ] **ERROR-01:** Remove unwrap() on .first() in python.rs:95 - use safer pattern
- [ ] **ERROR-02:** Remove unwrap() on .last() in python.rs:201 - use safer pattern
- [ ] **ERROR-03:** Replace all unwrap() calls in python.rs test code with proper error handling
- [ ] **ERROR-04:** Replace all unwrap() calls in cpp.rs test code with proper error handling
- [ ] **ERROR-05:** Replace all unwrap() calls in javascript.rs test code with proper error handling
- [ ] **ERROR-06:** Replace all unwrap() calls in java.rs test code with proper error handling
- [ ] **ERROR-07:** Replace all unwrap() calls in typescript.rs test code with proper error handling
- [ ] **ERROR-08:** Fix unwrap() on parent() in backup.rs:417 - handle None case
- [ ] **ERROR-09:** Review and replace expect() calls in graph/mod.rs production code
- [ ] **ERROR-10:** Fix execution log errors swallowed in log.rs (insert_result.err())
- [ ] **ERROR-11:** Improve main.rs execution logging error handling (16 locations)
- [ ] **ERROR-12:** Add proper error propagation instead of silent logging

### Data Lifetime & UTF-8 Safety (LIFETIME-01 through LIFETIME-08)

- [ ] **LIFETIME-01:** Fix string slicing in cpp.rs:86 to handle UTF-8 correctly
- [ ] **LIFETIME-02:** Fix string slicing in cpp.rs:96 to handle UTF-8 correctly
- [ ] **LIFETIME-03:** Fix string slicing in javascript.rs:218 to handle UTF-8 correctly
- [ ] **LIFETIME-04:** Fix string slicing in typescript.rs:266 to handle UTF-8 correctly
- [ ] **LIFETIME-05:** Replace to_string_lossy() in cli/mod.rs with proper path handling (4 locations)
- [ ] **LIFETIME-06:** Replace to_string_lossy() in patch/pattern.rs with proper path handling (14 locations)
- [ ] **LIFETIME-07:** Review tree_walker.rs lifetime annotations for correctness
- [ ] **LIFETIME-08:** Add UTF-8 safety tests for all string manipulation code

### Boundary Safety (BOUNDARY-01 through BOUNDARY-06)

- [ ] **BOUNDARY-01:** Add bounds check for text.len() < 2 in cpp.rs before slicing
- [ ] **BOUNDARY-02:** Review and fix line number calculations in patch/mod.rs:988-993
- [ ] **BOUNDARY-03:** Add edge case tests for span validation in patch/mod.rs:182-189
- [ ] **BOUNDARY-04:** Fix array indexing in resolve/mod.rs:140 - use safer pattern
- [ ] **BOUNDARY-05:** Improve regex capture group handling in validate/mod.rs:243-257
- [ ] **BOUNDARY-06:** Add byte-to-character conversion edge case handling

### Concurrency Safety (CONCURRENCY-01 through CONCURRENCY-03)

- [ ] **CONCURRENCY-01:** Fix test environment variable race in execution/log.rs:229-234
- [ ] **CONCURRENCY-02:** Add proper SQLite connection sharing protection in execution/log.rs
- [ ] **CONCURRENCY-03:** Improve temp directory cleanup race handling in verify.rs

### API Consolidation (API-01 through API-03)

- [ ] **API-01:** Consolidate duplicate parser_for_language functions into shared module
- [ ] **API-02:** Extract common import extraction patterns into trait-based approach
- [ ] **API-03:** Review and simplify resolve symbol API surface

### State Management (STATE-01 through STATE-02)

- [ ] **STATE-01:** Improve rope mutation tracking in patch/mod.rs for atomicity
- [ ] **STATE-02:** Track partial replacement success/failure in batch application

### Resource Management (RESOURCE-01)

- [ ] **RESOURCE-01:** Improve TempDir cleanup handling in patch/mod.rs:926-929

### Mathematical Correctness (MATH-01)

- [ ] **MATH-01:** Improve disk space calculation heuristic in verify.rs:200-213

### Global State (GLOBAL-01)

- [ ] **GLOBAL-01:** Refactor environment variable feature toggle for testability

## Out of Scope

| Feature | Reason |
|---------|--------|
| New features | Milestone is focused on bug fixes only |
| Performance optimization | Defer to v2.3 |
| New language support | Defer to v2.3+ |
| LSP/IDE integration | Defer to v2.3+ |

## Traceability

Which phases cover which requirements. Updated during roadmap creation.

| Requirement | Phase | Status |
|-------------|-------|--------|
| ERROR-01 through ERROR-03 | Phase 19 | Pending |
| ERROR-04 through ERROR-07 | Phase 19 | Pending |
| ERROR-08 through ERROR-12 | Phase 20 | Pending |
| LIFETIME-01 through LIFETIME-04 | Phase 19 | Pending |
| LIFETIME-05 through LIFETIME-08 | Phase 20 | Pending |
| BOUNDARY-01 through BOUNDARY-06 | Phase 19 | Pending |
| CONCURRENCY-01 through CONCURRENCY-03 | Phase 20 | Pending |
| API-01 through API-03 | Phase 21 | Pending |
| STATE-01 through STATE-02 | Phase 20 | Pending |
| RESOURCE-01 | Phase 20 | Pending |
| MATH-01 | Phase 19 | Pending |
| GLOBAL-01 | Phase 21 | Pending |

**Coverage:**
- v2.2.1 requirements: 42 total
- Mapped to phases: 0 (pending roadmap)

---
*Requirements defined: 2026-01-23*
