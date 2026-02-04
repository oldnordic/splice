# Architecture: Magellan v2.0.0 Integration

**Project:** Splice v2.2.4 - Magellan Graph Algorithms & Cross-File Rename
**Researched:** 2026-02-04
**Overall confidence:** HIGH

## Executive Summary

Magellan v2.0.0 introduces **graph algorithms** (reachability, dead code detection, cycles, slicing) and **reference queries with byte spans** that enable **cross-file rename** operations in Splice. This architecture research focuses on integrating these NEW capabilities with Splice's existing refactoring infrastructure.

**Key architectural changes:**
1. **Cross-file rename with byte spans**: Use `CodeGraph::references_to_symbol()` for exact reference locations
2. **Graph algorithm CLI commands**: Expose reachability, dead code, cycles, slicing as `splice analyze` subcommands
3. **BLAKE3 SymbolId migration**: Magellan v2.0.0 uses 32-char BLAKE3 IDs (vs Splice's 16-char SHA-256)
4. **Proof generation**: `--proof` flag for rename operations to show before/after evidence
5. **Library API**: `GraphAnalysis` struct for programmatic access to algorithms

**Integration points:**
- `MagellanIntegration` (864 lines) is the primary wrapper for Magellan's `CodeGraph`
- `resolve/cross_file.rs` handles cross-file symbol resolution (implementation unknown)
- Graph algorithms are in `magellan::graph::algorithms` (reachable, dead, cycles, slice)
- `ReferenceFact` provides byte spans for references (already exists in Magellan v0.5.3)

**Existing integration status:**
- `src/graph/magellan_integration.rs`: Magellan wrapper with `query_by_labels()`, `get_code_chunk()`, `index_file()` methods
- `src/main.rs`: `execute_query()` and `execute_get()` already delegate to MagellanIntegration
- `src/output.rs`: `SpanResult`, `SymbolMatch` structures already Magellan-compatible
- `src/graph/mod.rs`: `CodeGraph` stores symbols using SQLiteGraph (separate from Magellan)

## Delegation Pattern

### Pattern: Library Delegation (Not Subprocess)

Splice calls Magellan as an **in-process Rust library**, not as a subprocess or HTTP API.

```rust
// Current pattern (already implemented in src/graph/magellan_integration.rs:19-29)
pub fn open(db_path: &Path) -> Result<Self> {
    let db_path_str = db_path.to_str()?;
    let inner = MagellanGraph::open(db_path_str)  // Direct library call
        .map_err(|e| SpliceError::Other(format!("Failed to open Magellan graph: {}", e)))?;
    Ok(Self { inner })
}
```

**Why library delegation:**
- **Zero serialization overhead:** Direct Rust function calls, no JSON/RPC
- **Shared database access:** Both Splice and Magellan can access the same SQLite file
- **Type safety:** Compile-time guarantees on Magellan API usage
- **Error handling:** Splice wraps Magellan's `anyhow::Error` into `SpliceError`

**Alternative considered (rejected):**
- Subprocess delegation (`magellan query --label rust`): Requires CLI parsing, JSON serialization, slower
- HTTP API delegation: Adds network overhead, requires separate Magellan server process

### Delegation Boundary

| Operation | Handler | Location |
|-----------|---------|----------|
| **Label-based queries** | Magellan (delegated) | `MagellanIntegration::query_by_labels()` |
| **Code chunk retrieval** | Magellan (delegated) | `MagellanIntegration::get_code_chunk()` |
| **File indexing** | Magellan (delegated) | `MagellanIntegration::index_file()` |
| **Symbol editing (patch)** | Splice (native) | `src/patch/` modules |
| **Symbol deletion** | Splice (native) | `src/resolve/` + `src/patch/` |
| **Relationship traversal** | Splice (native) | `src/resolve/relationships.rs` |

### Data Flow: Query Command

```
User: splice query --db codegraph.db --label rust --label fn
                |
                v
[src/main.rs: execute_query()]
                |
                +-- Parse CLI arguments (labels, context, relationships flags)
                |
                v
[MagellanIntegration::open(db_path)]  <-- Open Magellan database
                |
                v
[MagellanIntegration::query_by_labels(&["rust", "fn"])]  <-- DELEGATED to Magellan
                |
                +-- Returns Vec<SymbolInfo> {entity_id, name, file_path, kind, byte_start, byte_end}
                |
                v
[Splice: Enrich results]
                |
                +-- Add context lines (via context::extract_context)
                +-- Add semantic kind (via ingest::semantic_kind)
                +-- Add tool hints (via hints::derive_tool_hints)
                +-- Add suggested action (via action::suggest_action)
                +-- Add relationships (via CodeGraph, if --relationships flag)
                |
                v
[Splice: Convert to SpanResult]  <-- Splice's unified output format
                |
                v
[JSON output]  <-- Magellan-compatible JSON schema
```

### Data Flow: Get Command

```
User: splice get --db codegraph.db --file src/main.rs --start 100 --end 200
                |
                v
[src/main.rs: execute_get()]
                |
                +-- Parse CLI arguments (file, byte span, expand flag)
                |
                v
[MagellanIntegration::open(db_path)]
                |
                v
[MagellanIntegration::get_code_chunk(file, start, end)]  <-- DELEGATED to Magellan
                |
                +-- Returns Option<CodeChunk> {content, file_path, byte_start, byte_end, symbol_name, symbol_kind}
                |
                v
[Splice: Enrich result]
                |
                +-- Add context lines
                +-- Add semantic info (kind, language)
                +-- Add checksums (via checksum module)
                +-- Add error code (if applicable)
                |
                v
[JSON output]  <-- Magellan-compatible GetResponse format
```

## New Components

### Query Command Handler Module — `src/query.rs` (NEW)

**Purpose:** Centralize query delegation logic currently scattered in `src/main.rs`.

**Rationale:** Currently, `execute_query()` and `execute_get()` are 200+ line functions in `src/main.rs`. Extracting them to a dedicated module improves testability and separation of concerns.

**API:**
```rust
pub struct QueryExecutor {
    magellan: MagellanIntegration,
    code_graph: Option<CodeGraph>,  // Opened only if --relationships flag
}

impl QueryExecutor {
    pub fn new(db_path: &Path) -> Result<Self>;
    pub fn query_by_labels(
        &self,
        labels: &[String],
        context: ContextConfig,
        options: QueryOptions,
    ) -> Result<QueryResponse>;
    pub fn get_code_chunk(
        &self,
        file: &Path,
        start: usize,
        end: usize,
        context: ContextConfig,
        options: QueryOptions,
    ) -> Result<GetResponse>;
}

pub struct QueryOptions {
    pub relationships: bool,
    pub expand: bool,
    pub expand_level: usize,
    pub show_code: bool,
}
```

**Dependencies:**
- `crate::graph::magellan_integration::MagellanIntegration`
- `crate::graph::CodeGraph` (optional, for relationships)
- `crate::output::{SpanResult, QueryResult, GetResponse}`
- `crate::context` (for context extraction)
- `crate::ingest::semantic_kind` (for semantic enrichment)

**Integration point:** Replace inline `execute_query()` and `execute_get()` in `src/main.rs` with `QueryExecutor` calls.

### Symbol ID Generator — `src/symbol_id.rs` (NEW)

**Purpose:** Generate 16-character stable symbol IDs compatible with Magellan's symbol_id format.

**Rationale:** Magellan uses 16-character hex symbol IDs. Splice currently uses SHA-256 derived IDs (`generate_span_id()` in `src/output.rs:430-445`). For compatibility, Splice needs to generate Magellan-format IDs.

**API:**
```rust
/// Generate a 16-character hex symbol ID compatible with Magellan.
///
/// Magellan's symbol_id format: First 8 bytes of SHA-256 hash, hex-encoded.
/// Format: "a1b2c3d4e5f6g7h8" (16 lowercase hex chars)
pub fn generate_symbol_id(
    file_path: &str,
    symbol_name: &str,
    byte_start: usize,
) -> String;

/// Generate a match ID for query results.
///
/// Match IDs are unique per query result, not stable across runs.
pub fn generate_match_id(
    symbol_name: &str,
    file_path: &str,
    byte_start: usize,
) -> String;
```

**Implementation notes:**
- Use SHA-256 hash of `file_path:symbol_name:byte_start`
- Take first 8 bytes, convert to 16-character hex string
- Matches Magellan's internal ID generation

**Dependencies:**
- `sha2::Sha256` (already in dependencies)

**Integration point:** Call from `SpanResult::from_byte_span()` and `SymbolMatch::new()`.

### Format Alignment Module — `src/format/magellan.rs` (NEW)

**Purpose:** Ensure Splice's JSON output is byte-for-byte compatible with Magellan's query response format.

**Rationale:** LLM tools consuming Splice output expect Magellan-compatible JSON. This module provides conversion functions and validation.

**API:**
```rust
/// Convert Splice SpanResult to Magellan SymbolMatch.
pub fn span_result_to_symbol_match(span: SpanResult) -> SymbolMatch;

/// Convert Splice QueryResult to Magellan LabelQueryResponse.
pub fn query_result_to_magellan_response(result: QueryResult) -> LabelQueryResponse;

/// Validate JSON output matches Magellan schema.
#[cfg(test)]
pub fn validate_magellan_compatibility(json: &serde_json::Value) -> Result<()>;
```

**Schema alignment points:**
- `span_id`: Splice uses SHA-256 (64 hex chars), Magellan uses 16-char hex
- `symbol_id`: Splice currently None, needs to match Magellan format
- `match_id`: Splice uses UUID, Magellan uses stable hash
- `semantics.kind`: Splice uses `SemanticKind` enum, Magellan uses string labels

**Dependencies:**
- `crate::output::{SpanResult, SymbolMatch, QueryResult}`

**Integration point:** Call from `execute_query()` and `execute_get()` before JSON serialization.

## Modified Components

### `src/main.rs` — CLI Command Handlers (MODIFIED)

**Current state:**
- `execute_query()` (lines 1982-2360): 378 lines, handles all query logic inline
- `execute_get()` (lines 2502-2750): 248 lines, handles all get logic inline

**Changes needed:**
1. **Extract to `src/query.rs`:** Move `execute_query()` and `execute_get()` logic to `QueryExecutor`
2. **Simplify handlers:** Replace inline logic with `QueryExecutor` calls
3. **Add `--db` flag handling:** Ensure all query commands accept `--db` flag consistently

**Before:**
```rust
fn execute_query(...) -> Result<...> {
    // 378 lines of query logic
    let integration = MagellanIntegration::open(db_path)?;
    let results = integration.query_by_labels(&labels_ref)?;
    // ... enrichment logic
}
```

**After:**
```rust
fn execute_query(...) -> Result<...> {
    let executor = QueryExecutor::new(db_path)?;
    let options = QueryOptions {
        relationships,
        expand,
        expand_level,
        show_code,
    };
    let response = executor.query_by_labels(&labels, context_config, options)?;
    Ok(CliSuccessPayload::with_data(message, serde_json::to_value(response)?))
}
```

**Breaking changes:** NO — CLI interface unchanged, only internal refactoring

### `src/cli/mod.rs` — CLI Argument Structure (MODIFIED)

**Current state:** `Query` and `Get` commands already defined (lines 241-329)

**Changes needed:**
1. **Add `--limit` flag:** Limit number of results returned
2. **Add `--offset` flag:** Pagination support
3. **Add `--format` flag:** Choose between "splice" (rich) or "magellan" (minimal) output

**Added fields:**
```rust
Query {
    // ... existing fields ...

    /// Maximum number of results to return.
    #[arg(long, default_value = "100")]
    limit: usize,

    /// Skip first N results (for pagination).
    #[arg(long, default_value = "0")]
    offset: usize,

    /// Output format: "splice" (rich) or "magellan" (minimal).
    #[arg(long, value_name = "FORMAT", default_value = "splice")]
    format: String,
}
```

**Breaking changes:** NO — new fields are optional with defaults

### `src/graph/magellan_integration.rs` — Magellan Wrapper (EXTEND)

**Current state:** Basic wrapper with `query_by_labels()`, `get_code_chunk()`, `index_file()`

**Changes needed:**
1. **Add pagination support:** `query_by_labels_paginated(labels, offset, limit)`
2. **Add label listing:** `get_all_labels()` already exists (line 69-73)
3. **Add label counting:** `count_by_label()` already exists (line 76-80)
4. **Add symbol lookup:** `get_symbol_by_id(symbol_id)` for ID-based queries

**New methods:**
```rust
impl MagellanIntegration {
    /// Query symbols with pagination.
    pub fn query_by_labels_paginated(
        &self,
        labels: &[&str],
        offset: usize,
        limit: usize,
    ) -> Result<Vec<SymbolInfo>>;

    /// Get symbol by its 16-character symbol ID.
    pub fn get_symbol_by_id(&self, symbol_id: &str) -> Result<Option<SymbolInfo>>;

    /// Get all symbols for pagination metadata (total count).
    pub fn count_symbols_with_labels(&self, labels: &[&str]) -> Result<usize>;
}
```

**Breaking changes:** NO — additive methods only

### `src/output.rs` — JSON Output Types (MODIFIED)

**Current state:** `SpanResult`, `SymbolMatch`, `QueryResult`, `GetResponse` defined

**Changes needed:**
1. **Add `symbol_id` field:** 16-character hex string compatible with Magellan
2. **Shorten `span_id`:** Option to use 16-char format instead of 64-char SHA-256
3. **Add `total_count` to `QueryResult`:** For pagination metadata

**Modified structures:**
```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SymbolMatch {
    /// Stable match ID (16-char hex for Magellan compatibility)
    pub match_id: String,
    /// Symbol span
    pub span: Span,
    /// Symbol name
    pub name: String,
    /// Symbol kind (normalized)
    pub kind: String,
    /// Parent symbol name (if nested)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub parent: Option<String>,
    /// Stable symbol ID (16-char hex, Magellan-compatible)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub symbol_id: Option<String>,  // NEW: Always populated in queries
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueryResult {
    // ... existing fields ...

    /// Total number of results (for pagination)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub total_count: Option<usize>,  // NEW: Populated when using --limit/--offset
}
```

**Breaking changes:** NO — new fields are optional

## Data Format Alignment

### Symbol ID Format

| Property | Splice Current | Magellan | Alignment Strategy |
|----------|---------------|----------|-------------------|
| `symbol_id` length | 64 chars (SHA-256 hex) | 16 chars (first 8 bytes of SHA-256) | Splice adopts Magellan 16-char format for queries |
| `symbol_id` stability | Per-session UUID | Content-based hash | Splice uses content hash for `symbol_id` |
| `match_id` format | UUID v4 | Content-based hash | Splice keeps UUID for match uniqueness, adds Magellan-compatible `symbol_id` |

### JSON Schema Alignment

**Splice `QueryResult` (current):**
```json
{
  "labels": ["rust", "fn"],
  "count": 42,
  "symbols": [
    {
      "span_id": "a1b2c3d4e5f6...",  // 64-char SHA-256
      "file_path": "src/main.rs",
      "byte_start": 100,
      "byte_end": 200,
      "symbol": "main",
      "kind": "function",
      "match_id": "uuid-v4-here",  // UUID
      "semantics": {"kind": "function", "language": "rust"}
    }
  ]
}
```

**Magellan `LabelQueryResponse` (target):**
```json
{
  "labels": ["rust", "fn"],
  "count": 42,
  "symbols": [
    {
      "match_id": "a1b2c3d4e5f6g7h8",  // 16-char content hash
      "span": {
        "span_id": "a1b2c3d4e5f6g7h8",  // 16-char
        "file_path": "src/main.rs",
        "byte_start": 100,
        "byte_end": 200,
        "start_line": 10,
        "start_col": 0,
        "end_line": 15,
        "end_col": 1
      },
      "name": "main",
      "kind": "fn",
      "symbol_id": "i8j7k6l5m4n3o2p1"  // 16-char, separate from span_id
    }
  ]
}
```

**Alignment strategy:**
1. Splice keeps rich fields (`semantics`, `tool_hints`, `suggested_action`) for LLM value
2. Splice adds `symbol_id` field in Magellan 16-char format
3. Splice changes `match_id` to match Magellan content-based hash
4. Splice provides `--format magellan` flag to exclude rich fields for strict compatibility

### Format Modes

| Mode | Fields | Use Case |
|------|--------|----------|
| `--format splice` (default) | All Splice fields + Magellan-compatible IDs | LLM consumption, maximum context |
| `--format magellan` | Magellan-only fields | Tools expecting exact Magellan output |

## Build Order

Based on dependency analysis and integration points:

### Step 1: Symbol ID Generator Foundation
**Create `src/symbol_id.rs`**
- **Why first:** All other components depend on consistent symbol ID generation
- **Dependencies:** None (uses `sha2` already in dependencies)
- **Risk:** LOW — pure function module
- **Tests:** Unit tests for determinism, uniqueness

### Step 2: Format Alignment Module
**Create `src/format/mod.rs` and `src/format/magellan.rs`**
- **Why second:** Provides conversion utilities for integration
- **Dependencies:** Step 1 (symbol_id generator)
- **Risk:** LOW — conversion functions, no side effects
- **Tests:** Validate JSON schema compatibility

### Step 3: Magellan Integration Extensions
**Extend `src/graph/magellan_integration.rs`**
- **Why third:** Adds pagination and ID-based queries needed by executor
- **Dependencies:** None (extends existing wrapper)
- **Risk:** MEDIUM — wraps Magellan library API
- **Tests:** Integration tests with test database

### Step 4: Query Executor Module
**Create `src/query.rs`**
- **Why fourth:** Centralizes query logic, uses all previous components
- **Dependencies:**
  - Step 1 (symbol_id for ID generation)
  - Step 2 (format conversion)
  - Step 3 (Magellan integration)
- **Risk:** MEDIUM — refactors existing `main.rs` logic
- **Tests:** Unit tests for executor, integration tests with Magellan

### Step 5: CLI Argument Extensions
**Modify `src/cli/mod.rs`**
- **Why fifth:** Adds `--limit`, `--offset`, `--format` flags
- **Dependencies:** None (CLI-only)
- **Risk:** LOW — additive changes to argument parsing
- **Tests:** CLI parsing tests

### Step 6: Main.rs Refactoring
**Modify `src/main.rs`**
- **Why sixth:** Replaces inline query logic with `QueryExecutor` calls
- **Dependencies:** Step 4 (QueryExecutor)
- **Risk:** MEDIUM — refactors existing code, requires careful testing
- **Tests:** Integration tests for full CLI flow

### Step 7: Output Schema Extensions
**Modify `src/output.rs`**
- **Why seventh:** Adds `symbol_id`, `total_count` fields to output types
- **Dependencies:** Step 1 (symbol_id generator)
- **Risk:** LOW — additive optional fields
- **Tests:** JSON serialization tests, backward compatibility

### Step 8: Documentation and Examples
**Create docs/magellan_integration.md**
- **Why eighth:** Documents integration patterns for users
- **Dependencies:** All previous steps complete
- **Risk:** NONE — documentation only
- **Tests:** Example validation (run examples in docs)

## Integration Risks & Mitigation

### Risk 1: Symbol ID Collision
**Severity:** MEDIUM
**Impact:** Magellan and Splice generate different IDs for the same symbol → confusion in downstream tools
**Mitigation:**
- Verify Magellan's ID generation algorithm matches Splice implementation
- Test with real Magella database to ensure ID consistency
- Document ID format clearly for consumers
- Add validation tests for ID uniqueness

### Risk 2: JSON Schema Drift
**Severity:** LOW
**Impact:** Splice adds rich fields that Magellan consumers don't expect → parsing errors
**Mitigation:**
- Use `#[serde(skip_serializing_if = "Option::is_none")]` for all Splice-specific fields
- Provide `--format magellan` flag for strict compatibility
- Document schema differences clearly
- Validate against Magella's test cases

### Risk 3: Main.rs Refactoring Complexity
**Severity:** MEDIUM
**Impact:** Refactoring 600+ lines of query logic could introduce bugs
**Mitigation:**
- Extract logic incrementally (query first, then get)
- Maintain existing tests as refactoring safety net
- Add new tests for `QueryExecutor` before removing old code
- Keep old functions as deprecated wrappers during transition

### Risk 4: Magell an API Changes
**Severity:** LOW
**Impact:** Magell 0.5.x API changes could break integration
**Mitigation:**
- Pin Magell version in Cargo.toml (`= "0.5.3"`)
- Wrap Magell API behind `MagellanIntegration` abstraction layer
- Monitor Magell releases for breaking changes
- Version compatibility tests

### Risk 5: Performance Regression
**Severity:** LOW
**Impact:** Additional format conversions and ID generation slow queries
**Mitigation:**
- Benchmark query performance before/after
- Optimize ID generation (cache SHA-256 hasher)
- Lazy evaluation for format conversions
- Only compute `symbol_id` when needed (query output)

## Dependencies on Existing Components

### Required (Already Implemented)
- `src/graph/magellan_integration.rs`: Magell wrapper (HIGH confidence, production-ready)
- `src/main.rs::execute_query()`: Query delegation logic (needs refactoring)
- `src/main.rs::execute_get()`: Get command logic (needs refactoring)
- `src/output.rs`: `SpanResult`, `SymbolMatch`, `QueryResult` (needs `symbol_id` field)
- `src/context.rs`: Context extraction (HIGH confidence, used by queries)
- `src/ingest/semantic_kind.rs`: Semantic kind detection (HIGH confidence)
- `src/hints.rs`: Tool hints derivation (HIGH confidence)
- `src/action.rs`: Suggested actions (HIGH confidence)

### Optional (For Full Feature Set)
- `src/resolve/relationships.rs`: Cross-file relationships (needed for `--relationships` flag)
- `src/checksum.rs`: Span checksums (needed for checksum fields in output)
- `src/expand.rs`: Symbol expansion (needed for `--expand` flag)

## Database Schema Considerations

### Magellan Database (Separate from Splice Graph)

**Location:** User-specified via `--db` flag

**Contents:**
- Indexed symbols with labels (language, kind)
- Code chunks (source code stored by byte span)
- Entity graph (nodes and edges)

**Access pattern:** Read-only for queries (Splice never writes to Magellan database)

### Splice Graph Database (Separate from Magellan)

**Location:** `.splice_graph.db` or project-specific

**Contents:**
- Symbol nodes with DEFINES edges from File nodes
- Span metadata (byte_start, byte_end, line/col)
- Relationship edges (callers, callees, imports, exports)

**Access pattern:** Read-write for edits, read-only for relationships

**Key insight:** Splice and Magella maintain separate databases. Splice does NOT write to Magella's database. Delegation is query-only.

## Confidence Assessment

| Area | Confidence | Reason |
|------|------------|--------|
| Delegation pattern | HIGH | Read `src/graph/magellan_integration.rs` (256 lines) — library delegation already implemented and working |
| Query executor design | HIGH | Based on existing `execute_query()` and `execute_get()` patterns — clear extraction path |
| Symbol ID generation | MEDIUM | Inferred from Magell usage; LOW confidence on exact algorithm without Magell source verification |
| Format alignment | HIGH | Read `src/output.rs` — schema differences clear, alignment strategy straightforward |
| Main.rs refactoring | HIGH | Existing functions are modular; extraction to module low-risk |
| CLI flag additions | HIGH | Clap pattern established; additive changes only |
| Database separation | HIGH | Confirmed by reading `src/graph/mod.rs` — Splice uses SQLiteGraph, Magell uses its own storage |

**Overall confidence: HIGH**

The delegation pattern is already implemented and working in production. The remaining work is refactoring for clarity, adding pagination, and ensuring format alignment. The primary uncertainty is Magell's exact symbol ID generation algorithm, which can be verified through testing.

## Gaps to Address

1. **Magella symbol ID algorithm:** Need to verify exact hash input for 16-char symbol IDs
   - **Validation:** Test against real Magella database with known symbols
   - **Fallback:** Use Splice's content-based hash if algorithm differs

2. **Pagination behavior:** Need to define Magella's pagination semantics
   - **Question:** Does Magella support offset/limit natively?
   - **Fallback:** Implement client-side pagination if Magella lacks it

3. **Error code alignment:** Magella may have different error codes
   - **Action:** Map Magella errors to Splice error codes
   - **Document:** Error code translation table

4. **Performance benchmarks:** Need baseline query performance
   - **Action:** Benchmark `execute_query()` on 1K/10K file codebases
   - **Target:** < 100ms for typical queries

## Sources

- **Source code analysis:** Read `/home/feanor/Projects/splice/src/main.rs` (lines 1982-2750) — query/get command implementation
- **Source code analysis:** Read `/home/feanor/Projects/splice/src/graph/magellan_integration.rs` (256 lines) — Magella wrapper
- **Source code analysis:** Read `/home/feanor/Projects/splice/src/output.rs` (1090 lines) — JSON output types
- **Source code analysis:** Read `/home/feanor/Projects/splice/src/cli/mod.rs` (728 lines) — CLI argument structure
- **Source code analysis:** Read `/home/feanor/Projects/splice/src/resolve/mod.rs` (543 lines) — symbol resolution
- **Dependency check:** `/home/feanor/Projects/splice/Cargo.toml` — `magellan = "0.5.3"` dependency confirmed
- **Existing research:** `/home/feanor/Projects/splice/.planning/research/ARCHITECTURE.md` — Rich span architecture context

---

*Architecture research complete: 2026-01-24*
*Focus: Magellan query delegation integration with existing Splice architecture*
*Ready for roadmap phase creation*

---

# Magellan v2.0.0 Capabilities Integration

**Research Date:** 2026-02-04
**Focus:** Graph algorithms, cross-file rename, BLAKE3 IDs

## NEW Magellan v2.0.0 Features

### 1. Graph Algorithms (magellan::graph::algorithms)

Magellan v2.0.0 provides static analysis algorithms via `sqlitegraph`:

```rust
// From magellan/src/graph/algorithms.rs
pub struct SymbolInfo {
    pub symbol_id: Option<String>,  // 32-char BLAKE3
    pub fqn: Option<String>,
    pub file_path: String,
    pub kind: String,
}

// Reachability analysis
graph.reachable_symbols(symbol_id, max_depth)?
graph.reverse_reachable_symbols(symbol_id, max_depth)?

// Dead code detection
graph.dead_symbols(entry_symbol_id)?
// Returns: Vec<DeadSymbol> { symbol: SymbolInfo, reason: String }

// Cycle detection
graph.detect_cycles()?
// Returns: CycleReport { cycles: Vec<Cycle>, total_count }
graph.find_cycles_containing(symbol_id)?
graph.condense_call_graph()?
// Returns: CondensationGraph with supernodes (SCCs collapsed to DAG)

// Path enumeration
graph.enumerate_paths(from_id, to_id)?
// Returns: PathEnumerationResult { paths: Vec<ExecutionPath> }

// Program slicing
graph.backward_slice(symbol_id)?
graph.forward_slice(symbol_id)?
// Returns: SliceResult { slice: ProgramSlice, statistics }
```

**Key Types:**
- `SymbolInfo`: {symbol_id (BLAKE3), fqn, file_path, kind}
- `DeadSymbol`: SymbolInfo + reason (unreachable from entry)
- `Cycle`: members (Vec<SymbolInfo>), kind (MutualRecursion/SelfLoop)
- `Supernode`: SCC collapsed into single node (id, members)
- `ProgramSlice`: target, direction, included_symbols

### 2. Reference Queries with Byte Spans

**CRITICAL:** This already exists in Magellan v0.5.3. Splice just needs to use it.

```rust
// From magellan/src/references.rs
pub struct ReferenceFact {
    pub file_path: PathBuf,
    pub referenced_symbol: String,
    pub byte_start: usize,
    pub byte_end: usize,
    pub start_line: usize,    // 1-indexed
    pub start_col: usize,     // 0-indexed
    pub end_line: usize,      // 1-indexed
    pub end_col: usize,       // 0-indexed
}

// Query API (already in CodeGraph)
graph.references_to_symbol(symbol_id: i64) -> Result<Vec<ReferenceFact>>
```

**Integration Point:** Add wrapper to `MagellanIntegration`

```rust
// NEW: src/graph/magellan_integration.rs
impl MagellanIntegration {
    /// Get all references to a symbol with exact byte spans.
    pub fn get_references_with_spans(
        &mut self,
        symbol_id: i64,
    ) -> Result<Vec<ReferenceWithSpan>> {
        let refs = self.inner.references_to_symbol(symbol_id)?;
        // Convert to Splice type...
    }
}
```

### 3. BLAKE3 SymbolId Format

**Breaking Change:** Magellan v2.0.0 uses 32-char BLAKE3 hashes.

```rust
// Current Splice (src/symbol_id.rs)
SHA-256(name:file_path:byte_start)[0..8] -> 16 hex chars

// Magellan v2.0.0
BLAKE3(name:file_path:byte_start) -> 32 hex chars

// Magellan provides both:
generate_symbol_id(language, fqn, span_id) -> 16 chars (SHA-256, current prod)
generate_symbol_id_v2(language, fqn, byte_start, byte_end) -> 32 chars (BLAKE3)
```

**Migration Impact:**
- Splice's `SymbolId` type (16 chars) breaks
- `find_symbol_by_id()` needs BLAKE3 variant
- Stored symbol IDs (plans, execution logs) need migration tool

### 4. UTF-8 Safe Extraction

```rust
// From magellan::common
pub fn extract_symbol_content_safe(
    source: &[u8],
    byte_start: usize,
    byte_end: usize,
) -> Result<String>
```

Replaces direct unsafe byte slicing in refactoring operations.

## Integration Architecture

### Cross-File Rename with Reference Spans

**Current State:**
- `resolve/cross_file.rs` exists (implementation unknown)
- `MagellanIntegration::find_symbol_by_name()` does O(N) file iteration
- `CodeGraph::references_to_symbol()` already provides byte spans

**Integration Design:**

```
User: splice patch <symbol> <new-name>
    ↓
[main.rs] execute_single_patch()
    ↓
[patch/mod.rs] apply_rename()
    ↓
    ├─→ [MagellanIntegration] find_symbol_by_id()
    │   ↓
    │   └─→ [graph/analysis.rs] get_references_with_spans()
    │       ↓
    │       └─→ For each reference:
    │           ├─→ get_code_chunk() → extract content
    │           └─→ [ropey] → apply rename at byte span
    │
    ├─→ [patch/proof.rs] generate_proof()  (if --proof flag)
    │   └─→ Collect all before/after chunks
    │
    ├─→ [patch/backup.rs] create_backup()
    ├─→ [validate.rs] validate_all_edited_files()
    └─→ [execution.rs] record_operation()
    ↓
[Output] success + proof.json (if --proof)
```

**New Types:**

```rust
// src/graph/magellan_integration.rs (NEW)
#[derive(Debug, Clone)]
pub struct ReferenceWithSpan {
    pub file_path: String,
    pub byte_start: usize,
    pub byte_end: usize,
    pub start_line: usize,
    pub start_col: usize,
    pub end_line: usize,
    pub end_col: usize,
}

// src/patch/proof.rs (NEW)
#[derive(Debug, Clone, Serialize)]
pub struct RenameProof {
    pub old_symbol: SymbolInfo,
    pub new_name: String,
    pub definition: Change,
    pub references: Vec<Change>,
    pub timestamp: i64,
}

#[derive(Debug, Clone, Serialize)]
pub struct Change {
    pub file_path: String,
    pub byte_span: (usize, usize),
    pub before: String,
    pub after: String,
}
```

### Graph Algorithm CLI Commands

**Option A: New `analyze` subcommand**

```bash
# Reachability analysis
splice analyze reachable --from <symbol-id> [--max-depth N]

# Dead code detection
splice analyze dead --entry <symbol-id>

# Cycle detection
splice analyze cycles [--symbol-filter <id>]

# Program slicing
splice analyze slice --target <symbol-id> --direction <backward|forward>

# Path enumeration
splice analyze paths --from <id> --to <id>
```

**Option B: Library API (for programmatic use)**

```rust
// src/graph/analysis.rs (NEW)
pub struct GraphAnalysis {
    integration: MagellanIntegration,
}

impl GraphAnalysis {
    pub fn reachable_from(
        &mut self,
        entry: &str,
        max_depth: Option<usize>,
    ) -> Result<Vec<SymbolInfo>> {
        // Delegate to Magellan
        self.integration.inner().reachable_symbols(entry, max_depth)
            .map(|results| results.into_iter().map(SymbolInfo::from).collect())
    }

    pub fn dead_code(&mut self, entry: &str) -> Result<Vec<DeadSymbol>> {
        // ...
    }

    pub fn detect_cycles(&mut self) -> Result<CycleReport> {
        // ...
    }
}
```

**Decision:** Start with CLI commands (Option A) for immediate utility, add library API (Option B) later.

### BLAKE3 SymbolId Migration

**Phase 1: Dual Support (Non-Breaking)**

```rust
// src/symbol_id/mod.rs (EXTENDED)
pub enum SymbolId {
    V1 { id: String },  // 16-char SHA-256
    V2 { id: String },  // 32-char BLAKE3
}

impl SymbolId {
    pub fn parse(s: &str) -> Result<Self> {
        match s.len() {
            16 => Ok(Self::V1 { id: s.to_string() }),
            32 => Ok(Self::V2 { id: s.to_string() }),
            _ => Err(SpliceError::Other("Invalid symbol ID length")),
        }
    }

    pub fn generate_v2(name: &str, file_path: &str, byte_start: usize) -> Self {
        use magellan::graph::generate_symbol_id_v2;
        let fqn = format!("{}:{}", file_path, name);
        let id = generate_symbol_id_v2("rust", &fqn, byte_start, byte_start + 1);
        Self::V2 { id }
    }
}
```

**Phase 2: MagellanIntegration Extension**

```rust
// src/graph/magellan_integration.rs (EXTENDED)
impl MagellanIntegration {
    /// Find symbol by 32-char BLAKE3 ID.
    pub fn find_symbol_by_id_v2(&mut self, symbol_id: &str) -> Result<Option<SymbolInfo>> {
        // Direct database query (Magellan doesn't store symbol_id in entities table)
        // Need to scan and regenerate IDs...
    }
}
```

**Phase 3: Migration Tool**

```bash
# NEW: splice migrate command
splice migrate --plan old_plan.json --output new_plan.json
splice migrate --execution-log .splice/operations.db
```

### --proof Flag Implementation

**Decision:** Flag to existing commands (not new command)

```bash
splice patch <symbol> <new-name> --proof --output proof.json
splice delete <file> <symbol> --proof
```

**Integration:**

```rust
// src/cli.rs (EXTENDED)
pub struct PatchCommand {
    // ... existing fields ...
    pub proof: bool,
    pub proof_output: Option<PathBuf>,
}

// src/main.rs (EXTENDED)
fn execute_single_patch(...) -> Result<...> {
    // ... existing logic ...
    
    if cmd.proof {
        let proof = generate_rename_proof(&integration, &symbol, &replacement)?;
        write_proof(&proof, cmd.proof_output)?;
    }
    
    // ... rest of logic ...
}
```

**Proof Generation:**

```rust
// src/patch/proof.rs (NEW)
impl RenameProof {
    pub fn generate(
        integration: &MagellanIntegration,
        old_symbol: &SymbolInfo,
        new_name: &str,
    ) -> Result<Self> {
        // Get definition chunk
        let def_chunk = integration.get_code_chunk(
            &old_symbol.file_path,
            old_symbol.byte_start,
            old_symbol.byte_end,
        )?.ok_or_else(|| SpliceError::Other("No code chunk found"))?;

        // Get all reference chunks
        let refs = integration.get_references_with_spans(old_symbol.entity_id)?;
        let ref_changes = refs.iter()
            .filter_map(|r| {
                integration.get_code_chunk(&r.file_path, r.byte_start, r.byte_end).ok()?
            })
            .map(|chunk| Change {
                file_path: chunk.file_path.clone(),
                byte_span: (chunk.byte_start, chunk.byte_end),
                before: chunk.content.clone(),
                after: chunk.content.replace(old_symbol.name, new_name),
            })
            .collect();

        Ok(Self {
            old_symbol: old_symbol.clone(),
            new_name: new_name.to_string(),
            definition: Change { /* ... */ },
            references: ref_changes,
            timestamp: chrono::Utc::now().timestamp(),
        })
    }
}
```

## Project Structure (Post-Integration)

```
src/
├── main.rs                  # CLI entry point (+ execute_analyze)
├── lib.rs                   # Library exports
├── cli.rs                   # Commands (+ --proof flag, Analyze subcommand)
├── error.rs                 # Error types
│
├── graph/                   # Graph integration layer
│   ├── magellan_integration.rs  # EXTENDED:
│   │   # - get_references_with_spans()
│   │   # - find_symbol_by_id_v2()
│   │   # - Algorithm wrapper methods
│   │
│   └── analysis.rs          # NEW: Graph analysis API
│       - GraphAnalysis struct
│       - reachable_from(), dead_code(), detect_cycles()
│
├── ingest/
│   └── magellan.rs          # Unchanged (wrapper)
│
├── patch/
│   ├── mod.rs               # EXTENDED with proof generation
│   ├── backup.rs            # Unchanged
│   ├── batch_loader.rs      # Unchanged
│   ├── pattern.rs           # Unchanged
│   └── proof.rs             # NEW: Proof generation logic
│
├── resolve/
│   ├── mod.rs               # EXTENDED for BLAKE3 IDs
│   ├── cross_file.rs        # MODIFIED: Use get_references_with_spans()
│   └── references/          # Unchanged
│
├── symbol_id/
│   ├── mod.rs               # EXTENDED with SymbolId enum (V1/V2)
│   ├── v1.rs                # Current SHA-256 implementation
│   └── v2.rs                # NEW: BLAKE3 implementation
│
├── execution.rs             # EXTENDED to record proof metadata
├── validate.rs              # Unchanged
└── verify.rs                # Unchanged
```

## Data Flow: Cross-File Rename

```
[User: splice patch my_func new_func_name]
    ↓
[main.rs] → parse CLI with proof flag
    ↓
[patch/mod.rs] → apply_rename()
    ↓
    ├─→ [MagellanIntegration] → find_symbol_by_name()
    │   ↓
    │   └─→ [resolve/cross_file.rs] → resolve_references()
    │       ↓
    │       └─→ [graph/magellan_integration.rs] → get_references_with_spans()
    │           ↓
    │           └─→ For each ReferenceWithSpan:
    │               ├─→ get_code_chunk() → extract content
    │               └─→ [ropey] → apply rename at byte span
    │
    ├─→ [patch/proof.rs] → generate_rename_proof()
    │   └─→ Collect all before/after chunks → proof.json
    │
    ├─→ [patch/backup.rs] → create_backup()
    ├─→ [validate.rs] → validate_all_edited_files()
    └─→ [execution.rs] → record_operation()
    ↓
[Output] → success + proof.json (if --proof)
```

## Data Flow: Dead Code Analysis

```
[User: splice analyze dead --entry main]
    ↓
[main.rs] → parse analyze command
    ↓
[graph/analysis.rs] → GraphAnalysis::dead_code()
    ↓
    ├─→ [MagellanIntegration] → inner()
    │   ↓
    │   └─→ [magellan::CodeGraph] → dead_symbols(entry_id)
    │       ↓
    │       └─→ [sqlitegraph::algo] → reverse reachability traversal
    │
    └─→ Format results (JSON/human)
    ↓
[Output] → list of DeadSymbol { symbol, reason }
```

## Build Order (Phase A/B/C)

### Phase A: Rename Credibility (Cross-File Rename)

**Priority 1: Cross-File Rename with Reference Spans**

1. **Add `get_references_with_spans()` to `MagellanIntegration`** (~50 lines)
   - Wrapper around `CodeGraph::references_to_symbol()`
   - Returns `Vec<ReferenceWithSpan>`

2. **Extend `resolve/cross_file.rs`** (~100 lines modified)
   - Use `get_references_with_spans()` instead of custom resolution
   - Apply edits at exact byte offsets

3. **Add `--proof` flag** (~150 lines)
   - Extend `cli::Commands` enum
   - Create `src/patch/proof.rs`
   - Add proof generation logic

**Acceptance Criteria:**
- `splice patch <symbol> <new-name>` renames across all files
- `splice patch <symbol> <new-name> --proof` generates proof JSON
- All references updated at exact byte spans (no regex)

### Phase B: Analysis Commands (Graph Algorithms)

**Priority 2: CLI Commands for Static Analysis**

1. **Add `analyze` subcommand to CLI** (~50 lines in cli.rs)
   - `analyze reachable --from <symbol>`
   - `analyze dead --entry <symbol>`
   - `analyze cycles`
   - `analyze slice --target <symbol> --direction <backward|forward>`

2. **Create `src/graph/analysis.rs`** (~200 lines)
   - `GraphAnalysis` struct
   - Wrapper methods for Magellan algorithms
   - Output formatting (JSON/human)

3. **Wire up in `main.rs`** (~80 lines)
   - `execute_analyze()` function
   - Error handling, exit codes

**Acceptance Criteria:**
- `splice analyze reachable --from main` lists all reachable symbols
- `splice analyze dead --entry main` lists unreachable symbols
- `splice analyze cycles` detects mutual recursion

### Phase C: Metrics and Migration (BLAKE3 IDs)

**Priority 3: BLAKE3 SymbolId Support**

1. **Extend `src/symbol_id/mod.rs`** (~80 lines)
   - Add `SymbolId` enum (V1/V2)
   - Add `generate_v2()` using BLAKE3

2. **Add `find_symbol_by_id_v2()`** (~60 lines)
   - Handle 32-char BLAKE3 IDs
   - Backward compatible with 16-char IDs

3. **Update `execution.rs`** (~30 lines)
   - Record `symbol_id_version` in execution log

4. **Migration tool** (~100 lines)
   - `splice migrate --plan <file>`
   - `splice migrate --execution-log <db>`

**Acceptance Criteria:**
- Old 16-char IDs still work
- New operations use 32-char BLAKE3 IDs
- Migration tool converts old plans

## Integration Points Summary

| Component | Integration Type | Notes |
|-----------|------------------|-------|
| **get_references_with_spans()** | New method in MagellanIntegration | Uses existing Magellan API |
| **Graph algorithms** | New CLI commands + library API | Delegates to magellan::graph::algorithms |
| **BLAKE3 IDs** | Breaking change (dual support) | SymbolId enum + migration tool |
| **--proof flag** | Flag to existing commands | New proof.rs module |
| **analyze subcommand** | New CLI command | ~4 subcommands |

## Estimated Effort

| Phase | New Lines | Modified Lines | Total |
|-------|-----------|----------------|-------|
| **A: Rename Credibility** | ~250 | ~150 | ~400 |
| **B: Analysis Commands** | ~330 | ~50 | ~380 |
| **C: BLAKE3 Migration** | ~310 | ~80 | ~390 |
| **TOTAL** | ~890 | ~280 | ~1170 |

## Quality Gates

- [x] Integration points identified (MagellanIntegration, cross_file.rs)
- [x] New vs modified components explicit
- [x] Build order considers existing dependencies
- [x] Phase structure (A/B/C) validated
- [x] BLAKE3 migration path defined (dual support)
- [x] Proof generation design complete

## Sources

- Magellan v2.0.0 source: `/home/feanor/Projects/magellan/src/`
  - `src/graph/mod.rs` - CodeGraph API (724 lines)
  - `src/graph/algorithms.rs` - Graph algorithm wrappers
  - `src/references.rs` - ReferenceFact structure
  - `src/graph/query.rs` - Symbol/reference queries

- Splice v2.2.4 source: `/home/feanor/Projects/splice/src/`
  - `src/graph/magellan_integration.rs` - Magellan wrapper (864 lines)
  - `src/ingest/magellan.rs` - MagellanIngestor (80 lines)
  - `src/symbol_id.rs` - 16-char SHA-256 IDs (553 lines)
  - `src/main.rs` - CLI entry point (31026 tokens)
  - `src/resolve/cross_file.rs` - Cross-file resolution (exists, implementation unknown)

- sqlitegraph v1.2.7: Graph database with 35 algorithms
  - `native-v2` feature enabled in Splice's Cargo.toml

---

*Magellan v2.0.0 integration research complete: 2026-02-04*
*Focus: Graph algorithms, cross-file rename, BLAKE3 ID migration*
*Ready for roadmap phase creation*
