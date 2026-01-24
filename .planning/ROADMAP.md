# Roadmap: Splice Refactoring Tool

## Overview

Splice is a span-safe refactoring tool for 7 programming languages (Rust, Python, C, C++, Java, JavaScript, TypeScript) using tree-sitter for AST parsing and SQLiteGraph for code relationship storage.

## Milestones

- ✅ **v2.0 Production Safety** — Phases 1-10 (shipped 2026-01-18)
- ✅ **v2.2 Unified JSON & LLM Optimization** — Phases 11-18 (shipped 2026-01-23)
- ✅ **v2.2.1 Code Quality & Bug Fixes** — Phases 19-21 (shipped 2026-01-24)
- 🚧 **v2.2.2 Magellan Integration** — Phases 22-26 (in progress)

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

</details>

<details>
<summary>✅ v2.2.1 Code Quality & Bug Fixes (Phases 19-21) — SHIPPED 2026-01-24</summary>

**Milestone Goal:** Fix all 67 issues identified in comprehensive bug analysis, improving code reliability and safety

**Bug Analysis:** docs/BUG_ANALYSIS.md

**Summary:**
- Fixed 67 issues across 11 bug categories
- Eliminated all unsafe unwrap() patterns in production code
- Improved UTF-8 handling across all language modules
- Consolidated duplicate APIs (parser creation, import extraction, resolve symbol)
- Introduced testable configuration for execution logging
- All 340+ tests passing

**Delivered Plans:** 20/20
- Phase 19: 7 plans (Critical Error Handling)
- Phase 20: 7 plans (Lifetime & Resource Safety)
- Phase 21: 6 plans (API Consolidation & Code Quality)

</details>

### 🚧 v2.2.2 Magellan Integration (In Progress)

**Milestone Goal:** Unified CLI interface - Splice provides both Magellan query commands and span-safe editing

#### Phase 22: Symbol ID & Format Foundation
**Goal**: Establish Magellan-compatible ID formats and field translation
**Depends on**: Phase 21
**Requirements**: DATA-01, DATA-02
**Success Criteria** (what must be TRUE):
  1. Symbol IDs are generated as 16-character hex strings (SHA-256, first 8 bytes)
  2. Execution IDs follow {timestamp_hex}-{pid_hex} format for delegated queries
  3. Field translation utilities convert between Magellan (start_line) and Splice (line_start) conventions
  4. JSON schema compatibility tests verify format alignment
**Plans**: 4 plans

Plans:
- [x] 22-01-PLAN.md — Create src/symbol_id.rs with 16-char hex ID generation
- [x] 22-02-PLAN.md — Create src/format/magellan.rs with field translation utilities
- [x] 22-03-PLAN.md — Execution ID generation matching Magellan format
- [x] 22-04-PLAN.md — JSON schema compatibility tests

#### Phase 23: Magellan Integration Extensions
**Goal**: Extend MagellanIntegration wrapper with query methods for status, query, find, refs, files commands
**Depends on**: Phase 22
**Requirements**: QUERY-01, QUERY-02, QUERY-03, QUERY-04, QUERY-05
**Success Criteria** (what must be TRUE):
  1. status command displays database statistics (files, symbols, references, calls, code_chunks counts)
  2. query command lists symbols in a file with optional context/relationship flags
  3. find command locates symbols by name or symbol_id with disambiguation support
  4. refs command shows callers/callees for a symbol with bidirectional traversal
  5. files command lists indexed files with optional symbol counts per file
**Plans**: 5 plans

Plans:
- [x] 23-01-PLAN.md — Add get_statistics() method for database statistics (QUERY-01)
- [x] 23-02-PLAN.md — Add query_symbols_by_file() method with kind filter and relationships (QUERY-02)
- [x] 23-03-PLAN.md — Add find_symbol_by_name() and find_symbol_by_id() methods (QUERY-03)
- [x] 23-04-PLAN.md — Add get_call_relationships() method with bidirectional traversal (QUERY-04)
- [x] 23-05-PLAN.md — Add list_indexed_files() method and integration tests (QUERY-05)

