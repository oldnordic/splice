# Roadmap: Splice Refactoring Tool

## Overview

Splice is a span-safe refactoring tool for 7 programming languages (Rust, Python, C, C++, Java, JavaScript, TypeScript) using tree-sitter for AST parsing and SQLiteGraph for code relationship storage.

## Milestones

- ✅ **v2.0 Production Safety** — Phases 1-10 (shipped 2026-01-18)
- ✅ **v2.2 Unified JSON & LLM Optimization** — Phases 11-18 (shipped 2026-01-23)
- 🚧 **v2.2.1 Code Quality & Bug Fixes** — Phases 19-21 (in progress)

## Phases

<details>
<summary>✅ v2.0 Production Safety (Phases 1-10) — SHIPPED 2026-01-18</summary>

**Milestone Goal:** Comprehensive overhaul for production safety, SQLiteGraph v1.0 upgrade, and structured output

See `.planning/milestones/v2.0-ROADMAP.md` for complete details of phases 1-10.

**Key Features Delivered:**
- Safety Foundation: Eliminated all unwrap() calls in production paths
- SQLiteGraph v1.0: Migrated to Native V2 backend
- Structured Output: Explicit field schema with stable identifiers
- Span-Aware Metadata: Line/column coordinates in all output
- Deterministic Ordering: Sorted output across all operations
- Validation Hooks: Pre/post verification with checksums
- Execution Logging: Complete audit trail with query capabilities
- Integration Testing: 75+ new tests for 7 languages
- Documentation: Full v2.0 documentation (README, manual, API)

</details>

<details>
<summary>✅ v2.2 Unified JSON & LLM Optimization (Phases 11-18) — SHIPPED 2026-01-23</summary>

**Milestone Goal:** Unified JSON Schema across all LLM tools with rich span extensions optimized for AI agent consumption and human-friendly CLI improvements

See `.planning/milestones/v2.2-ROADMAP.md` for complete details of phases 11-18.

**Key Features Delivered:**
- Rich Span Extensions: Context, semantic kind, language, checksums, error codes with zero breaking changes
- Rich Span Advanced: Relationships (callers, callees, imports, exports), tool hints, suggested actions
- CLI Conventions: `-n` dry-run, `-A`/`-B`/`-C` context flags, unified diff output, git-style exit codes
- Enhanced Errors: SPL-E### codes, severity levels, fuzzy suggestions, `splice explain` command
- Symbol Expansion: AST-aware parent chain walking with 6 language expanders
- Search & Apply: `splice search --pattern` with glob filtering, atomic find-and-replace
- Integration Testing: 340 tests passing across 7 languages, Magellan alignment
- Error Code Integration: All 28 error-level variants mapped with explain_command field

</details>

### v2.2.1 Code Quality & Bug Fixes (IN PROGRESS)

**Milestone Goal:** Fix all 67 issues identified in comprehensive bug analysis, improving code reliability and safety

**Bug Analysis:** docs/BUG_ANALYSIS.md

---

#### Phase 19: Critical & High-Priority Error Handling

**Goal**: Fix panic-risk bugs and unsafe unwrap/expect patterns in import extraction modules

**Depends on**: Phase 18
**Requirements**: ERROR-01 through ERROR-07, LIFETIME-01 through LIFETIME-04, BOUNDARY-01 through BOUNDARY-06, MATH-01

**Success Criteria**:
1. No unwrap() on first()/last() without proper empty checks
2. All string slicing operations handle UTF-8 correctly
3. All test code unwrap() calls replaced with proper error handling
4. Boundary checks added for all slice operations
5. Disk space calculation improved

**Plans**: 7 plans in 2 waves
- [ ] 19-01-PLAN.md — Fix unwrap() on first()/last() in python.rs (wave 1)
- [ ] 19-02-PLAN.md — Replace unwrap() calls in cpp.rs test code (wave 1)
- [ ] 19-03-PLAN.md — Replace unwrap() calls in javascript.rs test code (wave 1)
- [ ] 19-04-PLAN.md — Replace unwrap() calls in java.rs test code (wave 1)
- [ ] 19-05-PLAN.md — Replace unwrap() calls in typescript.rs test code (wave 1)
- [ ] 19-06-PLAN.md — Fix UTF-8 string slicing across all import modules (wave 2)
- [ ] 19-07-PLAN.md — Add boundary checks and improve disk space calculation (wave 2)

---

#### Phase 20: Lifetime & Resource Safety

**Goal**: Fix data lifetime issues, improve path handling, and fix concurrency issues

**Depends on**: Phase 19
**Requirements**: ERROR-08 through ERROR-12, LIFETIME-05 through LIFETIME-08, BOUNDARY-05 through BOUNDARY-06, CONCURRENCY-01 through CONCURRENCY-03, STATE-01 through STATE-02, RESOURCE-01

**Success Criteria**:
1. All to_string_lossy() replaced with proper path handling
2. Execution log errors properly propagated
3. Test environment variable race condition fixed
4. SQLite connection sharing improved
5. TempDir cleanup improved
6. Rope mutation tracking improved

**Plans**:
- [ ] 20-01-PLAN.md — Fix unwrap() on parent() in backup.rs and expect() calls in graph/mod.rs
- [ ] 20-02-PLAN.md — Replace to_string_lossy() in cli/mod.rs (4 locations)
- [ ] 20-03-PLAN.md — Replace to_string_lossy() in patch/pattern.rs (14 locations)
- [ ] 20-04-PLAN.md — Fix execution log error handling in log.rs
- [ ] 20-05-PLAN.md — Improve main.rs execution logging error handling (16 locations)
- [ ] 20-06-PLAN.md — Fix test environment variable race condition
- [ ] 20-07-PLAN.md — Improve temp directory and resource cleanup

---

#### Phase 21: API Consolidation & Code Quality

**Goal**: Consolidate duplicate functions, improve state management, and refactor global state

**Depends on**: Phase 20
**Requirements**: API-01 through API-03, STATE-01 through STATE-02, GLOBAL-01

**Success Criteria**:
1. Single parser_for_language function in shared module
2. Import extraction uses trait-based approach
3. Resolve symbol API simplified
4. Environment variable feature toggle refactored for testability
5. All 67 issues from bug analysis verified fixed

**Plans**:
- [ ] 21-01-PLAN.md — Consolidate duplicate parser_for_language functions
- [ ] 21-02-PLAN.md — Extract common import extraction patterns into trait
- [ ] 21-03-PLAN.md — Simplify resolve symbol API surface
- [ ] 21-04-PLAN.md — Refactor environment variable feature toggle
- [ ] 21-05-PLAN.md — Run comprehensive tests to verify all fixes
- [ ] 21-06-PLAN.md — Update bug analysis document with verification status

---

## Progress

| Phase | Milestone | Plans Complete | Status | Completed |
|-------|-----------|----------------|--------|-----------|
| 1-10 | v2.0 | 31/31 | Complete | 2026-01-18 |
| 11-18 | v2.2 | 55/55 | Complete | 2026-01-23 |
| 19. Critical Error Handling | v2.2.1 | 7/7 | Complete | 2026-01-23 |
| 20. Lifetime & Resource Safety | v2.2.1 | 0/7 | Not started | - |
| 21. API Consolidation | v2.2.1 | 0/6 | Not started | - |

**Milestone Progress:**
- v2.0: 31/31 plans complete (100%)
- v2.2: 55/55 plans complete (100%)
- v2.2.1: 7/20 plans complete (35%)
