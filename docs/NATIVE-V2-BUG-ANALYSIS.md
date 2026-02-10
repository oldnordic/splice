# Native-V2 Backend Persistence Bug Analysis

**Date:** 2026-02-10
**Severity:** CRITICAL - Data loss
**Status:** Documented, fix pending

## Problem

The native-v2 backend does not persist data to disk. Data written to the database cannot be read back after reopening.

## Root Cause

**Splice does NOT call `backend.flush()` after write operations.**

Native-v2 backend uses a **Write-Ahead Log (WAL)** for transaction management. Data written via `insert_node()` and `insert_edge()` stays in the WAL in memory until `flush()` is called.

### Code Comparison

**Splice (BROKEN):**
```rust
// src/graph/mod.rs:209-270
pub fn store_symbol_with_file_and_language(...) -> Result<NodeId> {
    // ...
    self.backend.insert_node(node_spec)?;   // Line 250
    self.backend.insert_edge(edge_spec)?;   // Line 260
    // NO flush() - data never reaches disk!
    Ok(symbol_id)
}
```

**Magellan (WORKS):**
```rust
// magellan/src/kv/mod.rs:99-148
pub fn populate_symbol_index(...) -> Result<()> {
    // ...
    backend.kv_set(fqn_key, KvValue::Integer(*node_id), None)?;
    backend.kv_set(fqn_of_key, KvValue::String(fqn.clone()), None)?;
    backend.kv_set(rev_key, KvValue::Integer(1), None)?;
    backend.kv_set(file_key, KvValue::Bytes(encoded_ids), None)?;
    backend.flush()?;  // Line 145 - CRITICAL!
    Ok(())
}
```

## Evidence

Test `test_native_v2_write_and_read` shows:
```
Stored 5 symbols
Retrieved 0 symbols: []
assertion failed: Should have 5 symbols
  left: 0
 right: 5
```

Data is "written" but returns 0 on reopen because it was never flushed to disk.

## Required Fix

Add `backend.flush()` call after write operations in splice:

```rust
// After inserts in store_symbol_with_file_and_language
self.backend.insert_node(node_spec)?;
self.backend.insert_edge(edge_spec)?;
self.backend.flush()?;  // ADD THIS LINE
```

Also need to add flush in other write operations:
- `store_symbol()`
- Any batch write operations
- Snapshot operations

## Feature Gate

The flush call should be conditional on backend type:
```rust
#[cfg(feature = "native-v2")]
self.backend.flush()?;
```

Or use runtime detection if sqlitegraph backend provides `is_native_v2()` method.

## References

- Magellan KV module: `/home/feanor/Projects/magellan/src/kv/mod.rs`
- Splice graph module: `/home/feanor/Projects/splice/src/graph/mod.rs`
- Test failure: `/home/feanor/Projects/splice/tests/test_native_v2_write_read.rs`

## Action Items

1. [ ] Add `backend.flush()` call to `store_symbol_with_file_and_language`
2. [ ] Add `backend.flush()` call to `store_symbol`
3. [ ] Add `backend.flush()` call to batch operations
4. [ ] Add flush to snapshot restore operations
5. [ ] Re-run persistence tests to verify fix
6. [ ] Update CHANGELOG to note bugfix
