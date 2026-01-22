# Architecture for Rich Span Extensions

**Project:** Splice v2.2 Unified JSON Schema
**Researched:** 2026-01-22
**Overall confidence:** HIGH

## Executive Summary

Rich span extensions enhance Splice's existing `SpanResult` output structure with seven major additions: context lines, semantic kind detection, cross-file relationships, checksums for race protection, suggested actions for LLM guidance, tool hints, and unified error codes. These integrate cleanly into the existing Rust/tree-sitter/SQLiteGraph architecture by extending current structures rather than replacing them.

**Key architectural insight:** The existing `SpanResult` structure (lines 234-273 in `src/output.rs`) already contains the core fields needed for rich spans. The extensions add optional fields (`#[serde(skip_serializing_if = "Option::is_none")]`) that maintain backward compatibility while enabling the new features.

## New Components

### Context Extraction Module — `src/context.rs` (NEW)
- **Purpose:** Extract `before`, `selected`, and `after` lines around spans for context
- **Location:** New top-level module
- **Dependencies:**
  - `std::fs::File` — for reading source files
  - `ropey::Rope` — for efficient line/column calculations (already in dependencies)
  - `crate::error::{Result, SpliceError}` — for error handling
- **API:**
  ```rust
  pub fn extract_context(
      file_path: &Path,
      byte_start: usize,
      byte_end: usize,
      lines_before: usize,
      lines_after: usize,
  ) -> Result<ContextLines>
  ```
- **Integration point:** Called from CLI commands (`execute_query`, `execute_get`, `execute_delete`) before JSON serialization

### Semantic Kind Detection — `src/ingest/semantic_kind.rs` (NEW)
- **Purpose:** Map tree-sitter node types to semantic kinds (`function`, `variable`, `parameter`, `type`, etc.)
- **Location:** New submodule under `ingest/`
- **Dependencies:**
  - `tree_sitter::Node` — for accessing node kinds
  - `crate::ingest::Language` — for language-specific detection
  - Existing language-specific modules (`rust.rs`, `python.rs`, etc.)
- **API:**
  ```rust
  pub fn detect_semantic_kind(
      node: &tree_sitter::Node,
      language: Language,
  ) -> SemanticKind
  ```
- **Integration point:** Called during symbol extraction in `ingest/` modules, stored in graph as node property

### Cross-File Relationship Builder — `src/resolve/relationships.rs` (NEW)
- **Purpose:** Build full-codebase relationship graph (callers, callees, imports, exports)
- **Location:** New submodule under `resolve/`
- **Dependencies:**
  - `crate::graph::CodeGraph` — for querying symbol nodes and edges
  - `sqlitegraph::GraphBackend` — for graph traversal
  - `crate::resolve::ResolvedSpan` — for span metadata
- **API:**
  ```rust
  pub fn build_relationships(
      graph: &CodeGraph,
      symbol: &ResolvedSpan,
  ) -> Result<Relationships>

  pub struct Relationships {
      pub callers: Vec<RelatedSpan>,
      pub callees: Vec<RelatedSpan>,
      pub imports: Vec<RelatedSpan>,
      pub exports: Vec<RelatedSpan>,
  }
  ```
- **Integration point:** Called from CLI commands when output format includes relationships

### Span Checksum Service — `src/checksum.rs` (EXTEND)
- **Purpose:** Compute checksums for span content (currently only supports file-level and diff checksums)
- **Current state:** Has `checksum_file`, `checksum_span`, `checksum_line_range`, `checksum_diff`
- **Extension needed:** Add to `SpanResult` at creation time
- **Integration point:** Called from `SpanResult::from_byte_span()` and `SpanResult::from()`

