# Splice v2.0 Manual

Comprehensive guide for Splice v2.0 span-safe refactoring tool with validation infrastructure.

---

## Splice v2.0 Overview

### What is Splice?

Splice is a span-safe code refactoring tool that provides AST-validated code modifications across 7 programming languages (Rust, Python, C, C++, Java, JavaScript, TypeScript). It uses tree-sitter for parsing and SQLiteGraph for code relationship storage, with multi-stage validation (tree-sitter + compiler) before applying changes.

### v2.0 Major Improvements

Splice v2.0 represents a comprehensive overhaul across 9 development phases:

- **Phase 1: Safety Foundation** - Eliminated unsafe `unwrap()` calls in production paths
- **Phase 2: SQLiteGraph v1.0 Upgrade** - Migrated to SQLiteGraph 1.0 with Native V2 backend
- **Phase 3: Structured Output** - Unified JSON schema with explicit fields and versioning
- **Phase 4: Stable Identifiers** - Added execution_id, match_id, span_id for traceability
- **Phase 5: Span-Aware Metadata** - Included line/column information for every operation
- **Phase 6: Deterministic Ordering** - Ensured all output is sorted consistently
- **Phase 7: Validation Hooks** - Added SHA-256 checksums and pre/post verification
- **Phase 8: Execution Logging** - Implemented audit trail with operations.db
- **Phase 9: Integration Testing** - Comprehensive test coverage across all features

### Key Features

**Span-Safe Operations:**
- Byte-accurate span extraction and modification
- Multi-language support (Rust, Python, C, C++, Java, JavaScript, TypeScript)
- Tree-sitter based parsing for AST accuracy

**Validation Infrastructure:**
- Pre-verification: File state, workspace conditions, graph synchronization, checksums
- Post-verification: Tree-sitter reparse, compiler validation, semantic preservation
- Automatic rollback on validation failure

**Structured Output:**
- Versioned JSON schema (currently "2.0.0")
- Stable identifiers (execution_id, match_id, span_id)
- Span-aware metadata (byte offsets + line/column)
- Deterministic ordering for reproducible results

**Audit Trail:**
- Every operation logged to `.splice/operations.db`
- Query execution history with filters
- Statistics and diagnostics

### Migration from v0.5.x

**Breaking Changes:**
- CLI output now uses structured JSON schema with `version` field
- All operations include `execution_id` for tracking
- Line/column information populated (was placeholder in v0.5.x)

**New Capabilities:**
- `splice log` command for querying execution history
- Checksum verification for integrity checking
- Enhanced error messages with validation diagnostics

**Compatibility:**
- Magellan v0.5.3 integration maintained
- Database schemas backward compatible
- Existing workflows continue to work

For detailed phase-by-phase implementation, see [ROADMAP.md](../.planning/ROADMAP.md).

---

## Structured Output Schema

Splice v2.0 uses a structured JSON output schema with explicit fields, versioning, and stable identifiers.

### Top-Level Structure

All Splice operations return a top-level `OperationResult` object:

```json
{
  "version": "2.0.0",
  "operation_id": "550e8400-e29b-41d4-a716-446655440000",
  "operation_type": "patch|delete|plan|query|apply_files",
  "status": "ok|error|partial",
  "message": "Human-readable status message",
  "timestamp": "2026-01-17T12:34:56.789Z",
  "workspace": "/path/to/workspace",
  "result": { /* OperationData variant */ },
  "error": { /* ErrorDetails, if status is "error" */ }
}
```

**Fields:**
- `version` (string): Schema version (currently "2.0.0")
- `operation_id` (string): UUID v4 uniquely identifying this operation execution
- `operation_type` (string): One of: "patch", "delete", "plan", "query", "apply_files"
- `status` (string): "ok" (success), "error" (failure), or "partial" (mixed results)
- `message` (string): Human-readable description of operation outcome
- `timestamp` (string): ISO 8601 timestamp (RFC 3339 format, UTC)
- `workspace` (string, optional): Absolute path to workspace root
- `result` (object, optional): Operation-specific result data (see below)
- `error` (object, optional): Error details if status is "error"

### Operation Types

#### Patch Operation

Single file symbol modification:

