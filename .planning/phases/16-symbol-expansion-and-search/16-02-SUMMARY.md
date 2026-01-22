---
phase: 16-symbol-expansion-and-search
plan: 02
subsystem: cli
tags: [cli-flags, symbol-expansion, tree-sitter, magellan]

# Dependency graph
requires:
  - phase: 16-symbol-expansion-and-search
    plan: 01
    provides: expand_symbol_with_level function, ExpansionLevel enum, SymbolExpander trait
provides:
  - --expand and --expand-level CLI flags for Get and Query commands
  - Symbol expansion integration in execute_get and execute_query functions
affects: [16-03, 16-04, 16-05, 16-06]

# Tech tracking
tech-stack:
  added: []
  patterns:
    - CLI flag pattern: boolean flag with level override (expand, expand-level)
    - Expansion fallback pattern: expand on success, fall back to original span on error
    - Language detection pattern: auto-detect from file path, pass to expansion function

key-files:
  created: []
  modified:
    - src/cli/mod.rs - Added expand and expand_level fields to Commands::Query and Commands::Get
    - src/main.rs - Updated execute_get and execute_query with expansion logic
    - src/expand/tree_walker.rs - Created tree_walker module with parent chain walking utilities

key-decisions:
  - "Default expand-level to 1 (body) for convenience when --expand flag is used alone"
  - "Allow --expand-level 0 to disable expansion even with --expand set (flexibility for testing)"
  - "Fall back to original span on language detection or expansion failure (graceful degradation)"
  - "Use expanded span for all operations: content retrieval, context extraction, checksums"

patterns-established:
  - "Pattern: Boolean flag with numeric level override (--expand, --expand-level)"
  - "Pattern: Language detection from file path before expansion operations"
  - "Pattern: Graceful fallback when expansion fails (return original span)"

# Metrics
duration: 45min
completed: 2026-01-22
---

# Phase 16: Symbol Expansion and Search - Plan 02 Summary

**CLI expansion flags with --expand/--expand-level for Get and Query commands, using tree-sitter parent chain walking to retrieve full symbol bodies**

## Performance

- **Duration:** 45 min
- **Started:** 2026-01-22T19:01:39Z
- **Completed:** 2026-01-22T19:46:39Z
- **Tasks:** 3
- **Files modified:** 3

## Accomplishments

- Added --expand and --expand-level CLI flags to Get and Query commands
- Integrated symbol expansion in execute_get function with language detection
- Integrated symbol expansion in execute_query function with per-result expansion
- Created tree_walker module with parent chain walking utilities (from 16-01)
- Ensured backward compatibility (flags default to no expansion)

## Task Commits

Each task was committed atomically:

1. **Task 1: Add --expand and --expand-level flags to CLI** - `1d97cc6` (feat)
2. **Task 2: Wire expand flags in execute_get function** - `e975b2b` (feat)
3. **Task 3: Wire expand flags in execute_query function** - `fedd2b1` (feat)

**Plan metadata:** No separate metadata commit (tasks included all changes)

_Note: Tree_walker.rs was created as part of resolving blocking issue during Task 1_

## Files Created/Modified

- `src/cli/mod.rs` - Added expand and expand_level fields to Commands::Query (lines 279-285) and Commands::Get (lines 322-328)
- `src/main.rs` - Updated execute_get signature and implementation with expansion logic (lines 2386-2503)
- `src/main.rs` - Updated execute_query signature and implementation with expansion logic (lines 1970-2228)
- `src/expand/tree_walker.rs` - Created parent chain walking utilities (508 lines with comprehensive tests)

## Decisions Made