### Error Code Registry — `src/error_codes.rs` (NEW)
- **Purpose:** Define stable error codes (SPL-E001 format) with taxonomy and documentation
- **Location:** New top-level module
- **Dependencies:** None (pure data)
- **Structure:**
  ```rust
  pub struct ErrorDefinition {
      pub code: &'static str,        // "SPL-E001"
      pub severity: ErrorSeverity,   // Error, Warning, Note
      pub category: ErrorCategory,   // ParseError, ValidationError, etc.
      pub message: &'static str,
      pub hint: Option<&'static str>,
      pub remediation: Option<&'static str>,
  }

  pub fn lookup_error(code: &str) -> Option<&'static ErrorDefinition>;
  ```
- **Integration point:** Used in CLI error formatting and `splice explain` command

### Suggested Action Engine — `src/suggest.rs` (NEW)
- **Purpose:** Generate suggested actions (action_type + params) for LLM guidance based on span context
- **Location:** New top-level module
- **Dependencies:**
  - `crate::ingest::Language` — for language-specific suggestions
  - `crate::symbol::Symbol` — for symbol metadata
- **API:**
  ```rust
  pub fn suggest_action(
      symbol: &dyn Symbol,
      context: &OperationContext,
  ) -> SuggestedAction

  pub struct SuggestedAction {
      pub action_type: ActionType,  // Rename, Extract, Inline, etc.
      pub params: serde_json::Value,
      pub confidence: f32,
  }
  ```
- **Integration point:** Added to `SpanResult` before JSON output

## Modified Components

### `src/output.rs` — SpanResult Structure (MODIFIED)
- **Current structure:** Lines 234-273 define `SpanResult` with 15 fields
- **Changes needed:** Add 7 new optional fields:
  ```rust
  // Context lines (RICHSPAN-01)
  #[serde(skip_serializing_if = "Option::is_none")]
  pub context_before: Option<Vec<String>>,

  #[serde(skip_serializing_if = "Option::is_none")]
  pub context_selected: Option<String>,

  #[serde(skip_serializing_if = "Option::is_none")]
  pub context_after: Option<Vec<String>>,

  // Semantic kind (RICHSPAN-02)
  #[serde(skip_serializing_if = "Option::is_none")]
  pub semantic_kind: Option<String>,

  // Relationships (RICHSPAN-03)
  #[serde(skip_serializing_if = "Option::is_none")]
  pub relationships: Option<Relationships>,

  // Suggested action (RICHSPAN-05)
  #[serde(skip_serializing_if = "Option::is_none")]
  pub suggested_action: Option<SuggestedAction>,

  // Tool hints (RICHSPAN-06)
  #[serde(skip_serializing_if = "Option::is_none")]
  pub tool_hints: Option<ToolHints>,
  ```
- **Breaking changes:** NO — all fields are optional with `skip_serializing_if`
- **Integration:** Update `SpanResult::from_byte_span()` and `SpanResult::from()` to accept new fields

### `src/resolve/mod.rs` — ResolvedSpan Structure (MODIFIED)
- **Current structure:** Lines 18-56 define `ResolvedSpan` with byte spans, line/col, language
- **Changes needed:** Add `semantic_kind` field (populated during resolution)
- **Integration:** Extend `resolve_symbol()` and `resolve_symbol_in_file()` to call semantic kind detection
- **Breaking changes:** NO — add as optional field

### `src/graph/mod.rs` — CodeGraph Storage (MODIFIED)
- **Current behavior:** Stores symbol nodes with `kind` (language-agnostic string)
- **Changes needed:** Add `semantic_kind` property to node data during ingestion
- **Integration:** Update `store_symbol_with_file_and_language()` to accept semantic kind
- **Breaking changes:** NO — property is optional in JSON data

### `src/error.rs` — SpliceError Enum (MODIFIED)
- **Current behavior:** Error variants with diagnostic extraction via `diagnostics()` method
- **Changes needed:** Add error code property to all error variants
- **Integration:**
  1. Add `pub code: Option<String>` field to `SpliceError` enum
  2. Map each variant to error code via new `error_codes::lookup_error()`
  3. Update `diagnostics()` to include error code in output
