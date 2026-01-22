---
phase: 11-rich-span-core
plan: 02
subsystem: context-extraction
tags: [ropey, utf-8, line-context, span-coordinates]

# Dependency graph
requires:
  - phase: 11-01
    provides: SpanContext type and rich span infrastructure
provides:
  - Context extraction module using ropey for UTF-8 aware line calculations
  - extract_context function that extracts before/selected/after line arrays
  - Tested edge cases: multi-byte UTF-8, empty files, start/end boundaries
affects: [11-03, 11-04, llm-integration, patch-operations]

# Tech tracking
tech-stack:
  added: [ropey (already in dependencies)]
  patterns: [utf-8-byte-offset-to-line-conversion, context-line-extraction, exclusive-end-byte-ranges]

key-files:
  created: [src/context.rs]
  modified: [src/lib.rs]

key-decisions:
  - "Use byte_end.saturating_sub(1) for byte_to_line conversion since tree-sitter byte ranges are exclusive"
  - "Filter empty trailing lines from ropey's len_lines() behavior"
  - "Return early for empty files to avoid ropey edge cases"

patterns-established:
  - "Context extraction: 3-line default context with configurable boundaries"
  - "UTF-8 safety: Always use ropey for line/col calculations, never manual byte counting"
  - "Span semantics: byte_end is exclusive (first byte AFTER the span)"

# Metrics
duration: 8min
completed: 2026-01-22
---

# Phase 11: Rich Span Core - Plan 02 Summary

**UTF-8 aware context extraction using ropey with before/selected/after line arrays for LLM code context**

## Performance

- **Duration:** 8 min
- **Started:** 2026-01-22T09:06:05Z
- **Completed:** 2026-01-22T09:14:09Z
- **Tasks:** 3
- **Files modified:** 2

## Accomplishments

- Created `src/context.rs` module with `extract_context` function using ropey for efficient line operations
- Integrated context module into lib.rs with re-exports of `extract_context` and `SpanContext`
- Implemented comprehensive test suite covering 8 edge cases including multi-byte UTF-8, empty files, and boundary conditions
- Fixed byte offset semantics to handle tree-sitter's exclusive end byte ranges correctly

## Task Commits

Each task was committed atomically:

1. **Task 1: Create context.rs module with extract_context function** - `4a9cdc7` (feat)
2. **Task 2: Export context module from lib.rs** - `4644be7` (feat)
3. **Task 3: Fix context extraction edge cases and run tests** - `a511f04` (fix)

## Files Created/Modified

- `src/context.rs` - Context extraction using ropey for UTF-8 aware line/col calculations
- `src/lib.rs` - Added `pub mod context;` and re-exported `extract_context` and `SpanContext`

## Decisions Made

- **Byte offset semantics**: Used `byte_end.saturating_sub(1)` when calling `rope.byte_to_line()` because tree-sitter's `end_byte()` returns the first byte AFTER the node (exclusive end)
- **Empty file handling**: Return early with empty context for files of size 0 to avoid ropey edge cases where `len_lines()` returns 1 even for empty files
- **Trailing line filtering**: Filter empty strings from `after` lines to handle ropey's behavior where `len_lines()` includes an empty line after the final newline
- **Test byte offsets**: Fixed tests to use exact byte ranges instead of approximate values to ensure deterministic behavior

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Bug] Fixed byte_to_line calculation for exclusive end byte ranges**
- **Found during:** Task 3 (Running context tests)
- **Issue:** Initial implementation used `byte_end` directly, but tree-sitter's byte ranges are exclusive (end points to first byte AFTER the span). This caused `byte_to_line(byte_end)` to return the line after the span, resulting in one extra line being extracted
- **Fix:** Changed to `rope.byte_to_line(byte_end.saturating_sub(1))` to get the line containing the last byte of the span
- **Files modified:** src/context.rs
- **Verification:** All 8 tests now pass with correct line counts
- **Committed in:** a511f04 (Task 3 commit)

**2. [Rule 1 - Bug] Filtered empty trailing lines from ropey's len_lines() behavior**
- **Found during:** Task 3 (Running context tests)
- **Issue:** ropey's `len_lines()` returns the number of line breaks + 1, so a file with "line 1\nline 2\n" has `len_lines() = 3` (lines 0, 1, and an empty line 2 after the last newline). This caused empty lines to appear in `after` context
- **Fix:** Added `.filter(|line| !line.is_empty())` to the `after` extraction to filter out empty trailing lines
- **Files modified:** src/context.rs
- **Verification:** `test_extract_context_end_of_file` now passes with `after.len() = 0`
- **Committed in:** a511f04 (Task 3 commit)

**3. [Rule 1 - Bug] Fixed empty file handling**
- **Found during:** Task 3 (Running context tests)
- **Issue:** For empty files (size 0), ropey still reports `len_lines() = 1` with an empty line at index 0. This caused `selected.len() = 1` instead of 0
- **Fix:** Added early return for `file_size == 0` before creating the rope, returning empty context
- **Files modified:** src/context.rs
- **Verification:** `test_extract_context_empty_file` now passes with all empty arrays
- **Committed in:** a511f04 (Task 3 commit)

**4. [Rule 1 - Bug] Fixed test byte offsets to use exact ranges**
- **Found during:** Task 3 (Running context tests)
- **Issue:** Original plan used approximate byte offsets (e.g., "bytes 8-24 approx") which didn't match actual file layout, causing tests to fail
- **Fix:** Recalculated all test byte offsets to exact values based on "line N\n" = 7 bytes per line
- **Files modified:** src/context.rs (test functions)
- **Verification:** All 8 tests pass with deterministic byte ranges
- **Committed in:** a511f04 (Task 3 commit)

---

**Total deviations:** 4 auto-fixed (all Rule 1 - bugs)
**Impact on plan:** All fixes were necessary for correct operation. The core functionality worked as designed, but edge cases and byte offset semantics needed refinement for production correctness.

## Issues Encountered

**ropey byte_to_line semantics with exclusive end ranges**: Initial implementation used `byte_end` directly, but needed to use `byte_end.saturating_sub(1)` because tree-sitter's byte ranges are exclusive. Fixed by understanding the semantics and adjusting the calculation.

**ropey len_lines() includes trailing empty line**: ropey reports one more line than visible content for files ending with newlines. Fixed by filtering empty strings from the `after` context.

**Empty file edge case**: ropey reports `len_lines() = 1` even for empty files. Fixed by early-return check for `file_size == 0`.

## User Setup Required

None - no external service configuration required.

## Next Phase Readiness

- Context extraction module complete and tested with 8 passing tests
- SpanContext type available in output.rs from plan 11-01
- extract_context function available at `splice::extract_context` for use in LLM integration
- Ready for plan 11-03 (semantic kind detection) and plan 11-04 (LLM action taxonomy)

---
*Phase: 11-rich-span-core*
*Plan: 02*
*Completed: 2026-01-22*
