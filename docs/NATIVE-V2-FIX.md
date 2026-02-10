# Native-V2 Backend Persistence Fix

**Date:** 2026-02-10
**Status:** FIXED

## Problem

Native-v2 backend was correctly persisting data to disk, but splice's query methods only read from in-memory cache, not the database. This caused symbols to be "lost" on reopen.

## Root Cause

Splice's `all_symbol_names()` and `find_symbols_by_name()` methods only searched the in-memory `symbol_cache`, never the database. When reopening a database:
- `symbol_cache` starts empty
- Methods return empty results even though data is on disk

## Solution

Updated both methods to query the database directly:

### Before (BROKEN)
```rust
pub fn all_symbol_names(&self) -> Vec<String> {
    for key in self.symbol_cache.keys() {  // Cache only!
        names.insert(key.clone());
    }
}
```

### After (FIXED)
```rust
pub fn all_symbol_names(&self) -> Vec<String> {
    // Collect from cache (for recent writes)
    for key in self.symbol_cache.keys() {
        names.insert(key.clone());
    }

    // Also collect from database (for persisted symbols)
    if let Ok(all_ids) = self.backend.entity_ids() {
        for node_id in all_ids {
            if let Ok(node) = self.backend.get_node(snapshot, node_id) {
                if node.kind != "File" && node.kind != "file" {
                    names.insert(node.name);
                }
            }
        }
    }
}
```

## Changes Made

1. **src/graph/mod.rs**:
   - `all_symbol_names()`: Added database query, filter out File nodes
   - `find_symbols_by_name()`: Added database query, filter out File nodes
   - Added `backend.flush()` call after writes (for future safety)

## Testing

All tests pass with native-v2 backend:
- `test_native_v2_write_and_read` ✅
- `test_native_v2_multiple_symbols` ✅
- `test_native_v2_database_file_exists_after_close` ✅
- `backend_specific_tests` ✅ (5 passed, 1 ignored)

## Notes

- The native-v2 backend was **always correctly persisting data**
- The bug was purely in splice's **query methods**, not the storage
- File nodes are filtered out since they're not actual code symbols
