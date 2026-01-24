# Common Pitfalls - Rich Span Extensions

**Research Date:** 2026-01-22
**Milestone:** v2.2 - Unified JSON Schema & LLM Optimization
**Confidence:** MEDIUM (research from WebSearch + codebase analysis)

---

## Performance Pitfalls

### 1. Context Extraction on Large Files

**What goes wrong:** Tree-sitter performance degrades significantly on files above 32KB, causing noticeable lag during context extraction. Real-time parsing during editing recalculates everything with each character change.

**Why it happens:**
- Tree-sitter reparses entire tree on each query for large files
- Context extraction (`before`/`selected`/`after`) triggers multiple tree traversals
- No caching mechanism for frequently queried spans

**Consequences:**
- CLI commands hang perceptibly on large files (>1000 lines)
- LLM tools timeout waiting for context responses
- Poor UX for interactive workflows

**Warning signs:**
- Commands taking >2 seconds on files with 500+ lines
- Memory usage spiking during context extraction
- User reports of "freeze" during symbol queries

**Prevention:**
- **Lazy context loading:** Only extract context when explicitly requested via `-A`/`-B`/`-C` flags
- **Span-based caching:** Cache tree-sitter nodes keyed by byte offsets, invalidate on file modification
- **Chunking strategy:** For files >100KB, offer `--max-context` flag to limit extraction window
- **Progress indicators:** Show parsing progress for files >500 lines

**Address in:** Phase 11 (Context Extraction)

