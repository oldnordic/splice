# Research Summary: Magellan Integration Milestone

**Project:** Splice v2.2.2 - Magellan Query Command Delegation
**Domain:** Unified CLI interface - Code discovery + refactoring in one tool
**Researched:** 2026-01-24
**Confidence:** HIGH

---

## Executive Summary

Splice v2.2.2 is a **CLI delegation milestone** that unifies code discovery (via Magellan) and code modification (via Splice) into a single tool. The integration follows a **library delegation pattern**: Splice calls Magellan as an in-process Rust dependency (not subprocess), forwards query commands to Magellan's existing APIs, then normalizes output to match Splice's JSON schema conventions. No new dependencies are required—Magellan 0.5.3 is already integrated in `src/graph/magellan_integration.rs`.

The recommended approach is **thin adapter delegation** with format alignment. Splice delegates `status`, `find`, `refs`, `files`, and `export` commands to Magellan's CodeGraph API, adds optional Splice-specific enhancements (context, semantics, checksums), and outputs JSON that is a superset of Magellan's format. Critical risks include field name translation (`start_line` vs `line_start`), symbol ID format compatibility (16-char hex), and preventing test breakage when adding optional fields. The milestone is estimated at 6-10 days of focused work, with the highest-risk items being main.rs refactoring (600+ lines of query logic extraction) and ensuring Magellan API compatibility.

---

## Key Findings

### Recommended Stack

**No new dependencies required.** Magellan 0.5.3 with `native-v2` feature is already in Cargo.toml and provides all query APIs needed.

**Core technologies (existing):**
- `magellan 0.5.3` — Code indexing, label queries, status/find/refs/files APIs
- `sqlitegraph 1.0` — Graph backend (re-exported from magellan)
- `rusqlite 0.31` — Direct SQLite access
- `clap 4.5` — CLI argument parsing (already used)
- `serde/serde_json` — JSON serialization (already used)
- `sha2 0.10` — Stable ID generation (already used for span_id)

**Code additions only:**
- CLI command variants (`Status`, `Find`, `Refs`, `Files`) in `src/cli/mod.rs`
- Response types (`StatusResponse`, `FindResponse`, etc.) in `src/output.rs`
- Delegation wrappers in `src/graph/magellan_integration.rs`

### Expected Features

**Must have (table stakes):**
- `status` — Database statistics (files, symbols, references counts)
- `query` — List symbols in a file with optional context/semantics
- `find` — Find symbol by name or symbol_id with disambiguation
- `refs` — Show callers/callees for a symbol
- `files` — List indexed files with optional symbol counts
- `--output` flag — human/json/pretty format selection
- JSON schema alignment — Magellan-compatible wrapper with Splice extensions

**Should have (differentiators):**
- `--with-context` flag — Add context lines via Splice's context module
- `--with-callers`/`--with-callees` — Relationship enhancement
- `--with-semantics` — Semantic kind detection enhancement
- `--with-checksums` — SHA-256 checksums for validation
- Single-tool workflow — LLMs can discover and modify using one CLI

**Defer (v2.3+):**
- Real-time indexing delegation — Users run `magellan watch` separately
- Full LSP integration — Belongs in separate tool, not Splice
- Custom Magellan commands — Use Magellan CLI directly for advanced ops

### Architecture Approach

Magellan integration follows a **library delegation pattern**: Splice calls Magellan's CodeGraph API directly as an in-process Rust library, not as a subprocess or HTTP service. The delegation boundary is clear—queries go to Magellan, edits stay in Splice.

**Major components:**

1. **`src/query.rs` (NEW)** — Centralizes query delegation logic, extracts 600+ lines from main.rs into `QueryExecutor` with `query_by_labels()` and `get_code_chunk()` methods

2. **`src/symbol_id.rs` (NEW)** — Generates 16-character stable symbol IDs compatible with Magellan format (SHA-256 hash, first 8 bytes)

3. **`src/format/magellan.rs` (NEW)** — Format alignment module ensuring Splice output is Magellan-compatible with field translations (`start_line` -> `line_start`)

4. **`src/graph/magellan_integration.rs` (EXTEND)** — Add pagination (`query_by_labels_paginated`), symbol lookup (`get_symbol_by_id`), count methods

5. **`src/cli/mod.rs` (MODIFY)** — Add `Status`, `Find`, `Refs`, `Files` command variants with `--output`, `--limit`, `--offset`, `--format` flags

6. **`src/output.rs` (MODIFY)** — Add `symbol_id` (16-char), `total_count` (pagination) fields to response types

**Data flow:** User command -> main.rs handler -> MagellanIntegration wrapper -> Magellan CodeGraph API -> Splice enrichment -> JSON output

### Critical Pitfalls

1. **Context extraction on large files** — Tree-sitter degrades on files >32KB. Prevention: lazy context loading, span-based caching, `--max-context` flag.

