# Roadmap: Splice Refactoring Tool - v2.0 Overhaul

## Overview

A comprehensive overhaul of the Splice refactoring tool to improve code safety, upgrade dependencies, and align with SQLiteGraph v1.0 design principles. The journey progresses from establishing error handling foundations through dependency upgrades to implementing structured outputs, identifiers, metadata, ordering, validation, and logging—culminating in integration testing and documentation updates.

## Domain Expertise

None

## Phases

**Phase Numbering:**
- Integer phases (1, 2, 3): Planned milestone work
- Decimal phases (2.1, 2.2): Urgent insertions (marked with INSERTED)

Decimal phases appear between their surrounding integers in numeric order.

- [x] **Phase 1: Safety Foundation** - Eliminate unwrap() calls, establish error handling patterns (COMPLETED 2026-01-17)
- [x] **Phase 2: SQLiteGraph v1.0 Upgrade** - Migrate to 1.0 with Native V2 backend (COMPLETED 2026-01-17)
- [x] **Phase 3: Structured Output Schema** - Design and implement structured JSON with explicit fields (COMPLETED 2026-01-17)
- [x] **Phase 4: Stable Identifiers** - Add execution_id, match_id, span_id to all operations (COMPLETED 2026-01-17)
- [x] **Phase 5: Span-Aware Metadata** - Line/column in graph, byte+line/col in all output (COMPLETED 2026-01-17)
- [x] **Phase 6: Deterministic Ordering** - Ensure sorted output across all operations (COMPLETED 2026-01-17)
- [x] **Phase 7: Validation Hooks** - Checksums and pre/post verification (COMPLETED 2026-01-17)
- [x] **Phase 8: Execution Logging** - Audit trail with execution_id (COMPLETED 2026-01-17)
- [x] **Phase 9: Integration Testing** - Verify Magellan compatibility, end-to-end tests (COMPLETED 2026-01-18)
- [x] **Phase 10: Documentation Update** - Update docs/manual for v2.0 (COMPLETED 2026-01-18)

## Phase Details

### Phase 1: Safety Foundation
**Goal**: Eliminate unsafe unwrap() calls throughout the codebase, establish consistent error handling patterns using `?` operator and context
**Depends on**: Nothing (first phase)
**Research**: Unlikely (established Rust error patterns, `?` operator, thiserror)
**Plans**: 3

Plans:
- [x] 01-01: Audit & Helpers - Add error context helpers, catalog all unwrap() calls (COMPLETED 2026-01-17)
- [x] 01-02: Fix Core Production Paths - Replace unwrap() in plan, main, patch, validate modules (COMPLETED 2026-01-17)
- [x] 01-03: Fix Language Modules - Replace unwrap() in ingest and import resolution modules (COMPLETED 2026-01-17)

### Phase 2: SQLiteGraph v1.0 Upgrade
**Goal**: Migrate from SQLiteGraph 0.2.11 to v1.0 with Native V2 backend
**Depends on**: Phase 1 (clean codebase before dependency upgrade)
**Research**: Complete (API changes documented, Native V2 backend patterns understood)
**Research topics**: SQLiteGraph v1.0 API differences from 0.2.11, Native V2 backend patterns, migration guide
**Plans**: 4 (PLANNED 2026-01-17)

Plans:
- [x] 02-01: Study API differences and migration path (COMPLETED 2026-01-17)
- [x] 02-02: Update Cargo.toml dependencies (COMPLETED 2026-01-17)
- [x] 02-03: Migrate code to new API (COMPLETED 2026-01-17)
- [x] 02-04: Verify compatibility with existing databases (COMPLETED 2026-01-17)

### Phase 3: Structured Output Schema
**Goal**: Design and implement structured JSON output with explicit fields
**Depends on**: Phase 2 (uses SQLiteGraph v1.0 patterns)
**Research**: Likely (new schema design)
**Research topics**: SQLiteGraph v1.0 structured output patterns, JSON schema design principles
**Plans**: TBD

Plans:
- [x] 03-01: Design output schema structure (COMPLETED 2026-01-17)
- [x] 03-02: Implement structured output types (COMPLETED 2026-01-17)
- [x] 03-03: Replace ad-hoc JSON with structured schema (COMPLETED 2026-01-17)

### Phase 4: Stable Identifiers
**Goal**: Add execution_id, match_id, span_id to all operations for traceability
**Depends on**: Phase 3 (structured output provides foundation)
**Research**: Unlikely (UUID generation, existing patterns in codebase)
**Plans**: TBD

Plans:
- [x] 04-01: Implement ID generation utilities (COMPLETED 2026-01-17)
- [x] 04-02: Add execution_id to all operations (COMPLETED 2026-01-17)
- [x] 04-03: Add match_id and span_id to results (COMPLETED 2026-01-17)

