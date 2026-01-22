---
phase: 14-context-flags
plan: 03
subsystem: context-extraction
tags: [ropey, asymmetric-context, context-lines]

# Dependency graph
requires:
  - phase: 14-context-flags
    plan: 14-01
    provides: Unix-style context flags (-A, -B, -C) and extract_context_with_before_after() function
  - phase: 11-rich-span-core
    provides: SpanContext struct and context extraction infrastructure
provides:
  - extract_context_asymmetric() function for separate before/after context line counts
  - Refactored extract_context() to delegate to asymmetric version (DRY principle)
  - Three new tests covering asymmetric context edge cases (zero-before, zero-after)
affects: [14-04, CLI context flag implementations]

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "Asymmetric context extraction: separate before/after parameters for flexible context"
    - "Delegation pattern: symmetric extract_context() delegates to extract_context_asymmetric()"

key-files:
  modified:
    - src/context.rs - Added extract_context_asymmetric(), refactored extract_context(), added 3 tests
    - src/lib.rs - Exported extract_context_asymmetric from crate root

key-decisions:
  - "Naming: extract_context_asymmetric() as primary API, extract_context_with_before_after() kept as alias for backward compatibility"
  - "Code reuse: extract_context() refactored to delegate to asymmetric version rather than duplicate logic"

patterns-established:
  - "Asymmetric context pattern: when before/after needs differ, use extract_context_asymmetric()"
  - "Convenience alias pattern: keep longer names as aliases for code readability"

# Metrics
duration: ~3min
completed: 2026-01-22
---

# Phase 14 Plan 03: Asymmetric Context Extraction Summary

**Asymmetric context extraction with separate before/after line counts using ropey, refactored symmetric version to delegate**

## Performance

- **Duration:** 2 min 49 sec (169s)
- **Started:** 2026-01-22T14:19:41Z
- **Completed:** 2026-01-22T14:22:30Z
- **Tasks:** 3
- **Files modified:** 2

## Accomplishments

- Added `extract_context_asymmetric()` function supporting different `context_before` and `context_after` parameters
- Refactored `extract_context()` to delegate to asymmetric version (DRY - reduced 89 lines to 3)
- Kept `extract_context_with_before_after()` as convenience alias for backward compatibility
- Exported `extract_context_asymmetric` from crate root for public API access
- Added 3 comprehensive tests covering zero-before and zero-after edge cases

## Task Commits

1. **Task 1: Add extract_context_asymmetric function to context.rs** - `5e55e20` (feat)
2. **Task 2: Export extract_context_asymmetric from lib.rs** - `5e55e20` (feat)
3. **Task 3: Add tests for asymmetric context extraction** - `5e55e20` (feat)

**Note:** All three tasks were committed together as a single atomic feature commit.

## Files Created/Modified

- `src/context.rs` - Added `extract_context_asymmetric()` (73 lines), refactored `extract_context()` (3 lines), kept `extract_context_with_before_after()` as alias (2 lines), added 3 tests (57 lines)
- `src/lib.rs` - Added `extract_context_asymmetric` to context re-exports

## Decisions Made

1. **Naming convention**: Used `extract_context_asymmetric()` as the primary public API name, with `context_before`/`context_after` parameters (shorter than `context_lines_before`/`context_lines_after`)
2. **Backward compatibility**: Kept `extract_context_with_before_after()` as a convenience alias since it may be used elsewhere in the codebase
3. **DRY refactoring**: Converted `extract_context()` from 87-line duplicate implementation to 3-line delegation to `extract_context_asymmetric()`

## Deviations from Plan

None - plan executed exactly as written. The implementation:
- Added `extract_context_asymmetric()` with the exact signature specified in the plan
- Refactored `extract_context()` to delegate to the asymmetric version
- Added tests for zero-before and zero-after cases as specified

## Issues Encountered

**Test byte offset calculation error** - Initial tests failed because byte offsets were calculated incorrectly:
- **Issue**: Using `iter().position(|&b| b == b'3')` found the position of character '3' within "line 3", not the start of the line
- **Fix**: Changed to use exact byte calculations based on known line lengths (7 bytes per line: "line N\n")
- **Verification**: All 11 context tests pass (8 original + 3 new asymmetric)

## User Setup Required

None - no external service configuration required.

## Next Phase Readiness

- **Asymmetric context API ready**: `extract_context_asymmetric()` can be called from CLI commands
- **Export verified**: Function is accessible from `splice::extract_context_asymmetric`
- **Tests passing**: All 11 context tests pass, including edge cases for zero-before and zero-after
- **Integration ready**: CLI can now use asymmetric extraction for -A/-B/-C flags in subsequent plans

---
*Phase: 14-context-flags*
*Plan: 03*
*Completed: 2026-01-22*
