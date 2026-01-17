# Plan 02-01 Summary: Study API differences and migration path

**Status:** ✅ COMPLETE
**Date:** 2026-01-17
**Tasks Completed:** 5/5

---

## Objective

Study SQLiteGraph API differences between 0.2.11 and 1.0, document migration path, and verify Native V2 backend availability.

---

## Key Findings

### 1. Version Availability

**SQLiteGraph 1.0.0 is published and ready for use.**

```bash
$ cargo search sqlitegraph
sqlitegraph = "1.0.0"  # Deterministic, embedded graph database with SQLite and Native V2 backends
```

All required types are exported:
- `GraphConfig::native()` - Native V2 configuration
- `open_graph()` - Graph opening function
- `NodeSpec`, `EdgeSpec`, `NodeId` - Core graph types
- `GraphBackend` - Backend trait
- `Label`, `PropertyKey` - Schema types
- `SqliteGraphError` - Error type

### 2. Type Compatibility

**Excellent news:** All types in use are compatible with v1.0.

| Type | Compatibility | Notes |
|------|---------------|-------|
| `GraphBackend` trait | ✅ Compatible | Same trait |
| `NodeId` | ✅ Compatible | Same type |
| `NodeSpec` | ✅ Compatible | Same fields |
| `EdgeSpec` | ✅ Compatible | Same fields |
| `Label` | ✅ Compatible | Same type |
| `PropertyKey` | ✅ Compatible | Same type |
| `SqliteGraphError` | ✅ Compatible | Same error type |

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

### 3. Code Changes Required

**Only ONE line needs to change in the entire codebase:**

**File:** `src/graph/mod.rs`
**Line:** 34
**Change:**
```rust
// Before
let cfg = sqlitegraph::GraphConfig::sqlite();

// After
let cfg = sqlitegraph::GraphConfig::native();
```

### 4. Feature Flag Requirement

**Native V2 backend requires feature flag:**

```toml
[dependencies]
# Before (0.2.11)
sqlitegraph = { version = "0.2.11", default-features = false }

# After (1.0 with Native V2)
sqlitegraph = { version = "1.0", default-features = false, features = ["native-v2"] }
```

### 5. Database Compatibility Considerations

**Critical:** Backend migration required, not just API upgrade.

**Key Findings:**
- ❌ Existing 0.2.11 databases **cannot** be opened with Native V2 backend
- ✅ Different storage format (SQLite → clustered adjacency)
- ⚠️ **Export/import migration path required**

**Risk Assessment:**
| Risk | Likelihood | Impact | Mitigation |
|------|-----------|--------|------------|
| Existing databases incompatible | **Certain** | High | Export/import migration |
| Data corruption during migration | Low | High | Comprehensive testing |
| Performance regression | Low | Medium | Benchmarking |
| Missing feature | Low | Medium | API verified complete |

**Testing Approach:**
1. Export all nodes/edges from 0.2.11 database to JSON
2. Import JSON into Native V2 database
3. Verify data integrity
4. Test all operations on migrated database

**Rollback Plan:**
- Original database untouched
- Migration creates new file
- Can revert to 0.2.11 if needed

---

## Deliverables

✅ **api-differences.md** created at `.planning/phases/02-sqlitegraph-upgrade/api-differences.md`

Contains:
1. Version availability verification
2. Type compatibility matrix (all 10 types documented)
3. Feature flag requirement documentation
4. File-by-file change list (single line identified)
5. Database compatibility risk assessment
6. Migration steps for Plans 02-02, 02-03, 02-04

---

## Next Steps

### Plan 02-02: Update Cargo.toml dependencies
- Change sqlitegraph version from "0.2.11" to "1.0"
- Add "native-v2" feature flag
- Run `cargo update` and verify compilation

### Plan 02-03: Migrate code to new API
- Change `GraphConfig::sqlite()` to `GraphConfig::native()` in src/graph/mod.rs:34
- Run tests to verify no regressions

### Plan 02-04: Verify database compatibility
- Create export tool for 0.2.11 databases
- Create import tool for Native V2 databases
- Test migration on sample databases
- Document migration process

---

## Technical Risk Assessment

**Overall Risk:** Low technical risk, high data migration risk

**Technical Migration:** ✅ Very Low Risk
- Only 1 line of code to change
- All types compatible
- Feature flag straightforward

**Data Migration:** ⚠️ High Risk
- Backend format change
- Export/import required
- Extensive testing needed
- User-facing migration process

---

## Commits

- `b9e296d`: docs(02-01): verify v1.0 availability and document type compatibility

---

## Files Modified

1. `.planning/phases/02-sqlitegraph-upgrade/api-differences.md` (created)
2. `.planning/phases/02-sqlitegraph-upgrade/02-01-SUMMARY.md` (created)
3. `.planning/STATE.md` (to be updated)

---

## Time Estimate

**Actual:** ~1 hour (research and documentation)
**Estimated for Plans 02-02, 02-03, 02-04:** 2-3 hours
- 02-02: ~30 minutes (dependency update, compilation check)
- 02-03: ~30 minutes (single-line change, testing)
- 02-04: ~1-2 hours (migration tools, testing)

---

## References

- SQLiteGraph v1.0 Manual: `/home/feanor/Projects/splice/docs/manual.md`
- SQLiteGraph API Reference: `/home/feanor/Projects/splice/docs/API.md`
- API Differences Document: `/home/feanor/Projects/splice/.planning/phases/02-sqlitegraph-upgrade/api-differences.md`