**Sources:**
- [Tree-sitter performance issues on large files (GitHub discussions, 2025)](https://github.com/tree-sitter/tree-sitter/issues)
- [Slice: SAST + LLM Interprocedural Context Extractor (2025)](https://github.com/Pres Kush/slice) - uses depth-limited call graphs to control context size

---

### 2. Full-Codebase Relationship Graph Scalability

**What goes wrong:** Callers/callees/imports/exports queries across entire codebase become O(n) or worse, causing exponential slowdown as project grows.

**Why it happens:**
- Naive graph traversal without pruning explores all possible paths
- No indexing on relationship types (call vs import vs inherit)
- Full cross-file analysis for every query, even when single-file scope suffices

**Consequences:**
- `splice query` takes 30+ seconds on 10K LOC projects
- Memory exhaustion on large monorepos
- LLM agents give up on relationship queries

**Warning signs:**
- Query time doubling with each 2K LOC added
- SQLite "database is locked" errors during concurrent queries
- OOM kills on machines with <8GB RAM

**Prevention:**
- **Application-centered traversal:** Only construct call relations from application functions (Jarvis approach), not stdlib/deps
- **Relationship indexing:** Create separate indexes for calls, imports, inherits in SQLiteGraph
- **Scope-aware queries:** Add `--scope=file|module|workspace` flag to control traversal depth
- **Result caching:** Cache relationship query results keyed by graph modification time
- **Incremental updates:** Update relationship graph incrementally on file changes, not full rebuild

**Address in:** Phase 12 (Relationship Graph)

**Sources:**
- [Jarvis: Scalable and Precise Application-Centered Call Graph Construction (arXiv 2024)](https://arxiv.org/html/2305.05949v3)
- [Graph-Based Retrieval: How AI Code Agents Navigate Million-Line Codebases (Medium 2025)](https://medium.com/data-science-collective/graph-based-retrieval-how-ai-code-agents-navigate-million-line-codebases-96f22d702902)
- [Type-Based Call Graph Construction (USENIX Sec 2023)](https://www.usenix.org/system/files/sec23winter-prepub-350-cai.pdf) - scales to millions of LOC in minutes

---

## Integration Pitfalls

### 3. Race Conditions in Checksum Validation

**What goes wrong:** File checksum computed during planning phase, but file modified by external process (IDE auto-format, git pull, another LLM tool) before execution, causing false validation failures or silent corruption.

**Why it happens:**
- No file locking mechanism
- Time window between checksum computation and patch application
- Multiple concurrent tools (LLM agents, formatters) modifying same files

**Consequences:**
- Splice rejects valid patches due to checksum mismatch
- Silent data corruption when checksum passes but content changed
- Poor UX in multi-agent workflows

**Warning signs:**
- Intermittent `FileExternallyModified` errors in CI
- Patches succeed locally but fail in parallel test runs
- User reports: "worked before but now fails"

**Prevention:**
- **Atomic checksum-apply:** Compute checksum and apply patch in single transaction
- **Optimistic locking:** Store `checksum_before`, abort if mismatch with retry hint
- **File-level advisory locks:** Use `flock` on Unix for coordination (with timeout)
- **Graceful degradation:** If checksum mismatch detected, provide 3-way merge suggestion instead of hard fail
- **Conflict detection:** Include `checksum_before` in JSON output, let LLM detect conflicts

**Address in:** Phase 13 (Checksum Integration)

**Sources:**
- [Dynamic AST (DAST) with Semantic Anchoring (GitHub Issue 4730, 2025)](https://github.com/RooCodeInc/Roo-Code/issues/4730) - discusses file structure synchronization issues
- [SQLite Hybrid Graph Models (SQLite Forum 2025)](https://www.sqliteforum.com/p/sqlite-and-graph-hybrids) - transaction patterns for consistency

---

### 4. Breaking 334+ Existing Tests

**What goes wrong:** Adding rich span fields changes JSON output structure, breaking existing test assertions that expect specific schema.

**Why it happens:**
- Tests assert exact JSON structure without field filtering
- No schema versioning in output
- Backward compatibility not considered when adding fields

**Consequences:**
- Test suite regresses from 334 passing to 200+ failures
- CI blocks all progress
- Team loses confidence in changes

**Warning signs:**
- `cargo test` shows 50+ failures after adding new field
- Tests failing with "field `X` not found in JSON"
- Need to update 50+ test fixtures per feature

**Prevention:**
- **Additive schema:** Only add optional fields, never remove or rename existing ones
- **Field selection:** Add `--fields` flag to allow tests to request specific output schema
- **Schema versioning:** Include `"schema_version": "2.2"` in JSON output
- **Test fixtures:** Use `assert_json_include!` pattern (check subset of fields) not exact match
- **Golden test updates:** Script to update all golden files with new fields in bulk
- **Back-compat mode:** Add `--compat=v2.0` flag to suppress new fields for legacy consumers

**Address in:** Phase 11 (all phases)

**Sources:**
- [A Taxonomy of Compiler Error Messages (research 2025)](https://www.mdpi.com/2227-7390/13/19/3211) - discusses schema evolution in compiler tools

---

## Design Pitfalls

### 5. Semantic Kind Detection Edge Cases

**What goes wrong:** Anonymous functions, closures, macros, and template specializations don't fit cleanly into `function`/`variable`/`parameter` taxonomy, leading to misclassification or crashes.

**Why it happens:**
- Tree-sitter node types don't map 1:1 to semantic kinds
- Language-specific edge cases (Rust closures, C++ templates, JS arrow functions)
- Macros generate code that doesn't exist in source

**Consequences:**
- Wrong `kind` field in JSON output ("macro" classified as "function")
- LLM makes incorrect refactoring decisions based on bad kind
- Parse errors on complex Rust macros

**Warning signs:**
- `unwrap()` calls when accessing node types
- "Unknown node kind" log messages
- LLM tries to delete macro invocations as functions

**Prevention:**
- **Extended kind taxonomy:** Add `closure`, `macro`, `template`, `lambda` kinds to schema
- **Kind hierarchy:** Use `kind` + `subkind` (e.g., `{"kind": "function", "subkind": "closure"}`)
- **Language-specific overrides:** Allow per-language kind mappings in ingest modules
- **Graceful fallback:** When kind uncertain, use `"kind": "unknown"` with `node_type` from tree-sitter
- **Test edge cases:** Add test cases for anonymous functions in all 7 languages

**Address in:** Phase 14 (Semantic Kind Detection)

**Sources:**
- [Static Validation of C Preprocessor Macros (research)](https://ink.library.smu.edu.sg/cgi/viewcontent.cgi?article=1494&context=sis_research) - macro validation challenges
- [Metaprogramming - Julia Documentation](https://docs.julialang.org/en/v1/manual/metaprogramming/) - macros operating at AST level

---

### 6. Over-Engineering Suggested Actions and Tool Hints

**What goes wrong:** `suggested_action` and `tool_hints` fields become complex nested structures with 20+ action types, making JSON parsing difficult and LLMs ignore them.

**Why it happens:**
- Premature optimization: trying to predict every possible action
- No real-world usage data to guide design
- Confusing "helpful" with "feature-rich"

**Consequences:**
- LLMs hallucinate actions or ignore hints entirely
- JSON schema becomes 500+ lines of documentation
- Maintenance burden: every new feature requires updating 10+ action types

**Warning signs:**
- `suggested_action` has nested `params` with 10+ optional fields
- Tool hints include "maybe", "sometimes", "might" conditions
- Users complain: "I don't know what to do with this field"

**Prevention:**
- **Start minimal:** Only 3 actions in v2.2: `delete`, `replace`, `expand`
- **Action composition:** Complex actions built from primitives (e.g., "rename" = `delete` + `replace`)
- **Tool hints as booleans:** Simple flags like `requires_full_context`, not complex objects
- **Iterative expansion:** Add actions based on actual LLM usage patterns, not speculation
- **Defer to LLM:** Let LLM decide what to do, tool only provides data (not commands)

**Address in:** Phase 15 (Suggested Actions)

**Sources:**
- [Patterns and Anti-Patterns for Building with LLMs (Medium 2025)](https://medium.com/marvelous-mlops/patterns-and-anti-patterns-for-building-with-llms-42ea9c2ddc90) - "Start with the simplest viable solution"
- [My LLM coding workflow going into 2026 (Addy Osmani)](https://addyosmani.com/blog/ai-coding-workflow/) - emphasizes keeping tools simple

---

### 7. Error Code Taxonomy Inconsistencies

**What goes wrong:** Error codes like `SPL-E001` are assigned ad-hoc without taxonomy, leading to gaps (E001, E002, E050), inconsistent categories, and LLMs unable to interpret patterns.

**Why it happens:**
- No error code planning before implementation
- Different developers assign codes independently
- Error categories not considered (parse vs runtime vs validation)

**Consequences:**
- `splice explain` command can't route to correct documentation
- LLMs can't learn patterns (E0XX = parse, E1XX = runtime)
- Users can't remember error codes

**Warning signs:**
- Error codes skip numbers (E001, E002, E010, E011)
- Need lookup table to understand what E0042 means
- `explain` command uses if-else chain instead of pattern matching

**Prevention:**
- **Planned ranges:** Define code ranges before implementing:
  - `E01X-E09X`: Parse/syntax errors
  - `E10X-E19X`: Validation errors
  - `E20X-E29X`: Runtime I/O errors
  - `E30X-E39X`: Graph/query errors
- **Hierarchical codes:** Use `E0304` = graph error (03) subclass (04)
- **Documentation-first:** Write `explain` documentation before adding code
- **Code assignment:** Use macro to enforce range at compile time
- **LLM-friendly:** Include category in JSON: `"error_category": "parse"`

**Address in:** Phase 16 (Error Codes)

**Sources:**
- [Towards Understanding the Characteristics of Code Generation Errors (ICSE 2025)](https://arxiv.org/html/2512.05239v1) - comprehensive taxonomy through open coding
- [Design and Application of a C++ Compiler Error Solution Query Platform (ACM 2025)](https://dl.acm.org/doi/10.1145/3696002) - LLM technology for compiler error resolution

---

## Cross-Cutting Pitfalls

### 8. Breaking LLM Compatibility While "Improving" UX

**What goes wrong:** v2.2 makes JSON more human-readable (e.g., renaming `byte_start` to `start_byte`) but breaks existing LLM agents trained on v2.0 schema.

**Why it happens:**
- Human-centric design without considering LLM consumers
- No deprecation strategy for field renames
- Treating JSON as UI instead of API

**Consequences:**
- LLM agents (Claude Code, Cursor, Copilot) break on Splice v2.2
- Users stuck on v2.0 despite wanting new features
- Fragmented ecosystem

**Warning signs:**
- Proposal to rename "confusing" field names
- Schema change discussion doesn't mention LLM impact
- No backward compat flag proposed

**Prevention:**
- **Field aliases:** Support both old and new names in v2.2 (e.g., `byte_start` + `start_byte`)
- **Deprecation header:** Include `"deprecated_fields": ["byte_start"]` in JSON root
- **Versioned output:** Add `--output-schema=2.0|2.2` flag
- **LLM testing:** Include test suite with real LLM calls (not just schema validation)
- **Migration guide:** Document field mapping for LLM prompt engineers

**Address in:** Phase 11 (all phases)

**Sources:**
- [Best Practices to Build LLM Tools in 2025 (TechInfo 2025)](https://techinfotech.tech.blog/2025/06/09/best-practices-to-build-llm-tools-in-2025/) - tool-assisted validation strategies

---

### 9. Ignoring Human Usability in "LLM-First" Design

**What goes wrong:** So focused on LLM optimization that human CLI experience degrades (e.g., no unified diff, poor error messages, no dry-run).

**Why it happens:**
- "LLM-first" interpreted as "humans don't matter"
- LLM can parse messy JSON, humans can't
- No human testing in development workflow

**Consequences:**
- Humans stop using CLI directly, only through LLM wrappers
- Debugging becomes impossible (can't read raw JSON)
- Community loss: no human advocates for tool

**Warning signs:**
- Default output is machine-readable JSON
- Error messages are raw diagnostics without formatting
- No human-readable documentation, only API docs

**Prevention:**
- **Dual output modes:** Default to human-readable TTY output, JSON via `--json` flag
- **Test both outputs:** Every command has tests for both TTY and JSON modes
- **Human in loop:** Developer workflow requires using CLI directly (not just via LLM)
- **TTY detection:** Auto-detect terminal and format appropriately (colors, diff, tables)
- **Error messages:** Write for humans first, let LLM parse structured fields

**Address in:** Phase 11-15 (all CLI phases)

**Sources:**
- [Command Line Interface Guidelines (clig.dev)](https://clig.dev/) - human-centric CLI design
- [My LLM coding workflow going into 2026 (Addy Osmani)](https://addyosmani.com/blog/ai-coding-workflow/) - balance of AI + human collaboration

---

## Magellan Integration Pitfalls

**Note:** For detailed Magellan-specific pitfalls, see `MAGELLAN_INTEGRATION_PITFALLS.md`.

### 10. Magellan Query Command Delegation Risks

**What goes wrong:** Adding query command delegation to Magellan introduces flag conflicts, data format misalignment, and database path confusion.

**Key risks:**
- **Flag namespace collision** between Splice and Magellan CLI flags
- **Data format misalignment** breaking LLM consumption (Magellan's `SymbolQueryResult` vs Splice's `SpanResult`)
- **Database path confusion** — users don't know which `magellan.db` is being used
- **Performance collapse** on relationship queries without depth limiting
- **Test coverage gaps** at the Splice-Magellan integration boundary

**Prevention:**
- Use explicit flag namespacing (`--magellan-*` prefix) for delegated flags
- Ensure Splice's JSON output is a superset of Magellan's format
- Auto-detect Magellan database location with clear error messages
- Default to shallow relationship queries (depth=1)
- Add end-to-end tests for full delegation path

**Address in:** Phase 18 (Error Code Integration) and earlier phases

**Sources:**
- `src/graph/magellan_integration.rs` — Current Magellan integration
- `src/ingest/magellan.rs` — Magellan-based ingestion
- `.planning/research/MAGELLAN_INTEGRATION_PITFALLS.md` — Detailed analysis

---

## Summary: Phase-Specific Warnings

| Phase | Topic | Critical Pitfall | Mitigation Priority |
|-------|-------|------------------|---------------------|
| 11 | Context Extraction | Large file performance | HIGH - cache + lazy loading |
| 12 | Relationship Graph | O(n) scalability | HIGH - indexing + scope limits |
| 13 | Checksums | Race conditions | MEDIUM - optimistic locking |
| 14 | Semantic Kinds | Edge case misclassification | MEDIUM - extended taxonomy |
| 15 | Suggested Actions | Over-engineering | HIGH - start with 3 primitives |
| 16 | Error Codes | Taxonomy gaps | MEDIUM - plan ranges upfront |
| 18 | Magellan Integration | Flag conflicts + data alignment | HIGH - namespace + schema superset |

---

## Confidence Assessment

| Area | Confidence | Reason |
|------|------------|--------|
| Tree-sitter performance | HIGH | Multiple GitHub issues + research papers confirm |
| Relationship graph scalability | HIGH | Jarvis paper provides proven solution |
| Checksum race conditions | MEDIUM | Common problem but SQLite patterns well-documented |
| Test breaking | HIGH | Seen in v2.0 upgrade, predictable pattern |
| Semantic kind edge cases | MEDIUM | Language-specific, requires per-lang testing |
| Over-engineering | HIGH | LLM anti-patterns widely documented |
| Error code taxonomy | MEDIUM | Research shows taxonomies but Splice-specific needs unknown |
| LLM compat breaking | HIGH | Common API design mistake, preventable |
| Magellan integration | HIGH | Reviewed all integration source code |

---

## Gaps Requiring Phase-Specific Research

1. **Anonymous function detection:** Test tree-sitter behavior for closures in all 7 languages
2. **Macro expansion:** Determine if Splice should expand macros before semantic analysis
3. **LLM field usage:** Survey real LLM agents to see which JSON fields they actually use
4. **Checksum performance:** Benchmark SHA-256 overhead on large files (>1MB)
5. **Relationship indexing:** Prototype SQLiteGraph indexes and measure query speedup
6. **Magellan CLI semantics:** Verify Magellan's actual flag behavior with `magellan --help`

---

## Research Sources

**Official Documentation:**
- [SQLite Query Optimizer Overview (2025)](https://sqlite.org/optoverview.html)
- [Command Line Interface Guidelines (clig.dev)](https://clig.dev/)

**Academic Papers:**
- [Jarvis: Application-Centered Call Graph Construction (arXiv 2024)](https://arxiv.org/html/2305.05949v3)
- [Type-Based Call Graph Construction (USENIX Sec 2023)](https://www.usenix.org/system/files/sec23winter-prepub-350-cai.pdf)
- [Towards Understanding Code Generation Errors (ICSE 2025)](https://arxiv.org/html/2512.05239v1)

**Community Resources:**
- [Patterns and Anti-Patterns for Building with LLMs (Medium 2025)](https://medium.com/marvelous-mlops/patterns-and-anti-patterns-for-building-with-llms-42ea9c2ddc90)
- [Graph-Based Retrieval for Million-Line Codebases (Medium 2025)](https://medium.com/data-science-collective/graph-based-retrieval-how-ai-code-agents-navigate-million-line-codebases-96f22d702902)
- [Recast Security Best Practices: 7 Common Pitfalls (CSDN 2025)](https://blog.csdn.net/gitblog_00669/article/details/153756131)

**Codebase Analysis:**
- `/home/feanor/Projects/splice/src/symbol/mod.rs` - Symbol trait implementation
- `/home/feanor/Projects/splice/src/error.rs` - Error type definitions
- `/home/feanor/Projects/splice/src/graph/magellan_integration.rs` - Magellan integration layer
- `/home/feanor/Projects/splice/.planning/research/SUMMARY.md` - v2.1 research findings

---

*Research compiled: 2026-01-22*
*Updated: 2026-01-24 (added Magellan integration section)*
*Target milestone: Splice v2.2*
