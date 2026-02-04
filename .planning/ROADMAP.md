# Roadmap: Splice Refactoring Tool

## Overview

Splice is a span-safe refactoring tool for 7 programming languages (Rust, Python, C, C++, Java, JavaScript, TypeScript) using tree-sitter for AST parsing and SQLiteGraph for code relationship storage.

## Milestones

- ✅ **v2.0 Production Safety** — Phases 1-10 (shipped 2026-01-18)
- ✅ **v2.2 Unified JSON & LLM Optimization** — Phases 11-18 (shipped 2026-01-23)
- ✅ **v2.2.1 Code Quality & Bug Fixes** — Phases 19-21 (shipped 2026-01-24)
- ✅ **v2.2.2 Magellan Integration** — Phases 22-26 (shipped 2026-01-24)
- ✅ **v2.2.4 Code Cleanup** — Phase 27 (shipped 2026-02-04)

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

<details>
<summary>✅ v2.2.2 Magellan Integration (Phases 22-26) — SHIPPED 2026-01-24</summary>

**Milestone Goal:** Unified CLI interface - Splice provides both Magellan query commands and span-safe editing

See `.planning/milestones/v2.2.2-ROADMAP.md` for complete details of phases 22-26.

**Key Features Delivered:**
- Symbol ID Format: 16-character hex IDs (SHA-256, first 8 bytes) compatible with Magellan
- Field Translation: Automatic conversion between Magellan (start_line) and Splice (line_start) field conventions
- Query Commands: status, find, refs, files, query commands with Magellan database backend
- Export Formats: json, jsonl, csv export with proper schema versioning
- Error Mapping: Magellan errors mapped to SPL-E### codes with original error preserved
- Exit Codes: Magellan-compatible exit codes (0=success, 1=error, 2=usage, 3=database, 4=file not found, 5=validation)
- Response Types: Magellan-compatible JSON response types for all query commands
- Documentation: Comprehensive Magellan integration documentation (docs/magellan_integration.md)

</details>

<details>
<summary>✅ v2.2.4 Code Cleanup (Phase 27) — SHIPPED 2026-02-04</summary>

**Milestone Goal:** Remove vestigial code from early Splice design phase

See `.planning/milestones/v2.2.4-ROADMAP.md` for complete details of phase 27.

**Key Features Delivered:**
- Dead Code Removal: Removed unused `Ingestor` struct stub from `src/ingest/mod.rs`
- Documentation Update: Updated all documentation to reflect Magellan-based architecture
- Migration Guidance: CHANGELOG.md entry with clear migration path to `MagellanIngestor`

</details>

---

## Progress

**Execution Order:**
Phases execute in numeric order: 22 → 23 → 24 → 25 → 26 → 27

| Phase | Milestone | Plans Complete | Status | Completed |
|-------|-----------|----------------|--------|-----------|
| 1-10 | v2.0 | 31/31 | Complete | 2026-01-18 |
| 11-18 | v2.2 | 55/55 | Complete | 2026-01-23 |
| 19 | v2.2.1 | 7/7 | Complete | 2026-01-23 |
| 20 | v2.2.1 | 7/7 | Complete | 2026-01-24 |
| 21 | v2.2.1 | 6/6 | Complete | 2026-01-24 |
| 22. Symbol ID & Format Foundation | v2.2.2 | 4/4 | Complete | 2026-01-24 |
| 23. Magellan Integration Extensions | v2.2.2 | 5/5 | Complete | 2026-01-24 |
| 24. CLI Commands & Response Types | v2.2.2 | 5/5 | Complete | 2026-01-24 |
| 25. Export Command & Error Mapping | v2.2.2 | 4/4 | Complete | 2026-01-24 |
| 26. Integration Testing | v2.2.2 | 6/6 | Complete | 2026-01-24 |
| 27. Code Cleanup | v2.2.4 | 3/3 | Complete | 2026-02-04 |

**Milestone Progress:**
- v2.0: 31/31 plans complete (100%)
- v2.2: 55/55 plans complete (100%)
- v2.2.1: 20/20 plans complete (100%)
- v2.2.2: 24/24 plans complete (100%)
- v2.2.4: 3/3 plans complete (100%)

---
*Last updated: 2026-02-04*
