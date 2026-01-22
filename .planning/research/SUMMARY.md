# Research Summary: Splice v2.2 - Unified JSON Schema & LLM Optimization

**Project:** Splice v2.2
**Domain:** AST-aware code patching CLI with LLM integration
**Researched:** 2026-01-22
**Milestone:** v2.2 - Unified JSON Schema & LLM Optimization
**Confidence:** HIGH

---

## Executive Summary

Splice v2.2 is an **LLM-first code patching tool** that extends Splice v2.0's existing span-based patching with rich JSON output optimized for AI agent consumption. The research confirms that **no new dependencies are required** — all v2.2 features (context extraction, semantic kind detection, cross-file relationships, checksums, suggested actions, tool hints, unified error codes) can be implemented using the existing stack: tree-sitter 0.22, sha2 0.10, serde 1.0, rusqlite 0.31, and magellan 0.5.3.

The recommended approach is **additive, not breaking**: extend the existing `SpanResult` structure with optional fields (`#[serde(skip_serializing_if = "Option::is_none")]`) to maintain backward compatibility while enabling new LLM-optimized capabilities. The architecture cleanly separates concerns: context extraction via ropey for line boundaries, semantic kind detection via tree-sitter node type mapping, relationship building via Magellan's CALLER/CALLS edges, and checksums via existing SHA-256 infrastructure.

**Critical risks identified:** (1) **Large file performance** — tree-sitter degrades on files >32KB, mitigated by lazy loading and span-based caching; (2) **Relationship graph scalability** — O(n²) traversal on full codebase queries, mitigated by application-centered traversal and scope limits; (3) **Breaking 334 existing tests** — schema changes can break test assertions, mitigated by additive-only fields and golden test update scripts; (4) **Over-engineering suggested actions** — anti-pattern of building complex taxonomies before seeing real LLM usage, mitigated by starting with 3 primitives (delete, replace, expand) and iterating based on actual patterns.

The MVP recommendation prioritizes **80% of LLM value with 20% effort**: context extension, semantic kind detection, and checksums for race protection. Defer full relationship graph and suggested actions to v2.3+ after foundation is solid.

---

## Key Findings

### Recommended Stack

**From STACK.md:** Splice v2.2 requires **ZERO new external dependencies**. All rich span extensions build on existing infrastructure.

**Core technologies:**
- **tree-sitter 0.22** — Context extraction and semantic kind detection via Node API — already installed
- **ropey 1.6** — Efficient line-based context extraction without reparsing entire files — already in dependencies
- **sha2 0.10** — Span checksums for race condition protection — `checksum_span()` already implemented
- **magellan 0.5.3** — Caller/callee relationships via CALLER/CALLS edges from codegraph.db — already integrated
- **serde 1.0** — JSON serialization for new structs (SuggestedAction, ToolHints) — already in use
- **rusqlite 0.31** — Query relationships from codegraph.db — already installed

**Key insight:** The existing stack is sufficient. Avoid adding strsim (unnecessary for v2.2), regex (tree-sitter is more accurate), tokio (no async needed), or graph libraries (Magellan + SQLiteGraph already handle graphs).

### Expected Features

**From FEATURES.md:** Features organized by table stakes (must-have), differentiators (competitive advantage), and anti-features (explicitly NOT building).

**Must have (table stakes) — MVP blockers:**
- **Context extension** — Before/after/selected lines with configurable count — LLMs need context to make accurate edits without hallucinating surrounding structure
- **Semantic kind detection** — Per-language symbol kinds (function, class, variable, etc.) — Enables AST-aware operations instead of text-level patching
- **Checksums for race protection** — Content and file SHA-256 before modification — Prevents applying patches to shifted spans in multi-agent workflows

**Should have (competitive differentiators):**
- **Relationships block** — Callers/callees/imports/exports via Magellan CALLER/CALLS edges — LLMs can perform impact analysis before patching ("What breaks if I change this?")
- **Tool hints** — Behavioral flags (requires_full_context, apply_atomically) — Cross-tool coordination without LLM understanding language specifics
- **Unified error codes** — Machine-readable SPL-E### format with taxonomy — Automatic repair strategies, LLM retry logic without hallucination

**Defer to v2.3+:**
- **Suggested actions** — Action type taxonomy (rename, extract, inline) — Future-proofing, not immediate LLM need
- **Full LSP integration** — Belongs in llmfilewrite, not Splice