```json
{
  "version": "2.0.0",
  "operation_id": "550e8400-e29b-41d4-a716-446655440000",
  "operation_type": "patch",
  "status": "ok",
  "message": "Patched function 'old_name' in src/lib.rs",
  "timestamp": "2026-01-17T12:34:56.789Z",
  "workspace": "/home/user/project",
  "result": {
    "type": "patch",
    "file": "src/lib.rs",
    "symbol": "old_name",
    "kind": "function",
    "spans": [
      {
        "file_path": "src/lib.rs",
        "symbol": "old_name",
        "kind": "function",
        "byte_start": 120,
        "byte_end": 256,
        "line_start": 8,
        "line_end": 15,
        "col_start": 4,
        "col_end": 2,
        "span_id": "660e8400-e29b-41d4-a716-446655440000",
        "match_id": "770e8400-e29b-41d4-a716-446655440000",
        "before_hash": "a1b2c3d4",
        "after_hash": "e5f6g7h8",
        "span_checksum_before": "9f8e7d6c5b4a3210fedcba0987654321abcdeff1234567890abcdef123456789",
        "span_checksum_after": "9876543210abcdef0987654321abcdeff1234567890abcdef1234567890abcde"
      }
    ],
    "before_hash": "sha256:...",
    "after_hash": "sha256:...",
    "lines_added": 8,
    "lines_removed": 8
  }
}
```

**PatchResult Fields:**
- `file` (string): Path to patched file (relative to workspace)
- `symbol` (string): Symbol name that was modified
- `kind` (string): Symbol kind (function, struct, enum, etc.)
- `spans` (array): List of `SpanResult` objects modified
- `before_hash` (string): File hash before patching (SHA-256)
- `after_hash` (string): File hash after patching (SHA-256)
- `lines_added` (number): Total lines added
- `lines_removed` (number): Total lines removed

#### Delete Operation

Symbol deletion (definition + all references):

```json
{
  "version": "2.0.0",
  "operation_id": "550e8400-e29b-41d4-a716-446655440000",
  "operation_type": "delete",
  "status": "ok",
  "message": "Deleted function 'unused_func' and 3 references",
  "timestamp": "2026-01-17T12:34:56.789Z",
  "workspace": "/home/user/project",
  "result": {
    "type": "delete",
    "file": "src/lib.rs",
    "symbol": "unused_func",
    "kind": "function",
    "spans": [
      {
        "file_path": "src/lib.rs",
        "symbol": "unused_func",
        "kind": "function",
        "byte_start": 100,
        "byte_end": 200,
        "line_start": 5,
        "line_end": 12,
        "col_start": 0,
        "col_end": 1,
        "span_id": "660e8400-e29b-41d4-a716-446655440000",
        "match_id": "770e8400-e29b-41d4-a716-446655440000",
        "span_checksum_before": "abc123...",
        "span_checksum_after": null
      }
      // ... more spans for references
    ],
    "bytes_removed": 512,
    "lines_removed": 24,
    "references_removed": 3,
    "file_checksum_before": "sha256:...",
    "span_checksums": ["sha256:...", "sha256:...", ...]
  }
}
```

**DeleteResult Fields:**
- `file` (string): Path to file containing deleted symbol
- `symbol` (string): Symbol name that was deleted
- `kind` (string): Symbol kind
- `spans` (array): All removed spans (definition + references)
- `bytes_removed` (number): Total bytes removed
- `lines_removed` (number): Total lines removed
- `references_removed` (number): Number of reference spans removed
- `file_checksum_before` (string): File checksum before deletion
- `span_checksums` (array of string): SHA-256 checksums of each removed span

#### Plan Operation

Multi-step plan execution:

```json
{
  "version": "2.0.0",
  "operation_id": "550e8400-e29b-41d4-a716-446655440000",
  "operation_type": "plan",
  "status": "ok",
  "message": "Executed 3 steps successfully",
  "timestamp": "2026-01-17T12:34:56.789Z",
  "workspace": "/home/user/project",
  "result": {
    "type": "plan",
    "total_steps": 3,
    "steps_completed": 3,
    "steps": [
      {
        "step": 1,
        "status": "ok",
        "message": "Patched 'func_a' in src/a.rs",
        "file": "src/a.rs",
        "symbol": "func_a"
      },
      {
        "step": 2,
        "status": "ok",
        "message": "Patched 'func_b' in src/b.rs",
        "file": "src/b.rs",
        "symbol": "func_b"
      },
      {
        "step": 3,
        "status": "ok",
        "message": "Patched 'func_c' in src/c.rs",
        "file": "src/c.rs",
        "symbol": "func_c"
      }
    ],
    "files_affected": ["src/a.rs", "src/b.rs", "src/c.rs"],
    "total_bytes_changed": 1024
  }
}
```

