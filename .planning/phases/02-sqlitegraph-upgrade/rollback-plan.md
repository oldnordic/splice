# Rollback Plan: SQLiteGraph 1.0 → 0.2.11

**Created:** 2026-01-17
**Purpose:** Emergency rollback if critical issues found in production

---

## When to Rollback

Consider rollback if:
- Critical data corruption bugs discovered
- Performance regression in production workloads
- Unexpected incompatibilities with existing databases
- Magellan integration breaks in production

---

## Rollback Steps

### 1. Revert Cargo.toml Dependencies

**File:** `/home/feanor/Projects/splice/Cargo.toml`

**Current (v1.0.0):**
```toml
[dependencies]
sqlitegraph = { version = "1.0.0", default-features = false, features = ["native-v2"] }
magellan = { version = "0.5.3", features = ["native-v2"] }
```

**Rollback to (v0.2.11):**
```toml
[dependencies]
sqlitegraph = { version = "0.2.11", default-features = false }
magellan = { version = "0.5.3" }
```

**Changes:**
- Remove `version = "1.0.0"` → `version = "0.2.11"`
- Remove `features = ["native-v2"]` from sqlitegraph
- Remove `features = ["native-v2"]` from magellan

---

### 2. Revert Code Changes

**File:** `/home/feanor/Projects/splice/src/graph/mod.rs`

**Line 34 - Current (Native V2):**
```rust
let cfg = sqlitegraph::GraphConfig::native();
```

**Rollback to (SQLite backend):**
```rust
let cfg = sqlitegraph::GraphConfig::sqlite();
```

---

### 3. Database Migration Considerations

#### If Native V2 Databases Already Created:

**Problem:** SQLite backend (0.2.11) cannot open Native V2 databases.

**Options:**

**Option A: Re-index Source Files (Recommended)**
```bash
# Delete old Native V2 database
rm ~/.splice_graph.db

# Re-index with rolled-back version
cargo build --release
splice index <project-directory>
```

**Option B: Manual Export/Import**
```bash
# Before rollback, export data from Native V2
# (requires building export utility)

# After rollback, import to SQLite backend
# (requires building import utility)
```

**Note:** Export/import utilities not yet implemented. Re-indexing is the only viable option.

---

### 4. Verify Rollback

```bash
# Clean build
cargo clean

# Check compilation
cargo check

# Run tests
cargo test --workspace

# Verify database creation
cargo run --example test_db

# Verify Magellan integration
cargo test magellan -- --nocapture
```

Expected results:
- ✅ All 111 tests pass
- ✅ Database creation succeeds (SQLite backend format)
- ✅ Magellan tests pass
- ✅ No errors or warnings

---

## Known Issues After Rollback

### 1. Performance Regression
- **Impact:** 10x slower graph traversals (per SQLiteGraph docs)
- **Workaround:** None (this is why we upgraded)
- **Mitigation:** Optimize queries, reduce traversal depth

### 2. Native V2 Features Lost
- **Impact:** No clustered adjacency storage
- **Workaround:** None (requires v1.0.0)
- **Mitigation:** Optimize database schema

### 3. Duplicate Dependencies
- **Impact:** Larger binary size (both v0.2.11 and v1.0.0 linked)
- **Workaround:** None (Magellan uses v0.2.11 internally)
- **Mitigation:** Accept larger binary

---

## Magellan Version Update Path

If Magellan updates to SQLiteGraph v1.0.0:

### Check Current Magellan Version
```bash
cargo tree -p magellan | grep sqlitegraph
```

### If Magellan >= 0.6.0 with v1.0.0 Support:
```toml
[dependencies]
magellan = { version = ">=0.6.0", features = ["native-v2"] }
sqlitegraph = { version = "1.0.0", default-features = false, features = ["native-v2"] }
```

### Benefits:
- Single sqlitegraph version in binary
- Smaller binary size
- Unified database format

### Status as of 2026-01-17:
- Magellan v0.5.3 uses sqlitegraph v0.2.11
- No update available yet
- Monitor: https://crates.io/crates/magellan

---

## Rollback Decision Tree

```
Production Issue?
├─ Yes: Check severity
│  ├─ Critical data corruption?
│  │  └─ Yes: IMMEDIATE ROLLBACK
│  │     1. Revert Cargo.toml
│  │     2. Revert src/graph/mod.rs
│  │     3. Re-index all databases
│  │
│  ├─ Performance regression?
│  │  └─ Yes: Evaluate impact
│  │     ├─ Acceptable: Document and monitor
│  │     └─ Unacceptable: ROLLBACK
│  │
│  └─ Compatibility issue?
│     └─ Yes: Check workaround
│        ├─ Workaround exists: Use workaround
│        └─ No workaround: ROLLBACK
│
└─ No: Continue monitoring
```

---

## Rollback Timeline Estimate

- Revert code changes: 5 minutes
- Clean build: 2 minutes
- Run tests: 3 minutes
- Re-index single database: 1-5 minutes (varies by project size)
- **Total**: ~15 minutes for small project, ~1 hour for large monorepo

---

## Post-Rollback Verification

### Checklist:
- [ ] Cargo.toml reverted to v0.2.11
- [ ] src/graph/mod.rs line 34 reverted to `GraphConfig::sqlite()`
- [ ] `cargo check` passes
- [ ] All 111 tests pass
- [ ] Database creation works
- [ ] Magellan integration works
- [ ] Production workflows verified

---

## Long-term Solution

If rollback is necessary:
1. Document the specific issue that caused rollback
2. File issue with sqlitegraph crate if applicable
3. Monitor for sqlitegraph v1.0.1+ bugfix releases
4. Re-test upgrade path with fixed version
5. Re-attempt upgrade when issues resolved

---

## References

- SQLiteGraph v1.0.0 Manual: `/home/feanor/Projects/splice/docs/manual.md`
- API Differences: `/home/feanor/Projects/splice/.planning/phases/02-sqlitegraph-upgrade/api-differences.md`
- Plan 02-01 Summary: Migration analysis and risk assessment

---

## Contact

For questions about rollback:
- Check Phase 2 documentation: `/home/feanor/Projects/splice/.planning/phases/02-sqlitegraph-upgrade/`
- Review git history: `git log --oneline --grep="02-0"`
- Run tests: `cargo test --workspace`
