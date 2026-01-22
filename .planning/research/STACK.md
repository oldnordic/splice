# Technology Stack: Splice v2.2 Rich Span Extensions

**Research Date:** 2026-01-22
**Milestone:** v2.2 - Unified JSON Schema & LLM Optimization
**Research Focus:** Stack additions needed for rich span extensions

---

## Executive Summary

**Key Finding:** Splice v2.2 requires **ZERO new external dependencies** for rich span extensions. All required functionality can be built using:

1. **Existing stack** - tree-sitter 0.22, sha2 0.10, serde 1.0, rusqlite 0.31
2. **Magellan integration** - Magellan 0.5.3 already provides call graph data via `CALLER`/`CALLS` edges
3. **Pure Rust implementations** - String similarity, error codes, and context extraction using stdlib

**Confidence: HIGH** - All capabilities verified with existing codebase and official documentation.

---

## Stack Additions for v2.2

### Required Libraries

#### None

All v2.2 features can be implemented with existing dependencies:
- tree-sitter 0.22 (already installed)
- sha2 0.10 (already installed)
- serde 1.0 (already installed)
- rusqlite 0.31 (already installed)
- magellan 0.5.3 (already integrated)

### No Additional Dependencies

| Feature | Implementation Approach |
|---------|------------------------|
| **Context field** (before/selected/after lines) | Use tree-sitter `Node` API + ropey 1.6 for line extraction |
| **Semantic kind detection** | Use existing `SymbolKind` enum in `src/cli/mod.rs:273-298` |
| **Relationships** (callers/callees) | Query Magellan's `CALLER`/`CALLS` edges from codegraph.db |
| **Additional checksums** | Use existing `checksum::checksum_span()` from `src/checksum.rs:64` |
| **Suggested action metadata** | Add struct to `src/output.rs`, serialize with serde |
| **Tool hints** | Add struct to `src/output.rs`, serialize with serde |
| **Unified error codes** | Add constants to `src/error.rs`, no external lib needed |

---

## Detailed Implementation Approach

### 1. Context Field (before/selected/after lines)

**Existing capability:**
```rust
// From src/output.rs:233-333
pub struct SpanResult {
    pub file_path: String,
    pub byte_start: usize,
    pub byte_end: usize,
    pub line_start: usize,
    pub line_end: usize,
    // ...
}
```

**Add to SpanResult:**
```rust
/// Context lines surrounding the span
#[serde(skip_serializing_if = "Option::is_none")]
pub context: Option<SpanContext>,
```

**Implementation:**
- Use `ropey::Rope` to read file (already in Cargo.toml)
- Extract lines at `(line_start - N)` to `(line_end + N)`
- Parse with tree-sitter to verify semantic boundaries

**No new dependencies needed.**

### 2. Semantic Kind Detection

**Existing code:**
```rust
// From src/cli/mod.rs:273-298
pub enum SymbolKind {
    Function,
    Method,
    Class,
    Struct,
    Interface,
    Enum,
    Trait,
    Impl,
    Module,
    Variable,
    Constructor,
    TypeAlias,
}
```

**Implementation:**
- Map tree-sitter node types to `SymbolKind` enum
- Add `language: String` field to output (from existing `Language` enum)

**No new dependencies needed.**

### 3. Relationships (callers/callees/imports/exports)

**Existing Magellan integration:**
```rust
// Magellan provides CALLER and CALLS edges in codegraph.db
// From magellan/src/graph/call_ops.rs
pub fn callees_of_symbol(&mut self, symbol_id: i64) -> Result<Vec<CallFact>>
pub fn callers_of_symbol(&mut self, symbol_id: i64) -> Result<Vec<CallFact>>
```

**Implementation:**
1. Query `CALLER` edges (functions that call this symbol)
2. Query `CALLS` edges (functions called by this symbol)
3. Query import/export data (already in `src/ingest/imports/`)

**No new dependencies needed - use existing magellan 0.5.3.**

### 4. Additional Checksums

**Existing checksum module:**
```rust
// From src/checksum.rs:64-88
pub fn checksum_span(path: &Path, start: usize, end: usize) -> Result<Checksum>

// Already supports SHA-256 of byte spans
```

**Add to SpanResult:**
```rust
/// Checksum of span content before modification
#[serde(skip_serializing_if = "Option::is_none")]
pub checksum_before: Option<String>,

/// SHA-256 of entire file before operation
#[serde(skip_serializing_if = "Option::is_none")]
pub file_checksum_before: Option<String>,
```

**No new dependencies needed - sha2 0.10 already in use.**

### 5. Suggested Action Metadata

**New struct in src/output.rs:**
```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SuggestedAction {
    pub action_type: String,  // "rename", "delete", "extract", "inline"
    pub params: serde_json::Value,
}
```

**No new dependencies needed - serde 1.0 already in use.**

### 6. Tool Hints

**New struct in src/output.rs:**
```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolHints {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub requires_full_context: Option<bool>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub apply_atomically: Option<bool>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub search_case_sensitive: Option<bool>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub language_hints: Option<serde_json::Value>,
}
```

**No new dependencies needed - serde 1.0 already in use.**

### 7. Unified Error Codes

**New constants in src/error.rs:**
```rust
pub const SPLICE_E001: &str = "SPL-E001";
pub const SPLICE_E002: &str = "SPL-E002";
// ...
```

**Implementation:**
- Add `code` field to existing error types
- Map existing errors to SPL-E### format

**No new dependencies needed - pure constants.**

---

## Integration Points

### Existing Stack → New Features