**PlanResult Fields:**
- `total_steps` (number): Number of steps in plan
- `steps_completed` (number): Number of successfully executed steps
- `steps` (array): List of `StepResult` objects
- `files_affected` (array of string): All files modified (sorted)
- `total_bytes_changed` (number): Total bytes changed across all steps

#### Query Operation

Magellan label-based symbol search:

```json
{
  "version": "2.0.0",
  "operation_id": "550e8400-e29b-41d4-a716-446655440000",
  "operation_type": "query",
  "status": "ok",
  "message": "Found 5 matching symbols",
  "timestamp": "2026-01-17T12:34:56.789Z",
  "workspace": "/home/user/project",
  "result": {
    "type": "query",
    "labels": ["Function", "Method"],
    "count": 5,
    "symbols": [
      {
        "file_path": "src/lib.rs",
        "symbol": "process_request",
        "kind": "function",
        "byte_start": 100,
        "byte_end": 500,
        "line_start": 10,
        "line_end": 25,
        "col_start": 0,
        "col_end": 1,
        "span_id": "660e8400-e29b-41d4-a716-446655440000",
        "match_id": "770e8400-e29b-41d4-a716-446655440000"
      }
      // ... more symbols
    ]
  }
}
```

**QueryResult Fields:**
- `labels` (array of string): Labels queried (sorted)
- `count` (number): Number of results found
- `symbols` (array): Matching `SpanResult` objects (sorted by file_path, byte_start, byte_end)

#### ApplyFiles Operation

Pattern replacement across multiple files:

```json
{
  "version": "2.0.0",
  "operation_id": "550e8400-e29b-41d4-a716-446655440000",
  "operation_type": "apply_files",
  "status": "ok",
  "message": "Replaced pattern in 3 files",
  "timestamp": "2026-01-17T12:34:56.789Z",
  "workspace": "/home/user/project",
  "result": {
    "type": "apply_files",
    "glob_pattern": "src/**/*.rs",
    "find_pattern": "old_pattern",
    "replace_pattern": "new_pattern",
    "files_matched": 5,
    "files_modified": 3,
    "files": [
      {
        "file": "src/a.rs",
        "matches": 2,
        "replacements": 2,
        "spans": [/* SpanResult objects */],
        "before_hash": "sha256:...",
        "after_hash": "sha256:..."
      }
      // ... more files
    ]
  }
}
```

**ApplyFilesResult Fields:**
- `glob_pattern` (string): Glob pattern used for file matching
- `find_pattern` (string): Pattern that was searched for
- `replace_pattern` (string): Replacement pattern
- `files_matched` (number): Number of files matching glob pattern
- `files_modified` (number): Number of files actually modified
- `files` (array): List of `FilePatternResult` objects (sorted by file path)

### SpanResult Structure

Unified span representation used across all operations:

```json
{
  "file_path": "src/lib.rs",
  "symbol": "function_name",
  "kind": "function",
  "byte_start": 100,
  "byte_end": 500,
  "line_start": 10,
  "line_end": 25,
  "col_start": 0,
  "col_end": 1,
  "span_id": "550e8400-e29b-41d4-a716-446655440000",
  "match_id": "660e8400-e29b-41d4-a716-446655440000",
  "before_hash": "sha256:...",
  "after_hash": "sha256:...",
  "span_checksum_before": "sha256:...",
  "span_checksum_after": "sha256:..."
}
```

**Fields:**
- `file_path` (string): Absolute or relative path to file
- `symbol` (string, optional): Symbol name (if applicable)
- `kind` (string, optional): Symbol kind (function, struct, etc.)
- `byte_start` (number): Start byte offset (0-based)
- `byte_end` (number): End byte offset (exclusive)
- `line_start` (number): Start line number (1-based, 0 if unavailable)
- `line_end` (number): End line number (1-based, 0 if unavailable)
- `col_start` (number): Start column (0-based, 0 if unavailable)
- `col_end` (number): End column (0-based, 0 if unavailable)
- `span_id` (string): UUID v4 uniquely identifying this span
- `match_id` (string, optional): UUID from symbol resolution (for traceability)
- `before_hash` (string, optional): Content hash before modification
- `after_hash` (string, optional): Content hash after modification
- `span_checksum_before` (string, optional): SHA-256 checksum before modification
- `span_checksum_after` (string, optional): SHA-256 checksum after modification

### Deterministic Ordering

All arrays in Splice v2.0 output are deterministically sorted for reproducibility:

