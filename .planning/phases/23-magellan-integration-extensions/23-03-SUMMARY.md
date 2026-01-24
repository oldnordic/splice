---
phase: 23-magellan-integration-extensions
plan: 03
subsystem: magellan-query
tags: [magellan, symbol-lookup, sqlite, symbol-id, o(n)-iteration]

# Dependency graph
requires:
  - phase: 22-symbol-id-and-format-foundation
    provides: generate_symbol_id() for 16-char hex ID generation
  - phase: 23-02
    provides: MagellanIntegration with db_path for direct SQL access
provides:
  - find_symbol_by_name() method for global symbol name search across all files
  - find_symbol_by_id() method for reverse symbol ID lookup via O(N) entity scan
affects: [23-04, 23-05, cli-find-command]

# Tech tracking
tech-stack:
  added: []
  patterns:
    - File iteration pattern for global queries (all_file_nodes + symbol_extents)
    - Direct SQL pattern for entity iteration when Magellan API doesn't expose methods
    - Symbol ID regeneration for reverse lookup (SHA-256 match)

key-files:
  modified:
    - src/graph/magellan_integration.rs (find_symbol_by_name, find_symbol_by_id)

key-decisions:
  - "Use direct SQL for find_symbol_by_id() because Magellan doesn't expose entity_ids() or get_node() publicly"
  - "Accept O(N) performance for both methods - optimization deferred until profiling indicates need"
  - "Regenerate symbol_id during iteration rather than maintaining reverse index (simpler, works for MVP)"

patterns-established:
  - "File iteration: all_file_nodes() → symbol_extents() for each file"
  - "Direct SQL entity scan: SELECT FROM graph_entities WHERE kind = 'Symbol' → parse JSON → regenerate ID → match"

# Metrics
duration: 3min
completed: 2026-01-24
---

# Phase 23: Magellan Integration Extensions - Plan 03 Summary

**Name-based and ID-based symbol lookup methods using file iteration and direct SQL entity scanning**

## Performance

- **Duration:** 3 minutes
- **Started:** 2026-01-24T11:57:40Z
- **Completed:** 2026-01-24T12:00:45Z
- **Tasks:** 1
- **Files modified:** 1

## Accomplishments

- `find_symbol_by_name()`: Global symbol search across all indexed files with ambiguous flag support
- `find_symbol_by_id()`: Reverse symbol ID lookup using SQL entity scanning and symbol_id regeneration
- Both methods handle Magellan API limitations (no global name index, no reverse ID lookup)

## Task Commits

Each task was committed atomically:

1. **Task 1: Add find_symbol_by_name() and find_symbol_by_id() methods** - `e4ae113` (feat)

**Plan metadata:** (none - only one task)

## Files Created/Modified

- `src/graph/magellan_integration.rs` - Added two public methods:
  - `find_symbol_by_name()`: Iterates all files via `all_file_nodes()`, calls `symbol_extents()` for each
  - `find_symbol_by_id()`: Queries `graph_entities` table directly, regenerates symbol_id to match

## Decisions Made

1. **Direct SQL for entity iteration**: Magellan's CodeGraph doesn't expose `entity_ids()` or `get_node()` publicly despite these existing on the internal backend. Used direct SQL to query `graph_entities` table (same pattern as `count_call_nodes()` from 23-01).

2. **Accept O(N) performance for MVP**: Both lookup methods are O(N) where N = number of files (name search) or symbols (ID search). No optimization now - will profile later and add symbol_id index if needed.

3. **Symbol ID regeneration approach**: Instead of maintaining a symbol_id → entity_id reverse index, regenerate symbol_id during entity scan using Phase 22's `generate_symbol_id()`. Simpler implementation, works correctly for MVP.

## Deviations from Plan

### API Discovery Required

**1. [Rule 1 - Bug] Magellan doesn't expose entity_ids() and get_node() publicly**
- **Found during:** Task 1 (implementing find_symbol_by_id)
- **Issue:** Research document assumed `entity_ids()` and `get_node()` are publicly available on Magellan's CodeGraph, but verification showed these are only on internal backend (not exposed)
- **Fix:** Changed approach from using Magellan API methods to direct SQL queries (same pattern as count_call_nodes from 23-01)
- **Files modified:** src/graph/magellan_integration.rs (find_symbol_by_id implementation)
- **Verification:** cargo check passes, implementation uses SQL to query graph_entities table
- **Committed in:** e4ae113 (Task 1 commit)

**Impact:** The deviation required adjusting the implementation approach but didn't change the overall functionality. The direct SQL approach is consistent with the pattern established in 23-01 for Call node counting.

---

**Total deviations:** 1 discovery (API limitation)
**Impact on plan:** Implementation adapted to work with actual Magellan API. No scope creep, same outcome delivered.

## Issues Encountered

### Magellan API Gaps

The research document incorrectly assumed `entity_ids()` and `get_node()` are publicly available. Verification showed:

- Magellan's `CodeGraph` struct has private backend modules (files, symbols, references, calls, chunks)
- `entity_ids()` and `get_node()` exist only on the internal `sqlitegraph::GraphBackend` trait
- Magellan wraps this internally and doesn't re-export the iteration methods

**Resolution:** Used direct SQL queries to the `graph_entities` table, which has stable schema in sqlitegraph. This is the same workaround used for Call node counting in 23-01.

## User Setup Required

None - no external service configuration required.

## Next Phase Readiness

- Symbol lookup methods complete and ready for CLI integration (23-04: Call Graph Traversal)
- Direct SQL pattern established for entity iteration when Magellan API is insufficient
- Performance concerns noted: O(N) scans may need optimization for large codebases (consider symbol_id index in future phase)
- Integration tests planned for 23-05

---
*Phase: 23-magellan-integration-extensions*
*Completed: 2026-01-24*
