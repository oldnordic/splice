# Architecture Research: Native-V2 Backend Integration for Splice

**Domain:** CLI refactoring tool with dual backend support
**Researched:** 2026-02-09
**Confidence:** HIGH

## Executive Summary

Splice's existing `CodeGraph` abstraction using `Box<dyn GraphBackend>` is already well-positioned for native-v2 backend integration. The sqlitegraph crate provides a unified `GraphBackend` trait that both `SqliteGraphBackend` and `NativeGraphBackend` implement, enabling compile-time feature flag selection with runtime auto-detection of existing databases.

**Key Finding:** Magellan v2.1.0 already demonstrates the integration pattern for dual backend support. Splice can follow the same pattern with minimal changes - primarily adding the `native-v2` feature flag to `Cargo.toml` and updating the backend detection in `CodeGraph::open()`.

The integration requires:
1. **Zero architectural changes** to `CodeGraph`'s public API
2. **Feature flag conditional compilation** for backend selection
3. **Enhanced backend detection** to distinguish SQLite vs Native V2 files
4. **Optional KV store integration** for splice-specific metadata

## Standard Architecture

### System Overview

```
                    Splice CLI
                        |
          ┌─────────────┴─────────────┐
          │                           │
     [Feature Flags]            [Runtime Detection]
          │                           │
    ┌─────┴─────┐             ┌──────┴──────┐
    |   cargo   |             | File Header |
    | features  |             |    Check     |
    └─────┬─────┘             └──────┬──────┘
          │                           │
    ┌─────┴───────────────────────────┴─────┐
    │         GraphBackend Trait             │
    │        (sqlitegraph crate)            │
    ├──────────────┬────────────────────────┤
    │              │                        │
[SQLite]      [Native-V2]           [Future?]
Backend        Backend              Backends
    │              │                        │
SqliteGraph   NativeGraph         (extensible)
    │              │
 ┌───┴───┐      ┌───┴───┐
 │ WAL   │      │  WAL  │
 │ PRAGMA│      │  KV   │
 └───────┘      └───────┘
```

### Component Responsibilities

| Component | Responsibility | Implementation |
|-----------|----------------|----------------|
| **CodeGraph** | Unified graph operations interface | `Box<dyn GraphBackend>` with auto-detection |
| **SqliteGraphBackend** | SQLite-backed graph storage | sqlitegraph's `SqliteGraphBackend` |
| **NativeGraphBackend** | Native file-backed graph storage | sqlitegraph's `NativeGraphBackend` |
| **MagellanIntegration** | Magellan-specific wrapper | Transparent to backend choice |
| **proof::generation** | Snapshot export/import | Uses `GraphBackend` trait methods |
| **GraphConfig** | Backend selection configuration | `sqlitegraph::GraphConfig` |

## Recommended Project Structure

```
splice/
├── Cargo.toml                    # Add native-v2 feature
├── src/
│   ├── graph/
│   │   ├── mod.rs                # UPDATED: Backend detection logic
│   │   ├── magellan_integration.rs  # NO CHANGE: Uses Magellan's abstraction
│   │   ├── schema.rs              # NO CHANGE: Labels unchanged
│   │   └── rename/                # NO CHANGE: Works on source files
│   ├── proof/
│   │   ├── generation.rs         # NO CHANGE: Uses Magellan's APIs
│   │   ├── data_structures.rs    # NO CHANGE: Backend-agnostic
│   │   └── validation.rs         # NO CHANGE: Backend-agnostic
│   └── ingest/
│       └── magellan.rs           # NO CHANGE: Uses Magellan's ingestion
```

### Structure Rationale