**Sorting Rules:**
- `SpanResult`: sorted by `file_path`, then `byte_start`, then `byte_end`
- `FilePatternResult`: sorted by `file` path
- `DiagnosticPayload`: sorted by `tool`, `file`, `line`, `column`, `level`, `message`
- String arrays (labels, files_affected): lexicographic sort

This ensures identical operations produce identical JSON output, enabling:
- Diff-based verification
- Reliable testing
- Caching strategies

### Error Response Format

```json
{
  "version": "2.0.0",
  "operation_id": "550e8400-e29b-41d4-a716-446655440000",
  "operation_type": "patch",
  "status": "error",
  "message": "Validation failed: syntax error in src/lib.rs",
  "timestamp": "2026-01-17T12:34:56.789Z",
  "workspace": "/home/user/project",
  "error": {
    "kind": "ValidationError",
    "message": "Post-verification tree-sitter reparse failed",
    "symbol": "my_function",
    "file": "src/lib.rs",
    "hint": "Check for mismatched braces or missing semicolons",
    "diagnostics": [
      {
        "tool": "tree-sitter",
        "level": "error",
        "message": "Unexpected token",
        "file": "src/lib.rs",
        "line": 42,
        "column": 10
      }
    ]
  }
}
```

**ErrorDetails Fields:**
- `kind` (string): Error category (ValidationError, IOError, etc.)
- `message` (string): Human-readable error description
- `symbol` (string, optional): Symbol context
- `file` (string, optional): File context
- `hint` (string, optional): Remediation suggestion
- `diagnostics` (array, optional): Validation tool diagnostics

---

## SQLiteGraph API Reference

Splice uses SQLiteGraph as a dependency. The following sections document SQLiteGraph-specific usage patterns.

---

## 1. Quick Start

### Installation

```toml
[dependencies]
sqlitegraph = "1.0"

# For Native V2 backend
sqlitegraph = { version = "1.0", features = ["native-v2"] }
```

### Basic Usage

```rust
use sqlitegraph::{SqliteGraph, GraphEntity};

let graph = SqliteGraph::open_in_memory()?;

let entity = GraphEntity {
    id: 0,
    kind: "User".to_string(),
    name: "Alice".to_string(),
    file_path: None,
    data: serde_json::json!({"age": 30}),
};

let id = graph.insert_entity(&entity)?;
println!("Created entity: {}", id);
```

---

## 2. Backend Selection

### SQLite Backend (Default)

**Use**: General purpose, ACID transactions, existing SQLite data

```rust
use sqlitegraph::{SqliteGraph, GraphEntity, GraphEdge};

let graph = SqliteGraph::open_in_memory()?;
// SQLite operations with full ACID compliance
```

### Native V2 Backend

**Use**: High-performance scenarios, large graphs, traversal-heavy workloads

```rust
use sqlitegraph::{GraphConfig, open_graph, NodeSpec, EdgeSpec};

let config = GraphConfig::native();
let graph = open_graph("graph.db", &config)?;
// Clustered adjacency for locality
```

### Backend Comparison

| Characteristic | SQLite Backend | Native V2 Backend |
|----------------|----------------|-------------------|
| **Performance** | Standard SQLite | 10x faster for traversals |
| **Transactions** | Full ACID | Atomic commits, WAL |
| **Memory Usage** | SQLite overhead | Configurable buffers |
| **Use Cases** | General purpose | High performance, large graphs |

---

## 3. Core Operations

### Entity Management (SQLite Backend)

```rust
use sqlitegraph::{SqliteGraph, GraphEntity};

let graph = SqliteGraph::open_in_memory()?;

// Create entity
let entity = GraphEntity {
    id: 0,
    kind: "User".to_string(),
    name: "Alice".to_string(),
    file_path: None,
    data: serde_json::json!({"age": 30}),
};

let entity_id = graph.insert_entity(&entity)?;
let retrieved = graph.get_entity(entity_id)?;

// Update entity
let mut updated_entity = retrieved;
updated_entity.name = "Alice Smith".to_string();
graph.update_entity(&updated_entity)?;
```

### Node Management (Native V2 Backend)

```rust
use sqlitegraph::{GraphConfig, open_graph, NodeSpec, EdgeSpec};

let config = GraphConfig::native();
let graph = open_graph("graph.db", &config)?;

// Create node
let node_spec = NodeSpec {
    kind: "User".to_string(),
    name: "Alice".to_string(),
    file_path: None,
    data: serde_json::json!({"age": 30}),
};

let node_id = graph.insert_node(node_spec)?;

// Create edge
let edge_spec = EdgeSpec {
    from: node_id,
    to: node_id,
    edge_type: "self_ref".to_string(),
    data: serde_json::json!({"type": "demo"}),
};

let edge_id = graph.insert_edge(edge_spec)?;
```

