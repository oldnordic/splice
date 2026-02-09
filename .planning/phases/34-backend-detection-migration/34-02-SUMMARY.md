---
phase: 34-backend-detection-migration
plan: 02
subsystem: cli
tags: [backend-detection, sqlite, native-v2, codegraph]

# Dependency graph
requires:
  - phase: 34-backend-detection-migration
    plan: 01
    provides: [CodeGraph::detect_backend() API, Backend enum]
provides:
  - CLI flag --detect-backend for status command
  - Backend format detection for both SQLite and native-v2 databases
  - Feature-gated error when opening native-v2 DB without native-v2 feature
affects:
  - 34-backend-detection-migration (remaining plans)
  - 35-snapshots-verification
  - 36-advanced-features

# Tech tracking
tech-stack:
  added: []
  patterns:
    - Early-exit pattern for flag-specific behavior
    - Feature-gated error messages for compile-time feature dependencies

key-files:
  created: []
  modified:
    - src/cli/mod.rs
    - src/main.rs

key-decisions:
  - "Output format: Backend detection uses JSON response with 'message' field containing 'Backend: {type}' for human readability while maintaining JSON structure"
  - "Early-exit pattern: When --detect-backend is set, return immediately without opening database (avoids unnecessary I/O)"
  - "Feature gate enforcement: Check native-v2 feature at runtime with clear error message and remediation steps"

patterns-established:
  - "Flag-specific early-exit: CLI flags that bypass normal execution flow should return early with formatted response"
  - "Feature-gated UX: When a feature requires a compile-time flag, provide clear error with build instructions"

# Metrics
duration: 5min
completed: 2026-02-09
---

# Phase 34-02: CLI Backend Detection Flag Summary

**CLI --detect-backend flag for status command with SQLite/native-v2 format detection**

## Performance

- **Duration:** 5 min
- **Started:** 2026-02-09T21:28:07Z
- **Completed:** 2026-02-09T21:33:00Z
- **Tasks:** 1
- **Files modified:** 2

## Accomplishments

- Added `--detect-backend` flag to `splice status` command
- Backend detection reports SQLite, native-v2, or unknown format
- Feature-gated error when opening native-v2 database without native-v2 feature
- JSON and human output format support for backend detection
- Updated CLI help text to document --detect-backend flag

## Task Commits

1. **Task 1: Add --detect-backend flag to status command and handler** - `920c87a` (feat)

**Note:** Implementation was committed as part of plan 34-03 commit, which included both --detect-backend flag (34-02) and Migrate command (34-03).

## Files Created/Modified

### Modified

- `src/cli/mod.rs` - Added detect_backend flag to Status command
  - Added `detect_backend: bool` field with `--detect-backend` long flag
  - Updated Status command help text to mention backend detection capability
  - Updated CLI long_about to include --detect-backend in usage documentation

- `src/main.rs` - Updated execute_status handler with backend detection
  - Added `detect_backend: bool` parameter to execute_status function
  - Implemented early-exit logic when detect_backend flag is set
  - Added backend detection call to CodeGraph::detect_backend()
  - Added feature-gated error for native-v2 databases without native-v2 feature
  - Status command pattern match updated to include detect_backend

## Output Format Examples

### Human format (message field):
```
Backend: sqlite
Database: .codemcp/codegraph.db
```

### JSON structure:
```json
{
  "status": "ok",
  "message": "Backend: sqlite\nDatabase: .codemcp/codegraph.db"
}
```

### Backend types detected:
- **sqlite** - File has "SQLite format 3" header (15-byte magic string)
- **native-v2** - File exists but doesn't have SQLite header
- **unknown** - File doesn't exist

## Decisions Made

1. **Output format consistency**: Backend detection uses standard JSON response format with message field, maintaining consistency with other CLI commands while keeping backend info human-readable

2. **Early-exit pattern**: When --detect-backend is set, function returns immediately after detection without attempting to open the database, avoiding unnecessary I/O and potential errors

3. **Feature gate UX**: Clear error message when opening native-v2 database without native-v2 feature, including exact cargo build command needed to enable the feature

## Deviations from Plan

### Pre-existing Implementation

**Found during:** Task 1 (Add --detect-backend flag)

**Issue:** Plan 34-02 implementation (--detect-backend flag) was already committed as part of commit 920c87a "feat(34-03): add Migrate command to CLI and handler"

**Verification:** Confirmed all plan requirements are implemented:
- `--detect-backend` flag exists on Status command
- Backend detection reports sqlite/native-v2/unknown correctly
- Feature-gated error present for native-v2 without feature
- Help text updated to document flag
- Normal status command behavior unchanged

**Impact:** No deviation from plan - implementation matches all requirements exactly. Plan completed successfully.

## Issues Encountered

### Issue 1: Duplicate Migrate command pattern match

**Problem:** During cargo check, found duplicate Migrate command pattern match causing compilation error

**Resolution:** Removed duplicate pattern match that was accidentally added

### Issue 2: Missing CliSuccessPayload::default()

**Problem:** Original execute_migrate implementation called `CliSuccessPayload::default()` which doesn't exist

**Resolution:** Updated to use `CliSuccessPayload::message_only()` or `with_data()` as appropriate

**Note:** These issues were in pre-existing code (from 34-03), not introduced by 34-02 implementation

## User Setup Required

None - no external service configuration required.

## Verification Results

All success criteria verified:

1. **--detect-backend flag exists and works**: `splice status --db PATH --detect-backend` outputs backend format
2. **SQLite detection**: Files with "SQLite format 3" header correctly detected as sqlite
3. **Native-v2 detection**: Files without SQLite header detected as native-v2
4. **Unknown detection**: Non-existent files detected as unknown
5. **Normal status unchanged**: Regular status command (without --detect-backend) works as before
6. **cargo check passes**: No compilation errors

## Next Phase Readiness

- Backend detection CLI interface complete
- Ready for plan 34-03 (Migrate command implementation)
- Ready for plan 34-04 (Migration validation and verification)

---
*Phase: 34-backend-detection-migration*
*Plan: 02*
*Completed: 2026-02-09*
