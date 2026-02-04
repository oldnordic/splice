# Splice Refactoring Tool - v2.2

## What This Is

Splice is a span-safe code refactoring tool that provides AST-validated code modifications across 7 programming languages (Rust, Python, C, C++, Java, JavaScript, TypeScript). It uses tree-sitter for parsing and SQLiteGraph for code relationship storage, with multi-stage validation (tree-sitter + compiler) before applying changes.

## Core Value

**Span-safe refactoring with validation** — Every modification is validated at both AST and compiler level before being applied, with automatic backup and rollback capabilities.

## Shipped Versions

### v2.0 Production Safety (2026-01-18)

Comprehensive overhaul establishing production safety and structured output with stable identifiers.

### v2.2 Unified JSON & LLM Optimization (2026-01-23)

Unified JSON Schema across all LLM tools with rich span extensions optimized for AI agent consumption and human-friendly CLI improvements.

### v2.2.1 Code Quality & Bug Fixes (2026-01-24)

Fixed all 67 issues identified in comprehensive bug analysis, eliminating unsafe patterns and improving code reliability.

## Current State

**Shipped:** v2.2.1 (2026-01-24)

**Production-ready refactoring tool with:**
- 312 tests passing (unit + integration)
- Comprehensive documentation
- Zero unsafe unwrap() patterns (production + test code)
- Modern dependency stack (SQLiteGraph 1.0)
- Execution logging for audit trails
- Rich span output optimized for LLM consumption
- Standard CLI conventions (dry-run, context flags, unified diff)
- Structured error codes with explain command
- Consolidated APIs (parser creation, import extraction, symbol resolution)

**Technical Environment:**
- Rust 2021 edition, cargo build system
- Databases: codegraph.db, operations.db
- External dependencies: tree-sitter parsers, compiler tooling

## Design Principles Implemented

- Structured JSON output with explicit fields
- Stable identifiers (execution_id, match_id, span_id)
- Span-aware (byte offsets + line/col for every match)
- Deterministic ordering (sorted output)
- Validation hooks (checksums, pre/post verification)
- Execution logging (every run logged with execution_id)
- Additive schema evolution (all new fields optional)
- Builder pattern for optional fields
- CLI conventions following Unix/Git standards

## Requirements

### Validated (v0.5.x baseline)

- ✓ Multi-language symbol extraction (tree-sitter 0.21)
- ✓ Code graph storage and querying (SQLiteGraph 1.0)
- ✓ Reference finding within files
- ✓ Byte-accurate span operations
- ✓ Multi-stage validation (tree-sitter + compiler gates)
- ✓ Automatic backup before modifications
- ✓ CLI with JSON output
- ✓ 7 language support (Rust, Python, C, C++, Java, JavaScript, TypeScript)

### Delivered (v2.0)

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

### Delivered (v2.2)

- ✅ Rich Span Extensions — Context, semantic kind, relationships, checksums, suggested actions, tool hints
- ✅ CLI Conventions — Dry-run (`-n`), context flags (`-A`/`-B`/`-C`), unified diff, git-style exit codes
- ✅ Enhanced Errors — Severity levels, SPL-E### codes, fuzzy suggestions, `splice explain` command
- ✅ Symbol Expansion — AST-aware parent chain walking with 6 language expanders
- ✅ Search & Apply — `splice search --pattern` with glob filtering, atomic find-and-replace
- ✅ Integration & Testing — 340 tests, performance validation, Magellan alignment

### Delivered (v2.2.1)

- ✅ Error Handling — All unsafe unwrap() patterns replaced with proper error handling
- ✅ UTF-8 Safety — Char-based iteration for string manipulation across all language modules
- ✅ API Consolidation — Single parser_for_language(), ImportExtractor trait, unified resolve API
- ✅ Path Handling — and_then(to_str()) instead of to_string_lossy()
- ✅ Execution Logging — ExecutionLogConfig for testable execution logging
- ✅ Concurrency — env_lock() for thread-safe test environment
- ✅ Resource Cleanup — Documented TempDir Drop behavior, stale file cleanup
- ✅ Bug Analysis — All 67 issues verified and documented

## Out of Scope

| Feature | Reason |
|---------|--------|
| New programming languages | Current 7 languages are sufficient |
| Web UI or IDE integration | CLI-only tool, LLM-friendly CLI is priority |
| Parallel batch processing | Existing single-threaded is fine for current scale |
| Real-time relationship updates | Lazy evaluation sufficient for performance |
| Macro expansion before semantic detection | Language-specific complexity, defer |

## Constraints

- **Magellan Compatibility:** Maintained compatibility with Magellan v0.5.3 integration
- **Rust 1.70+:** Minimum compiler version for building
- **7 Language Support:** Maintained support for all existing languages
- **CLI-First:** No web UI or IDE integration

## Key Decisions

| Decision | Rationale | Outcome |
|----------|-----------|---------|
| v2.0 Comprehensive 10-phase overhaul | All features needed for production-ready tool | Complete v2.0 with 31 plans |
| SQLiteGraph v1.0 Native V2 | High performance, modern API, structured output | Migrated successfully |
| v2.2 Additive schema evolution | Zero breaking changes for backward compatibility | All new fields optional |
| Builder pattern for optional fields | Fluent API while maintaining optionality | 8 builder methods added |
| Session-based relationship caching | O(1) lookup for repeated queries | RelationshipCache implemented |
| Git-style CLI conventions | Familiar UX for developers | `-n`, `-A`/`-B`/`-C`, exit code 1 for changes |
| AST-aware symbol expansion | Accurate symbol boundaries across languages | 6 language expanders |

## Future Work

Potential areas for future development:
- Performance optimization for large codebases
- Additional language support (Go, Ruby, PHP)
- Enhanced undo/redo capabilities
- IDE integration (LSP support)
- Parallel batch processing
- Online error documentation (splice.dev/errors/)

---
## Current Milestone: v2.3 - Magellan v2 Integration

**Goal:** Integrate Magellan v2.0.0 capabilities - enable cross-file rename with byte-accurate references and add semantic program transformation features

**Target features:**
- Cross-file rename refactoring using ReferenceFact byte offsets
- Graph algorithms integration (reachable, dead-code, cycles, paths, slice)
- CLI --proof flag for impact analysis before refactoring
- BLAKE3 SymbolId for unambiguous symbol reference
- Safe UTF-8 content extraction using extract_symbol_content_safe

**Key insight from ChatGPT analysis:**
> Cross-file rename is no longer a "feature request" — it is mechanically possible and safe now.
> The algorithm surface is the difference between "find & replace with guardrails" and semantic program transformation.
> Externalized ground truth: sqlitegraph = truth, Magellan = extractor, Splice = controlled mutation

**Integration approach:**
- Magellan v2.0.0 provides byte-accurate references and graph algorithms
- Splice consumes facts and proof objects via CodeGraph API
- Forward/backward slice for impact-bounded edits
- Detect cycles for rename/extract safety
- Dead symbols for delete with proof

**Recommended phases from ChatGPT:**
- Phase A: Make rename boring and bulletproof (credibility phase)
- Phase B: Add analysis before mutation (differentiation phase)
- Phase C: Metrics as advice, not automation (trust phase)

---
*Last updated: 2026-02-04 — v2.3 milestone started*
*See .planning/milestones/ for detailed milestone archives*
