---
phase: 30-impact-analysis-graph-algorithms
plan: 03
subsystem: graph-algorithms
tags: [tarjan, scc, cycle-detection, call-graph, magellan]

# Dependency graph
requires:
  - phase: 30-01
    provides: CLI pattern, output structure, MagellanIntegration foundation
provides:
  - Cycle detection via Tarjan's SCC algorithm
  - Cycles CLI command for finding call graph cycles
  - CycleInfo and CycleDetectionResult output types
affects: [30-04, 30-05, 30-06]

# Tech tracking
tech-stack:
  added: []
  patterns:
    - Tarjan's strongly connected components (SCC) algorithm
    - Self-loop detection for recursive calls
    - Cycle member enumeration with representative selection

key-files:
  created: []
  modified:
    - src/cli/mod.rs - Added Cycles command variant
    - src/output.rs - Added CycleDetectionResult and CycleInfo
    - src/graph/magellan_integration.rs - Added detect_cycles methods
    - src/main.rs - Added execute_cycles handler and wiring

key-decisions:
  - "Implemented Tarjan's SCC algorithm directly in MagellanIntegration instead of delegating to Mag subprocess"
  - "Cycle detection identifies SCCs with size > 1 OR self-loops (single node calling itself)"
  - "Used HashMap<(String, String), HashSet<(String, String)>> for call graph representation"

patterns-established:
  - "Graph algorithm pattern: build graph structure, run algorithm, convert results to output types"
  - "Mutable borrow handling: re-open database when needing both mutable query and subsequent immutable operations"

# Metrics
duration: 25min
completed: 2026-02-04
---

# Phase 30 Plan 03: Cycle Detection Command Summary

**Tarjan's SCC algorithm for detecting call graph cycles with self-loop detection and symbol filtering**

## Performance

- **Duration:** 25 min
- **Started:** 2026-02-04T13:30Z
- **Completed:** 2026-02-04T13:55Z
- **Tasks:** 5
- **Files modified:** 4

## Accomplishments

- Added `splice cycles` CLI command to detect cycles in the call graph
- Implemented Tarjan's strongly connected components (SCC) algorithm
- Added support for filtering cycles by specific symbol with --symbol/--path
- Added JSON/human/pretty output format support

## Task Commits

Each task was committed atomically:

1. **Task 1: Add Cycles command to CLI** - `b451efc` (feat)
2. **Task 2: Add CycleDetectionResult and CycleInfo types** - `2ec1635` (feat)
3. **Task 3: Add detect_cycles methods to MagellanIntegration** - `f3f2556` (feat)
4. **Task 4: Add execute_cycles handler function** - `c307943` (feat)
5. **Task 5: Wire command in main match** - (included in Task 4 commit)

## Files Created/Modified

- `src/cli/mod.rs` - Added Cycles subcommand with --db, --symbol, --path, --max-cycles, --show-members, --output flags
- `src/output.rs` - Added CycleDetectionResult and CycleInfo structs for JSON serialization
- `src/graph/magellan_integration.rs` - Added detect_cycles(), find_cycles_containing(), find_sccs(), scc_dfs(), has_self_loop(), scc_to_cycle_info() methods, plus CycleInfo struct
- `src/main.rs` - Added execute_cycles() handler and wiring in command match

## Decisions Made

- Implemented Tarjan's SCC algorithm directly in MagellanIntegration instead of delegating to Mag subprocess for better performance and type safety
- Used HashMap<(String, String), HashSet<(String, String)>> for call graph representation with (file_path, symbol_name) keys
- Cycles defined as SCCs with size > 1 OR self-loops (single node calling itself)
- Representative symbol selected as alphabetically first member for consistent output

## Deviations from Plan

None - plan executed exactly as written.

## Issues Encountered

1. **Compilation errors with HashMap/HashSet type resolution**
   - Fixed by using fully-qualified paths (std::collections::HashMap) in function signatures

2. **Mutable borrow conflicts in execute_cycles**
   - Fixed by using inner_mut() instead of inner() for MagellanGraph method calls

## User Setup Required

None - no external service configuration required.

## Next Phase Readiness

- Cycle detection complete and ready for use
- Foundation laid for 30-04 (Path Enumeration), 30-05 (Condensation), and 30-06 (Dead Code Analysis)
- No blockers or concerns

---
*Phase: 30-impact-analysis-graph-algorithms*
*Plan: 03*
*Completed: 2026-02-04*
