# Stack Research: Native-V2 Backend Integration for Splice

**Domain:** Span-safe refactoring kernel with optional native-v2 backend support
**Researched:** 2026-02-09
**Confidence:** HIGH

## Executive Summary

Splice currently hardcodes `features = ["native-v2"]` on the magellan dependency. This research documents the stack changes needed to make native-v2 an optional feature flag on splice itself, keeping SQLite as the default backend while enabling new features when native-v2 is active.

**Key finding:** The sqlitegraph and magellan ecosystems already provide the necessary feature flag infrastructure. Splice needs only to expose these features through its own feature declarations and conditionally compile native-v2-specific functionality.

## Recommended Stack

### Core Dependencies (No Changes Required)

| Technology | Current Version | Required for native-v2 | Purpose |
|------------|----------------|------------------------|---------|
| `sqlitegraph` | 1.3.0 -> 1.5.5+ | YES | Graph storage with dual backend support |
| `magellan` | 2.1 -> 2.1+ | YES | Code indexing with native-v2 feature propagation |

**Why upgrade sqlitegraph:**
- Version 1.5.5+ includes the complete native-v2 feature set with stable API
- Current splice uses 1.3.0 which lacks native-v2 exports
- Native-v2 was introduced in sqlitegraph 1.5.0 and stabilized in 1.5.5

### New Feature Flags Required

```toml
[features]
default = ["unix", "sqlite-backend"]
sqlite-backend = ["magellan/sqlite-backend", "sqlitegraph/sqlite-backend"]
native-v2 = ["magellan/native-v2", "sqlitegraph/native-v2"]
# Optional performance feature
native-v2-perf = ["native-v2", "magellan/native-v2-perf", "sqlitegraph/v2_experimental"]
```

**Feature flag rationale:**
- `sqlite-backend`: Explicitly opt into SQLite (default for backward compatibility)
- `native-v2`: Optional native backend with WAL, snapshots, and KV store
- `native-v2-perf`: Experimental performance optimizations (advanced users)

## Native-V2 Backend Features

### 1. Write-Ahead Logging (WAL)

**Source:** `/home/feanor/Projects/sqlitegraph/sqlitegraph/src/backend/native/v2/wal/mod.rs`

The WAL system provides:
- 5-10x write throughput improvement through sequential I/O
- ACID transactions with crash recovery
- Cluster-affinity logging for V2's edge architecture
- Incremental checkpointing

**API Surface:**
```rust
// Re-exported when native-v2 feature is enabled
use sqlitegraph::{
    V2WALConfig, V2WALManager, WALManagerMetrics,
    IsolationLevel, GraphWALIntegrationConfig
};
```

### 2. Backup and Restore

**Source:** `/home/feanor/Projects/sqlitegraph/sqlitegraph/src/backend/native/v2/backup/mod.rs`

- **Backup:** Creates consistent snapshots with optional checkpoint
- **Restore:** Recovers from backup with overwrite control
- Both functions are re-exported at crate root when `native-v2` is enabled

**API Surface:**
```rust
// Re-exported when native-v2 feature is enabled
use sqlitegraph::{
    create_backup, BackupConfig, BackupResult,
    restore_from_backup, RestoreConfig, RestoreResult
};
```

### 3. Snapshot Export/Import

**Source:** `/home/feanor/Projects/sqlitegraph/sqlitegraph/src/backend/native/v2/export/mod.rs`

Instant database state exports without WAL complexity:
- Atomic graph file copies
- No WAL recovery needed for restore
- Four export modes: CheckpointAligned, LsnBounded, Full, Snapshot

**API Surface:**
```rust
// Re-exported when native-v2 feature is enabled
use sqlitegraph::backend::native::v2::export::{
    SnapshotExportConfig, SnapshotExporter,
    SnapshotImportConfig, SnapshotImporter,
    ExportMode
};
```

### 4. Key-Value Store

