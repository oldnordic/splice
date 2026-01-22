---
phase: 12-rich-span-advanced
plan: 08
subsystem: cli-integration
tags: [tool-hints, suggested-action, json-output, serde, cli]

# Dependency graph
requires:
  - phase: 12-rich-span-advanced
    plans: [01, 02, 03, 04, 05]
    provides: Relationships, ToolHints, SuggestedAction types, SpanResult extensions, CLI --relationships flag
provides:
  - All CLI commands (Query, Get, Delete, Patch) populate tool_hints in JSON output
  - All CLI commands populate suggested_action in JSON output
  - ToolHintOperation enum extended with Query and Get variants
  - ActionType enum extended with Query and Read variants
affects: [12-06, future-phases-using-cli-json]

# Tech tracking
tech-stack:
  added: []
  patterns: [tool-hints-derivation, suggested-action-generation, json-output-enrichment]

key-files:
  created: []
  modified:
    - src/hints/mod.rs - Added Query and Get to ToolHintOperation enum
    - src/action/mod.rs - Added Query and Read to ActionType enum, updated generate_reason and generate_params
    - src/main.rs - Integrated tool_hints and suggested_action into execute_query, execute_get, execute_delete, execute_single_patch

key-decisions:
  - "Infer is_public from semantic kind (true for functions, types, traits, enums) - simple heuristic for LLM guidance"
  - "Map Magellan kind strings to SemanticKind for query results - pragmatic approach without tree-sitter re-parsing"
  - "Delete command confidence: High if no callers, Medium if callers exist - safety-aware confidence scoring"
  - "Patch/Replace confidence: High for uniquely resolved symbols - reflects successful symbol resolution"
  - "Query/Get confidence: High by default - these are read-only operations"

patterns-established:
  - "Pattern: Derive tool_hints using derive_tool_hints(semantic_kind, is_public, operation) before adding to span"
  - "Pattern: Generate suggested_action using SuggestedAction struct directly for custom reasons (Delete with caller awareness)"
  - "Pattern: Use Confidence::High for unique/read-only operations, Confidence::Medium for potentially unsafe operations"
  - "Pattern: Import all necessary types (SemanticKind, ToolHints, SuggestedAction, etc.) in JSON output blocks"

# Metrics
duration: 7m
completed: 2026-01-22
---

# Phase 12 Plan 08: Tool Hints and Suggested Action CLI Integration Summary

**Tool hints and suggested action metadata integrated into all four CLI commands (Query, Get, Delete, Patch) with appropriate operation types and confidence levels**

## Performance

- **Duration:** 7 minutes
- **Started:** 2026-01-22T12:07:15Z
- **Completed:** 2026-01-22T12:14:13Z
- **Tasks:** 3 completed
- **Files modified:** 3

## Accomplishments

- All CLI commands (Query, Get, Delete, Patch) now populate tool_hints with operation-specific flags
- All CLI commands populate suggested_action with appropriate action_type, confidence, and reason
- ToolHintOperation extended with Query and Get variants for read-only operations
- ActionType extended with Query and Read variants for action suggestions
- Delete command includes caller-aware safety information in suggested_action reason
- JSON output structure includes exact field names: tool_hints (requires_full_context, may_break_tests, requires_compilation, apply_atomically) and suggested_action (action_type, confidence, reason)

## Task Commits

Each task was committed atomically:

1. **Task 1: Wire up tool_hints and suggested_action for Query and Get commands** - `2b36bb6` (feat)
2. **Task 2: Wire up tool_hints and suggested_action for Delete command** - `c8b5f60` (feat)
3. **Task 3: Wire up tool_hints and suggested_action for Patch command** - `2d26dce` (feat)

**Plan metadata:** TBD (docs: complete plan)

## Files Created/Modified

- `src/hints/mod.rs` - Added Query and Get variants to ToolHintOperation enum
- `src/action/mod.rs` - Added Query and Read variants to ActionType enum, updated generate_reason and generate_params functions
- `src/main.rs` - Integrated tool_hints and suggested_action population into execute_query, execute_get, execute_delete, execute_single_patch JSON output paths

## Decisions Made

- **Infer is_public from semantic kind**: Functions, types, traits, and enums default to public for LLM guidance
- **Map Magellan kind strings to SemanticKind**: Pragmatic approach for query results without re-parsing with tree-sitter
- **Delete confidence varies by callers**: High confidence if no callers (safe to delete), Medium if callers exist (may break dependencies)
- **Patch/Replace confidence always High**: Unique symbol resolution from resolve_symbol() justifies high confidence
- **Query/Get confidence always High**: Read-only operations are inherently safe

## Deviations from Plan

None - plan executed exactly as written.

## Authentication Gates

None - no external authentication required for this plan.

## Issues Encountered

- **Type mismatch in execute_query**: `labels` field expected `Vec<String>` but got `&[String]` - fixed by using `.to_vec()`
- **Borrow of moved value in execute_query**: `query_result.count` accessed after move - fixed by storing count in variable before move
- **Missing SuggestedAction import**: Used `SuggestedAction` struct without importing - fixed by adding `use splice::action::SuggestedAction;`
- **Unused import warning**: Imported `suggest_action` but didn't use it (used struct directly instead) - fixed by removing unused import

## Next Phase Readiness

- Tool hints and suggested action infrastructure complete for all CLI commands
- Ready for integration testing with actual LLM agents consuming JSON output
- Plan 12-06 (Performance tests) can verify tool hints don't impact performance significantly
- All 220 tests passing with new metadata fields

---
*Phase: 12-rich-span-advanced*
*Plan: 08*
*Completed: 2026-01-22*