### Phase 5: Span-Aware Metadata
**Goal**: Store line/column in graph, provide byte+line/col in all output
**Depends on**: Phase 4 (identifiers enable metadata tracking)
**Research**: Likely (line/col tracking, tree-sitter integration)
**Research topics**: tree-sitter line/col APIs, storage patterns for metadata
**Plans**: TBD

Plans:
- [x] 05-01: Implement TODOs for line/col in graph storage (COMPLETED 2026-01-17)
- [x] 05-02: Update output to include line/column with byte offsets (COMPLETED 2026-01-17)
- [x] 05-03: Add line/col conversion utilities (COMPLETED 2026-01-17)

### Phase 6: Deterministic Ordering
**Goal**: Ensure sorted output across all operations
**Depends on**: Phase 4 (identifiers required for consistent ordering)
**Research**: Unlikely (sorting algorithms, BTree ordering)
**Plans**: 3

Plans:
- [x] 06-01: Ord implementations for sorting (COMPLETED 2026-01-17)
- [x] 06-02: Main command sorting (delete, plan) (COMPLETED 2026-01-17)
- [x] 06-03: Query and batch sorting (COMPLETED 2026-01-17)

### Phase 7: Validation Hooks
**Goal**: Implement checksums and pre/post verification hooks
**Depends on**: Phase 5 (metadata enables checksums)
**Research**: Complete (SHA-256 established, verify patterns understood)
**Research topics**: Checksum algorithms for code, pre/post hook patterns
**Plans**: 3

Plans:
- [x] 07-01: Design checksum system (COMPLETED 2026-01-17)
- [x] 07-02: Implement pre-verification hooks (COMPLETED 2026-01-17)
- [x] 07-03: Implement post-verification hooks (COMPLETED 2026-01-17)

### Phase 8: Execution Logging
**Goal**: Implement audit trail with execution_id for all runs
**Depends on**: Phase 4 (execution_id required)
**Research**: Complete (logging patterns established, SQLite schema designed)
**Research topics**: SQLite audit log design, append-only table patterns, query builders
**Plans**: 3 (COMPLETED 2026-01-17)

Plans:
- [x] 08-01: Design execution log schema (COMPLETED 2026-01-17)
- [x] 08-02: Implement logging for all operations (COMPLETED 2026-01-17)
- [x] 08-03: Add query capabilities for audit trail (COMPLETED 2026-01-17)

### Phase 9: Integration Testing
**Goal**: Verify Magellan v0.5.3 compatibility, comprehensive end-to-end tests
**Depends on**: Phases 1-8 (all features implemented)
**Research**: Unlikely (testing patterns established in codebase)
**Plans**: 3 (COMPLETED 2026-01-18)

Plans:
- [x] 09-01: Test Magellan integration (COMPLETED 2026-01-18)
- [x] 09-02: End-to-end refactoring tests (COMPLETED 2026-01-18)
- [x] 09-03: Cross-language compatibility tests (COMPLETED 2026-01-18)

### Phase 10: Documentation Update
**Goal**: Update docs and manual for v2.0 changes
**Depends on**: Phases 1-9 (all changes complete)
**Research**: Unlikely (documentation updates)
**Plans**: TBD

Plans:
- [x] 10-01: Update README with new features (COMPLETED 2026-01-18)
- [x] 10-02: Update manual with v2.0 patterns (COMPLETED 2026-01-18)
- [x] 10-03: Document API changes (COMPLETED 2026-01-18)

## Progress

**Execution Order:**
Phases execute in numeric order: 1 → 2 → 3 → 4 → 5 → 6 → 7 → 8 → 9 → 10

| Phase | Plans Complete | Status | Completed |
|-------|----------------|--------|------------|
| 1. Safety Foundation | 3/3 | **COMPLETE** | 2026-01-17 |
| 2. SQLiteGraph v1.0 Upgrade | 4/4 | **COMPLETE** | 2026-01-17 |
| 3. Structured Output Schema | 3/3 | **COMPLETE** | 2026-01-17 |
| 4. Stable Identifiers | 3/3 | **COMPLETE** | 2026-01-17 |
| 5. Span-Aware Metadata | 3/3 | **COMPLETE** | 2026-01-17 |
| 6. Deterministic Ordering | 3/3 | **COMPLETE** | 2026-01-17 |
| 7. Validation Hooks | 3/3 | **COMPLETE** | 2026-01-17 |
| 8. Execution Logging | 3/3 | **COMPLETE** | 2026-01-17 |
| 9. Integration Testing | 3/3 | **COMPLETE** | 2026-01-18 |
| 10. Documentation Update | 3/3 | **COMPLETE** | 2026-01-18 |