**Source:** `/home/feanor/Projects/sqlitegraph/sqlitegraph/src/backend/native/v2/kv_store/mod.rs`

Transactional KV store built as a VIEW over V2 storage:
- O(1) HashMap lookups
- WAL integration for durability
- Snapshot isolation via snapshot_id
- Lazy TTL cleanup (no background threads)

**API Surface:**
```rust
// Re-exported when native-v2 feature is enabled
use sqlitegraph::backend::native::v2::kv_store::{
    KvStore, KvValue, KvEntry, KvMetadata, KvStoreError
};
```

## Feature Flag Patterns in Rust

### Conditional Compilation

```rust
// Feature-gated code
#[cfg(feature = "native-v2")]
pub mod native_v2_commands;

#[cfg(feature = "native-v2")]
use sqlitegraph::{create_backup, restore_from_backup};

#[cfg(not(feature = "native-v2"))]
pub fn native_v2_not_enabled() -> Result<(), SpliceError> {
    Err(SpliceError::Other(
        "native-v2 feature is not enabled. \
         Use: cargo install splice --features native-v2".to_string()
    ))
}
```

### Dependency Feature Propagation

```toml
[dependencies]
magellan = { version = "2.1", path = "../magellan", default-features = false }
sqlitegraph = { version = "1.5.5", default-features = false }

[features]
default = ["unix", "sqlite-backend"]
sqlite-backend = ["magellan/sqlite-backend", "sqlitegraph/sqlite-backend"]
native-v2 = ["magellan/native-v2", "sqlitegraph/native-v2"]
```

**Why `default-features = false`:**
- Prevents implicit feature activation
- Ensures explicit backend selection
- Avoids pulling in both backends simultaneously

## Splice Integration Points

### 1. CodeGraph Abstraction (Already Backend-Agnostic)

**Source:** `/home/feanor/Projects/splice/src/graph/mod.rs`

The `CodeGraph` struct already uses the `GraphBackend` trait and auto-detects backend format:

```rust
pub fn open(path: &std::path::Path) -> Result<Self> {
    let cfg = if Self::is_sqlite_db(path)? {
        sqlitegraph::GraphConfig::sqlite()
    } else {
        sqlitegraph::GraphConfig::native()
    };
    // ... works with either backend
}
```

**Implication:** No changes needed to core graph operations. The abstraction already supports both backends.

### 2. New Commands to Add (Feature-Gated)

| Command | Purpose | Feature Required |
|---------|---------|------------------|
| `splice backup` | Create database backup | native-v2 |
| `splice restore` | Restore from backup | native-v2 |
| `splice snapshot export` | Export database snapshot | native-v2 |
| `splice snapshot import` | Import database snapshot | native-v2 |
| `splice verify` | Verify database integrity | native-v2 |
| `splice batch` | Batch operations with WAL | native-v2 |

### 3. CLI Feature Detection

```rust
// At startup, log active features
#[cfg(feature = "native-v2")]
{
    eprintln!("Native-V2 backend support: ENABLED");
}

#[cfg(not(feature = "native-v2"))]
{
    eprintln!("Native-V2 backend support: disabled (use --features native-v2)");
}
```

## Version Compatibility Matrix

| Splice Version | sqlitegraph | magellan | Features Available |
|----------------|-------------|----------|-------------------|
| 2.4.1 (current) | 1.3.0 | 2.1 | SQLite only (native-v2 hardcoded, not exposed) |
| 2.5.0 (target) | 1.5.5+ | 2.1+ | SQLite (default) + native-v2 (optional) |
| 2.5.0-perf | 1.5.5+ | 2.1+ | All features + experimental optimizations |

**Breaking changes:** None. SQLite remains default. native-v2 is opt-in.

## Installation Examples

```bash
# Default: SQLite backend only
cargo install splice

# Native-V2 backend support
cargo install splice --features native-v2

# Native-V2 with experimental performance features
cargo install splice --features native-v2-perf

# Development: enable everything
cargo build --features "native-v2,native-v2-perf"
```

