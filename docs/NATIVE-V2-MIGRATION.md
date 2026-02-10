# Native-V2 Migration Plan

**Created:** 2026-02-09
**Status:** Planning Phase
**Target:** Full native-v2 support across all tools

---

## Executive Summary

This document outlines the migration plan for updating the SQLiteGraph toolset (llmgrep, Mirage, splice) to support the native-v2 backend. The goal is feature parity between SQLite and native-v2 backends, plus new features enabled by native-v2's capabilities.

**Context:** Magellan v2.2.0 achieved full algorithm parity with native-v2. The dependent tools must be updated to leverage this foundation.

---

## The SQLiteGraph Toolset

```
┌─────────────────────────────────────────────────────────────────┐
│                     sqlitegraph 1.5.5                            │
│  (Graph Database Foundation - 35 algorithms, dual backend)      │
└───────────────────────┬─────────────────────────────────────────┘
                        │
┌───────────────────────┴─────────────────────────────────────────┐
│                    Magellan 2.2.0                               │
│  (Indexer - creates codegraph.db with native-v2 support)        │
└───────────────────────┬─────────────────────────────────────────┘
                        │ codegraph.db
        ┌───────────────┼───────────────┐
        ▼               ▼               ▼
┌──────────────┐ ┌──────────────┐ ┌──────────────┐
│   llmgrep    │ │    Mirage    │ │    splice    │
├──────────────┤ ├──────────────┤ ├──────────────┤
│ Status: TODO │ │ Status: TODO │ │ Status: TODO │
│ Priority: 1  │ │ Priority: 3  │ │ Priority: 2  │
└──────────────┘ └──────────────┘ └──────────────┘
```

---

## Native-V2 Backend Advantages

### Performance Benefits

| Feature | SQLite Backend | Native-V2 Backend | Improvement |
|---------|----------------|-------------------|-------------|
| Symbol lookup by FQN | Table scan | O(1) KV get | 10-100x faster |
| File symbols listing | JOIN query | Prefix scan | 5-20x faster |
| Graph traversal | Row-by-row | Clustered adjacency | 2-5x faster |
| Call graph queries | Multiple queries | KV index lookups | 3-10x faster |

### New Capabilities

| Feature | Description |
|---------|-------------|
| **Snapshot System** | Export/import full database state at any point |
| **Pub/Sub Events** | Real-time notification of graph changes |
| **Clustered Adjacency** | Optimized edge storage for traversal-heavy workloads |
| **TTL Expiration** | Automatic cache invalidation for time-based data |
| **WAL Recovery** | Crash recovery with write-ahead logging |

### File Size Benefits

- Native-v2 databases are typically **70%+ smaller** than SQLite equivalents
- Better for large codebases and CI/CD artifact storage

---

## Tool-Specific Migration Plans

### 1. llmgrep (Priority: 1)

**Current State:**
- Uses Magellan library API (`magellan::CodeGraph`)
- Benefits automatically from Magellan's native-v2 support
- No direct SQL queries

**Migration Effort:** LOW
- Already uses library API
- May need feature flags for native-v2 detection

**New Features Enabled:**

```bash
# O(1) symbol lookup via KV
llmgrep lookup --fqn "crate::module::function" --db codegraph.db

# Fuzzy/autocomplete search using prefix scans
llmgrep complete --prefix "parse_" --db codegraph.db

# Purpose-based semantic search (uses label scanning)
llmgrep search --purpose "authentication" --db codegraph.db

# Watch mode - live search updates via pub/sub
llmgrep watch --query "Widget" --db codegraph.db
```

**Implementation Tasks:**
- [ ] Add `--detect-backend` flag to show active backend
- [ ] Implement `complete` command for prefix search
- [ ] Add `purpose` search mode using label scans
- [ ] Add `watch` command for real-time updates
- [ ] Update documentation for native-v2

---

### 2. splice (Priority: 2)

**Current State:**
- Uses Magellan library API (`magellan::CodeGraph`)
- Benefits automatically from Magellan's native-v2 support
- Has its own graph algorithms integration

**Migration Effort:** LOW
- Already uses library API
- Graph algorithms now work with both backends

**New Features Enabled:**

```bash
# Snapshot before edit for safe rollback
splice rename --symbol "foo" --to "bar" --snapshot-before

# Generate refactor proof by comparing snapshots
splice verify --before snapshot_v1 --after snapshot_v2

# Dry-run with impact visualization
splice rename --symbol "foo" --to "bar" --preview --impact-graph

# Multi-file batch refactor with proof
splice batch --refactors refactor.yaml --generate-proof
```

**Implementation Tasks:**
- [ ] Add snapshot export/import to refactor workflow
- [ ] Implement `verify` command for comparison
- [ ] Add `--impact-graph` visualization
- [ ] Implement `batch` command
- [ ] Update documentation for native-v2

---

### 3. Mirage (Priority: 3)

