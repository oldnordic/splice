---
phase: 15-enhanced-errors
plan: 02
subsystem: error-handling
tags: [error-location, line-column, tree-sitter, cli-output]

# Dependency graph
requires:
  - phase: 15-enhanced-errors
    plan: 01
    provides: SpliceErrorCode enum, ErrorCode struct with location field
provides:
  - SpliceError::location() method for extracting file/line/column from error variants
  - byte_offset_to_line_column() helper for converting tree-sitter byte offsets to line/column
  - CliErrorPayload integration with error location extraction
affects: [15-03, 15-04, 15-05, 15-06]

# Tech tracking
tech-stack:
  added: []
  patterns: [error location extraction, 1-based line numbers, 0-based column numbers]

key-files:
  created: []
  modified: [src/error.rs, src/cli/mod.rs]

key-decisions:
  - "Location extraction returns (Option<&str>, Option<usize>, Option<usize>) for file/line/column"
  - "Line numbers are 1-based (compiler convention), column numbers are 0-based (compiler convention)"
  - "byte_offset_to_line_column handles newline boundaries correctly (offset after \\n is column 0)"

patterns-established:
  - "Error location pattern: Single location() method on error enum provides consistent API"
  - "Byte offset conversion: Count newlines + 1 for line, check ends_with('\\n') for column reset"
  - "Optional location: Some errors have file only, others have file+line+column, others have none"

# Metrics
duration: 5min
completed: 2026-01-22
---

# Phase 15: Enhanced Errors - Plan 02 Summary

**SpliceError location extraction with line/column support for CLI-16 precision error reporting**

## Performance

- **Duration:** 5 minutes
- **Started:** 2026-01-22T15:15:14Z
- **Completed:** 2026-01-22T15:20:21Z
- **Tasks:** 3
- **Files modified:** 2

## Accomplishments

- Added `SpliceError::location()` method that extracts (file, line, column) from all error variants
- Updated `CliErrorPayload::from_error()` to use location extraction instead of TODO placeholder
- Created `byte_offset_to_line_column()` helper for future tree-sitter Parse error enhancement
- Established compiler conventions: 1-based line numbers, 0-based column numbers

## Task Commits

Each task was committed atomically:

1. **Task 1: Add location() helper to SpliceError enum** - `57e2f60` (feat)
2. **Task 2: Use SpliceError::location() in CliErrorPayload** - `0ce36c4` (feat)
3. **Task 3: Add byte_offset_to_line_column helper for tree-sitter** - `579415a` (feat)

**Plan metadata:** (to be committed)

## Files Created/Modified

- `src/error.rs` - Added `location()` method (lines 527-560), added `byte_offset_to_line_column()` helper (lines 740-766), added tests (lines 768-802)
- `src/cli/mod.rs` - Replaced TODO comment at line 578 with actual location extraction using `error.location()` (lines 572-583)

## Deviations from Plan

None - plan executed exactly as written.

## Issues Encountered

**Test expectations correction during Task 3:**
- Initial test expected offset 10 to be at line 2, column 4
- Analysis revealed byte offsets represent positions BETWEEN characters, not AT characters
- Offset 10 is between `{` and `\n`, so it's still on line 1
- Corrected test expectations to match byte offset semantics
- Final test validates multi-line behavior including newline boundary handling

## User Setup Required

None - no external service configuration required.

## Next Phase Readiness

- Error location infrastructure complete for CLI-16 requirement
- `byte_offset_to_line_column()` helper available for Parse error enhancement (15-03 or later)
- All error variants now support location extraction (file only, or file+line+column, or none)
- TODO comment removed from cli/mod.rs line 578

**Remaining work for full location coverage:**
- Parse errors need byte offset tracking to use `byte_offset_to_line_column()`
- Validation errors could benefit from node-level location extraction
- Consider adding line/column to InvalidSpan and InvalidLineRange variants

---
*Phase: 15-enhanced-errors*
*Completed: 2026-01-22*