2. **Full-codebase relationship graph scalability** — O(n) queries cause exponential slowdown. Prevention: application-centered traversal, relationship indexing, scope-aware queries (`--scope=file|module|workspace`).

3. **Breaking 334+ existing tests** — Adding fields changes JSON structure. Prevention: additive schema only, `--fields` flag, `assert_json_include!` pattern, golden test update scripts.

4. **Magellan flag namespace collision** — Splice and Magellan CLI flags may conflict. Prevention: explicit flag namespacing (`--magellan-*`), auto-detect database path.

5. **Data format misalignment** — Magellan uses `start_line`/`start_col`, Splice uses `line_start`/`col_start`. Prevention: field translation in format module, `--format magellan` flag for strict compatibility.

---

## Implications for Roadmap

Based on combined research, the milestone breaks into 5 phases following dependency order:

### Phase 1: Symbol ID & Format Foundation

**Rationale:** Symbol ID generation and format translation are foundational dependencies for all query commands. Build these first to establish the Magellan compatibility layer.

**Delivers:**
- `src/symbol_id.rs` with 16-char hex ID generation
- `src/format/magellan.rs` with field translation utilities
- JSON schema compatibility tests

**Addresses:**
- Stack: SHA-256 usage (already in dependencies)
- Features: Symbol ID format alignment
- Architecture: Format alignment module

**Avoids:**
- Pitfall #4 (breaking tests) — test format changes early
- Pitfall #5 (data format misalignment) — establish translation layer

**Complexity:** LOW — pure functions, no side effects

**Research flag:** SKIP — standard hash-and-encode patterns

---

### Phase 2: Magellan Integration Extensions

**Rationale:** Extend the existing `MagellanIntegration` wrapper with pagination and ID-based queries before building the executor that depends on them.

**Delivers:**
- `query_by_labels_paginated()` method
- `get_symbol_by_id()` method
- `count_symbols_with_labels()` method
- Integration tests with test database

**Addresses:**
- Stack: Magellan 0.5.3 API usage
- Features: Pagination support in query/find
- Architecture: Magellan wrapper extensions

**Uses:**
- Existing `src/graph/magellan_integration.rs` infrastructure
- Magellan crate APIs

**Avoids:**
- Pitfall #2 (scalability) — pagination prevents full dataset scans

**Complexity:** MEDIUM — wraps Magellan library API

**Research flag:** NEEDS RESEARCH — verify exact Magellan 0.5.3 method signatures

---

### Phase 3: Query Executor Module

**Rationale:** Centralizes query logic by extracting 600+ lines from main.rs into a testable module. Depends on Phases 1-2 for symbol IDs and Magellan extensions.

**Delivers:**
- `src/query.rs` with `QueryExecutor` struct
- `query_by_labels()` and `get_code_chunk()` methods
- Unit tests for executor logic
- Integration tests with Magellan

**Addresses:**
- Features: Core query/get command functionality
- Architecture: Query executor module

**Uses:**
- Phase 1 (symbol_id, format translation)
- Phase 2 (Magellan extensions)
- Existing context, semantic_kind modules

**Avoids:**
- Pitfall #3 (breaking tests) — maintain existing tests during extraction

**Complexity:** MEDIUM — refactors existing code, requires careful testing

**Research flag:** SKIP — extraction pattern is straightforward

---

### Phase 4: CLI Commands & Output Types

**Rationale:** Add new CLI commands and response types. Depends on executor being defined but can be developed in parallel.

**Delivers:**
- CLI variants: `Status`, `Find`, `Refs`, `Files` in `src/cli/mod.rs`
- Response types: `StatusResponse`, `FindResponse`, `RefsResponse`, `FilesResponse` in `src/output.rs`
- `--output`, `--limit`, `--offset`, `--format` flags
- CLI parsing tests

**Addresses:**
- Features: All 5 delegated commands
- Stack: clap 4.5 usage

**Uses:**
- Existing clap patterns
- Phase 1 (format module)

**Avoids:**
- Pitfall #5 (flag namespace collision) — explicit namespacing

**Complexity:** LOW — additive CLI changes

**Research flag:** SKIP — clap patterns are well-established

---

### Phase 5: Main Integration & Testing

**Rationale:** Wire everything together, refactor main.rs to use QueryExecutor, and validate end-to-end. Highest risk due to main.rs refactoring.

**Delivers:**
- Refactored `execute_query()`, `execute_get()` in main.rs
- New command handlers for status/find/refs/files
- End-to-end integration tests
- Documentation (`docs/magellan_integration.md`)
- Performance benchmarks

**Addresses:**
- Features: Complete unified CLI
- Architecture: Main.rs refactoring

**Uses:**
- All previous phases (1-4)
- Existing main.rs structure

**Avoids:**
- Pitfall #1 (large file performance) — benchmark and optimize
- Pitfall #3 (breaking tests) — comprehensive test suite

**Complexity:** MEDIUM — refactors existing 600+ lines

**Research flag:** SKIP — integration pattern is established

---

### Phase Ordering Rationale

