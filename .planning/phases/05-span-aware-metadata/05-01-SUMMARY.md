# Plan 05-01 Summary: Graph Storage Line/Col Infrastructure

**Status:** ✅ Complete
**Commits:** `16d3766`

## Objective
Update graph storage layer to accept and store line/column metadata for symbols.

## Changes Made

### 1. Updated `store_symbol_with_file_and_language` signature
- Added 4 new parameters: `line_start`, `line_end`, `col_start`, `col_end` (all `usize`)
- Store line/col in node data JSON alongside byte spans

### 2. Updated backward-compatible wrapper
- Deprecated `store_symbol_with_file` now passes 0 placeholders for line/col
- Maintains backward compatibility for existing code

### 3. Added unit tests (src/graph/mod.rs)
- `test_store_symbol_with_line_col`: Verifies line/col are stored and retrievable
- `test_backward_compatibility_zero_values`: Verifies 0 placeholders work
- `test_deprecated_method_passes_zeros`: Verifies deprecated method behavior

### 4. Fixed call sites
- Updated src/plan/mod.rs to pass 0 placeholders
- Updated src/main.rs to pass 0 placeholders
- Updated all test files to pass 0 placeholders
- Updated examples/test_db.rs

## Verification
- ✅ cargo check passes
- ✅ cargo test graph::tests passes (3 new tests)
- ✅ All 115 existing tests still pass

## Notes
- Line/col are now stored in graph but still using 0 placeholders in call sites
- Plan 05-02 will update call sites to pass actual computed values
