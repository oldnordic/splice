# Splice Refactoring Tool - v2.0 Overhaul

## What This Is

Splice is a span-safe code refactoring tool that provides AST-validated code modifications across 7 programming languages (Rust, Python, C, C++, Java, JavaScript, TypeScript). It uses tree-sitter for parsing and SQLiteGraph for code relationship storage, with multi-stage validation (tree-sitter + compiler) before applying changes.

## Core Value

**Span-safe refactoring with validation** — Every modification is validated at both AST and compiler level before being applied, with automatic backup and rollback capabilities.

## Requirements

### Validated

- ✓ Multi-language symbol extraction (tree-sitter 0.21) — existing v0.5.x
- ✓ Code graph storage and querying (SQLiteGraph 0.2.11) — existing v0.5.x
- ✓ Reference finding within files — existing v0.5.x
- ✓ Byte-accurate span operations — existing v0.5.x
- ✓ Multi-stage validation (tree-sitter + compiler gates) — existing v0.5.x
- ✓ Automatic backup before modifications — existing v0.5.x
- ✓ CLI with JSON output — existing v0.5.x
- ✓ 7 language support (Rust, Python, C, C++, Java, JavaScript, TypeScript) — existing v0.5.x

### Active

- [ ] **Eliminate unsafe unwrap() calls** — Replace all `unwrap()` in production paths with proper error handling using `?` operator and context
- [ ] **Upgrade SQLiteGraph to v1.0** — Migrate from 0.2.11 to 1.0 with Native V2 backend
- [ ] **Structured JSON output with explicit fields** — Replace ad-hoc JSON with structured schema
- [ ] **Stable identifiers (execution_id, match_id, span_id)** — Add unique IDs to all operations and results
- [ ] **Span-aware output (byte + line/col)** — Include both byte offsets and line/column for every match
- [ ] **Deterministic ordering** — Ensure all output is sorted consistently
- [ ] **Validation hooks** — Add checksums and pre/post verification hooks
- [ ] **Execution logging** — Log every run with execution_id for audit trail
- [ ] **Maintain Magellan v0.5.3 compatibility** — Preserve integration during upgrade
- [ ] **Line/column metadata in graph** — Implement TODOs for storing line/col in code graph

### Out of Scope

- New programming language support — current 7 languages are sufficient
- Web UI or IDE integration — CLI-only tool
- Distributed processing — single-machine operation
- Breaking changes to Magellan integration — must maintain compatibility

## Context

**Existing State:**
- Current version: v0.5.3 with Magellan v0.5.3 integration
- Codebase has tech debt: excessive unwrap() calls, large files, incomplete TODO items
- Uses older dependencies: tree-sitter 0.21 (vs 0.22 current), SQLiteGraph 0.2.11 (vs 1.0 current)

**Design Principles from SQLiteGraph v1.0:**
- Structured JSON output with explicit fields
- Stable identifiers (execution_id, match_id, span_id)
- Span-aware (byte offsets + line/col for every match)
- Deterministic ordering (sorted output)
- Validation hooks (checksums, pre/post verification)
- Execution logging (every run logged with execution_id)

**Technical Environment:**
- Rust 2021 edition, cargo build system
- Databases: magellan.db, operations.db, codegraph.db, splice_map.db
- External dependencies: tree-sitter parsers, compiler tooling for validation

## Constraints

- **Magellan Compatibility:** Must maintain compatibility with Magellan v0.5.3 integration
- **Rust 1.70+:** Minimum compiler version for building
- **7 Language Support:** Must maintain support for all existing languages
- **CLI-First:** No web UI or IDE integration planned

## Key Decisions

| Decision | Rationale | Outcome |
|----------|-----------|---------|
| Comprehensive overhaul | All three areas (safety, upgrade, design principles) equally important | — Pending |
| SQLiteGraph v1.0 Native V2 | High performance, modern API, structured output | — Pending |
| CLI can change | Willing to redesign output format to match design principles | — Pending |
| Maintain Magellan integration | Existing codebase mapping integration must be preserved | — Pending |

---
*Last updated: 2026-01-17 after initialization*
