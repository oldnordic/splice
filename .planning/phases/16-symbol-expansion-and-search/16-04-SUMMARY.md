---
phase: 16-symbol-expansion-and-search
plan: 04
subsystem: symbol-expansion
tags: [doc-comments, tree-sitter, symbol-expansion, rust, python, java, javascript, typescript]

# Dependency graph
requires:
  - phase: 16-symbol-expansion-and-search
    plan: 01
    provides: [SymbolExpander trait, parent chain walking, tree_walker module]
provides:
  - Doc comment extraction for all 7 supported languages
  - expand_to_body_with_docs() function including leading documentation
  - CLI integration for doc comment inclusion in Get and Query commands
affects: [16-05-search-command]

# Tech tracking
tech-stack:
  added: []
  patterns:
    - prev_sibling walking for documentation extraction
    - Language-specific doc comment style detection
    - Graceful fallback on expansion errors

key-files:
  created: []
  modified:
    - src/expand/tree_walker.rs - extract_leading_docs() function
    - src/expand/mod.rs - expand_to_body_with_docs() function
    - src/main.rs - execute_get/execute_query wired to expand_to_body_with_docs

key-decisions:
  - "Python docstrings wrapped in expression_statement nodes - special handling required"
  - "Only doc-style comments captured (///, /**, //!), not regular comments (//)"
  - "Single blank line allowed between docs and symbol for readability"
  - "Graceful degradation on expansion errors - fall back to original span"

patterns-established:
  - "Doc comment extraction pattern: walk prev_sibling chain checking node kinds"
  - "Multi-language doc style detection via text content inspection"
  - "User-facing expansion uses expand_to_body_with_docs (includes docs)"
  - "Internal expansion uses expand_to_body (excludes docs for performance)"

# Metrics
duration: 75min
completed: 2026-01-22
---

# Phase 16 Plan 04: Leading Doc Comments in Symbol Expansion Summary

**Doc comment extraction across 7 languages using prev_sibling walking, with expand_to_body_with_docs() API for user-facing expansion**

## Performance

- **Duration:** 75 min
- **Started:** 2026-01-22T20:21:28Z
- **Completed:** 2026-01-22T21:36:00Z (estimated)
- **Tasks:** 3
- **Files modified:** 3

## Accomplishments

- Leading doc comment extraction supporting all 7 languages (Rust, Python, C/C++, Java, JavaScript, TypeScript)
- `expand_to_body_with_docs()` function that includes documentation in expanded spans
- CLI integration in `execute_get` and `execute_query` for automatic doc inclusion with `--expand` flag
- Comprehensive test coverage (9 tests) covering all doc comment styles

## Task Commits

Each task was committed atomically:

1. **Task 1: Add extract_leading_docs function** - `b1e0e5d` (feat)
2. **Task 2: Add expand_to_body_with_docs and wire to CLI** - `d9d4cd6` (feat)
3. **Task 3: Fix Python docstring support** - `9248e56` (fix)

**Plan metadata:** TBD (docs: complete plan)

## Files Created/Modified

- `src/expand/tree_walker.rs` - Added `extract_leading_docs()` function that walks prev_sibling chain to find doc comments
- `src/expand/mod.rs` - Added `expand_to_body_with_docs()` function that expands symbol body and includes leading docs
- `src/main.rs` - Updated `execute_get` and `execute_query` to call `expand_to_body_with_docs()` when `--expand` flag is used

## Decisions Made

- **Python docstrings require special handling**: Python `"""` docstrings are wrapped in `expression_statement` nodes, not bare `string` nodes like other languages. Added `expression_statement` to the node kind check.
- **Only doc-style comments captured**: Regular comments (`//`, `#`) are excluded - only `///`, `/**`, `//!`, `/*!`, `"""` patterns are recognized as documentation.
- **Single blank line allowed**: Up to one blank line between doc comments and symbol definition is permitted for code readability.
- **Graceful degradation**: If expansion fails (language detection, parse error), the system falls back to the original span rather than failing the entire request.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Bug] Python docstring extraction not working**
- **Found during:** Task 3 (Doc comment extraction tests)
- **Issue:** Initial implementation only checked for `string` nodes, but Python docstrings are wrapped in `expression_statement` nodes
- **Fix:** Added `expression_statement` to `is_string` check in `extract_leading_docs()`
- **Files modified:** src/expand/tree_walker.rs
- **Verification:** All 9 doc extraction tests now pass, including Python docstring test
- **Committed in:** 9248e56

**2. [Rule 3 - Blocking] Test assertions using wrong byte offsets**
- **Found during:** Task 3 (Running tests)
- **Issue:** Tests used `descendant_for_byte_range()` which finds child nodes, not the symbol definition node itself
- **Fix:** Updated tests to use tree walking cursor to find the actual `function_item`/`function_definition` nodes
- **Files modified:** src/expand/tree_walker.rs (test code)
- **Verification:** Rust block comment test and Python docstring test now pass
- **Committed in:** 9248e56

---

**Total deviations:** 2 auto-fixed (1 bug, 1 blocking)
**Impact on plan:** Both auto-fixes necessary for correctness. Python docstrings would not have been extracted without the fix. No scope creep.

## Issues Encountered

- **Python AST structure different than expected**: Python docstrings are wrapped in `expression_statement` nodes, unlike other languages where doc comments are direct siblings. Required updating the extraction logic to handle this case.
- **Test node selection issues**: Initial tests used `descendant_for_byte_range()` which finds child nodes rather than the symbol definition nodes. Fixed by using tree cursor to walk and find the correct node type.

## User Setup Required

None - no external service configuration required.

## Next Phase Readiness

- Doc comment extraction complete and tested
- `expand_to_body_with_docs()` API ready for use in search command
- All 7 languages support doc comment extraction
- Tests verify correct behavior for all doc comment styles

**Blockers:** None

**Concerns:**
- Pre-existing test failure in `test_cli_patch_preview` (from Phase 13) - unrelated to this plan, known issue documented in STATE.md

---
*Phase: 16-symbol-expansion-and-search*
*Plan: 04*
*Completed: 2026-01-22*
