# SQLiteGraph API Differences: 0.2.11 → 1.0

**Date:** 2026-01-17
**Purpose:** Document API changes, breaking changes, and migration path for Splice

---

## 1. Version Availability

### SQLiteGraph 1.0.0 Published

```bash
$ cargo search sqlitegraph
sqlitegraph = "1.0.0"  # Deterministic, embedded graph database with SQLite and Native V2 backends
```

**Status:** Version 1.0.0 is published on crates.io and ready for use.

### Exported Types (v1.0)

The following types are confirmed to be exported by sqlitegraph 1.0:

**Native V2 Backend:**
- `sqlitegraph::GraphConfig::native()` - Configuration for Native V2 backend
- `sqlitegraph::open_graph()` - Open graph function
- `sqlitegraph::NodeSpec` - Node specification struct
- `sqlitegraph::EdgeSpec` - Edge specification struct
- `sqlitegraph::NodeId` - Node identifier type
- `sqlitegraph::GraphBackend` - Backend trait

**Common Types:**
- `sqlitegraph::Label` - Label type for schema definitions
- `sqlitegraph::PropertyKey` - Property key type for schema definitions
- `sqlitegraph::SqliteGraphError` - Error type

---

## 2. Type Compatibility Matrix

| Current Type (0.2.11) | v1.0 Equivalent | Backend | Compatibility | Notes |
|----------------------|-----------------|---------|---------------|-------|
| `GraphBackend` trait | `GraphBackend` trait | Native V2 | ✅ Compatible | Same trait, verify methods |
| `NodeId` | `NodeId` | Both | ✅ Compatible | Same type name |
| `NodeSpec` | `NodeSpec` | Native V2 | ✅ Compatible | Same fields: kind, name, file_path, data |
| `EdgeSpec` | `EdgeSpec` | Native V2 | ✅ Compatible | Same fields: from, to, edge_type, data |
| `Label` | `Label` | Both | ✅ Compatible | Same type name |
| `PropertyKey` | `PropertyKey` | Both | ✅ Compatible | Same type name |
| `GraphConfig::sqlite()` | `GraphConfig::native()` | Native V2 | ⚠️ Change | Different config method |
| `open_graph(path, cfg)` | `open_graph(path, cfg)` | Native V2 | ✅ Compatible | Same function signature |
| `SqliteGraphError` | `SqliteGraphError` | Both | ✅ Compatible | Same error type |

### Key Field Verification

**NodeSpec fields (unchanged):**
```rust
pub struct NodeSpec {
    pub kind: String,
    pub name: String,
    pub file_path: Option<String>,
    pub data: serde_json::Value,
}
```

**EdgeSpec fields (unchanged):**
```rust
pub struct EdgeSpec {
    pub from: u64,
    pub to: u64,
    pub edge_type: String,
    pub data: serde_json::Value,
}
```

---

## 3. Feature Flag Requirement

### Native V2 Backend Requires Feature Flag

**Before (0.2.11):**
```toml
[dependencies]
sqlitegraph = { version = "0.2.11", default-features = false }
```

**After (1.0 with Native V2):**
```toml
[dependencies]
sqlitegraph = { version = "1.0", default-features = false, features = ["native-v2"] }
```

**Note:** The `default-features = false` is kept to minimize dependencies.

---

## 4. Code Changes Required by File

### src/graph/mod.rs (284 lines)

**Line 34:** Configuration change required
```rust
// Before (0.2.11)
let cfg = sqlitegraph::GraphConfig::sqlite();

// After (1.0)
let cfg = sqlitegraph::GraphConfig::native();
```

**Line 13:** Verify imports (no change expected)
```rust
// Current (should still work)
use sqlitegraph::{EdgeSpec, GraphBackend, NodeId, NodeSpec};
```

**Lines 66-75, 117-128, 188-195:** NodeSpec usage (no change expected)
- Fields are identical: kind, name, file_path, data
- No code changes needed

**Lines 135-141:** EdgeSpec usage (no change expected)
- Fields are identical: from, to, edge_type, data
- No code changes needed

### src/graph/schema.rs (131 lines)

**Line 7:** Verify imports (no change expected)
```rust
// Current (should still work)
use sqlitegraph::{Label, PropertyKey};
```

