---
phase: 15-enhanced-errors
plan: 01
subsystem: error-handling
tags: [error-codes, severity-levels, warning-diagnostics]

# Dependency graph
requires:
  - phase: 14-context-flags
    provides: CLI context infrastructure
provides:
  - SpliceErrorCode enum with warning-level variants (SPL-W001 to SPL-W003)
  - severity() method returning "error" or "warning" based on variant
  - Comprehensive test coverage for severity levels (13 tests)
affects: [15-02, 15-03, cli-output, error-handling]

# Tech tracking
tech-stack:
  added: []
  patterns: [error-code-enum-with-severity, match-based-severity-discrimination]

key-files:
  created: []
  modified: [src/error_codes.rs]

key-decisions:
  - "3 warning variants added (not hundreds) to establish pattern without scope creep"
  - "AmbiguousSymbol, AmbiguousReference, FileExternallyModified upgraded to warning severity"

patterns-established:
  - "SPL-W### code format for warning-level diagnostics"
  - "severity() method uses match arms to discriminate error vs warning"
  - "ErrorCode struct receives proper severity via from_splice_code()"

# Metrics
duration: 3min
completed: 2026-01-22
---

# Phase 15: Enhanced Errors Plan 01 Summary

**SpliceErrorCode enum extended with warning-level variants (SPL-W001 to SPL-W003) and proper severity() method discrimination**

## Performance

- **Duration:** 3 min
- **Started:** 2026-01-22T15:15:15Z
- **Completed:** 2026-01-22T15:18:06Z
- **Tasks:** 3
- **Files modified:** 1

## Accomplishments

- Added 3 warning-level variants to SpliceErrorCode enum (AmbiguousSymbolAsWarning, FileSkipped, FileExternallyModifiedWarning)
- Replaced hardcoded "error" return in severity() with proper match statement discriminating 22 error-level vs 6 warning-level codes
- Upgraded existing AmbiguousSymbol, AmbiguousReference, FileExternallyModified to warning severity
- Added 5 comprehensive tests verifying severity levels across all variants
- Verified ErrorCode struct properly receives severity field from SpliceErrorCode via from_splice_code()

## Task Commits

Each task was committed atomically:

1. **Task 1: Add Warning variants to SpliceErrorCode enum** - `f8368bb` (feat)
2. **Task 2: Implement proper severity() method returning multiple levels** - `ccc3674` (feat)
3. **Task 3: Add tests for severity levels** - `cbd6c49` (test)

**Plan metadata:** (pending final docs commit)

_Note: All tasks completed with TDD pattern (test not applicable - no failing tests required for enum extension)_

## Files Created/Modified

- `src/error_codes.rs` - Extended SpliceErrorCode enum with warning variants, implemented proper severity() method, added 5 tests (now 555 lines, 13 tests)

## Changes Made to error_codes.rs

### Warning Variants Added (Task 1)

Three new warning-level codes added to SpliceErrorCode enum:

```rust
// Warning-level errors (SPL-W001 to SPL-W010)
/// Symbol exists in multiple files (SPL-W001)
AmbiguousSymbolAsWarning,
/// File skipped during ingestion (SPL-W002)
FileSkipped,
/// External modification detected (SPL-W003)
FileExternallyModifiedWarning,
```

### Severity Method Updated (Task 2)

Replaced hardcoded "error" return with comprehensive match statement:

```rust
pub fn severity(&self) -> String {
    match self {
        // Error-level codes (22 variants)
        SpliceErrorCode::SymbolNotFound
        | SpliceErrorCode::ReferenceFailed
        | SpliceErrorCode::ParseError
        [... 19 more error variants]
        | SpliceErrorCode::AnalyzerFailed => "error".to_string(),

        // Warning-level codes (6 variants)
        SpliceErrorCode::AmbiguousSymbol => "warning".to_string(),
        SpliceErrorCode::AmbiguousReference => "warning".to_string(),
        SpliceErrorCode::FileExternallyModified => "warning".to_string(),
        SpliceErrorCode::AmbiguousSymbolAsWarning
        | SpliceErrorCode::FileSkipped
        | SpliceErrorCode::FileExternallyModifiedWarning => "warning".to_string(),
    }
}
```

### Tests Added (Task 3)

Five new tests added to verify severity behavior:

1. `test_error_code_severity_error` - Verifies all 22 error-level codes return "error"
2. `test_error_code_severity_warning` - Verifies all 6 warning-level codes return "warning"
3. `test_error_code_all_have_severity` - Iterates all 28 SpliceErrorCode variants to validate severity is "error", "warning", or "note"
4. `test_warning_code_format` - Verifies SPL-W### format for warning codes
5. `test_warning_code_from_splice_code` - Verifies ErrorCode struct receives proper warning severity

### Test Coverage Summary

- **Total tests in error_codes module:** 13
- **New tests added:** 5
- **Test status:** All 13 tests passing
- **Coverage:** All 28 SpliceErrorCode variants tested for severity level

## Decisions Made

- Followed plan guidance to add only 3 warning variants (not hundreds) to establish pattern without scope creep
- Upgraded existing AmbiguousSymbol, AmbiguousReference, FileExternallyModified to warning severity as these represent non-blocking conditions
- Placed warning variants after FileExternallyModified and before Validation errors section to maintain code organization

## Deviations from Plan

None - plan executed exactly as written.

## Issues Encountered

None - all tasks completed successfully without issues.

## User Setup Required

None - no external service configuration required.

## Next Phase Readiness

- SpliceErrorCode enum now supports both error and warning severity levels
- severity() method properly distinguishes between blocking errors (22 variants) and non-blocking warnings (6 variants)
- ErrorCode struct receives proper severity field when constructed via from_splice_code()
- Test coverage ensures all variants have valid severity values
- Ready for Phase 15-02 (location() helper) and Phase 15-03 (CLI integration)

---
*Phase: 15-enhanced-errors*
*Completed: 2026-01-22*