- **Breaking changes:** NO — error codes are added as optional field

### `src/main.rs` — CLI Command Handlers (MODIFIED)
- **Current behavior:** Commands create `SpanResult` and convert to JSON
- **Changes needed:**
  1. Call `context::extract_context()` before `SpanResult` creation
  2. Call `resolve::relationships::build_relationships()` for commands that need it
  3. Compute checksums via `checksum::checksum_span()` before output
  4. Add `--context <n>` flag handling for `-A`/`-B`/`-C` flags
  5. Add `--explain <code>` command handler
- **Breaking changes:** NO — additive changes to command flow

## Data Flow

### Existing Flow (v2.0 Baseline)

```
[CLI Command] → [Ingest/Resolve] → [Patch] → [Verify] → [Log] → [Output JSON]
                                                          ↓
                                                  SpanResult with basic fields
                                                  (file_path, byte_start/end,
                                                   line/col, symbol/kind)
```

### New Flow with Rich Span Extensions

```
[CLI Command] → [Ingest/Resolve] → [Patch] → [Verify] → [Log] → [Enrich SpanResult] → [Output JSON]
                                                          ↓
                                          ┌───────────────┴───────────────┐
                                          │                               │
                                   [Context Module]              [Relationship Builder]
                                   extract_context()              build_relationships()
                                          │                               │
                                          └───────────────┬───────────────┘
                                                          ↓
                                                   [Checksum Module]
                                                   checksum_span()
                                                          │
                                                          ↓
                                                   [Suggest Engine]
                                                   suggest_action()
                                                          │
                                                          ↓
                                              Enhanced SpanResult
                                              (all basic fields +
                                               context_lines +
                                               semantic_kind +
                                               relationships +
                                               checksums +
                                               suggested_action +
                                               tool_hints)
```

### Per-Feature Integration Flow

**1. Context Extraction (RICHSPAN-01)**
```
User runs: splice query --symbol foo --context 3
                ↓
CLI handler parses --context flag
                ↓
resolve::resolve_symbol() finds span
                ↓
context::extract_context() reads file, extracts lines
                ↓
SpanResult::from(resolved).with_context(context_lines)
                ↓
JSON output includes "context_before", "context_selected", "context_after"
```

**2. Semantic Kind Detection (RICHSPAN-02)**
```
User runs: splice get --file src/lib.rs --symbol foo
                ↓
ingest::extract_rust_symbols() parses with tree-sitter
                ↓
ingest::semantic_kind::detect_semantic_kind(node, language) maps node type
                ↓
graph::store_symbol_with_file_and_language(..., semantic_kind)
                ↓
resolve::resolve_symbol() retrieves node with semantic_kind
                ↓
SpanResult includes "semantic_kind": "function_definition"
```

**3. Cross-File Relationships (RICHSPAN-03)**
```
User runs: splice get --file src/lib.rs --symbol foo --relationships
                ↓
resolve::resolve_symbol() finds target symbol
                ↓
resolve::relationships::build_relationships(graph, symbol)
                ↓
Traverse CodeGraph:
  - Follow incoming edges → callers
  - Follow outgoing edges → callees
  - Query import/export edges → imports/exports
                ↓
SpanResult includes "relationships": {callers: [...], callees: [...]}
```

**4. Span Checksums (RICHSPAN-04)**
```
User runs: splice patch --file src/lib.rs --symbol foo --with new.rs
                ↓
resolve::resolve_symbol() finds span
                ↓
checksum::checksum_span(file, byte_start, byte_end) computes SHA-256
                ↓
Patch applied with validation
                ↓
SpanResult includes:
  "checksum_before": "abc123...",
  "checksum_after": "def456...",
  "file_checksum_before": "789ghi..."
```

