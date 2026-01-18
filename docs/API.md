# Splice API Reference

## Official Documentation

The complete API documentation is available at **[docs.rs/sqlitegraph](https://docs.rs/sqlitegraph)** for the SQLiteGraph dependency.

This document provides API reference for both Splice's v2.0 structured output types and the underlying SQLiteGraph dependency.

## Table of Contents

- [Splice v2.0 Output Types](#splice-v20-output-types)
- [Execution Logging API](#execution-logging-api)
- [Validation Hooks API](#validation-hooks-api)
- [SQLite Backend API](#sqlite-backend-api)
- [Native V2 Backend API](#native-v2-backend-api)
- [Graph Algorithms API](#graph-algorithms-api)
- [HNSW Vector Search API](#hnsw-vector-search-api)
- [Introspection API](#introspection-api)
- [Progress Tracking API](#progress-tracking-api)
- [Error Types](#error-types)

---

## Splice v2.0 Output Types

Splice v2.0 provides structured output types for all operations with deterministic ordering and JSON serialization.

### SpanReplacement

Represents a byte-exact replacement within a file.

```rust
use splice::patch::SpanReplacement;

pub struct SpanReplacement {
    /// Absolute or workspace-relative file path
    pub file: PathBuf,
    /// Start byte offset (inclusive)
    pub start: usize,
    /// End byte offset (exclusive)
    pub end: usize,
    /// Replacement contents
    pub content: String,
}
```

**JSON Example:**
```json
{
  "file": "/path/to/file.rs",
  "start": 1024,
  "end": 2048,
  "content": "fn new_function() { }"
}
```

### FilePatchSummary

Result summary for a patched file with cryptographic hash verification.

```rust
use splice::patch::FilePatchSummary;

pub struct FilePatchSummary {
    /// Path of the patched file
    pub file: PathBuf,
    /// SHA-256 before patching
    pub before_hash: String,
    /// SHA-256 after patching
    pub after_hash: String,
}
```

**JSON Example:**
```json
{
  "file": "/path/to/file.rs",
  "before_hash": "a1b2c3d4e5f6...",
  "after_hash": "f6e5d4c3b2a1..."
}
```

### PreviewReport

Preview metadata describing the diff produced by a patch operation.

```rust
use splice::patch::PreviewReport;

pub struct PreviewReport {
    /// The file that would be patched
    pub file: String,
    /// 1-based line number where the change begins
    pub line_start: usize,
    /// 1-based line number where the change ends
    pub line_end: usize,
    /// Number of lines added by the patch
    pub lines_added: usize,
    /// Number of lines removed by the patch
    pub lines_removed: usize,
    /// Number of bytes inserted
    pub bytes_added: usize,
    /// Number of bytes removed
    pub bytes_removed: usize,
}
```

**JSON Example:**
```json
{
  "file": "/path/to/file.rs",
  "line_start": 42,
  "line_end": 45,
  "lines_added": 3,
  "lines_removed": 2,
  "bytes_added": 128,
  "bytes_removed": 64
}
```

### Deterministic Ordering Guarantees

All output types follow deterministic ordering:
- **Replacements**: Sorted by start byte offset (descending for batch operations)
- **Files**: Sorted lexicographically by path
- **JSON Output**: Fields are serialized in declaration order
- **BTreeMap**: Used for all multi-file collections to ensure ordering

**Example: Batch replacements are always applied from highest offset to lowest**
```rust
// In src/patch/mod.rs:318
replacements.sort_by_key(|r| std::cmp::Reverse(r.start));
```

---

## Execution Logging API

Splice v2.0 provides persistent audit trail storage for all operations via the execution logging module.

### Database Initialization

```rust
use splice::execution::{init_execution_log_db, DB_FILENAME};
use std::path::Path;

/// Initialize the execution log database at the given path
/// Creates .splice/operations.db with all necessary tables
pub fn init_execution_log_db(db_dir: &Path) -> Result<Connection>
```

**Example:**
```rust
let workspace = Path::new("/path/to/workspace");
let splice_dir = workspace.join(".splice");
let conn = init_execution_log_db(&splice_dir)?;
```

### ExecutionLog Entry

Represents a single operation in the audit trail with full metadata.

```rust
use splice::execution::ExecutionLog;

#[derive(Debug, Clone, Serialize)]
pub struct ExecutionLog {
    /// Database row ID
    pub id: i64,
    /// Unique operation identifier (UUID)
    pub execution_id: String,
    /// Operation type (e.g., "patch", "delete", "plan")
    pub operation_type: String,
    /// Operation status ("ok", "error", "partial")
    pub status: String,
    /// ISO 8601 timestamp
    pub timestamp: String,
    /// Workspace root path (if available)
    pub workspace: Option<String>,
    /// Full command line for reproducibility
    pub command_line: Option<String>,
    /// Operation-specific parameters (JSON)
    pub parameters: Option<serde_json::Value>,
    /// Summary of results (JSON)
    pub result_summary: Option<serde_json::Value>,
    /// Error details if status != "ok" (JSON)
    pub error_details: Option<serde_json::Value>,
    /// Operation duration in milliseconds
    pub duration_ms: Option<i64>,
    /// Unix timestamp for sorting
    pub created_at: i64,
}
```

### ExecutionLogBuilder

Fluent builder for constructing execution log entries.

```rust
use splice::execution::ExecutionLogBuilder;

pub struct ExecutionLogBuilder { /* ... */ }

impl ExecutionLogBuilder {
    /// Create a new builder with required fields
    pub fn new(execution_id: String, operation_type: String) -> Self

    /// Set the operation status
    pub fn status(self, status: String) -> Self

    /// Set the workspace root path
    pub fn workspace(self, workspace: String) -> Self

    /// Set the command line
    pub fn command_line(self, command_line: String) -> Self

    /// Set the operation parameters (JSON)
    pub fn parameters(self, parameters: serde_json::Value) -> Self

    /// Set the result summary (JSON)
    pub fn result_summary(self, result_summary: serde_json::Value) -> Self

    /// Set the error details (JSON)
    pub fn error_details(self, error_details: serde_json::Value) -> Self

    /// Set the operation duration in milliseconds
    pub fn duration_ms(self, duration_ms: i64) -> Self

    /// Build the ExecutionLog entry
    pub fn build(self) -> ExecutionLog
}
```

**Example:**
```rust
use uuid::Uuid;

let execution_id = Uuid::new_v4().to_string();
let log = ExecutionLogBuilder::new(execution_id, "patch".to_string())
    .status("ok".to_string())
    .workspace("/path/to/workspace".to_string())
    .command_line("splice patch foo bar baz".to_string())
    .duration_ms(1234)
    .result_summary(serde_json::json!({
        "files_modified": 3,
        "replacements_applied": 7
    }))
    .build();
```

### Recording Operations

```rust
use splice::execution::insert_execution_log;

/// Insert an execution log entry into the database
/// Returns the database row ID of the inserted entry
pub fn insert_execution_log(conn: &Connection, log: &ExecutionLog) -> Result<i64>
```

### Querying Operations

```rust
use splice::execution::{
    ExecutionQuery,
    get_execution,
    get_recent_executions,
    get_execution_stats,
};

/// Query builder for flexible filtering
pub struct ExecutionQuery { /* ... */ }

impl ExecutionQuery {
    pub fn new() -> Self
    pub fn with_operation_type(self, op: String) -> Self
    pub fn with_status(self, status: String) -> Self
    pub fn after(self, timestamp: i64) -> Self
    pub fn before(self, timestamp: i64) -> Self
    pub fn with_limit(self, limit: usize) -> Self
    pub fn with_offset(self, offset: usize) -> Self
    pub fn for_execution(self, id: String) -> Self
    pub fn execute(&self, conn: &Connection) -> Result<Vec<ExecutionLog>>
}

/// Get execution by execution_id
pub fn get_execution(conn: &Connection, execution_id: &str) -> Result<Option<ExecutionLog>>

/// Get recent executions
pub fn get_recent_executions(conn: &Connection, limit: usize) -> Result<Vec<ExecutionLog>>

/// Get execution statistics
pub fn get_execution_stats(conn: &Connection) -> Result<ExecutionStats>
```

**Query Examples:**
```rust
// Get last 10 patch operations
let patches = ExecutionQuery::new()
    .with_operation_type("patch".to_string())
    .with_limit(10)
    .execute(&conn)?;

// Get failed operations from last hour
let one_hour_ago = chrono::Utc::now().timestamp() - 3600;
let failed = ExecutionQuery::new()
    .with_status("error".to_string())
    .after(one_hour_ago)
    .execute(&conn)?;

// Get specific execution
let exec = get_execution(&conn, "uuid-here")?;

// Get statistics
let stats = get_execution_stats(&conn)?;
println!("Total operations: {}", stats.total_operations);
```

### Database Schema

The execution log is stored in `.splice/operations.db` with the following schema:

```sql
CREATE TABLE execution_log (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    execution_id TEXT NOT NULL UNIQUE,
    operation_type TEXT NOT NULL,
    status TEXT NOT NULL,
    timestamp TEXT NOT NULL,
    workspace TEXT,
    command_line TEXT,
    parameters TEXT,
    result_summary TEXT,
    error_details TEXT,
    duration_ms INTEGER,
    created_at INTEGER NOT NULL
);

-- Indexes for efficient querying
CREATE INDEX idx_execution_log_execution_id ON execution_log(execution_id);
CREATE INDEX idx_execution_log_operation_type ON execution_log(operation_type);
CREATE INDEX idx_execution_log_timestamp ON execution_log(created_at);
CREATE INDEX idx_execution_log_status ON execution_log(status);
```

### ExecutionStats

Aggregated statistics about all operations.

```rust
use splice::execution::ExecutionStats;
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize)]
pub struct ExecutionStats {
    /// Total number of operations in the log
    pub total_operations: i64,
    /// Count of operations grouped by type
    pub by_type: HashMap<String, i64>,
    /// Count of operations grouped by status
    pub by_status: HashMap<String, i64>,
    /// Timestamp of the oldest execution (ISO 8601)
    pub oldest_execution: Option<String>,
    /// Timestamp of the newest execution (ISO 8601)
    pub newest_execution: Option<String>,
}
```

**JSON Example:**
```json
{
  "total_operations": 150,
  "by_type": {
    "patch": 100,
    "delete": 30,
    "plan": 20
  },
  "by_status": {
    "ok": 140,
    "error": 8,
    "partial": 2
  },
  "oldest_execution": "2026-01-01T00:00:00Z",
  "newest_execution": "2026-01-18T12:34:56Z"
}
```

---

## Validation Hooks API

Splice v2.0 provides pre and post-verification hooks for safe refactoring operations with automatic rollback on validation failure.

### Pre-Verification

Pre-verification runs BEFORE any file modifications to ensure safe conditions.

#### PreVerificationResult

Result of pre-verification checks with blocking and warning states.

```rust
use splice::verify::PreVerificationResult;

#[derive(Debug, Clone, PartialEq)]
pub enum PreVerificationResult {
    /// All checks passed, safe to proceed
    Pass,

    /// Check failed with details
    Fail {
        /// Check that failed
        check: String,
        /// Failure reason
        reason: String,
        /// True if blocking (error), false if warning
        blocking: bool,
    },
}

impl PreVerificationResult {
    pub fn is_pass(&self) -> bool
    pub fn is_blocking(&self) -> bool
    pub fn is_warning(&self) -> bool
}
```

#### Pre-Verification Functions

```rust
use splice::verify::{
    pre_verify_patch,
    verify_file_ready,
    verify_workspace_resources,
    verify_graph_sync,
};

/// Run all pre-verification checks for a patch operation
pub fn pre_verify_patch(
    file_path: &Path,
    expected_checksum: Option<&Checksum>,
    workspace_root: &Path,
    db_path: &Path,
    strict: bool,
    skip: bool,
) -> Result<Vec<PreVerificationResult>>

/// Verify file is ready for patching
pub fn verify_file_ready(
    file_path: &Path,
    expected_checksum: Option<&Checksum>,
    workspace_root: &Path,
) -> PreVerificationResult

/// Verify workspace has sufficient resources
pub fn verify_workspace_resources(
    workspace_root: &Path,
    estimated_size: usize,
) -> PreVerificationResult

/// Verify graph database is in sync with files
pub fn verify_graph_sync(
    file_path: &Path,
    db_path: &Path,
) -> PreVerificationResult
```

**Pre-Verification Checks:**

1. **File State**
   - File exists and is readable
   - File is writable
   - File checksum matches expected (no external modification)
   - File is within workspace bounds

2. **Workspace Conditions**
   - Workspace exists and is writable
   - Sufficient disk space (2x file size)
   - Backup directory can be created

3. **Graph Synchronization**
   - Database file exists and is readable
   - File mtime <= database mtime

**Example:**
```rust
use splice::verify::pre_verify_patch;

let results = pre_verify_patch(
    &file_path,
    None,
    &workspace_root,
    &db_path,
    false,  // strict mode
    false,  // skip verification
)?;

for result in results {
    if result.is_blocking() {
        eprintln!("Cannot proceed: {:?}", result);
        return Err(...);
    } else if result.is_warning() {
        eprintln!("Warning: {:?}", result);
    }
}
```

### Post-Verification

Post-verification runs AFTER file modifications to validate changes.

#### PostVerificationResult

Result of post-verification with validation status and checksums.

```rust
use splice::verify::PostVerificationResult;

#[derive(Debug, Clone, PartialEq)]
pub struct PostVerificationResult {
    /// Syntax validation passed
    pub syntax_ok: bool,
    /// Compiler validation passed
    pub compiler_ok: bool,
    /// Semantic validation passed (advisory)
    pub semantic_ok: bool,
    /// Checksums before and after
    pub before_checksum: String,
    /// After checksum
    pub after_checksum: String,
    /// Warnings (non-blocking issues)
    pub warnings: Vec<String>,
    /// Errors (blocking issues that would have failed validation)
    pub errors: Vec<String>,
}

impl PostVerificationResult {
    pub fn new(
        syntax_ok: bool,
        compiler_ok: bool,
        before_checksum: String,
        after_checksum: String,
    ) -> Self

    pub fn add_warning(&mut self, warning: impl Into<String>)
    pub fn add_error(&mut self, error: impl Into<String>)
    pub fn file_changed(&self) -> bool
}
```

#### Post-Verification Functions

```rust
use splice::verify::{
    verify_after_patch,
    verify_localized_change,
    checksum_diff,
};

/// Verify file after patching
pub fn verify_after_patch(
    file_path: &Path,
    workspace_root: &Path,
    expected_before: &str,
) -> Result<PostVerificationResult>

/// Verify changes were localized to target span
pub fn verify_localized_change(
    file_path: &Path,
    original_content: &[u8],
    target_span: (usize, usize),
) -> Result<bool>

/// Compare checksums to document changes
pub fn checksum_diff(before_checksum: &str, after_checksum: &str) -> ChecksumDiff
```

**Post-Verification Checks:**

1. **Syntax Validation**
   - Tree-sitter reparse for language-specific syntax
   - Supports: Rust, Python, C, C++, Java, JavaScript, TypeScript

2. **Compiler Validation**
   - Language-specific compiler validation
   - Rust: `cargo check`
   - Python: `python -m py_compile`
   - JavaScript: `node --check`
   - TypeScript: `tsc --noEmit`

3. **Checksum Verification**
   - SHA-256 before/after comparison
   - Confirms expected changes occurred
   - Detects unintended modifications

4. **Localized Change Verification**
   - Verifies only target span changed
   - Checks bytes before/after span intact
   - Detects file-level side effects

**Example:**
```rust
use splice::verify::verify_after_patch;

let result = verify_after_patch(&file_path, &workspace_root, &before_hash)?;

if !result.syntax_ok {
    eprintln!("Syntax validation failed");
    // Automatic rollback occurs in apply_patch_with_validation
}

if !result.compiler_ok {
    eprintln!("Compiler validation failed");
    // Automatic rollback occurs in apply_patch_with_validation
}

if !result.file_changed() {
    eprintln!("Warning: File checksum unchanged - no modification detected");
}

for warning in &result.warnings {
    eprintln!("Warning: {}", warning);
}
```

### Rollback Behavior

Splice automatically rolls back on validation failure:

```rust
use splice::patch::apply_patch_with_validation;

// This function handles rollback automatically:
match apply_patch_with_validation(&file_path, start, end, new_content, ...) {
    Ok((before_hash, after_hash)) => {
        // Success: all validation gates passed
    }
    Err(SpliceError::ParseValidationFailed { .. }) => {
        // Automatic rollback: file restored to original bytes
    }
    Err(SpliceError::CompilerValidationFailed { .. }) => {
        // Automatic rollback: file restored to original bytes
    }
    Err(_) => {
        // Any error triggers automatic rollback
    }
}
```

**Rollback Process:**
1. Original file bytes preserved in memory
2. On validation failure, write original bytes via atomic rename
3. Log rollback attempt (failure to rollback is logged but doesn't fail operation)
4. Return validation error to caller

### Integration Example

Complete example showing verification integration:

```rust
use splice::verify::{pre_verify_patch, verify_after_patch};
use splice::patch::apply_patch_with_validation;

// Step 1: Pre-verification
let db_path = workspace_root.join(".codemcp/codegraph.db");
let pre_checks = pre_verify_patch(
    &file_path,
    None,
    &workspace_root,
    &db_path,
    false,  // strict mode
    false,  // skip verification
)?;

// Check for blocking failures
for check in &pre_checks {
    if check.is_blocking() {
        eprintln!("Pre-verification failed: {:?}", check);
        return Err(...);
    }
}

// Step 2: Apply patch with validation
let (before_hash, after_hash) = apply_patch_with_validation(
    &file_path,
    start,
    end,
    new_content,
    &workspace_root,
    language,
    analyzer_mode,
)?;

// Step 3: Post-verification (already run in apply_patch_with_validation)
let post_result = verify_after_patch(&file_path, &workspace_root, &before_hash)?;

if !post_result.syntax_ok {
    // This won't happen as apply_patch_with_validation already rolled back
    unreachable!("Syntax validation should have failed in apply_patch_with_validation");
}

// Log warnings
for warning in &post_result.warnings {
    log::warn!("Post-verification warning: {}", warning);
}

println!("Patch applied successfully!");
println!("Before: {}", before_hash);
println!("After: {}", after_hash);
```

---

## SQLite Backend API

### Core Types

```rust
use sqlitegraph::{SqliteGraph, GraphEntity, GraphEdge};

// Open database
let graph = SqliteGraph::open_in_memory()?;
let graph = SqliteGraph::open("path/to/db")?;

// Entity
pub struct GraphEntity {
    pub id: u64,
    pub kind: String,
    pub name: String,
    pub file_path: Option<String>,
    pub data: serde_json::Value,
}

// Edge
pub struct GraphEdge {
    pub id: u64,
    pub from_id: u64,
    pub to_id: u64,
    pub edge_type: String,
    pub data: serde_json::Value,
}
```

### Main Methods

| Method | Description |
|--------|-------------|
| `open_in_memory()` | Create in-memory database |
| `open(path: &str)` | Open file-based database |
| `insert_entity(&entity)` | Insert new entity, returns ID |
| `get_entity(id)` | Retrieve entity by ID |
| `update_entity(&entity)` | Update existing entity |
| `delete_entity(id)` | Delete entity |
| `insert_edge(&edge)` | Insert new edge |
| `get_edge(id)` | Retrieve edge by ID |
| `neighbors(id, direction)` | Get neighbor entities |
| `has_path(from, to)` | Check if path exists |
| `snapshot()` | Create MVCC snapshot |

---

## Native V2 Backend API

### Core Types

```rust
use sqlitegraph::{GraphConfig, open_graph, NodeSpec, EdgeSpec};

// Configuration
let config = GraphConfig::native();
let config = GraphConfig::native()
    .with_buffer_size(128 * 1024 * 1024)
    .with_parallel_recovery(8);

// Open graph
let graph = open_graph("path/to/graph.db", &config)?;

// Node
pub struct NodeSpec {
    pub kind: String,
    pub name: String,
    pub file_path: Option<String>,
    pub data: serde_json::Value,
}

// Edge
pub struct EdgeSpec {
    pub from: u64,
    pub to: u64,
    pub edge_type: String,
    pub data: serde_json::Value,
}
```

### Main Methods

| Method | Description |
|--------|-------------|
| `open_graph(path, config)` | Open Native V2 graph |
| `insert_node(spec)` | Insert new node |
| `get_node(id)` | Retrieve node by ID |
| `update_node(&spec)` | Update existing node |
| `delete_node(id)` | Delete node |
| `insert_edge(spec)` | Insert new edge |
| `neighbors(query)` | Get neighbors with query options |
| `snapshot()` | Create MVCC snapshot |

---

## Graph Algorithms API

### Available Algorithms

```rust
use sqlitegraph::algo;

// PageRank - importance ranking
let scores: HashMap<u64, f64> = algo::pagerank(&graph, 0.85, 50)?;
let scores = algo::pagerank_with_progress(&graph, 0.85, 50, progress)?;

// Betweenness Centrality - node importance
let centrality: HashMap<u64, f64> = algo::betweenness_centrality(&graph)?;

// Label Propagation - fast community detection
let communities: HashMap<u64, u64> = algo::label_propagation(&graph)?;

// Louvain - modularity-based clustering
let partition: HashMap<u64, u64> = algo::louvain_communities(&graph, 0.01)?;
```

### Algorithm Characteristics

| Algorithm | Function | Complexity | Returns |
|-----------|----------|------------|---------|
| **PageRank** | `pagerank(graph, damping, iterations)` | O(|E| × iter) | `HashMap<u64, f64>` |
| **Betweenness** | `betweenness_centrality(graph)` | O(|V||E|) | `HashMap<u64, f64>` |
| **Label Propagation** | `label_propagation(graph)` | O(|E|) | `HashMap<u64, u64>` |
| **Louvain** | `louvain_communities(graph, tolerance)` | O(|E| log |V|) | `HashMap<u64, u64>` |

---

## HNSW Vector Search API

### Core Types

```rust
use sqlitegraph::hnsw::{HnswConfig, HnswIndex, DistanceMetric};

// Configuration
let config = HnswConfig::builder()
    .dimension(1536)
    .m_connections(16)
    .ef_construction(200)
    .ef_search(50)
    .distance_metric(DistanceMetric::Cosine)
    .build()?;

// Create index
let hnsw = HnswIndex::new(config)?;
```

### Main Methods

| Method | Description |
|--------|-------------|
| `new(config)` | Create new HNSW index |
| `insert_vector(&vec, metadata)` | Insert vector with optional metadata |
| `search(&query, k)` | Search k nearest neighbors |
| `get_vector(id)` | Retrieve vector by ID |
| `len()` | Get number of vectors |
| `is_empty()` | Check if index is empty |

### Distance Metrics

| Metric | Use Case |
|--------|----------|
| `Cosine` | Text embeddings |
| `Euclidean` | General similarity |
| `DotProduct` | Normalized vectors |
| `Manhattan` | Sparse vectors |

---

## Introspection API

### GraphIntrospection

```rust
use sqlitegraph::introspection::GraphIntrospection;

let intro = GraphIntrospection::new(&graph)?;

// Get statistics
let nodes: usize = intro.node_count()?;
let edges: (usize, usize) = intro.edge_count_estimate()?;
let info: serde_json::Value = intro.backend_info()?;
let json: String = intro.to_json()?;
```

### Methods

| Method | Returns | Description |
|--------|---------|-------------|
| `new(graph)` | `GraphIntrospection` | Create introspection instance |
| `node_count()` | `usize` | Exact node count |
| `edge_count_estimate()` | `(usize, usize)` | (min, max) edge estimate |
| `backend_info()` | `serde_json::Value` | Backend-specific info |
| `to_json()` | `String` | JSON serialization |

---

## Progress Tracking API

### ProgressCallback Trait

```rust
use sqlitegraph::progress::{ProgressCallback, ProgressState, ConsoleProgress, NoProgress};

// Use with algorithms
let scores = algo::pagerank_with_progress(&graph, 0.85, 50, ConsoleProgress::new())?;

// Custom implementation
struct MyProgress;
impl ProgressCallback for MyProgress {
    fn on_progress(&self, state: &ProgressState) {
        println!("{}: {}%", state.message, state.percent);
    }
}
```

### Implementations

| Implementation | Behavior |
|----------------|----------|
| `NoProgress` | No-op, zero overhead |
| `ConsoleProgress` | Progress bars to terminal |

---

## Error Types

### SqliteGraphError

```rust
use sqlitegraph::SqliteGraphError;

match result {
    Ok(value) => /* ... */,
    Err(SqliteGraphError::ValidationError(msg)) => { /* ... */ }
    Err(SqliteGraphError::ConnectionError(msg)) => { /* ... */ }
    Err(SqliteGraphError::TransactionError(msg)) => { /* ... */ }
    Err(SqliteGraphError::NotFoundError(msg)) => { /* ... */ }
    Err(e) => { /* ... */ }
}
```

### HnswConfigError

```rust
use sqlitegraph::hnsw::HnswConfigError;

match HnswConfig::builder().build() {
    Ok(config) => /* ... */,
    Err(HnswConfigError::InvalidDimension) => { /* ... */ }
    Err(HnswConfigError::InvalidMParameter) => { /* ... */ }
    Err(e) => { /* ... */ }
}
```

---

## Feature Flags

```toml
[dependencies]
# SQLite backend only
sqlitegraph = "1.0"

# Native V2 backend
sqlitegraph = { version = "1.0", features = ["native-v2"] }

# V2 I/O tracing (development)
sqlitegraph = { version = "1.0", features = ["trace_v2_io"] }
```

---

## Thread Safety

**Important**: `SqliteGraph` is **NOT thread-safe** for concurrent writes.

For concurrent access:
- Use MVCC snapshots for concurrent reads
- Each thread should have its own graph connection
- Use external synchronization for writes

```rust
// Correct: Snapshot for concurrent reads
let snapshot = graph.snapshot()?;
// snapshot can be cloned and sent to threads
```

---

## Full Documentation

For complete API documentation with examples, see:
- **[docs.rs/sqlitegraph](https://docs.rs/sqlitegraph)** - Official rustdoc
- **[manual.md](manual.md)** - Operator manual
- **[CHANGELOG.md](CHANGELOG.md)** - Version history
