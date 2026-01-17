# Plan 06-01 Summary: Ord Implementations for Output Types

**Status:** ✅ Complete
**Commits:** `66f10d1`

## Objective
Add deterministic sorting utilities to output types for consistent JSON output across all operations.

## Context
All Vec<> fields in output types need deterministic sorting for reproducible JSON output. This plan adds Ord trait implementations to the core output types.

## Changes Made

### 1. StepResult - Derive Ord (src/output.rs:170)
- Added `#[derive(PartialEq, Eq, PartialOrd, Ord)]`
- All fields (step, status, message, file, symbol) are simple types that implement Ord
- Sorting: by step number, then status, then message, then file, then symbol

### 2. SpanResult - Manual Ord (src/output.rs:316-346)
- Manual implementation to ignore span_id (random UUID), match_id, and hash fields
- Sorting: by file_path, then byte_start, then byte_end
- Custom PartialEq that compares only file_path and byte ranges

### 3. FilePatternResult - Manual Ord (src/output.rs:348-368)
- Manual implementation to ignore spans Vec field (Vec doesn't implement Ord)
- Sorting: by file path only
- Custom PartialEq that compares only file field

### 4. DiagnosticPayload - Manual Ord (src/output.rs:426-471)
- Manual implementation for Option<> fields (line, column, file, etc.)
- Sorting: by tool, then file, then line, then column, then level, then message
- None < Some ordering for Option fields groups diagnostics without location first

## Before/After
```rust
// Before: No Ord, can't use sort()
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpanResult { ... }

// After: Can use .sort() on Vec<SpanResult>
impl Ord for SpanResult {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        match self.file_path.cmp(&other.file_path) {
            std::cmp::Ordering::Equal => {}
            ord => return ord,
        }
        match self.byte_start.cmp(&other.byte_start) {
            std::cmp::Ordering::Equal => {}
            ord => return ord,
        }
        self.byte_end.cmp(&other.byte_end)
    }
}
```

## Verification
- ✅ cargo check passes
- ✅ All 118 unit tests pass
- ✅ All 16 relevant CLI tests pass (1 pre-existing failure unrelated)
- ✅ All output types with Vec<> fields now have Ord implemented

## Notes
- Custom PartialEq implementations ignore volatile fields (UUIDs, hashes) for deterministic comparison
- Manual Ord impls handle Option<> fields correctly with None < Some ordering
- Foundation for Plans 06-02 and 06-03 which use these Ord impls for sorting