## What NOT to Do

| Anti-Pattern | Why | Instead |
|--------------|------|---------|
| Make native-v2 the default | Breaking change for existing users | Keep SQLite as default |
| Remove SQLite support | Large ecosystem depends on it | Maintain both backends |
| Hardcode backend selection | Defeats purpose of dual backend | Use auto-detection (already implemented) |
| Add direct native-v2 dependencies | Bypasses sqlitegraph abstraction | Use sqlitegraph re-exports |

## Conditional Compilation Example: New Commands

```rust
// src/commands/mod.rs
pub mod backup;
pub mod restore;
pub mod snapshot;
pub mod verify;
pub mod batch;

// src/commands/backup.rs
#[cfg(feature = "native-v2")]
pub fn run_backup(db_path: &Path, backup_dir: &Path) -> Result<(), SpliceError> {
    use sqlitegraph::create_backup;
    let config = BackupConfig::new(backup_dir);
    let result = create_backup(db_path, config)?;
    println!("Backup created: {}", result.snapshot_path.display());
    Ok(())
}

#[cfg(not(feature = "native-v2"))]
pub fn run_backup(_db_path: &Path, _backup_dir: &Path) -> Result<(), SpliceError> {
    Err(SpliceError::Other(
        "backup command requires native-v2 feature. \
         Install with: cargo install splice --features native-v2".to_string()
    ))
}
```

## Integration with Existing CodeGraph

The existing `CodeGraph` inner() and inner_mut() methods provide access to the underlying GraphBackend trait:

```rust
// src/graph/mod.rs (existing code)
pub fn inner(&self) -> &dyn GraphBackend {
    self.backend.as_ref()
}

// New native-v2 functionality can be added as extension traits
#[cfg(feature = "native-v2")]
pub trait CodeGraphNativeV2Ext {
    fn backup(&self, backup_dir: &Path) -> Result<BackupResult, SpliceError>;
    fn checkpoint(&mut self) -> Result<(), SpliceError>;
}
```

## Sources

### High Confidence (Direct Source Code Verification)
- `/home/feanor/Projects/sqlitegraph/sqlitegraph/Cargo.toml` — Feature flag definitions
- `/home/feanor/Projects/sqlitegraph/sqlitegraph/src/lib.rs` — Public API re-exports
- `/home/feanor/Projects/sqlitegraph/sqlitegraph/src/backend.rs` — GraphBackend trait with native-v2 methods
- `/home/feanor/Projects/sqlitegraph/sqlitegraph/src/config/` — Backend selection API
- `/home/feanor/Projects/sqlitegraph/sqlitegraph/src/backend/native/v2/mod.rs` — V2 module organization
- `/home/feanor/Projects/sqlitegraph/sqlitegraph/src/backend/native/v2/wal/mod.rs` — WAL API
- `/home/feanor/Projects/sqlitegraph/sqlitegraph/src/backend/native/v2/backup/mod.rs` — Backup API
- `/home/feanor/Projects/sqlitegraph/sqlitegraph/src/backend/native/v2/export/mod.rs` — Snapshot export API
- `/home/feanor/Projects/sqlitegraph/sqlitegraph/src/backend/native/v2/kv_store/mod.rs` — KV store API
- `/home/feanor/Projects/magellan/Cargo.toml` — Magellan feature flags
- `/home/feanor/Projects/splice/Cargo.toml` — Current splice dependencies
- `/home/feanor/Projects/splice/src/graph/mod.rs` — CodeGraph implementation

### Version Information
- sqlitegraph local source: version 1.5.5
- magellan local source: version 2.1.0
- Cargo.lock resolved: magellan 2.1.1, sqlitegraph 1.4.2 (registry)
- Target sqlitegraph version: 1.5.5+ for native-v2 stability

---
*Stack research for: Splice native-v2 backend integration*
*Researched: 2026-02-09*