**Current State:**
- Uses direct SQL queries via `rusqlite`
- Extends Magellan schema with own tables:
  - `cfg_blocks` - Basic blocks within functions
  - `cfg_edges` - Control flow between blocks
  - `cfg_paths` - Enumerated execution paths
  - `cfg_dominators` - Dominance relationships
  - `cfg_post_dominators` - Reverse dominance

**Migration Effort:** HIGH
- Requires storage layer rewrite
- SQL tables → KV storage pattern
- Must maintain backward compatibility

**New Storage Pattern (KV):**

| Data Type | SQL Table | KV Key Pattern |
|-----------|-----------|-----------------|
| CFG blocks | `cfg_blocks` | `cfg:blocks:{function_id}` |
| CFG edges | `cfg_edges` | `cfg:edges:{function_id}` |
| Cached paths | `cfg_paths` | `cfg:paths:{function_id}` |
| Dominators | `cfg_dominators` | `cfg:dom:{function_id}` |
| Post-dominators | `cfg_post_dominators` | `cfg:pdom:{function_id}` |

**New Features Enabled:**

```bash
# CFG diff between snapshots (what changed?)
mirage diff --function "process" --before v1 --after v2

# Incremental analysis - only changed functions
mirage paths --incremental --since HEAD~1

# Hot path detection - most-traversed paths
mirage hotpaths --db codegraph.db

# Inter-procedural CFG across function boundaries
mirage icfg --entry "main" --depth 3
```

**Implementation Tasks:**
- [ ] Create backend-agnostic storage trait
- [ ] Implement KV storage backend
- [ ] Migrate SQL table data to KV format
- [ ] Add migration command for existing databases
- [ ] Implement snapshot-based CFG diff
- [ ] Add incremental path analysis
- [ ] Update documentation for native-v2

---

## Shared Cross-Tool Features

### Backend Detection

All tools should support:
```bash
tool-name --detect-backend --db codegraph.db
# Output: "native-v2" or "sqlite"
```

### Migration Utility

```bash
tool-name --migrate --from sqlite --to native-v2 --db codegraph.db
```

### Verification Mode

```bash
tool-name --verify --db codegraph.db
# Runs cross-backend consistency checks
```

---

## Implementation Phases

### Phase 1: Foundation (Week 1)
- [ ] Add backend detection to all tools
- [ ] Update dependencies (sqlitegraph 1.5.5+, magellan 2.2.0+)
- [ ] Add `--features native-v2` to all build scripts
- [ ] Document native-v2 benefits in each README

### Phase 2: llmgrep (Week 2)
- [ ] Implement `complete` command
- [ ] Add `purpose` search mode
- [ ] Add `watch` command (pub/sub)
- [ ] Full testing with both backends

### Phase 3: splice (Week 3)
- [ ] Add snapshot integration
- [ ] Implement `verify` command
- [ ] Add `--impact-graph` flag
- [ ] Implement `batch` command
- [ ] Full testing with both backends

### Phase 4: Mirage (Week 4-5)
- [ ] Design backend-agnostic storage trait
- [ ] Implement KV storage backend
- [ ] Migration utility for existing data
- [ ] Implement new features (diff, incremental, hotpaths)
- [ ] Full testing with both backends

### Phase 5: Documentation (Week 6)
- [ ] Update all README.md files
- [ ] Create migration guides
- [ ] Add examples in MANUAL.md
- [ ] Cross-tool integration examples

---

## Testing Strategy

### Cross-Backend Tests

For each tool, create tests that verify:
1. Same output on identical databases (SQLite vs native-v2)
2. Performance benchmarks
3. Feature parity verification

### Test Matrix

| Tool | Command | SQLite | Native-V2 | Notes |
|------|---------|--------|-----------|-------|
| llmgrep | search | ✅ | ✅ | Same results |
| llmgrep | complete | N/A | ✅ | New feature |
| splice | rename | ✅ | ✅ | Same results |
| splice | verify | N/A | ✅ | New feature |
| mirage | paths | ✅ | ✅ | Same results |
| mirage | diff | N/A | ✅ | New feature |

---

## Dependencies

All tools must update to:
- `sqlitegraph >= 1.5.5`
- `magellan >= 2.2.0`

Update `Cargo.toml`:
```toml
[dependencies]
sqlitegraph = { version = "1.5.5", default-features = false }
magellan = { version = "2.2.0" }
```

---

## Success Criteria

A tool is considered migrated when:
1. ✅ All existing commands work with native-v2 backend
2. ✅ Output is identical between SQLite and native-v2 (same input)
3. ✅ Performance is equal or better than SQLite
4. ✅ Documentation updated with native-v2 notes
5. ✅ Tests pass with `--features native-v2` flag

---

## Resources

- [Magellan Native-V2 Manual](../magellan/MANUAL.md#6-backend-compatibility)
- [sqlitegraph Documentation](https://docs.rs/sqlitegraph)
- [KV Storage Guide](../magellan/docs/NATIVE-V2.md)

---

## Notes

- SDK/Library development deferred until after battle testing
- Focus on CLI tool functionality first
- Maintain backward compatibility with SQLite format
- All changes must be non-breaking for existing users