**Feature dependencies:** Context → Selected lines → Checksums (validation chain). Semantic kind → Tool hints → Suggested actions (intelligence layer). Relationships depend on Magellan integration.

### Architecture Approach

**From ARCHITECTURE.md:** Rich span extensions integrate cleanly into existing Rust/tree-sitter/SQLiteGraph architecture by extending current structures, not replacing them.

**Major components:**
1. **src/context.rs (NEW)** — Context extraction module using ropey for efficient line calculations — extracts before/selected/after lines around spans
2. **src/ingest/semantic_kind.rs (NEW)** — Semantic kind detection mapping tree-sitter node types to standardized kinds — per-language detection for 7 supported languages
3. **src/resolve/relationships.rs (NEW)** — Cross-file relationship builder traversing CodeGraph for callers/callees/imports/exports — lazy evaluation, depth-limited traversal
4. **src/output.rs::SpanResult (MODIFIED)** — Add 7 optional fields (context, semantic_kind, relationships, checksums, suggested_action, tool_hints) — backward compatible via `skip_serializing_if`
5. **src/checksum.rs (EXTEND)** — Span-level checksums already implemented, just expose in JSON output
6. **src/error_codes.rs (NEW)** — Error code registry with SPL-E### format, taxonomy, and documentation lookup
7. **src/suggest.rs (NEW)** — Suggested action engine for LLM guidance — starts with 3 primitives (delete, replace, expand)

**Data flow enrichment:** CLI Command → Ingest/Resolve → [Context Module + Relationship Builder + Checksum Module + Suggest Engine] → Enhanced SpanResult → JSON output. All extensions are "side channel" enrichment that doesn't break core validation pipeline.

**Build order:** Phase 1 (Foundation: error codes, SpanResult extension, checksums) → Phase 2 (Detection: semantic_kind, context, suggest) → Phase 3 (Graph: relationships) → Phase 4 (CLI integration) → Phase 5 (Testing).

### Critical Pitfalls

**From PITFALLS.md:** Performance, integration, design, and cross-cutting pitfalls with prevention strategies.

**Top 5 pitfalls:**

1. **Context extraction on large files** — Tree-sitter performance degrades on files >32KB, causing CLI hangs. **Prevention:** Lazy context loading (only via `-A`/`-B`/`-C` flags), span-based caching keyed by byte offsets, chunking strategy for files >100KB with `--max-context` flag.

2. **Full-codebase relationship graph O(n²) scalability** — Callers/callees queries become exponential slowdown. **Prevention:** Application-centered traversal (only from application functions, not stdlib), relationship indexing in SQLiteGraph, scope-aware queries (`--scope=file|module|workspace`), result caching.

3. **Race conditions in checksum validation** — File checksum computed during planning, file modified by external process before execution. **Prevention:** Atomic checksum-apply in single transaction, optimistic locking with `checksum_before` field, file-level advisory locks via `flock`, graceful degradation with 3-way merge suggestion.

4. **Breaking 334+ existing tests** — Adding rich span fields changes JSON structure, breaking test assertions. **Prevention:** Additive schema only (optional fields), `--fields` flag for field selection, schema versioning (`"schema_version": "2.2"`), use `assert_json_include!` pattern (subset checks) not exact match, back-compat mode with `--compat=v2.0`.

5. **Over-engineering suggested actions** — Complex nested structures with 20+ action types that LLMs ignore. **Prevention:** Start minimal with 3 actions (delete, replace, expand), action composition for complex operations, tool hints as booleans not objects, iterative expansion based on actual LLM usage patterns, defer to LLM for decisions (tool provides data, not commands).

**Additional risks:** Semantic kind edge cases (closures, macros, templates) — extended taxonomy with `kind` + `subkind` hierarchy. Error code taxonomy inconsistencies — planned ranges (E01X-E09X parse, E10X-E19X validation, E20X-E29X runtime). Breaking LLM compatibility — field aliases support both old and new names, versioned output `--output-schema=2.0|2.2`. Ignoring human usability — dual output modes (TTY default, JSON via flag), test both outputs.

---

## Implications for Roadmap

Based on combined research (stack, features, architecture, pitfalls), the recommended phase structure for Splice v2.2:

### Phase 11: Foundation Extensions (Error Codes + Output Schema)
**Rationale:** Lowest risk, no dependencies, enables all downstream phases. Extends existing types with optional fields, maintains backward compatibility.