**5. Suggested Actions (RICHSPAN-05)**
```
User runs: splice query --symbol foo
                ↓
resolve::resolve_symbol() gets span with semantic_kind
                ↓
suggest::suggest_action(symbol, operation_context)
                ↓
SpanResult includes:
  "suggested_action": {
    "action_type": "rename",
    "params": {"new_name": "bar"},
    "confidence": 0.92
  }
```

**6. Tool Hints (RICHSPAN-06)**
```
User runs: splice delete --file src/lib.rs --symbol foo
                ↓
resolve::resolve_symbol() finds span
                ↓
Analyze span properties (size, location, language)
                ↓
SpanResult includes:
  "tool_hints": {
    "requires_full_context": true,
    "apply_atomically": true,
    "verify_after": true
  }
```

**7. Error Codes (RICHSPAN-07)**
```
Command fails with SpliceError::SymbolNotFound
                ↓
error::SpliceError::diagnostics() called
                ↓
error_codes::lookup_error("SPL-E001") retrieves definition
                ↓
ErrorDetails populated:
  "kind": "SPL-E001",
  "severity": "error",
  "category": "SymbolResolution",
  "message": "Symbol 'foo' not found",
  "hint": "Did you mean 'bar'?"
```

## Build Order

Based on dependency analysis, the recommended build order:

### Phase 1: Foundation Extensions (Low Risk)
**Rationale:** Extend existing types with optional fields, no breaking changes

1. **Extend `src/error.rs` with error codes** — Enrich error types, add error code lookup
   - Enables: RICHSPAN-07 (Unified error codes)
   - Dependencies: None (pure data module)
   - Risk: LOW — additive changes only

2. **Extend `src/output.rs::SpanResult`** — Add new optional fields to structure
   - Enables: All RICHSPAN features (output structure)
   - Dependencies: None
   - Risk: LOW — all fields optional, backward compatible

3. **Extend `src/checksum.rs`** — Add span-level checksum computation
   - Enables: RICHSPAN-04 (Checksums for race protection)
   - Dependencies: None (module already exists)
   - Risk: LOW — pure function addition

### Phase 2: Detection & Extraction Modules (Medium Risk)
**Rationale:** New modules with clear dependencies on Phase 1

4. **Create `src/ingest/semantic_kind.rs`** — Semantic kind detection from tree-sitter nodes
   - Enables: RICHSPAN-02 (Semantic kind detection)
   - Dependencies: Phase 1 (output structure to store results)
   - Risk: MEDIUM — new module, tree-sitter API integration

5. **Create `src/context.rs`** — Context line extraction from source files
   - Enables: RICHSPAN-01 (Context field)
   - Dependencies: Phase 1 (output structure to store results)
   - Risk: MEDIUM — file I/O, line calculation with ropey

6. **Create `src/suggest.rs`** — Suggested action engine for LLM guidance
   - Enables: RICHSPAN-05 (Suggested actions)
   - Dependencies: Phase 1 (output structure), Phase 2 semantic_kind
   - Risk: MEDIUM — heuristic logic, requires testing

### Phase 3: Graph Relationship Integration (High Risk)
**Rationale:** Complex graph traversal, depends on ingestion and graph structure

7. **Create `src/resolve/relationships.rs`** — Cross-file relationship builder
   - Enables: RICHSPAN-03 (Full-codebase relationships)
   - Dependencies:
     - Phase 1 (output structure)
     - Phase 2 (semantic_kind for filtering)
     - Existing `graph::CodeGraph` integration
   - Risk: HIGH — graph traversal, potential performance impact

8. **Extend `src/graph/mod.rs`** — Store semantic_kind in graph nodes
   - Enables: RICHSPAN-02 (persist semantic kind), RICHSPAN-03 (relationship queries)
   - Dependencies:
     - Phase 2 semantic_kind module
     - Phase 3 relationship builder
   - Risk: MEDIUM — modifies core graph operations

### Phase 4: CLI Integration (Medium Risk)
**Rationale:** Integrates all modules into user-facing commands

