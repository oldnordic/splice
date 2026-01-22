---
phase: 11-rich-span-core
plan: 06
subsystem: error-handling
tags: [error-codes, diagnostics, SPL-E-format, severity-levels]

# Dependency graph
requires:
  - phase: 11-01
    provides: SpanResult with error_code field
provides:
  - Stable error code registry (SPL-E### format) for all Splice operations
  - ErrorCode struct with severity, location, and hint fields
  - SpliceErrorCode enum with 26 error variants across 9 categories
  - Mapping from SpliceError to SpliceErrorCode via from_splice_error()
  - 8 comprehensive unit tests for error code functionality
affects: [11-07, 11-08, cli-output-formats, error-reporting]

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "SPL-E### error code format (tool + severity + number)"
    - "Single-sourced error registry in dedicated module"
    - "Automatic SpliceError to SpliceErrorCode conversion"
    - "Builder pattern for ErrorCode construction"

key-files:
  created: [src/error_codes.rs]
  modified: [src/lib.rs, src/output.rs]

key-decisions:
  - "Error codes follow SPL-{E|W|N}-### format for LLM consumption"
  - "ErrorCode struct single-sourced in error_codes.rs module"
  - "All error codes include actionable hints for user guidance"
  - "Non-exhaustive pattern handling for file:line:column location"

patterns-established:
  - "Pattern 1: Error code categories (symbol: E001-E010, parse: E011-E020, span: E021-E030, I/O: E031-E040, validation: E041-E050, plan: E051-E060, graph: E061-E070, execution: E071-E080, analyzer: E081-E090)"
  - "Pattern 2: from_splice_error() provides automatic error code mapping"
  - "Pattern 3: ErrorCode::from_splice_code() handles location formatting"

# Metrics
duration: 4min
completed: 2026-01-22
---

# Phase 11 Plan 06: Error Code Registry Summary

**Stable error code registry with SPL-E### format, severity levels, precise locations, and actionable hints for LLM consumption and user guidance**

## Performance

- **Duration:** 4 min
- **Started:** 2026-01-22T09:24:31Z
- **Completed:** 2026-01-22T09:29:09Z
- **Tasks:** 3
- **Files modified:** 3

## Accomplishments

- Created comprehensive error code registry with 26 error variants across 9 categories
- Implemented SPL-E### format (tool identifier + severity + sequential number)
- Added automatic SpliceError to SpliceErrorCode conversion via from_splice_error()
- Integrated ErrorCode struct into SpanResult for JSON serialization
- Removed duplicate ErrorCode definition from output.rs (single-sourced in error_codes.rs)
- Verified all 8 unit tests pass (error code format, severity, hints, location formatting, SpliceError mapping)

## Task Commits

Each task was committed atomically:

1. **Task 1: Create error_codes.rs module with error code registry** - `c6a465b` (feat)
2. **Task 2: Export error_codes module and remove duplicate** - `95832aa` (feat)
3. **Task 3: Run error codes tests** - (no commit - tests verified existing code)

**Total commits:** 2 (Task 3 verified existing implementation)

## Files Created/Modified

- `src/error_codes.rs` - Error code registry with ErrorCode struct, SpliceErrorCode enum (26 variants), ErrorSeverity enum, and 8 unit tests
- `src/lib.rs` - Added error_codes module declaration and re-exports (ErrorCode, ErrorSeverity, SpliceErrorCode)
- `src/output.rs` - Removed duplicate ErrorCode struct, added use crate::error_codes::ErrorCode import

## Decisions Made

- **SPL-E### format chosen** for clarity and machine readability (SPL tool identifier + E/W/N severity + 3-digit number)
- **Single-sourced ErrorCode in error_codes.rs** to avoid duplication and maintenance burden
- **Comprehensive hint messages** for each error code to provide actionable guidance to users and LLMs
- **Automatic SpliceError mapping** via from_splice_error() for seamless integration with existing error handling
- **Non-exhaustive location pattern handling** added for file:column case (Some(f), None, Some(c))

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Bug] Fixed non-exhaustive patterns in ErrorCode::from_splice_code()**
- **Found during:** Task 2 (cargo check --lib)
- **Issue:** Pattern `(Some(f), None, Some(c))` (file:column but no line) was not covered in match expression
- **Fix:** Added missing pattern arm: `(Some(f), None, Some(c)) => format!("{}:{}", f, c),`
- **Files modified:** src/error_codes.rs
- **Verification:** cargo check --lib succeeds without errors
- **Committed in:** 95832aa (Task 2 commit)

**2. [Rule 2 - Missing Critical] Added doc comments to all SpliceErrorCode enum variants**
- **Found during:** Task 2 (cargo check --lib showed 26 missing_docs warnings)
- **Issue:** All 26 SpliceErrorCode variants lacked documentation, violating project's #![warn(missing_docs)] policy
- **Fix:** Added /// doc comments to all 26 enum variants describing each error code
- **Files modified:** src/error_codes.rs
- **Verification:** cargo check --lib succeeds with zero warnings
- **Committed in:** 95832aa (Task 2 commit)

---

**Total deviations:** 2 auto-fixed (1 bug fix, 1 missing critical)
**Impact on plan:** Both auto-fixes necessary for correctness and code quality compliance. No scope creep.

## Issues Encountered

None - plan executed smoothly with only minor auto-fixes for completeness.

## User Setup Required

None - no external service configuration required.

## Next Phase Readiness

- Error code registry fully functional and tested
- ErrorCode available for use in SpanResult.error_code field (from 11-01)
- SpliceErrorCode enum can be extended with additional error codes as needed
- Error codes ready for integration with CLI output formatting and error reporting
- No blockers or concerns

## Verification Checklist

All verification criteria from plan met:

- [x] src/error_codes.rs module exists (431 lines)
- [x] SpliceErrorCode enum has 26 error code variants
- [x] ErrorCode struct has code, severity, location, hint fields
- [x] Error codes follow SPL-E### format
- [x] All error codes have hint messages (83 match arms in hint() method)
- [x] SpliceErrorCode::from_splice_error maps SpliceError to error codes (24 pattern matches)
- [x] All 8 unit tests pass
- [x] ErrorCode exported from crate root (pub use error_codes::{ErrorCode, ErrorSeverity, SpliceErrorCode})

## Error Code Categories

Error codes organized by category with sequential numbering:

1. **Symbol resolution (SPL-E001 to SPL-E010)**: SymbolNotFound, AmbiguousSymbol, ReferenceFailed, AmbiguousReference
2. **Parse/AST (SPL-E011 to SPL-E020)**: ParseError, InvalidUtf8, InvalidSyntax
3. **Span (SPL-E021 to SPL-E030)**: InvalidSpan, InvalidLineRange, SpanOutOfBounds
4. **I/O (SPL-E031 to SPL-E040)**: FileReadError, FileWriteError, FileNotFound, FileExternallyModified
5. **Validation (SPL-E041 to SPL-E050)**: PreVerificationFailed, ParseValidationFailed, CompilerValidationFailed
6. **Plan execution (SPL-E051 to SPL-E060)**: InvalidPlanSchema, PlanExecutionFailed, InvalidBatchSchema
7. **Graph/database (SPL-E061 to SPL-E070)**: GraphError, DatabaseError
8. **Execution log (SPL-E071 to SPL-E080)**: ExecutionLogError, ExecutionNotFound
9. **Analyzer (SPL-E081 to SPL-E090)**: AnalyzerNotAvailable, AnalyzerFailed

---
*Phase: 11-rich-span-core*
*Plan: 06*
*Completed: 2026-01-22*
