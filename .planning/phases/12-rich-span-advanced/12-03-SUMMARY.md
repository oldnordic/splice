---
phase: 12-rich-span-advanced
plan: 03
subsystem: llm-guidance
tags: [action-types, confidence-assessment, serde, suggested-actions]

# Dependency graph
requires:
  - phase: 11-rich-span-core
    provides: ResolvedSpan with uniqueness and metadata for confidence calculation
provides:
  - ActionType enum (Delete, Replace, Expand)
  - Confidence enum (High, Medium, Low)
  - SuggestedAction struct with action_type, confidence, reason, params
  - Confidence::calculate() for uniqueness-based confidence scoring
  - suggest_action() for generating action recommendations
affects: [12-04, cli-integration, json-output-schema]

# Tech tracking
tech-stack:
  added: []
  patterns:
    - Confidence scoring based on resolution uniqueness and metadata
    - Human-readable reason generation for LLM interpretability
    - Optional action parameters with serde serialization

key-files:
  created:
    - src/action/mod.rs (292 lines)
  modified:
    - src/lib.rs (added action module export and re-exports)

key-decisions:
  - "Three-tier confidence: High (unique+file+kind), Medium (partial metadata), Low (ambiguous)"
  - "Action params optional: include metadata when relevant (levels, preserve_signature, remove_references)"
  - "Lowercase JSON serialization: delete/replace/expand and high/medium/low for LLM friendliness"

patterns-established:
  - "Pattern: Confidence::calculate() uses boolean flags for deterministic assessment"
  - "Pattern: suggest_action() generates human-readable reasons with symbol context"
  - "Pattern: serde(skip_serializing_if) omits None params from JSON output"

# Metrics
duration: 5min
completed: 2026-01-22
---

# Phase 12 Plan 3: Suggested Action Engine Summary

**Action type recommendations with confidence scoring based on symbol uniqueness and ambiguity detection**

## Performance

- **Duration:** 5 min
- **Started:** 2026-01-22T11:53:24Z
- **Completed:** 2026-01-22T11:58:37Z
- **Tasks:** 4 (combined into single implementation)
- **Files created:** 1
- **Tests added:** 9

## Accomplishments

- ActionType enum with Delete, Replace, Expand variants for LLM operation guidance
- Confidence enum with High, Medium, Low levels based on resolution quality
- SuggestedAction struct combining action type, confidence, human-readable reason, and optional params
- Confidence::calculate() function using uniqueness, file presence, kind availability, and ambiguity flags
- suggest_action() function generating contextual reasons with symbol name, kind, and file path
- Full test coverage (9 tests) for enum serialization, confidence calculation, and action generation
- Module exported from lib.rs with public re-exports for crate root access

## Task Commits

All tasks completed in single atomic commit:

1. **Tasks 1-4: Action engine implementation** - `a20805a` (feat)

**Note:** Tasks 1-3 implemented in src/action/mod.rs, Task 4 (lib.rs export) already present from concurrent work

## Files Created/Modified

- `src/action/mod.rs` - Action type engine with ActionType, Confidence, SuggestedAction, and 9 tests
- `src/lib.rs` - Added `pub mod action;` and re-exports (already present from concurrent plan execution)

## Decisions Made

- **Three-tier confidence model**: High for unique symbols with complete metadata, Medium for partial metadata, Low for ambiguous/missing info
- **Action parameters optional**: Include contextual params (levels for expand, preserve_signature for replace, remove_references for delete)
- **Lowercase JSON serialization**: All enums use serde(rename_all = "lowercase") for LLM-friendly output
- **Reason generation**: Human-readable strings include symbol name, kind, and file path for context
- **None - followed plan as specified**: All implementations matched plan requirements exactly

## Deviations from Plan

None - plan executed exactly as written.

## Issues Encountered

None - smooth execution with all tests passing and cargo check successful.

**Note:** During verification, discovered compilation errors in src/relationships/mod.rs (from plan 12-01) attempting to access private `graph.symbol_cache` field. These errors were transient and resolved by the time verification completed, likely due to concurrent plan execution completing the relationships module implementation.

## User Setup Required

None - no external service configuration required.

## Next Phase Readiness

- Action engine complete and ready for integration into CLI JSON output
- Confidence calculation can be used with ResolvedSpan from resolve module
- SuggestedAction ready for inclusion in unified JSON schema (plan 12-04)
- All types serializable for LLM consumption

**Blockers/Concerns:** None identified

---
*Phase: 12-rich-span-advanced*
*Plan: 03*
*Completed: 2026-01-22*
