# Roadmap: Splice Refactoring Tool

## Overview

Splice is a span-safe refactoring tool for 7 programming languages (Rust, Python, C, C++, Java, JavaScript, TypeScript) using tree-sitter for AST parsing and SQLiteGraph for code relationship storage.

## Milestones

- ✅ **v2.0 Production Safety** — Phases 1-10 (shipped 2026-01-18)
- ✅ **v2.2 Unified JSON & LLM Optimization** — Phases 11-18 (shipped 2026-01-23)

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
- CLI Conventions: `-n` dry-run, `-A`/`-B`/`-C` context, unified diff output, git-style exit codes
- Enhanced Errors: Severity levels, SPL-E### codes, fuzzy suggestions, `splice explain` command
- Symbol Expansion: AST-aware parent chain walking with 6 language expanders
- Search & Apply: `splice search --pattern` with glob filtering, atomic find-and-replace
- Integration Testing: 340 tests passing across 7 languages, Magellan alignment
- Error Code Integration: All 28 error-level variants mapped with explain_command field

</details>

## Progress

| Phase | Milestone | Plans Complete | Status | Completed |
|-------|-----------|----------------|--------|-----------|
| 1. Safety Foundation | v2.0 | 3/3 | Complete | 2026-01-17 |
| 2. SQLiteGraph v1.0 | v2.0 | 4/4 | Complete | 2026-01-17 |
| 3. Structured Output | v2.0 | 3/3 | Complete | 2026-01-17 |
| 4. Stable Identifiers | v2.0 | 3/3 | Complete | 2026-01-17 |
| 5. Span-Aware Metadata | v2.0 | 3/3 | Complete | 2026-01-17 |
| 6. Deterministic Ordering | v2.0 | 3/3 | Complete | 2026-01-17 |
| 7. Validation Hooks | v2.0 | 3/3 | Complete | 2026-01-17 |
| 8. Execution Logging | v2.0 | 3/3 | Complete | 2026-01-17 |
| 9. Integration Testing | v2.0 | 3/3 | Complete | 2026-01-17 |
| 10. Documentation Update | v2.0 | 3/3 | Complete | 2026-01-18 |
| 11. Rich Span Core | v2.2 | 11/11 | Complete | 2026-01-22 |
| 12. Rich Span Advanced | v2.2 | 8/8 | Complete | 2026-01-22 |
| 13. Dry-run & Diff | v2.2 | 5/5 | Complete | 2026-01-22 |
| 14. Context Flags | v2.2 | 5/5 | Complete | 2026-01-22 |
| 15. Enhanced Errors | v2.2 | 6/6 | Complete | 2026-01-22 |
| 16. Symbol Expansion & Search | v2.2 | 11/11 | Complete | 2026-01-22 |
| 17. Integration & Testing | v2.2 | 7/7 | Complete | 2026-01-23 |
| 18. Error Code Integration | v2.2 | 1/1 | Complete | 2026-01-23 |

**Milestone Progress:**
- v2.0: 31/31 plans complete (100%)
- v2.2: 55/55 plans complete (100%) ✓