**Delivers:** Unified error code registry (SPL-E### format), extended SpanResult with 7 new optional fields, span checksums exposed in JSON output.

**Addresses:** FEATURES.md — unified error codes, checksums for race protection

**Avoids:** PITFALLS.md #4 (breaking tests) via additive schema only, PITFALLS.md #7 (error code taxonomy) via planned ranges upfront

**Stack elements:** Pure constants (error codes), existing sha2 0.10 (checksums), serde 1.0 (JSON serialization)

**Architecture:** src/error_codes.rs (NEW), src/output.rs::SpanResult (MODIFIED), src/checksum.rs (EXTEND)

**Research flags:** None — standard error handling patterns, well-documented serde usage

---

### Phase 12: Context Extraction & Semantic Kind Detection
**Rationale:** Medium risk, clear dependencies on Phase 11 output structure. Core LLM value: context + semantics = 80% of value with 20% effort.

**Delivers:** Context extraction module (before/after/selected lines with configurable count), semantic kind detection for 7 languages (function, class, variable, parameter, type, constant, module), integration into CLI commands.

**Addresses:** FEATURES.md — context extension (table stakes), semantic kind detection (table stakes)

**Avoids:** PITFALLS.md #1 (large file performance) via lazy loading and span-based caching, PITFALLS.md #5 (semantic kind edge cases) via extended taxonomy with graceful fallback

**Stack elements:** tree-sitter 0.22 (Node API), ropey 1.6 (efficient line calculations)

**Architecture:** src/context.rs (NEW), src/ingest/semantic_kind.rs (NEW), CLI integration in src/main.rs

**Research flags:** **Phase 12 needs `/gsd:research-phase`** — tree-sitter node type → semantic kind mappings vary per language, need per-language verification for closures, macros, templates

---

### Phase 13: Tool Hints & Suggested Actions (Minimal)
**Rationale:** Low-MEDIUM risk, builds on Phase 12 semantic kind detection. Competitive differentiators with minimal implementation.

**Delivers:** Tool hints module (requires_full_context, apply_atomically, search_case_sensitive, language_hints), suggested actions starting with 3 primitives (delete, replace, expand), integration into JSON output.

**Addresses:** FEATURES.md — tool hints (differentiators), suggested actions (future-proofing)

**Avoids:** PITFALLS.md #6 (over-engineering) via starting with 3 primitives only, action composition for complex operations

**Stack elements:** serde 1.0 (JSON serialization)

**Architecture:** src/suggest.rs (NEW), tool_hints field in SpanResult

**Research flags:** **Phase 13 needs `/gsd:research-phase`** — LLM action taxonomy completeness, survey real LLM agents to see which JSON fields they actually use (avoid speculative feature building)

---

### Phase 14: Cross-File Relationships (Callers/Callees/Imports/Exports)
**Rationale:** HIGH risk, complex graph traversal, depends on Phase 12 semantic kind for filtering. Power feature that can be deferred if velocity issues.

**Delivers:** Relationship builder traversing CodeGraph for callers, callees, imports, exports, lazy evaluation (only via `--relationships` flag), depth limiting with `--max-depth <n>`.

**Addresses:** FEATURES.md — relationships block (differentiators)

**Avoids:** PITFALLS.md #2 (O(n²) scalability) via application-centered traversal, relationship indexing, scope-aware queries, result caching

**Stack elements:** magellan 0.5.3 (CALLER/CALLS edges), rusqlite 0.31 (query codegraph.db)

**Architecture:** src/resolve/relationships.rs (NEW), src/graph/mod.rs (MODIFIED to store semantic_kind)

**Research flags:** **Phase 14 needs `/gsd:research-phase`** — relationship graph schema edge type taxonomy, performance testing on 10K+ file codebases to validate mitigation strategies

---

### Phase 15: CLI Integration & Error Documentation
**Rationale:** MEDIUM risk, integrates all modules into user-facing commands. Phase 11-14 must be complete before this phase.

**Delivers:** CLI handler updates for context (`--context <n>`, `-A`/`-B`/`-C`), relationships (`--relationships`, `--max-depth <n>`), checksums (automatic on patch/delete), `splice explain <code>` command for error documentation.

**Addresses:** All FEATURES.md features (user-facing exposure)

**Avoids:** PITFALLS.md #9 (ignoring human usability) via dual output modes (TTY default, JSON via flag), test both outputs

**Stack elements:** All Phase 11-14 modules

**Architecture:** src/main.rs (MODIFIED), src/error_codes.rs lookup integration

**Research flags:** None — standard CLI patterns, well-documented clap usage

---

### Phase 16: Integration Testing & Validation
**Rationale:** REQUIRED phase — ensure all features work across 7 languages. Not optional.

**Delivers:** Integration tests for context extraction accuracy, semantic kind detection coverage per language, relationship graph correctness, checksum computation, error code mapping.

**Addresses:** All PITFALLS.md via comprehensive test coverage

**Avoids:** PITFALLS.md #4 (breaking tests) via golden test update scripts, additive schema validation

**Stack elements:** Existing test infrastructure (334+ tests passing)

**Architecture:** tests/ directory (EXTEND)

**Research flags:** None — test implementation, not research

---

### Phase Ordering Rationale

**Foundation-first approach:** Phase 11 extends existing types with zero breaking changes, enabling all downstream phases. Error codes and output schema are pure data structures — no dependencies, lowest risk.

**Core value next:** Phase 12 delivers 80% of LLM value (context + semantics) with 20% effort. These are table stakes that LLMs fundamentally require to make accurate edits.

**Intelligence layer:** Phase 13 builds on semantic kind to provide tool hints and minimal suggested actions. Differentiators that don't add significant risk.

**Power features last:** Phase 14 (relationships) is highest risk due to graph traversal performance. Can be deferred to v2.3 if velocity issues, as it's a "nice-to-have" not blocker.

**Integration and testing:** Phase 15-16 expose features to users and validate correctness. Phase 15 is CLI wiring (medium risk), Phase 16 is required validation.

**How this avoids pitfalls:**
- **PITFALL #1 (large file performance):** Addressed in Phase 12 via lazy loading, caching
- **PITFALL #2 (O(n²) scalability):** Addressed in Phase 14 via indexing, scope limits
- **PITFALL #3 (race conditions):** Addressed in Phase 11 via atomic checksum-apply
- **PITFALL #4 (breaking tests):** Addressed in Phase 11 via additive schema, Phase 16 via golden test scripts
- **PITFALL #6 (over-engineering):** Addressed in Phase 13 via 3 primitives only

### Research Flags

**Phases likely needing deeper research during planning:**

- **Phase 12 (Context & Semantic Kind):** Tree-sitter node type → semantic kind mappings vary per language. Need per-language verification for closures, macros, templates. Edge case: anonymous functions don't fit cleanly into taxonomy. **Action:** `/gsd:research-phase` before implementation.

- **Phase 13 (Tool Hints & Suggested Actions):** LLM action taxonomy completeness is speculative. Need survey of real LLM agents to see which JSON fields they actually use. Avoid "helpful" features LLMs ignore. **Action:** `/gsd:research-phase` before implementation.

- **Phase 14 (Relationships):** Relationship graph schema edge type taxonomy needs definition. Performance testing on 10K+ file codebases required to validate mitigation strategies. **Action:** `/gsd:research-phase` before implementation.

**Phases with standard patterns (skip research-phase):**

- **Phase 11 (Foundation):** Error handling and JSON serialization are well-documented patterns. No research needed.
- **Phase 15 (CLI Integration):** CLI wiring and command patterns are standard. No research needed.
- **Phase 16 (Testing):** Test implementation doesn't require research. No research needed.

---

## Confidence Assessment

| Area | Confidence | Notes |
|------|------------|-------|
| **Stack** | **HIGH** | Verified all capabilities against existing dependencies. No new dependencies needed. All source code checked (src/checksum.rs, src/output.rs, src/cli/mod.rs). |
| **Features** | **HIGH** | Table stakes (context, semantic kind, checksums) validated against industry standards (VS Code, IntelliJ, Addy Osmani workflow). Differentiators supported by community consensus. |
| **Architecture** | **HIGH** | Read all existing source files (src/lib.rs, src/output.rs, src/resolve/mod.rs, src/graph/mod.rs). Clear extension points identified. Data flow enrichment is "side channel" that doesn't break core pipeline. |
| **Pitfalls** | **HIGH** | Performance pitfalls validated by GitHub issues and research papers (Jarvis, Type-Based Call Graph). Test breaking pattern seen in v2.0 upgrade. LLM anti-patterns widely documented. |

**Overall confidence:** HIGH

**Gaps to Address:**

1. **Semantic kind mapping coverage:** Need comprehensive mapping of tree-sitter node types → semantic kinds for all 7 languages. **Handle during Phase 12 planning:** Document node type coverage per language, add "unknown" kind gracefully for edge cases.

2. **Relationship graph schema:** Need to define edge types for callers/callees/imports/exports in SQLiteGraph. **Handle during Phase 14 planning:** Map out edge type taxonomy, prototype indexes, benchmark query speedup.

3. **LLM action taxonomy:** Need to define action_type vocabulary and param schemas. **Handle during Phase 13 planning:** Survey real LLM agents (Claude Code, Cursor, Copilot) for actual field usage patterns.

4. **Performance benchmarks:** Need tests for relationship building at scale. **Handle during Phase 14 testing:** Test on 10K+ file codebases to validate mitigation strategies (lazy evaluation, depth limiting, indexing).

5. **Anonymous function detection:** Test tree-sitter behavior for closures in all 7 languages. **Handle during Phase 12 research:** Add test cases for anonymous functions, closures, arrows functions per language.

---

## Sources

### Primary (HIGH confidence — verified official docs/source code)

- **Splice v2.0 codebase:**
  - `/home/feanor/Projects/splice/src/checksum.rs` — Existing checksum implementation (lines 64-88)
  - `/home/feanor/Projects/splice/src/output.rs` — Existing SpanResult structure (lines 234-273)
  - `/home/feanor/Projects/splice/src/cli/mod.rs` — Existing SymbolKind enum (lines 273-298)
  - `/home/feanor/Projects/splice/src/symbol/mod.rs` — Symbol trait implementation
  - `/home/feanor/Projects/splice/src/error.rs` — Error type definitions

- **Official documentation:**
  - [tree-sitter Rust bindings](https://github.com/tree-sitter/tree-sitter/blob/master/lib/rust/binding.rs) — Node API for context extraction
  - [serde JSON serialization](https://serde.rs/) — Struct to JSON conversion
  - [SQLite Query Optimizer Overview](https://sqlite.org/optoverview.html) — Indexing strategies
  - [Command Line Interface Guidelines (clig.dev)](https://clig.dev/) — Human-centric CLI design
  - [VS Code Semantic Highlight Guide](https://code.visualstudio.com/api/language-extensions/semantic-highlight-guide) — Industry standard for context in code tools

### Secondary (MEDIUM confidence — verified community sources/academic papers)

- **Academic papers:**
  - [Jarvis: Application-Centered Call Graph Construction (arXiv 2024)](https://arxiv.org/html/2305.05949v3) — Proven solution for O(n²) scalability via application-centered traversal
  - [Type-Based Call Graph Construction (USENIX Sec 2023)](https://www.usenix.org/system/files/sec23winter-prepub-350-cai.pdf) — Scales to millions of LOC in minutes
  - [Towards Understanding Code Generation Errors (ICSE 2025)](https://arxiv.org/html/2512.05239v1) — Error code taxonomy methodology

- **Community resources:**
  - [Addy Osmani's LLM Coding Workflow 2026](https://addyosmani.com/blog/ai-coding-workflow/) — Industry expert perspective on context management
  - [Nuanced Call Graph Context Layer](https://www.nuanced.dev/blog/python-open-source-launch) — Call graph context for AI coding tools
  - [Tree-sitter AST Parsing at Scale (40 Languages)](https://www.dropstone.io/blog/ast-parsing-tree-sitter-40-languages) — Tree-sitter best practices
  - [Patterns and Anti-Patterns for Building with LLMs (Medium 2025)](https://medium.com/marvelous-mlops/patterns-and-anti-patterns-for-building-with-llms-42ea9c2ddc90) — LLM anti-patterns

### Tertiary (LOW confidence — web search only, needs validation)

- **Exploratory features (needs real-world validation):**
  - Semantic kind detection per-language mappings (need tree-sitter grammar verification per language)
  - Callers/callees query performance (need benchmarking on actual codebases)
  - Suggested action taxonomy completeness (exploratory, defer to v2.3+)
  - LLM field usage patterns (survey needed, avoid speculative building)

---

*Research synthesis completed: 2026-01-22*
*Ready for roadmap creation: yes*
*Confidence: HIGH — all research files synthesized, clear phase recommendations, research flags identify where deeper investigation needed*