- **src/graph/mod.rs**: Backend detection and `GraphConfig` selection only changes needed
- **src/graph/magellan_integration.rs**: No changes - Magellan already abstracts backend
- **src/proof/***: No changes - Proof infrastructure works through Magellan APIs
- **Cargo.toml**: Feature flag addition for conditional compilation

## Architectural Patterns

### Pattern 1: Unified Trait Object Abstraction

**What:** `CodeGraph` uses `Box<dyn GraphBackend>` to store backend-agnostic operations.

**When to use:** When multiple storage backends implement the same interface.

**Trade-offs:**
- **Pros:** Runtime flexibility, clean API surface, easy testing
- **Cons:** Dynamic dispatch overhead (negligible for I/O-bound operations)

**Example (existing):**
```rust
// src/graph/mod.rs - No changes needed!
pub struct CodeGraph {
    backend: Box<dyn GraphBackend>,
    symbol_cache: HashMap<String, Vec<NodeId>>,
    file_cache: HashMap<String, NodeId>,
}
```

### Pattern 2: Compile-Time Feature Flag Selection

**What:** Backend implementation selected at compile time via `cfg` attributes.

**When to use:** When backends have optional dependencies or platform-specific requirements.

**Trade-offs:**
- **Pros:** Zero runtime overhead, dead code elimination, explicit opt-in
- **Cons:** Cannot switch backends without recompilation

**Example (to add):**
```toml
# Cargo.toml
[features]
default = ["unix", "sqlite-backend"]
sqlite-backend = ["sqlitegraph/sqlite-backend"]
native-v2 = ["sqlitegraph/native-v2", "magellan/native-v2"]
unix = []
windows = []
```

**Example (usage):**
```rust
// src/graph/mod.rs - Enhanced detection
impl CodeGraph {
    pub fn open(path: &Path) -> Result<Self> {
        // Compile-time default: SQLite if no feature set
        #[cfg(all(not(feature = "native-v2"), not(feature = "sqlite-backend")))]
        let default_backend = BackendKind::SQLite;

        // If native-v2 feature is set, default to Native
        #[cfg(feature = "native-v2")]
        let default_backend = BackendKind::Native;

        // If sqlite-backend feature is explicitly set, use SQLite
        #[cfg(feature = "sqlite-backend")]
        let default_backend = BackendKind::SQLite;

        // Runtime detection of existing files
        let detected = if path.exists() {
            Self::detect_backend_format(path)?
        } else {
            default_backend
        };

        let cfg = match detected {
            BackendKind::SQLite => sqlitegraph::GraphConfig::sqlite(),
            BackendKind::Native => sqlitegraph::GraphConfig::native(),
        };

        let backend = sqlitegraph::open_graph(path, &cfg)?;
        Ok(Self {
            backend,
            symbol_cache: HashMap::new(),
            file_cache: HashMap::new(),
        })
    }

    fn detect_backend_format(path: &Path) -> Result<BackendKind> {
        // Check for SQLite magic header
        if !path.exists() {
            return Ok(BackendKind::Native); // Default for new files
        }

        let mut file = std::fs::File::open(path)?;
        let mut header = [0u8; 16];
        file.read_exact(&mut header)?;

        if &header[..15] == b"SQLite format 3" {
            Ok(BackendKind::SQLite)
        } else {
            Ok(BackendKind::Native)
        }
    }
}
```

### Pattern 3: Magellan's Conditional Backend Initialization

**What:** Magellan already demonstrates the integration pattern in `src/graph/mod.rs:189-350`.

**When to use:** When your dependency (Magellan) already handles backend abstraction.

**Trade-offs:**
- **Pros:** Leverage existing implementation, no reinvention
- **Cons:** Must match Magellan's feature flag configuration

**Example (from Magellan - for reference):**
```rust
// Magellan's pattern - splice inherits via MagellanIntegration
#[cfg(feature = "native-v2")]
let backend: Rc<dyn GraphBackend> = {
    use sqlitegraph::NativeGraphBackend;
    let native_graph = if db_path_buf.exists() {
        NativeGraphBackend::open(&db_path_buf)?
    } else {
        NativeGraphBackend::new(&db_path_buf)?
    };
    Rc::new(native_graph)
};

#[cfg(not(feature = "native-v2"))]
let backend: Rc<dyn GraphBackend> = {
    use sqlitegraph::{SqliteGraph, SqliteGraphBackend};
    let sqlite_graph = SqliteGraph::open(&db_path_buf)?;
    Rc::new(SqliteGraphBackend::from_graph(sqlite_graph))
};
```

## Data Flow

### Database Open Flow

```
[User runs splice]
    ↓
[CodeGraph::open(path)]
    ↓
[detect_backend_format(path)]  ← Check file header
    ↓                    ↓
[SQLite header?]      [No header / Native]
    ↓                    ↓
[GraphConfig::    [GraphConfig::
 sqlite()]          native()]
    ↓                    ↓
[sqlitegraph::        [sqlitegraph::
 open_graph()]        open_graph()]
    ↓                    ↓
    └────────┬──────────┘
             ↓
    [Box<dyn GraphBackend>]
             ↓
    [All operations work identically]
```

### Feature Flag Compile Flow

```
[cargo build --features native-v2]
    ↓
[sqlitegraph/native-v2 feature enabled]
    ↓
[magellan/native-v2 feature enabled]
    ↓
[NativeGraphBackend compiled in]
    ↓
[SqliteGraphBackend compiled out]
    ↓
[Binary uses Native backend by default]
```

## Integration Points

### External Services

| Service | Integration Pattern | Notes |
|---------|---------------------|-------|
| **sqlitegraph** | `GraphBackend` trait | Both backends implement same trait |
| **magellan** | `MagellanIntegration` wrapper | Already abstracts backend choice |
| **tree-sitter** | Parser ingestion | Unaffected by backend choice |

### Internal Boundaries

| Boundary | Communication | Notes |
|----------|---------------|-------|
| **CodeGraph ↔ MagellanIntegration** | Direct method calls | Magellan already handles both backends |
| **CodeGraph ↔ proof::generation** | `MagellanIntegration` APIs | Proof generation uses Magellan queries |
| **splice CLI ↔ CodeGraph** | `open()`, `inner()`, `inner_mut()` | No API changes needed |

## API-Level Differences: SQLite vs Native-V2

### Identical Operations (No Changes Required)

| Operation | SQLite | Native-V2 |
|-----------|--------|-----------|
| `insert_node()` | `INSERT INTO entities` | Append to node store |
| `insert_edge()` | `INSERT INTO edges` | Append to edge store |
| `get_node()` | `SELECT FROM entities` | Read from node store |
| `neighbors()` | `SELECT FROM edges` | Read adjacency |
| `bfs()` | Queue-based traversal | Same algorithm |
| `checkpoint()` | `PRAGMA wal_checkpoint` | Flush WAL |

### Native-V2 Exclusive Operations

| Operation | Purpose | Available When |
|-----------|---------|----------------|
| `kv_get()` / `kv_set()` | O(1) metadata storage | `feature = "native-v2"` |
| `subscribe()` / `unsubscribe()` | Pub/sub change events | `feature = "native-v2"` |
| `kv_prefix_scan()` | Prefix-based KV queries | `feature = "native-v2"` |

### SQLite-Exclusive Operations

| Operation | Purpose | Available When |
|-----------|---------|----------------|
| Raw SQL queries | Complex joins/ad-hoc analysis | `feature = "sqlite-backend"` |
| External tools | DB Browser for SQLite | `feature = "sqlite-backend"` |

## Feature Flag Structure

### Recommended Cargo.toml Changes

```toml
[package]
name = "splice"
version = "2.5.0"  # Bump for native-v2 support

[features]
# Platform features (existing)
default = ["unix", "sqlite-backend"]
unix = []
windows = []

# Backend selection (NEW)
sqlite-backend = ["sqlitegraph/sqlite-backend", "magellan/sqlite-backend"]
native-v2 = ["sqlitegraph/native-v2", "magellan/native-v2"]

[dependencies]
magellan = { version = "2.1", path = "../magellan", default-features = false, optional = false }
sqlitegraph = { version = "1.5.5", default-features = false }

# For sqlite-backend feature
[dependencies.rusqlite]
version = "0.31"
features = ["bundled"]
optional = true
```

### Build Commands

```bash
# Default (SQLite backend)
cargo build --release

# Native-V2 backend
cargo build --release --features native-v2

# Explicit SQLite
cargo build --release --features sqlite-backend

# With native-v2 perf optimizations
cargo build --release --features "native-v2,native-v2-perf"
```

## Snapshot Export/Import Compatibility

### Existing Proof Infrastructure

Splice's `proof::generation` module uses `MagellanIntegration` APIs which are already backend-agnostic:

```rust
// src/proof/generation.rs - NO CHANGES NEEDED
pub fn generate_snapshot(db_path: &Path) -> Result<GraphSnapshot> {
    let mut integration = MagellanIntegration::open(db_path)?;

    // These methods work on both backends
    let file_nodes = integration.inner_mut().all_file_nodes()?;
    let symbols_in_file = integration.inner_mut().symbols_in_file(file_path)?;
    let callees = integration.inner_mut().calls_from_symbol(&file_path, &name)?;

    // Build snapshot...
}
```

### Backend-Agnostic Export

The `GraphBackend::snapshot_export()` method is implemented for both backends:

```rust
// Called from splice commands
let snapshot_metadata = code_graph
    .inner()
    .snapshot_export(&export_dir)?;

// SQLite backend → JSON dump
// Native-V2 backend → V2 binary format
```

## Anti-Patterns

### Anti-Pattern 1: Direct SQLite Queries

**What people do:** Use `rusqlite::Connection` directly for custom queries.

**Why it's wrong:** Breaks when using native-v2 backend (no SQLite file).

**Do this instead:**
```rust
// ❌ WRONG
use rusqlite::Connection;
let conn = Connection::open(&db_path)?;
let count: i64 = conn.query_row("SELECT COUNT(*) FROM graph_entities", [], |r| r.get(0))?;

// ✅ CORRECT
let ids = code_graph.inner().entity_ids()?;
let count = ids.len();
```

### Anti-Pattern 2: Hardcoded SQLite Assumptions

**What people do:** Assume `.db` files are always SQLite format.

**Why it's wrong:** Native-V2 uses a custom binary format.

**Do this instead:**
```rust
// ✅ CORRECT - Use backend detection
let backend_kind = if CodeGraph::is_sqlite_db(path)? {
    BackendKind::SQLite
} else {
    BackendKind::Native
};
```

### Anti-Pattern 3: Bypassing GraphBackend Trait

**What people do:** Access underlying backend directly for "performance".

**Why it's wrong:** Defeats the purpose of abstraction, breaks portability.

**Do this instead:**
```rust
// ✅ CORRECT - Always use trait methods
let node = code_graph.inner().get_node(SnapshotId::current(), node_id)?;
```

## Implementation Phases

### Phase 1: Feature Flag Infrastructure (Minimal)

1. Add `native-v2` feature to `Cargo.toml`
2. Update dependencies to make sqlitegraph features optional
3. No code changes required - sqlitegraph handles conditional compilation

### Phase 2: Enhanced Backend Detection (Recommended)

1. Add `detect_backend_format()` helper to `CodeGraph`
2. Update `CodeGraph::open()` to handle native-v2 files
3. Add documentation on backend selection behavior

### Phase 3: KV Store Integration (Optional)

1. Use `kv_set()` for splice-specific metadata (caches, indexes)
2. Implement change notifications via `subscribe()` for watcher scenarios
3. Benchmark KV performance vs existing in-memory caches

### Phase 4: Testing & Validation (Required)

1. Test suite with both backends enabled
2. Migration tests: SQLite → Native-V2 → SQLite
3. Performance benchmarking for common operations

## Build Order Considerations

1. **sqlitegraph** must be built first (provides `GraphBackend` trait)
2. **magellan** depends on sqlitegraph, must be built with matching features
3. **splice** depends on both, must align feature flags

**Critical:** All three crates must use the same sqlitegraph version and compatible feature sets.

## Confidence Assessment

| Area | Confidence | Reasoning |
|------|------------|-----------|
| Stack compatibility | HIGH | Verified sqlitegraph v1.5.5 and magellan v2.1.0 both support native-v2 |
| API compatibility | HIGH | `GraphBackend` trait is identical for both backends |
| Magellan integration | HIGH | Magellan already demonstrates dual backend support |
| Proof infrastructure | HIGH | Uses Magellan APIs which abstract backend |
| Required changes | HIGH | Minimal - feature flags + detection logic only |
| KV store integration | MEDIUM | Optional enhancement, needs design decisions |
| Performance impact | MEDIUM | No benchmarks yet, depends on workload |

## Sources

- [sqlitegraph GitHub Repository](https://github.com/agentflare-ai/sqlite-graph)
- [sqlitegraph v1.5.5 Documentation](https://docs.rs/sqlitegraph) - Backend trait unified interface
- [Magellan v2.1.0 Source Code](https://github.com/oldnordic/magellan) - Lines 189-350 demonstrate native-v2 integration
- Magellan CHANGELOG.md - Native-v2 backend support announced in v2.1.0
- /home/feanor/Projects/sqlitegraph/sqlitegraph/src/backend.rs - GraphBackend trait definition
- /home/feanor/Projects/magellan/src/graph/mod.rs - Conditional compilation pattern
- /home/feanor/Projects/splice/src/graph/mod.rs - Existing CodeGraph abstraction

---
*Architecture research for: Native-V2 Backend Integration for Splice*
*Researched: 2026-02-09*