---

## 4. Graph Algorithms

### PageRank

```rust
use sqlitegraph::algo;

// Basic PageRank
let scores = algo::pagerank(&graph, 0.85, 50)?;

// With progress tracking
use sqlitegraph::progress::ConsoleProgress;
let scores = algo::pagerank_with_progress(&graph, 0.85, 50, ConsoleProgress::new())?;
```

### Betweenness Centrality

```rust
// Node importance via shortest paths
let centrality = algo::betweenness_centrality(&graph)?;
```

### Community Detection

```rust
// Label Propagation (fast)
let communities = algo::label_propagation(&graph)?;

// Louvain (higher quality)
let partition = algo::louvain_communities(&graph, 0.01)?;
```

### Algorithm Characteristics

| Algorithm | Complexity | Best For |
|-----------|------------|----------|
| **PageRank** | O(|E| × iterations) | Importance ranking |
| **Betweenness** | O(|V||E|) | Critical nodes |
| **Label Propagation** | O(|E|) | Fast communities |
| **Louvain** | O(|E| log |V|) | Quality clustering |

---

## 5. Testing

### Running Tests

```bash
# All tests
cargo test --workspace

# With Native V2 backend
cargo test --workspace --features native-v2

# Specific test patterns
cargo test '*pagerank*'
cargo test '*wal*'
```

### Test Coverage

**v1.0 Test Results:**
- 42 WAL tests passing (recovery, corruption, checkpoints)
- 53 concurrent MVCC tests passing (snapshots, stress testing)
- 27 algorithm tests passing (PageRank, Betweenness, Louvain, Label Propagation)
- 134 HNSW tests passing
- 65 MVCC lifecycle tests passing

**Total**: 300+ tests passing

---

## 6. Performance

### Native V2 Performance

Based on actual benchmarks (Phase 3, 7):

| Operation | Performance |
|-----------|-------------|
| **Node Insert** | ~50K ops/sec |
| **Edge Insert** | ~100K ops/sec (bulk) |
| **Neighbor Query** | <1ms (clustered) |
| **Vector Search** | <1ms with 95%+ accuracy |

### Parallel WAL Recovery

```rust
use sqlitegraph::{GraphConfig, open_graph};

// Default: 4 threads
let config = GraphConfig::native();
let graph = open_graph("large.db", &config)?;

// Custom: 8 threads
let config = GraphConfig::native().with_parallel_recovery(8);
let graph = open_graph("large.db", &config)?;
```

**Performance**:
- 2-3x speedup for 500+ transactions
- 1.5-2x speedup for 50-100 transactions

---

## 7. Error Handling

### Common Error Types

```rust
use sqlitegraph::SqliteGraphError;

match graph.insert_entity(&entity) {
    Ok(id) => println!("Created: {}", id),
    Err(SqliteGraphError::ValidationError(msg)) => {
        eprintln!("Validation failed: {}", msg);
    }
    Err(SqliteGraphError::ConnectionError(msg)) => {
        eprintln!("Connection failed: {}", msg);
    }
    Err(err) => eprintln!("Error: {}", err),
}
```

### Debug Features

```toml
# Enable V2 I/O tracing
sqlitegraph = { version = "1.0", features = ["trace_v2_io"] }
```

```bash
# Run with debug output
RUST_LOG=debug cargo run --features trace_v2_io
```

---

## 8. Vector Search (HNSW)

### Basic HNSW Usage

```rust
use sqlitegraph::hnsw::{HnswConfig, DistanceMetric, HnswIndex};

let config = HnswConfig::builder()
    .dimension(1536)
    .distance_metric(DistanceMetric::Cosine)
    .build()?;

let hnsw = HnswIndex::new(config)?;

// Insert vector
let vector_id = hnsw.insert_vector(&embedding, Some(metadata))?;

// Search
let results = hnsw.search(&query, k)?;
```

### Distance Metrics

| Metric | Best For | Speed |
|--------|----------|-------|
| **Cosine** | Text embeddings | Fast |
| **Euclidean** | General similarity | Medium |
| **Dot Product** | Normalized vectors | Fastest |
| **Manhattan** | Sparse vectors | Slow |

### CLI Commands

