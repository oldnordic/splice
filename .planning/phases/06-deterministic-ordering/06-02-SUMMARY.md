# Plan 06-02 Summary: Main Command Sorting

**Status:** ✅ Complete
**Commits:** `2152669`

## Objective
Add sorting to main command output (patch, delete, plan) for deterministic JSON.

## Context
With Ord implementations from Plan 06-01 in place, this plan adds actual sorting calls to the main command outputs.

## Changes Made

### 1. DeleteResult.spans sorting (src/main.rs:420-421)
```rust
// Sort spans deterministically by file_path, then byte_start
spans.sort();
```
- Uses SpanResult Ord from 06-01
- Ensures consistent output order for definition + references

### 2. PlanResult.files_affected sorting (src/main.rs:980-984)
```rust
files_affected: {
    let mut files = vec![plan_path.to_string_lossy().to_string()];
    files.sort();
    files
},
```
- Sorts file paths alphabetically
- Currently only has one file, but prepared for multi-file plans

## Before/After
```rust
// Before: Unsorted output
spans.push(SpanResult::from(resolved_def.clone()));
for r in &ref_set.references {
    spans.push(SpanResult::from_byte_span(...));
}
let delete_result = DeleteResult { spans, ... };  // Unsorted

// After: Deterministic order
spans.push(SpanResult::from(resolved_def.clone()));
for r in &ref_set.references {
    spans.push(SpanResult::from_byte_span(...));
}
spans.sort();  // <-- Now sorted
let delete_result = DeleteResult { spans, ... };
```

## Verification
- ✅ cargo check passes
- ✅ All 118 unit tests pass
- ✅ Delete JSON output spans sorted by file_path then byte_start
- ✅ Plan JSON output files_affected sorted

## Notes
- Patch command already has single-span output (vec![span]), no additional sorting needed
- ReferenceSet.references still sorted descending for deletion order (internal, not exposed in JSON)
