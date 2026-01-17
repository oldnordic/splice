# Plan 06-03 Summary: Query and Batch Sorting

**Status:** ✅ Complete
**Commits:** `be704c4`

## Objective
Add sorting to query, batch, apply_files commands and reference finding for deterministic output.

## Context
Remaining commands (query, batch/apply_files) need sorting for complete deterministic output coverage.

## Changes Made

### 1. Query symbols sorting (src/main.rs:1220-1226)
```rust
let mut results = integration.query_by_labels(&labels_ref)?;
// Sort results deterministically by file_path, then byte_start
results.sort_by(|a, b| {
    a.file_path
        .cmp(&b.file_path)
        .then_with(|| a.byte_start.cmp(&b.byte_start))
});
```
- Uses sort_by to avoid borrow checker issues with references
- Sorts by file path first, then byte offset within file

### 2. ApplyFilesResult.files sorting (src/main.rs:845-851)
```rust
// Sort file_results deterministically by file path
file_results.sort();

// Sort spans within each file deterministically
for result in &mut file_results {
    result.spans.sort();
}
```
- Uses FilePatternResult Ord from 06-01 for file-level sorting
- Uses SpanResult Ord from 06-01 for span-level sorting
- Two-level sort: files first, then spans within each file

## Before/After
```rust
// Before: Query results in database order (non-deterministic)
let results = integration.query_by_labels(&labels_ref)?;
for result in &results { ... }  // Random order

// After: Sorted deterministically
let mut results = integration.query_by_labels(&labels_ref)?;
results.sort_by(|a, b| { ... });  // <-- Sorted
for result in &results { ... }
```

## Verification
- ✅ cargo check passes
- ✅ All 118 unit tests pass
- ✅ Query output sorted by file_path then byte_start
- ✅ Apply files output sorted by file path
- ✅ Spans within each file sorted by byte_start

## Notes
- ReferenceSet.references sorting unchanged (still descending for deletion safety)
- JSON output now fully deterministic across all commands
- query/batch/apply_files commands now match delete/plan consistency
