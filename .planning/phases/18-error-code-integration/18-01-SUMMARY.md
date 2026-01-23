---
phase: 18-error-code-integration
plan: 01
subsystem: error-handling
tags: [error-codes, cli, json, serde, rust]

# Dependency graph
requires:
  - phase: 11-rich-span-core
    provides: Error code infrastructure (SpliceErrorCode enum, error registry)
  - phase: 15-enhanced-errors
    provides: Enhanced error system with severity levels and location extraction
provides:
  - Complete error code mappings for all 22 error-level SpliceError variants
  - JSON error responses include explain_command field for user guidance
  - Comprehensive test coverage for error code mappings
affects: []

# Tech tracking
tech-stack:
  added: []
  patterns:
    - Exhaustive match statements for error code mapping
    - Optional fields in JSON with serde(skip_serializing_if)
    - TDD approach: RED (failing test) → GREEN (implement) → REFACTOR (clean up)

key-files:
  created: []
  modified:
    - src/error_codes.rs - Added 5 error mappings, comprehensive test coverage
    - src/cli/mod.rs - Added explain_command field to ErrorDetails

key-decisions:
  - "BrokenPipe, Utf8, Other variants intentionally unmapped - not user-fixable or covered by other variants"
  - "explain_command uses --code flag for direct executability"
  - "Removed unreachable catchall pattern after exhaustive matching"

patterns-established:
  - "Error code mapping pattern: All error-level variants map to specific SPL-E### codes"
  - "Explain command auto-generation: Format 'splice explain --code SPL-E###'"
  - "TDD for error mappings: Write failing test first, then implement mappings"

# Metrics
duration: 3min
completed: 2026-01-23
---

# Phase 18 Plan 1: Complete Error Code Mappings Summary

**All 22 error-level SpliceError variants now map to SPL-E### error codes with explain_command field in JSON responses**

## Performance

- **Duration:** 3 min
- **Started:** 2026-01-23T21:39:10Z
- **Completed:** 2026-01-23T21:43:03Z
- **Tasks:** 3
- **Files modified:** 2

## Accomplishments

- **Complete error code coverage:** All 22 error-level SpliceError variants now map to specific SPL-E### codes
- **Auto-referencing explain commands:** JSON error responses include `explain_command` field with copy-pasteable command
- **Comprehensive test coverage:** Added `test_error_code_coverage` and `test_explain_command_generation` tests
- **Intentional unmapping documented:** BrokenPipe, Utf8, Other variants explicitly documented as unmapped

## Task Commits

Each task was committed atomically:

1. **Task 1: Map remaining SpliceError variants to error codes** - `936b17a` (feat)
2. **Task 2: Add explain_command field to ErrorDetails struct** - `639d702` (feat)
3. **Task 3: Add tests for explain_command generation** - `2eceba7` (test)
4. **Fix: Correct explain_command format to use --code flag** - `218b8a3` (fix)

**Plan metadata:** Pending (docs commit after SUMMARY.md)

_Note: TDD approach used - failing test written first, then implementations added_

## Files Created/Modified

- `src/error_codes.rs` - Added mappings for InsufficientDiskSpace, InvalidDateFormat, QueryError, CargoCheckFailed, ExecutionRecordFailed; removed unreachable catchall pattern; added comprehensive test coverage
- `src/cli/mod.rs` - Added explain_command field to ErrorDetails struct; populated field in from_error() method when error_code present

## Decisions Made

- **Error code mappings:**
  - InsufficientDiskSpace → FileWriteError (SPL-E032) - disk space is write constraint
  - InvalidDateFormat → InvalidPlanSchema (SPL-E051) - date format is part of plan/query schema
  - QueryError → DatabaseError (SPL-E062) - query operations are database operations
  - CargoCheckFailed → CompilerValidationFailed (SPL-E043) - cargo check is Rust compiler validation
  - ExecutionRecordFailed → ExecutionLogError (SPL-E071) - recording is log operation

- **Intentionally unmapped variants:**
  - BrokenPipe - terminal state, stdout pipe closed, not user-fixable
  - Utf8 (std lib variant) - covered by InvalidUtf8 variant with file context
  - Other - generic catchall, no specific code applicable

- **Explain command format:** Used `--code` flag instead of positional argument for direct executability

## Deviations from Plan

None - plan executed exactly as written.

## Issues Encountered

- **Test compilation errors:** Initial test had issues constructing Utf8Error and SqliteGraphError due to private fields
  - **Resolution:** Used proper constructors (std::str::from_utf8 for Utf8Error, SqliteGraphError::connection for graph errors)

- **Explain command format:** Initial format "splice explain SPL-E001" wouldn't execute directly
  - **Resolution:** Changed to "splice explain --code SPL-E001" to match actual CLI syntax

## User Setup Required

None - no external service configuration required.

## Next Phase Readiness

- All error-level SpliceError variants now produce structured error codes
- JSON error responses include explain_command field with copy-pasteable command
- Error code infrastructure is complete and fully tested
- No blockers or concerns

## Verification

Run these commands to verify the implementation:

```bash
# Verify all error code tests pass
cargo test error_codes --lib

# Verify JSON output includes explain_command
./target/debug/splice delete --symbol test --file nonexistent.rs --json | jq .

# Verify explain command works
./target/debug/splice explain --code SPL-E001
```

All tests pass (311 library tests), no compilation errors, JSON output includes both `error_code` and `explain_command` fields.

---
*Phase: 18-error-code-integration*
*Plan: 01*
*Completed: 2026-01-23*
