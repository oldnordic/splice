# Splice Refactoring Tool - v2.1

## What This Is

Splice is a span-safe code refactoring tool that provides AST-validated code modifications across 7 programming languages (Rust, Python, C, C++, Java, JavaScript, TypeScript). It uses tree-sitter for parsing and SQLiteGraph for code relationship storage, with multi-stage validation (tree-sitter + compiler) before applying changes.

## Core Value

**Span-safe refactoring with validation** — Every modification is validated at both AST and compiler level before being applied, with automatic backup and rollback capabilities.

## Current Milestone: v2.1 - LLM & Human Usability

**Goal:** Make Splice more usable for both AI agents and humans by adding low-risk, high-impact UX improvements.

**Target features:**
- `--dry-run` / `--diff` mode — Preview exact changes before applying
- `--context-before` / `--context-after` flags — Show surrounding code (3-5 lines)
- Better error messages — Expected/actual tokens, parser suggestions, how to fix
- `--expand` / `--full-block` for symbols — Get full block without reading whole file
- `splice search` + atomic apply — Pattern search → patch workflow

## Status: v2.0 COMPLETE ✅ | v2.1 IN PROGRESS ◆

**Release Date:** 2026-01-18
**Version:** 2.0.0
**All planned features implemented and tested.**

### Completed Features (v2.0)

- ✅ **Safety Foundation** — Eliminated all unwrap() calls in production paths with proper error handling
- ✅ **SQLiteGraph v1.0 Upgrade** — Migrated to 1.0 with Native V2 backend
- ✅ **Structured JSON Output** — Explicit field schema for all operations
- ✅ **Stable Identifiers** — execution_id, match_id, span_id for traceability
- ✅ **Span-Aware Output** — Byte offsets + line/column for every match
- ✅ **Deterministic Ordering** — Sorted output across all operations
- ✅ **Validation Hooks** — Checksums and pre/post verification
- ✅ **Execution Logging** — Complete audit trail with query capabilities
- ✅ **Integration Testing** — 75+ integration tests across 7 languages
- ✅ **Comprehensive Documentation** — README, manual, and API docs updated

### Requirements

#### Validated (v0.5.x baseline)

- ✓ Multi-language symbol extraction (tree-sitter 0.21)
- ✓ Code graph storage and querying (SQLiteGraph 1.0)
- ✓ Reference finding within files
- ✓ Byte-accurate span operations
- ✓ Multi-stage validation (tree-sitter + compiler gates)
- ✓ Automatic backup before modifications
- ✓ CLI with JSON output
- ✓ 7 language support (Rust, Python, C, C++, Java, JavaScript, TypeScript)

#### Delivered (v2.0)

- ✅ Eliminated unsafe unwrap() calls — All production paths use proper error handling
- ✅ SQLiteGraph v1.0 — Native V2 backend with structured output
- ✅ Structured JSON output — Explicit field schema
- ✅ Stable identifiers — execution_id, match_id, span_id
- ✅ Span-aware output — byte + line/col
- ✅ Deterministic ordering — Sorted output
- ✅ Validation hooks — Checksums and pre/post verification
- ✅ Execution logging — operations.db audit trail
- ✅ Magellan v0.5.3 compatibility — Verified with integration tests
- ✅ Line/column metadata — Implemented in code graph

#### Active (v2.1)

- [ ] **DRYRUN-01:** Dry-run mode with diff preview — Show exact changes before applying
- [ ] **CONTEXT-01:** Context flags — `--context-before` / `--context-after` for surrounding lines
- [ ] **ERRORS-01:** Enhanced error messages — Expected/actual tokens, parser suggestions, fix hints
- [ ] **EXPAND-01:** Symbol expansion — `--expand` / `--full-block` to get full function body
- [ ] **SEARCH-01:** Search + patch atomic mode — `splice search` → `splice apply-files` workflow

### Out of Scope

- New programming language support — current 7 languages are sufficient
- Web UI or IDE integration — CLI-only tool
- Distributed processing — single-machine operation

## Context

**Current State (v2.0):**
- Production-ready refactoring tool
- 334+ tests passing (unit + integration)
- Comprehensive documentation
- Zero unwrap() calls in production paths
- Modern dependency stack (SQLiteGraph 1.0)
- Execution logging for audit trails

**Design Principles Implemented:**
- Structured JSON output with explicit fields
- Stable identifiers (execution_id, match_id, span_id)
- Span-aware (byte offsets + line/col for every match)
- Deterministic ordering (sorted output)
- Validation hooks (checksums, pre/post verification)
- Execution logging (every run logged with execution_id)

**Technical Environment:**
- Rust 2021 edition, cargo build system
- Databases: codegraph.db, operations.db
- External dependencies: tree-sitter parsers, compiler tooling

## Constraints

- **Magellan Compatibility:** Maintained compatibility with Magellan v0.5.3 integration
- **Rust 1.70+:** Minimum compiler version for building
- **7 Language Support:** Maintained support for all existing languages
- **CLI-First:** No web UI or IDE integration

## Key Decisions (v2.0)

| Decision | Rationale | Outcome |
|----------|-----------|---------|
| Comprehensive 10-phase overhaul | All features needed for production-ready tool | Complete v2.0 with 31 plans |
| SQLiteGraph v1.0 Native V2 | High performance, modern API, structured output | Migrated successfully |
| CLI output redesign | Willing to change to match design principles | Structured output with IDs |
| Non-blocking logging | Log failures shouldn't fail operations | Warning-only errors |
| SHA-256 checksums | Industry standard for integrity verification | Pre/post verification |

## Future Work

Potential areas for future development:
- Performance optimization for large codebases
- Additional language support (Go, Ruby, PHP)
- Enhanced undo/redo capabilities
- IDE integration (LSP support)
- Parallel batch processing

---
*Last updated: 2026-01-22 — v2.1 milestone started*