1. **Foundation first** — Symbol ID and format modules (Phase 1) have no dependencies and are used by all other phases
2. **Wrapper before use** — Magellan extensions (Phase 2) must exist before QueryExecutor can use them
3. **Extraction before wiring** — QueryExecutor (Phase 3) should be built and tested before integrating into main.rs
4. **Parallelizable** — CLI commands (Phase 4) can be developed alongside Phase 3, as they only depend on format module
5. **High-risk finale** — Main.rs refactoring (Phase 5) happens last when all dependencies are stable

**This order avoids the critical pitfalls:**
- Builds format translation early to prevent data misalignment (#5)
- Adds pagination to prevent scalability issues (#2)
- Maintains tests throughout to prevent breakage (#3)

---

### Research Flags

**Phases needing deeper research:**
- **Phase 2:** Verify exact Magellan 0.5.3 API signatures for `get_stats()`, `find_symbol()`, `get_references()`. Current research assumes typical code graph patterns.

**Phases with standard patterns (skip research-phase):**
- **Phase 1:** Standard hash-and-encode, field translation patterns
- **Phase 3:** Extract-to-module refactoring is well-established
- **Phase 4:** Clap CLI patterns are consistent across codebase
- **Phase 5:** Main handler wiring follows existing patterns

---

## Confidence Assessment

| Area | Confidence | Notes |
|------|------------|-------|
| Stack | HIGH | Verified from Cargo.toml, all dependencies already present |
| Features | HIGH | Command specs from `docs/CLI_PATTERNS.md`, API patterns from existing code |
| Architecture | HIGH | Delegation pattern already implemented in `src/graph/magellan_integration.rs` |
| Pitfalls | HIGH | Research from WebSearch + academic papers, specific sources cited |

**Overall confidence:** HIGH

Delegation pattern is production-ready. Primary uncertainty is exact Magellan 0.5.3 API method signatures, which can be verified through testing or crate documentation.

### Gaps to Address

1. **Magellan 0.5.3 API surface** — Verify exact method signatures for `get_stats()`, `find_symbol()`, `get_references()`, `list_files()`. Current research assumes based on typical code graph patterns. **Handle during Phase 2:** Write spike test to verify actual API.

2. **Type compatibility** — Confirm Magellan's `SymbolInfo`, `Reference` types convert to Splice's `SymbolMatch`, `ReferenceMatch` without data loss. **Handle during Phase 2:** Build conversion functions with unit tests.

3. **Error handling** — Verify Magellan's error types convert cleanly to `SpliceError`. **Handle during Phase 2:** Create error mapping table.

4. **Execution ID generation** — Test that `generate_magellan_execution_id()` produces IDs matching Magellan's `{timestamp_hex}-{pid_hex}` format. **Handle during Phase 1:** Unit test against format spec.

5. **Export format specifications** — Exact JSON/JSONL/CSV/SCIP output formats not fully verified (WebSearch blocked). **Handle during Phase 4:** Test with actual magellan CLI to observe output.

---

## Sources

### Primary (HIGH confidence)

**Codebase analysis (verified directly):**
- `/home/feanor/Projects/splice/Cargo.toml` — Dependency versions confirmed
- `/home/feanor/Projects/splice/src/cli/mod.rs` — Existing CLI commands (728 lines)
- `/home/feanor/Projects/splice/src/graph/magellan_integration.rs` — Magellan wrapper (256 lines)
- `/home/feanor/Projects/splice/src/main.rs` — Query/get handlers (lines 1982-2750)
- `/home/feanor/Projects/splice/src/output.rs` — JSON response types (1090 lines)
- `/home/feanor/Projects/splice/src/resolve/mod.rs` — Symbol resolution (543 lines)

**Project documentation:**
- `/home/feanor/Projects/splice/docs/CLI_PATTERNS.md` — Complete command specifications
- `/home/feanor/Projects/splice/docs/LLM_TOOL_ECOSYSTEM_ALIGNMENT.md` — Schema requirements
- `/home/feanor/Projects/splice/.planning/PROJECT.md` — v2.2.2 milestone definition

### Secondary (MEDIUM confidence)

**Academic and industry research:**
- Jarvis: Application-Centered Call Graph Construction (arXiv 2024) — scalability patterns
- Type-Based Call Graph Construction (USENIX Sec 2023) — performance benchmarks
- SQLite Query Optimizer Overview — database indexing strategies
- Command Line Interface Guidelines (clig.dev) — CLI design patterns

### Tertiary (LOW confidence)

**Assumptions needing verification:**
- Magellan 0.5.3 exact API method signatures — inferred from wrapper code, not directly verified
- Export format specifications (JSON/JSONL/CSV/SCIP) — WebSearch blocked, needs CLI testing
- Call graph query performance — no benchmarks yet, needs performance testing

---
*Research completed: 2026-01-24*
*Ready for roadmap: yes*
*Milestone: Splice v2.2.2 - Magellan Integration*