1. **Default expand-level to 1**: When --expand flag is used without --expand-level, default to level 1 (body expansion) for convenience
2. **Allow level 0 override**: --expand-level 0 explicitly disables expansion even if --expand is set, useful for testing and scripting
3. **Graceful degradation on errors**: If language detection fails or expansion returns an error, fall back to original span rather than failing the entire command
4. **Apply expansion to all span operations**: Expanded spans are used for content retrieval, context extraction, and checksum calculation to ensure consistency
5. **Per-result expansion in query**: For Query command, expansion is applied to each result individually since results may span multiple files with different languages

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 3 - Blocking] Created tree_walker module for expansion infrastructure**
- **Found during:** Task 1 (CLI flag implementation)
- **Issue:** Plan 16-01 (expansion infrastructure) had not been executed, but plan 16-02 depended on expand_symbol function
- **Fix:** Created src/expand/tree_walker.rs module with find_parent_symbol_node and expand_to_containing_block functions. Also added expand_symbol_with_level convenience function to expand/mod.rs for CLI integration (usize level parameter instead of ExpansionLevel enum)
- **Files modified:** src/expand/tree_walker.rs (created 508 lines), src/expand/mod.rs (added expand_symbol_with_level function)
- **Verification:** Module compiles, all unit tests pass (18 tests in tree_walker.rs)
- **Committed in:** Part of Task 1 commit (1d97cc6)

**2. [Rule 1 - Bug] Fixed tree_walker API mismatch with expand/mod.rs expectations**
- **Found during:** Task 2 (execute_get implementation)
- **Issue:** tree_walker.rs created with different API than expected by expand/mod.rs (find_parent_symbol_node signature incompatibility)
- **Fix:** Updated tree_walker.rs to use predicate-based closure API matching expand/mod.rs expectations. Changed find_parent_symbol_node signature from `(node, source, language: &str)` to `(node, source, F: Fn(&str) -> bool)`. Also updated expand_to_containing_block to take expander parameter
- **Files modified:** src/expand/tree_walker.rs (updated function signatures)
- **Verification:** Compilation succeeds, all 18 tree_walker tests pass
- **Committed in:** Part of Task 2 commit (e975b2b)

**3. [Rule 1 - Bug] Fixed linter revert of main.rs changes**
- **Found during:** Task 2 (execute_get implementation)
- **Issue:** Linter automatically reverted changes to main.rs match arms, causing compilation errors about missing expand/expand_level fields
- **Fix:** Re-applied changes to Commands::Query and Commands::Get match arms to include expand and expand_level field destructuring and function call parameters
- **Files modified:** src/main.rs (lines 120-149)
- **Verification:** Compilation succeeds, cargo check passes
- **Committed in:** Part of Task 2 commit (e975b2b)

---

**Total deviations:** 3 auto-fixed (1 blocking, 2 bugs)
**Impact on plan:** All auto-fixes necessary for correctness. Blocking issue (missing expand module) was essential prerequisite. Bug fixes addressed API mismatches and linter conflicts. No scope creep.

## Issues Encountered

- **Compilation errors after linter revert**: Linter automatically reverted main.rs changes, requiring re-application of match arm updates. Fixed by re-editing the file and committing again.
- **sccache compilation failures**: Build system errors with sccache causing temporary build failures. Resolved by running `cargo clean` and rebuilding without sccache.

## User Setup Required

None - no external service configuration required. All functionality is local CLI flags.

## Verification

- [x] `cargo check` passes with no errors
- [x] `splice get --help` shows --expand and --expand-level flags
- [x] `splice query --help` shows --expand and --expand-level flags
- [x] Flags default to no expansion (backward compatible)
- [x] --expand flag alone uses level 1
- [x] --expand-level 0 disables expansion even with --expand
- [x] expand_symbol_with_level returns byte offsets compatible with extract_context_asymmetric (CLI-14 integration verified)

## Next Phase Readiness

- Expansion infrastructure complete and wired to CLI
- Ready for plan 16-03 (Search Command) which can leverage expansion for full symbol retrieval
- Ready for plan 16-06 (End-to-End Integration) which will verify expansion works correctly with all context flags

**Blockers/Concerns:**
- None identified

---
*Phase: 16-symbol-expansion-and-search*
*Plan: 02*
*Completed: 2026-01-22*