```bash
# Create index
sqlitegraph --backend sqlite --db mygraph.db hnsw-create \
    --dimension 768 --distance-metric cosine

# Insert vectors
sqlitegraph --backend sqlite --db mygraph.db hnsw-insert \
    --index-name vectors --input vectors.json

# Search
sqlitegraph --backend sqlite --db mygraph.db hnsw-search \
    --index-name vectors --input query.json --k 10

# List indexes
sqlitegraph --backend sqlite --db mygraph.db hnsw-list
```

---

## 9. Developer Tools (Phase 9)

### Introspection API

```rust
use sqlitegraph::introspection::GraphIntrospection;

let intro = GraphIntrospection::new(&graph)?;
println!("Nodes: {}", intro.node_count()?);
println!("Edges (estimated): {}", intro.edge_count_estimate()?);
println!("JSON: {}", intro.to_json()?);
```

### Progress Tracking

```rust
use sqlitegraph::progress::{ProgressCallback, ConsoleProgress};

// No-op (default)
let scores = algo::pagerank_with_progress(&graph, 0.85, 50, NoProgress)?;

// Console progress bars
let scores = algo::pagerank_with_progress(&graph, 0.85, 50, ConsoleProgress::new())?;
```

### CLI Debug Commands

```bash
# Statistics
sqlitegraph --backend sqlite --db mygraph.db debug-stats

# Dump graph
sqlitegraph --backend sqlite --db mygraph.db debug-dump --output graph.json

# Trace operations
sqlitegraph --backend sqlite --db mygraph.db debug-trace
```

---

## 10. Safety & Integrity

### Safety Checks

```rust
use sqlitegraph::run_safety_checks;

let report = run_safety_checks(&graph)?;
if report.has_orphans() {
    eprintln!("Warning: {} orphan edges", report.orphan_count());
}
```

### V2 WAL Recovery

The Native V2 backend includes WAL recovery with:
- Transaction rollback for all operations
- Edge cascade cleanup on node deletion
- Cluster reference cleanup

**Tested**: 42 WAL tests passing (recovery, corruption, checkpoints)

---

## 11. CLI Usage

### Available Commands

```bash
# Status
sqlitegraph --command status --database mygraph.db

# List entities
sqlitegraph --command list --database mygraph.db

# Export/Import
sqlitegraph --command dump-graph --output backup.json --database mygraph.db
sqlitegraph --command load-graph --input backup.json --database mygraph.db

# Safety check
sqlitegraph --command safety-check --database mygraph.db

# Graph algorithms
sqlitegraph --backend sqlite --db mygraph.db pagerank --progress
sqlitegraph --backend sqlite --db mygraph.db betweenness --progress
sqlitegraph --backend sqlite --db mygraph.db louvain --progress
```

---

## 12. Migration

### SQLite to Native V2

```rust
// Before (SQLite)
let graph = SqliteGraph::open("data.db")?;
let entity = GraphEntity { /* fields */ };
let id = graph.insert_entity(&entity)?;

// After (Native V2)
let config = GraphConfig::native();
let graph = open_graph("data.db", &config)?;
let node_spec = NodeSpec { /* similar fields */ };
let id = graph.insert_node(node_spec)?;
```

### Key Differences

| Aspect | SQLite Backend | Native V2 Backend |
|--------|----------------|-------------------|
| **Data Types** | `GraphEntity`/`GraphEdge` | `NodeSpec`/`EdgeSpec` |
| **Edge Fields** | `from_id`/`to_id` | `from`/`to` |
| **Construction** | `SqliteGraph::open()` | `open_graph(&config)` |

---

## 13. Troubleshooting

### Common Issues

**Compilation Errors:**
- Add feature flags: `--features native-v2`
- Check backend-specific data types

**Runtime Issues:**
- Run integrity checks: `safety-check` command
- Check buffer configuration for large graphs

**Performance:**
- Use Native V2 for traversals
- Enable parallel WAL recovery for large databases

### Getting Help

```bash
# Check test status
cargo test --lib 2>&1 | tail -5

# Run specific test with output
cargo test test_name -- --nocapture

# Check compilation
cargo check --features native-v2
```

---

## Architecture Status

**v1.0 Features:**
- Native V2 Backend: Clustered adjacency with WAL
- Dual Backend Support: Unified API
- Graph Algorithms: 4 production algorithms
- HNSW Vector Search: Full persistence support
- MVCC Snapshots: Read isolation
- Developer Tools: Introspection, progress tracking, CLI

**Test Coverage:** 300+ tests passing