#### Phase 24: CLI Commands & Response Types
**Goal**: Add CLI command variants and response types for delegated queries
**Depends on**: Phase 22
**Requirements**: CLI-01, CLI-02, CLI-03, CLI-04, DATA-03, DATA-04
**Success Criteria** (what must be TRUE):
  1. --output flag supports human (default), json, and pretty formats
  2. --db flag specifies database path and delegates to Magellan
  3. Exit codes match Magellan conventions (0=success, 1=error, 2=usage, 3=database, 4=file not found, 5=validation)
  4. --help shows command categories (Query, Edit, Export, Validation)
  5. Response types (StatusResponse, FindResponse, RefsResponse, FilesResponse) use translated field names
**Plans**: 5 plans

Plans:
- [ ] 24-01-PLAN.md — CLI variants (Status, Find, Refs, Files) in src/cli/mod.rs
- [ ] 24-02-PLAN.md — --output and --db flag implementation
- [ ] 24-03-PLAN.md — Exit code mapping to Magellan conventions
- [ ] 24-04-PLAN.md — Response types in src/output.rs
- [ ] 24-05-PLAN.md — CLI parsing and help text tests

#### Phase 25: Export Command & Error Mapping
**Goal**: Implement export command and map Magellan errors to Splice codes
**Depends on**: Phase 23, Phase 24
**Requirements**: EXPORT-01, EXPORT-02, ERROR-01
**Success Criteria** (what must be TRUE):
  1. export command exports graph data in json, jsonl, or csv format
  2. Export output includes files, symbols, references, and calls with proper schema version
  3. Magellan errors are mapped to SPL-E### codes with original error preserved in chain
  4. Export command supports --output flag for file destination
**Plans**: TBD

Plans:
- [ ] 25-01-PLAN.md — Export command implementation (json/jsonl/csv formats)
- [ ] 25-02-PLAN.md — Export schema definition with version field
- [ ] 25-03-PLAN.md — Error mapping from Magellan to SPL-E### codes
- [ ] 25-04-PLAN.md — Export command tests

#### Phase 26: Integration Testing
**Goal**: End-to-end validation of unified CLI interface
**Depends on**: Phase 23, Phase 24, Phase 25
**Requirements**: All v2.2.2 requirements (integration validation)
**Success Criteria** (what must be TRUE):
  1. All query commands (status, query, find, refs, files) execute end-to-end
  2. Export command produces valid output in all three formats
  3. Error codes correctly map from Magellan errors
  4. LLM consumption tests verify single-tool workflow for discovery and editing
  5. Performance benchmarks confirm query performance within acceptable limits
**Plans**: TBD

Plans:
- [ ] 26-01-PLAN.md — End-to-end integration tests for all query commands
- [ ] 26-02-PLAN.md — Export format validation tests
- [ ] 26-03-PLAN.md — Error code mapping tests
- [ ] 26-04-PLAN.md — LLM consumption tests
- [ ] 26-05-PLAN.md — Performance benchmarks
- [ ] 26-06-PLAN.md — Documentation (docs/magellan_integration.md)

---

## Progress

**Execution Order:**
Phases execute in numeric order: 22 → 23 → 24 → 25 → 26

| Phase | Milestone | Plans Complete | Status | Completed |
|-------|-----------|----------------|--------|-----------|
| 1-10 | v2.0 | 31/31 | Complete | 2026-01-18 |
| 11-18 | v2.2 | 55/55 | Complete | 2026-01-23 |
| 19 | v2.2.1 | 7/7 | Complete | 2026-01-23 |
| 20 | v2.2.1 | 7/7 | Complete | 2026-01-24 |
| 21 | v2.2.1 | 6/6 | Complete | 2026-01-24 |
| 22. Symbol ID & Format Foundation | v2.2.2 | 4/4 | Complete | 2026-01-24 |
| 23. Magellan Integration Extensions | v2.2.2 | 5/5 | Complete | 2026-01-24 |
| 24. CLI Commands & Response Types | v2.2.2 | 0/5 | Not started | - |
| 25. Export Command & Error Mapping | v2.2.2 | 0/4 | Not started | - |
| 26. Integration Testing | v2.2.2 | 0/6 | Not started | - |

**Milestone Progress:**
- v2.0: 31/31 plans complete (100%)
- v2.2: 55/55 plans complete (100%)
- v2.2.1: 20/20 plans complete (100%)
- v2.2.2: 9/24 plans complete (38%)

---
*Last updated: 2026-01-24*