| Existing Component | New Capability |
|--------------------|----------------|
| **tree-sitter 0.22** | Context extraction, semantic kind detection |
| **sha2 0.10** | Span checksums, file checksums for race protection |
| **magellan 0.5.3** | Caller/callee relationships via CALLER/CALLS edges |
| **serde 1.0** | Serialize all new structs to JSON |
| **rusqlite 0.31** | Query relationships from codegraph.db |
| **ropey 1.6** | Efficient line-based context extraction |

### Data Flow

```
1. User runs: splice get --symbol "process" --with-context --with-callers

2. Splice queries codegraph.db:
   - Symbol definition (via magellan)
   - CALLER edges (callers)
   - CALLS edges (callees)

3. Splice reads file with ropey:
   - Extract lines around symbol
   - Compute SHA-256 checksum

4. Splice parses with tree-sitter:
   - Detect semantic kind (function, struct, etc.)
   - Validate span boundaries

5. Splice serializes with serde:
   {
     "span": {...},
     "context": {before: [...], selected: [...], after: [...]},
     "semantic_kind": "function",
     "language": "rust",
     "relationships": {callers: [...], callees: [...]},
     "checksums": {checksum_before: "sha256:...", file_checksum_before: "..."}
   }
```

---

## Avoid These

### Libraries NOT to Add

| Library | Why NOT to Use It |
|---------|-------------------|
| **strsim** (for Levenshtein) | Unnecessary for v2.2 - fuzzy symbol matching not in requirements |
| **regex** | Tree-sitter is more accurate for code patterns; avoid regex for AST queries |
| **anyhow** | Splice uses `thiserror 1.0` - consistent error handling is better |
| **tokio** | No async needed - all operations are synchronous I/O |
| **additional tree-sitter parsers** | Only 7 languages supported - adding more is out of scope |
| **graph libraries** (petgraph, etc.) | Magellan + SQLiteGraph already handle graph operations |
| **LLM clients** (ollama-rs, etc.) | Splice is CLI-only, LLM integration is separate concern |

---

## Migration Path

### Phase Structure

Based on stack research, recommended phase ordering:

| Phase | Capability | Stack Dependencies |
|-------|------------|-------------------|
| **RICHSPAN-01** | Context field | tree-sitter, ropey |
| **RICHSPAN-02** | Semantic kind | tree-sitter, existing SymbolKind |
| **RICHSPAN-03** | Relationships | magellan CALLER/CALLS edges |
| **RICHSPAN-04** | Checksums | sha2 (already implemented) |
| **RICHSPAN-05** | Suggested action | serde (already in use) |
| **RICHSPAN-06** | Tool hints | serde (already in use) |
| **RICHSPAN-07** | Error codes | Pure constants (no dependencies) |

**All phases can proceed independently** - no blocking dependencies between them.

### Integration Testing

**Use existing test infrastructure:**
```rust
// From tests/ (334+ tests already passing)
// Add integration tests for:
// - Context extraction accuracy
// - Relationship query correctness
// - Checksum validation
// - Error code assignment
```

---

## Confidence Assessment

| Area | Confidence | Reasoning |
|------|------------|-----------|
| **No new dependencies needed** | **HIGH** | Verified all capabilities against existing stack |
| **Context extraction** | **HIGH** | ropey + tree-sitter well-understood patterns |
| **Semantic kind detection** | **HIGH** | `SymbolKind` enum already exists, tree-sitter node types documented |
| **Relationships** | **HIGH** | Magellan CALLER/CALLS edges verified in source code |
| **Checksums** | **HIGH** | `checksum_span()` already implemented in src/checksum.rs |
| **Error codes** | **HIGH** | Pure constants, zero external dependencies |
| **Tool hints** | **HIGH** | Simple JSON serialization, serde already in use |

---

## Verification

### Sources

**Official Documentation:**
- [tree-sitter Rust bindings](https://github.com/tree-sitter/tree-sitter/blob/master/lib/rust/binding.rs) - Node API for context extraction
- [Magellan call_ops.rs](https://github.com/oldnordic/magellan) - CALLER/CALLS edge implementation
- [serde JSON serialization](https://serde.rs/) - Struct to JSON conversion
- [sha2 Rust crate](https://docs.rs/sha2/) - Checksum computation

**Research Files:**
- `docs/UNIFIED_JSON_SCHEMA.md` - Complete specification of rich span extensions
- `src/checksum.rs` - Existing checksum implementation
- `src/output.rs` - Existing SpanResult structure
- `src/cli/mod.rs` - Existing SymbolKind enum
- `.planning/PROJECT.md` - v2.2 requirements

**Code Verification:**
- Verified Magellan provides `callers_of_symbol()` and `callees_of_symbol()` functions
- Verified `checksum_span()` exists in src/checksum.rs:64
- Verified `SymbolKind` enum has 12 variants covering all 7 languages
- Verified tree-sitter 0.22 is current stable release

---

## Summary

**Recommendation:** Proceed with v2.2 implementation using **existing stack only**.

**Rationale:**
1. All capabilities already available in dependencies
2. No version conflicts or breaking changes
3. Zero bloat - lean tooling remains lean
4. Faster implementation - no learning curve for new libraries
5. Maintains compatibility with Magellan 0.5.3

**Next Steps:**
1. Implement rich span structs in `src/output.rs`
2. Add context extraction using tree-sitter + ropey
3. Query Magellan for relationship data
4. Add checksum fields to existing SpanResult
5. Define error code constants in `src/error.rs`

---

*Research completed: 2026-01-22*
*Stack analysis: HIGH confidence - all capabilities verified*