9. **Update `src/main.rs` CLI handlers** — Wire context, relationships, checksums
   - Enables: All RICHSPAN features (CLI exposure)
   - Dependencies:
     - All Phase 1-3 modules
     - Existing CLI infrastructure
   - Risk: MEDIUM — multiple integration points, needs testing

10. **Add `splice explain` command** — Error code documentation lookup
    - Enables: RICHSPAN-07 (Error documentation)
    - Dependencies: Phase 1 error_codes module
    - Risk: LOW — simple read-only command

### Phase 5: Testing & Validation (Required)
**Rationale:** Ensure all features work across 7 languages

11. **Integration tests for rich spans** — Test all features across languages
    - Context extraction accuracy (line counts, boundaries)
    - Semantic kind detection coverage per language
    - Relationship graph correctness
    - Checksum computation correctness
    - Error code mapping completeness

## Performance Considerations

### Feature Impact Analysis

| Feature | Component Impact | Performance Risk | Mitigation |
|---------|------------------|------------------|------------|
| **Context extraction** | File I/O for context lines | LOW | Uses existing `ropey` Rope for efficient line calculation; lazy loading (only when requested via --context flag) |
| **Semantic kind detection** | Ingest-time node type mapping | LOW | Pure computation during ingestion; O(1) lookup table per language; cached in graph |
| **Relationship building** | Graph traversal for callers/callees | HIGH | **Potential bottleneck:** full-codebase graph traversal. Mitigation: (1) Lazy evaluation (only when `--relationships` flag used), (2) Limit traversal depth with configurable `--max-depth`, (3) Cache results per query session |
| **Span checksums** | SHA-256 computation per span | LOW-MEDIUM | SHA-256 is fast; impact scales with span count. Mitigation: Compute only for operations that modify code (patch/delete), not read-only queries |
| **Suggested actions** | Heuristic analysis per span | LOW | Simple rule-based logic; negligible overhead |
| **Error codes** | Error variant → code lookup | LOW | O(1) HashMap lookup; negligible overhead |
| **Tool hints** | Span property analysis | LOW | Simple boolean checks; negligible overhead |

### Scaling Considerations

**At 100 files:**
- Context extraction: Negligible (file already in memory for parsing)
- Semantic kind: Negligible (computed during ingest)
- Relationships: < 1ms per query (small graph)
- Checksums: < 10ms for batch operations

**At 10K files:**
- Context extraction: Still negligible (per-file operation)
- Semantic kind: Still negligible (computed once per ingest)
- Relationships: **100ms - 1s** for deep traversal (mitigation: limit depth, add indices)
- Checksums: < 100ms for batch (SHA-256 is fast)

