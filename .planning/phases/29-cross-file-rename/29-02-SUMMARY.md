---
phase: 29-cross-file-rename
plan: 02
subsystem: refactoring
tags: [reference-fact, byte-spans, utf-8-validation, rename, magellan]

# Dependency graph
requires:
  - phase: 29-01
    provides: rename command structure and get_all_references method
provides:
  - Reference sorting utility for safe in-order replacement
  - UTF-8 span validation for byte-accurate operations
  - Ambiguous symbol error with complete file list
  - execute_rename with sorted reference collection
affects: [29-03, 29-04]

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "Descending byte offset sort within files prevents offset shift during replacement"
    - "UTF-8 boundary validation before byte manipulation"

key-files:
  created: []
  modified:
    - src/graph/magellan_integration.rs
    - src/main.rs

key-decisions:
  - "Sort by (file_path ascending, byte_start descending) for safe replacement"
  - "Use ambiguous=true for symbol lookup to provide complete error context"
  - "Return AmbiguousSymbol error with file:kind format for disambiguation"

patterns-established:
  - "Reference sorting before any byte manipulation"
  - "UTF-8 boundary validation before span operations"
  - "Ambiguous symbol lookup returns full context (all files with kinds)"

# Metrics
duration: 4min
completed: 2026-02-04
---

# Phase 29 Plan 02: ReferenceFact-Based Span Extraction Summary

**Reference sorting utility with descending byte offset order and UTF-8 span validation for safe byte-accurate replacement**

## Performance

- **Duration:** 4 min
- **Started:** 2026-02-04T11:45:51Z
- **Completed:** 2026-02-04T11:49:55Z
- **Tasks:** 4 (get_all_references already existed from 29-01)
- **Files modified:** 2

## Accomplishments

- **Reference sorting utility** - `sort_references_for_replacement()` sorts by (file_path, byte_start descending) to prevent offset shifts during multi-file rename
- **UTF-8 span validation** - `validate_utf8_span()` checks byte boundaries and returns appropriate errors for invalid spans
- **Ambiguous symbol handling** - `execute_rename` now returns `AmbiguousSymbol` error with complete file:kind list when multiple symbols match
- **Unit tests** - Comprehensive tests for sorting (multi-file, descending offsets) and UTF-8 validation (valid, out-of-bounds, multibyte boundaries, invalid UTF-8)

## Task Commits

Each task was committed atomically:

1. **Task 2: Add reference sorting and UTF-8 span validation** - `d19c5fe` (feat)
2. **Task 3-4: Complete execute_rename and add unit tests** - `c4ad1eb` (feat)

**Plan metadata:** (to be committed)

_Note: Task 1 (get_all_references) was already completed in plan 29-01_

## Files Created/Modified

- `src/graph/magellan_integration.rs` - Added `sort_references_for_replacement()`, `validate_utf8_span()`, and 5 unit tests
- `src/main.rs` - Updated `execute_rename()` with ambiguous symbol handling and reference sorting

## Decisions Made

1. **Sort by byte_start descending within files** - Descending order ensures that replacing earlier references doesn't affect byte offsets of later ones (critical when new name has different length)
2. **Use ambiguous=true for symbol lookup** - Returns all matches to provide complete error context with file:kind disambiguation
3. **UTF-8 validation uses str::is_char_boundary** - Validates both start and end positions are on valid UTF-8 character boundaries to prevent corrupting multibyte characters

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Bug] Fixed is_char_boundary on [u8] instead of str**
- **Found during:** Task 2 (validate_utf8_span implementation)
- **Issue:** `is_char_boundary()` method doesn't exist on `&[u8]`, only on `str`
- **Fix:** Convert bytes to str using `std::str::from_utf8()` before boundary checking
- **Files modified:** src/graph/magellan_integration.rs
- **Verification:** UTF-8 validation tests pass (valid, out-of-bounds, multibyte, invalid UTF-8 cases)
- **Committed in:** d19c5fe (Task 2 commit)

---

**Total deviations:** 1 auto-fixed (1 bug)
**Impact on plan:** Bug fix necessary for UTF-8 validation to compile and function correctly. No scope creep.

## Issues Encountered

- `is_char_boundary` initially used on `&[u8]` instead of `str` - fixed by converting to UTF-8 string first
- Initial compile error resolved by using proper type method

## User Setup Required

None - no external service configuration required.

## Next Phase Readiness

- Reference sorting and UTF-8 validation utilities ready for byte-accurate replacement implementation
- execute_rename collects and sorts references with proper error handling
- Plan 29-03 will implement the actual byte replacement logic using these sorted references
- Plan 29-04 will add backup and rollback capabilities

---
*Phase: 29-cross-file-rename*
*Completed: 2026-02-04*
