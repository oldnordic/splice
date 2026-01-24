---
phase: 21-api-consolidation
plan: 01
subsystem: api-refactoring
tags: [tree-sitter, parser, dry, code-consolidation]

# Dependency graph
requires: []
provides:
  - Single parser_for_language function in symbol module
  - Consolidated parser creation API
affects: [expand, patch]

# Tech tracking
tech-stack:
  added: []
  patterns: [single-source-of-truth, DRY principle]

key-files:
  created: []
  modified:
    - src/symbol/mod.rs
    - src/expand/mod.rs
    - src/patch/pattern.rs

key-decisions:
  - "Place parser_for_language in symbol module alongside Language enum"
  - "Make function public for reuse across modules"

patterns-established:
  - "Centralized parser creation: use crate::symbol::parser_for_language"
  - "API consolidation pattern for duplicate utilities"

# Metrics
duration: 3min
completed: 2026-01-24
---

# Phase 21 Plan 01: Parser Consolidation Summary

**Single shared parser_for_language function eliminates 48 lines of duplicate code across expand and patch modules**

## Performance

- **Duration:** 3 min
- **Started:** 2026-01-24T00:15:28Z
- **Completed:** 2026-01-24T00:18:42Z
- **Tasks:** 3
- **Files modified:** 3

## Accomplishments

- Consolidated duplicate `parser_for_language` functions from expand/mod.rs and patch/pattern.rs into symbol/mod.rs
- Established single source of truth for tree-sitter parser creation alongside the Language enum definition
- Eliminated 48 lines of identical code (25 from expand, 23 from patch)
- All 61 tests pass (24 pattern tests, 37 expand tests)

## Task Commits

Each task was committed atomically:

1. **Task 1: Add parser_for_language to symbol module** - `da00623` (feat)
2. **Task 2: Use shared parser_for_language in expand module** - `eff3279` (feat)
3. **Task 3: Use shared parser_for_language in patch/pattern module** - `d910230` (feat)

**Plan metadata:** Not yet committed

_Note: TDD tasks may have multiple commits (test → feat → refactor)_

## Files Created/Modified

- `src/symbol/mod.rs` - Added centralized parser_for_language function with proper error handling
- `src/expand/mod.rs` - Removed duplicate function, added import from symbol module
- `src/patch/pattern.rs` - Removed duplicate function, added import from symbol module

## Decisions Made

- **Placement in symbol module:** Chose symbol/mod.rs because it already defines the Language enum, making it the logical home for language-related utilities
- **Public API:** Made function public (`pub fn`) to allow reuse across modules while maintaining encapsulation
- **Import style:** Used grouped import `use crate::symbol::{parser_for_language, Language}` for clarity

## Deviations from Plan

None - plan executed exactly as written.

## Issues Encountered

None - all tasks completed without issues.

## User Setup Required

None - no external service configuration required.

## Next Phase Readiness

- Consolidation complete, no breaking changes
- Parser API now follows DRY principle with single implementation
- Ready for further API consolidation work in Phase 21

---
*Phase: 21-api-consolidation*
*Completed: 2026-01-24*
