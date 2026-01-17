# Plan 05-03 Summary: Retrieve Line/Col in Resolution

**Status:** ✅ Complete
**Commits:** `691d57b`

## Objective
Update symbol resolution to retrieve line/column from graph and use it in output.

## Context
With Plans 05-01 and 05-02 complete, line/col are now stored in the graph. This plan updates the resolution functions to retrieve and return those values instead of 0 placeholders.

## Changes Made

### 1. Updated `resolve_symbol()` function (src/resolve/mod.rs:158-193)
- Replaced placeholder line/col retrieval with graph node data lookup
- Retrieves line_start, line_end, col_start, col_end from node.data
- Uses unwrap_or(0) for backward compatibility with old data
- Removed TODO comment about line/col storage

### 2. Updated `resolve_symbol_in_file()` function (src/resolve/mod.rs:261-296)
- Same pattern as resolve_symbol
- Retrieves line/col from graph node data
- Removed TODO comment

## Before/After
```rust
// Before:
line_start: 0, line_end: 0, col_start: 0, col_end: 0,
// TODO: Return actual line/col when we store it

// After:
let line_start = node.data.get("line_start").and_then(|v| v.as_u64()).unwrap_or(0) as usize;
// ... (same for line_end, col_start, col_end)
line_start, line_end, col_start, col_end,
```

## Verification
- ✅ cargo check passes
- ✅ All 118 unit tests pass
- ✅ ResolvedSpan now returns actual line/col from graph

## Notes
- This completes the end-to-end line/col pipeline
- ResolvedSpan → SpanResult conversion already propagates line/col
- Output now includes accurate location information for all symbols