**All functions:** No code changes expected
- Label and PropertyKey types unchanged
- All schema functions should work as-is

### src/error.rs (line 28)

**Error type integration (no change expected)**
```rust
// Current (should still work)
#[from] sqlitegraph::SqliteGraphError
```

### src/resolve/mod.rs (line 13)

**NodeId type usage (no change expected)**
```rust
// Current (should still work)
use sqlitegraph::NodeId
```

---

## 5. Summary of Required Changes

### Single Line Change

Only **one line** needs to be changed in the entire codebase:

**File:** `src/graph/mod.rs`
**Line:** 34
**Change:** `GraphConfig::sqlite()` → `GraphConfig::native()`

### Cargo.toml Change

**File:** `Cargo.toml`
**Line:** 18
**Change:**
```toml
# Before
sqlitegraph = { version = "0.2.11", default-features = false }

# After
sqlitegraph = { version = "1.0", default-features = false, features = ["native-v2"] }
```

---

## 6. Database Compatibility Considerations

### Backend Migration Risk

**Critical:** Native V2 backend uses a different storage format than SQLite backend.

**Key Questions:**

1. **Can existing 0.2.11 databases be opened with v1.0?**
   - **No.** Native V2 backend uses a completely different storage format.
   - 0.2.11 used SQLite backend
   - 1.0 with Native V2 uses clustered adjacency storage

2. **Is a migration script needed?**
   - **Yes.** This is a backend migration, not just an API upgrade.
   - Data will need to be exported from 0.2.11 format and imported into Native V2 format.

3. **What testing is needed to verify compatibility?**
   - Create export script for 0.2.11 databases
   - Create import script for Native V2 format
   - Verify data integrity after migration
   - Test all Splice operations on migrated databases

### Risk Assessment

| Risk | Likelihood | Impact | Mitigation |
|------|-----------|--------|------------|
| Existing databases incompatible | **Certain** | High | Export/import migration path |
| Data corruption during migration | Low | High | Comprehensive testing |
| Performance regression | Low | Medium | Benchmarking |
| Missing feature in Native V2 | Low | Medium | API verified complete |

### Testing Approach for Plan 02-04

**Phase 1: Export (0.2.11)**
- Write script to dump all nodes and edges from existing database
- Export to JSON format (NodeId, NodeSpec, EdgeSpec)
- Verify export completeness

**Phase 2: Import (Native V2)**
- Write script to load JSON export into Native V2 database
- Verify all nodes and edges imported correctly
- Verify symbol cache rebuilt correctly

**Phase 3: Validation**
- Run existing tests on migrated database
- Verify all operations work identically
- Performance comparison

### Rollback Plan

If migration fails:
1. Keep original 0.2.11 database untouched
2. Native V2 migration creates new database file
3. Can revert to 0.2.11 code and original database
4. No destructive operations on original data

---

## 7. Migration Steps (for Plans 02-02, 02-03, 02-04)

### Plan 02-02: Update Dependencies
1. Update `Cargo.toml` to use sqlitegraph 1.0 with native-v2 feature
2. Run `cargo update`
3. Verify compilation succeeds (should work due to API compatibility)

### Plan 02-03: Migrate Code
1. Change line 34 in `src/graph/mod.rs`: `GraphConfig::sqlite()` → `GraphConfig::native()`
2. Run `cargo check` to verify
3. Run tests to ensure no regressions

### Plan 02-04: Database Migration
1. Create export tool for 0.2.11 databases
2. Create import tool for Native V2 databases
3. Test migration on sample databases
4. Document migration process for users

---

## 8. Unexpected Breaking Changes

**None discovered.**

The API is highly compatible:
- All types we use are unchanged
- Only configuration method changed
- Single-line code change required
- Feature flag addition is straightforward

**Total Risk Assessment:** Low technical risk, high data migration risk.

---

## 9. References

- SQLiteGraph v1.0 Manual: `/home/feanor/Projects/splice/docs/manual.md`
- SQLiteGraph API Reference: `/home/feanor/Projects/splice/docs/API.md`
- Current Implementation: `/home/feanor/Projects/splice/src/graph/mod.rs`
- Schema Definitions: `/home/feanor/Projects/splice/src/graph/schema.rs`