**At 1M+ files (monorepo scale):**
- Context extraction: Negligible (per-file operation doesn't scale with repo size)
- Semantic kind: Negligible (computed once per ingest)
- Relationships: **SEVERAL SECONDS** for full traversal. **Mitigation critical:**
  - Default to shallow relationships (direct callers/callees only)
  - Require explicit `--recursive` flag for deep traversal
  - Add graph indices on relationship edge types
  - Consider incremental relationship caching
- Checksums: < 1s for batch (linear with number of changed files, not repo size)

### Memory Considerations

**Context lines:**
- 3 lines before/after × ~80 chars/line ≈ 480 bytes per span
- For 1000 spans: ~480 KB (negligible)

**Relationships:**
- Each related span: ~200 bytes (file_path, byte_start/end, symbol, kind)
- For 10 callers + 10 callees + 10 imports + 10 exports = 40 related spans
- 40 × 200 bytes = 8 KB per symbol (negligible)

**Checksums:**
- SHA-256 hex string = 64 bytes per checksum
- 3 checksums per span (before, after, file) = 192 bytes
- Negligible

**Conclusion:** Rich span extensions add minimal memory overhead (< 1MB even for large queries). The primary concern is graph traversal performance for relationships at scale.

## Integration Risks & Mitigation

### Risk 1: Graph Traversal Performance (Relationship Building)
**Severity:** HIGH
**Impact:** Full-codebase relationship queries could timeout on large codebases (10K+ files)
**Mitigation:**
- Lazy evaluation: Only build relationships when `--relationships` flag is present
- Depth limiting: Default to depth=1 (direct relationships), require `--recursive` for deeper
- Progress indication: Show progress bar for long-running traversals
- Caching: Cache relationship results per query session
- Future: Add specialized graph indices on relationship edge types

### Risk 2: Context Extraction Accuracy
**Severity:** MEDIUM
**Impact:** Incorrect line boundaries or UTF-8 handling could crash context extraction
**Mitigation:**
- Reuse existing `ropey::Rope` infrastructure (proven in patch module)
- Comprehensive tests for edge cases (file start/end, empty files, very long lines)
- Fallback to empty context on extraction errors (non-fatal)

### Risk 3: Semantic Kind Coverage
**Severity:** MEDIUM
**Impact:** Missing mappings for tree-sitter node types → fallback to generic "unknown" kind
**Mitigation:**
- Start with high-confidence mappings only (function, class, variable, parameter)
- Add "unknown" kind gracefully (doesn't break output, just less informative)
- Per-language test coverage for common node types
- Document extension points for adding new mappings

### Risk 4: Backward Compatibility
**Severity:** LOW
**Impact:** Existing tooling that consumes Splice JSON output might not handle new fields
**Mitigation:**
- All new fields use `#[serde(skip_serializing_if = "Option::is_none")]`
- Default to `None` when not explicitly populated
- Old tooling sees no new fields, new tooling gets rich data
- Document JSON schema evolution clearly

### Risk 5: Checksum Computation Overhead
**Severity:** LOW
**Impact:** SHA-256 computation adds latency to patch/delete operations
**Mitigation:**
- SHA-256 is fast (~200MB/s on modern CPUs)
- Only compute checksums for modifying operations (not read-only queries)
- Cache checksums during batch operations (compute once per file)

## Existing Architecture Compatibility

The rich span extensions are designed to integrate seamlessly with the existing Splice v2.0 architecture:

**1. SQLiteGraph Integration (No Changes Required)**
- Current: Stores symbol nodes with `kind`, `byte_start`, `byte_end`, `language`, `line_start`, `line_end`, `col_start`, `col_end`, `file_path`
- Extension: Add `semantic_kind` as additional node property (JSON data field)
- Compatibility: Fully backward compatible — old queries ignore new property

**2. Tree-Sitter Parsing (No Changes Required)**
- Current: Parse AST, extract symbols via language-specific modules
- Extension: Add semantic kind detection as post-processing step on `tree_sitter::Node`
- Compatibility: No changes to parsing logic, purely additive metadata extraction

**3. Validation Gates (No Changes Required)**
- Current: Tree-sitter reparse + compiler validation
- Extension: None required — rich span output doesn't affect validation logic
- Compatibility: Independent concern

**4. Execution Logging (No Changes Required)**
- Current: Log operations to `.splice/operations.db` with execution_id
- Extension: Rich span data is output-only, not persisted to execution log
- Compatibility: Execution log schema unchanged

**5. Patch Application (No Changes Required)**
- Current: `SpanReplacement` with file, start, end, content
- Extension: Checksums computed before/after patch, but doesn't affect patch logic
- Compatibility: Patch module unchanged, checksums are computed in parallel

## Success Criteria

Rich span extensions are complete when:

1. **Context extraction (RICHSPAN-01):**
   - [ ] `--context <n>` flag works for query, get, delete, patch commands
   - [ ] JSON output includes `context_before`, `context_selected`, `context_after` arrays
   - [ ] Context respects `-A`/`-B`/`-C` flags (CLI-02 requirement)
   - [ ] UTF-8 and CRLF line endings handled correctly

2. **Semantic kind detection (RICHSPAN-02):**
   - [ ] All 7 languages have semantic kind mappings for common node types
   - [ ] JSON output includes `semantic_kind` field
   - [ ] Coverage for: function, method, class, variable, parameter, type, constant, module

3. **Cross-file relationships (RICHSPAN-03):**
   - [ ] `--relationships` flag triggers relationship building
   - [ ] JSON output includes `relationships` with callers, callees, imports, exports arrays
   - [ ] Relationship traversal depth limited with `--max-depth <n>` flag
   - [ ] Performance acceptable on 1K file codebase (< 1s for shallow traversal)

4. **Span checksums (RICHSPAN-04):**
   - [ ] Patch/delete operations include `checksum_before`, `checksum_after`
   - [ ] All operations include `file_checksum_before`
   - [ ] Checksums verified to match SHA-256 of actual span content
   - [ ] Race protection: checksum mismatch returns error

5. **Suggested actions (RICHSPAN-05):**
   - [ ] Query/get operations include `suggested_action` field
   - [ ] Action types include: rename, extract, inline, move, delete
   - [ ] Confidence scores populated (0.0 - 1.0 range)
   - [ ] Params field includes action-specific parameters

6. **Tool hints (RICHSPAN-06):**
   - [ ] All span output includes `tool_hints` field
   - [ ] Hints include: `requires_full_context`, `apply_atomically`, `verify_after`
   - [ ] Hints based on span properties (size, location, language)

7. **Error codes (RICHSPAN-07):**
   - [ ] All errors include stable `SPL-E###` code
   - [ ] `splice explain <code>` command returns documentation
   - [ ] Error taxonomy documented (ParseError, ValidationError, etc.)
   - [ ] Compiler error codes extracted (Rust E0XXX, TypeScript TSXXXX)

## Confidence Assessment

| Area | Confidence | Reason |
|------|------------|--------|
| Integration points | HIGH | Read all existing source files (src/lib.rs, src/output.rs, src/resolve/mod.rs, src/graph/mod.rs, src/checksum.rs, src/error.rs) — clear extension points identified |
| New components design | HIGH | Follows existing patterns (e.g., `src/ingest/` language modules, `src/resolve/` for reference finding) — modular design with clear boundaries |
| Data flow changes | HIGH | Rich span enrichment is a "side channel" to existing flow — doesn't break core validation pipeline |
| Build order rationale | HIGH | Based on explicit dependency analysis — Phase 1 (foundation) → Phase 2 (extraction) → Phase 3 (relationships) → Phase 4 (integration) |
| Performance impact | HIGH | Analyzed each feature independently — only relationships have HIGH risk, with clear mitigation strategies |
| Backward compatibility | HIGH | All extensions use optional fields with `skip_serializing_if — JSON output evolves without breaking existing consumers |

**Overall confidence: HIGH**

The architecture for rich span extensions is well-defined, minimally invasive, and follows established patterns in the Splice codebase. The primary risk is graph traversal performance for relationships, which has multiple mitigation strategies. All other features are low-risk additive extensions.

## Gaps to Address

1. **Relationship graph schema:** Need to define edge types for callers/callees/imports/exports in SQLiteGraph
   - **Phase 3 research:** Map out edge type taxonomy and traversal queries

2. **Semantic kind mapping coverage:** Need comprehensive mapping of tree-sitter node types → semantic kinds for all 7 languages
   - **Phase 2 research:** Document node type coverage per language

3. **LLM action taxonomy:** Need to define action_type vocabulary and param schemas for LLM guidance
   - **Phase 2 research:** Survey LLM refactoring tools (GitHub Copilot, CodeLlama) for action conventions

4. **Performance testing:** Need benchmarks for relationship building at scale
   - **Phase 3 testing:** Test on 10K+ file codebases to validate mitigation strategies

---

*Architecture research complete: 2026-01-22*
*Ready for roadmap phase creation*
