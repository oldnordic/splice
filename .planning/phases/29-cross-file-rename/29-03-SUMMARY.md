---
phase: 29-cross-file-rename
plan: 29-03
subsystem: refactoring
tags: [rename, magellan, byte-accurate, utf-8, referencefact]

# Dependency graph
requires:
  - phase: 29-cross-file-rename
    plan: 29-02
    provides: ReferenceFact-based span extraction and validation
provides:
  - Byte-accurate replacement at exact byte offsets
  - UTF-8 safe multi-replacement with offset recalculation
  - Reference grouping by file for parallel processing
affects: [29-04, 29-05]

# Tech tracking
tech-stack:
  added: []
  patterns:
    - Descending byte offset replacement order
    - UTF-8 boundary validation before byte manipulation
    - Group-then-apply pattern for cross-file operations

key-files:
  created:
    - src/graph/rename/mod.rs
  modified:
    - src/graph/mod.rs
    - src/lib.rs
    - src/main.rs

key-decisions:
  - "Apply replacements end-to-start (descending byte_start) to preserve offset validity"
  - "Use validate_utf8_span from MagellanIntegration for consistency"
  - "Group references by file before applying for potential parallel processing"
  - "Preview mode returns early without file modifications or backup creation"

patterns-established:
  - "Replacement order: Descending byte_start prevents offset shift issues"
  - "UTF-8 safety: Validate boundaries before any byte manipulation"
  - "Preview purity: Short-circuit before any I/O operations"

# Metrics
duration: 12min
completed: 2026-02-04
---

# Phase 29: Plan 03 - Byte-Accurate Replacement with UTF-8 Safety Summary

**Byte-accurate symbol renaming at exact ReferenceFact spans with UTF-8 boundary validation and descending offset replacement**

## Performance

- **Duration:** 12 minutes
- **Started:** 2026-02-04T11:52:16Z
- **Completed:** 2026-02-04T12:04:00Z
- **Tasks:** 4
- **Files modified:** 4
- **Test coverage:** 14 tests added

## Accomplishments

- Implemented `replace_at_span()` for single-byte-span replacement with UTF-8 validation
- Implemented `apply_replacements_in_file()` for multi-replacement with automatic offset handling
- Implemented `group_references_by_file()` to organize references by file path with descending sort
- Implemented `simulate_replacements()` for preview mode without file writes
- Updated `execute_rename()` to use real replacement logic instead of stub
- Added comprehensive unit tests covering UTF-8 safety, variable-length names, and edge cases

## Task Commits

Each task was committed atomically:

1. **All tasks combined** - `4a2c494` (feat)

**Plan metadata:** N/A (single atomic commit)

## Files Created/Modified

### Created
- `src/graph/rename/mod.rs` (434 lines) - Complete rename implementation with 14 unit tests

### Modified
- `src/graph/mod.rs` - Added `pub mod rename;`
- `src/lib.rs` - Re-exported rename functions for public API
- `src/main.rs` - Updated execute_rename() to use real rename operations

## Decisions Made

1. **Descending order replacement:** References are sorted by byte_start descending within each file. This ensures that replacing later (higher offset) references doesn't affect the byte offsets of earlier (lower offset) ones.

2. **UTF-8 validation reuse:** Used existing `MagellanIntegration::validate_utf8_span()` for consistency across the codebase rather than reimplementing validation.

3. **Preview purity:** Preview mode short-circuits before any backup logic or file I/O, making it truly read-only. This matches the context decision from plan 29-02.

4. **Module location:** Placed rename module under `src/graph/rename/` as a sibling to `magellan_integration` since it operates on graph data (ReferenceFact), not as a top-level module.

## Deviations from Plan

None - plan executed exactly as written.

## Issues Encountered

1. **Test byte offset miscalculation:** Initial unit tests had incorrect byte offsets for the test content. Fixed by calculating actual positions of "foo" and "old_name" substrings in the test strings.

2. **Borrow checker issue with backup_path:** Initial implementation had a borrow-of-moved-value error where `backup_path` was moved in the if-let pattern but used later in JSON. Fixed by using `ref` in the pattern to borrow instead of move.

3. **Unused variable warning:** The `simulation` variable in preview mode was unused. Removed it since the grouped reference data already provides the needed counts.

## User Setup Required

None - no external service configuration required.

## Next Phase Readiness

- Byte-accurate replacement implementation complete and tested
- Ready for plan 29-04 (Backup Creation and Restore)
- Preview simulation function available for enhanced preview UI in future plans
- All 14 unit tests passing, covering UTF-8 safety and edge cases

---
*Phase: 29-cross-file-rename*
*Completed: 2026-02-04*
