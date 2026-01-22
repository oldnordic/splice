---
phase: 11-rich-span-core
plan: 01
subsystem: output-schema
tags: [serde, rust, json, span-result, builder-pattern, additive-schema]

# Dependency graph
requires:
  - phase: 10-v2.0-documentation
    provides: SpanResult struct with basic fields (byte_start, byte_end, line_start, line_end, etc.)
provides:
  - Extended SpanResult with 6 new optional fields (context, semantic_kind, language, checksum_before, file_checksum_before, error_code)
  - SpanContext struct for before/selected/after line extraction
  - ErrorCode struct for structured error diagnostics
  - Builder methods for fluent API (with_context, with_semantic_kind, with_language, etc.)
affects: [11-02, 11-03, 11-04, 11-05, 11-06, 11-07]

# Tech tracking
tech-stack:
  added: []
  patterns: [additive-schema-evolution, builder-pattern-for-optional-fields, serde-skip-serializing-backward-compat]

key-files:
  created: []
  modified:
    - src/output.rs - Extended SpanResult with rich span fields

key-decisions:
  - "All new fields are optional with #[serde(skip_serializing_if = \"Option::is_none\")] for zero-breaking-change guarantee"
  - "Builder methods provide fluent API for populating optional fields"
  - "No new dependencies required - uses existing serde and ropey infrastructure"

patterns-established:
  - "Pattern 1: Additive schema evolution - all new fields optional with skip_serializing_if"
  - "Pattern 2: Builder pattern for optional field population"
  - "Pattern 3: Convenience methods for related fields (with_semantic_info, with_both_checksums)"

# Metrics
duration: 5min
completed: 2026-01-22
---

# Phase 11: Rich Span Core - Plan 1 Summary

**Extended SpanResult with 6 new optional fields for LLM consumption (context, semantic_kind, language, checksums, error codes) using additive-only schema with zero breaking changes**

## Performance

- **Duration:** 5 min
- **Started:** 2026-01-22T08:56:08Z
- **Completed:** 2026-01-22T09:01:09Z
- **Tasks:** 4
- **Files modified:** 1

## Accomplishments

- **SpanContext struct** added with before/selected/after line vectors for context extraction
- **ErrorCode struct** added with code, severity, location, hint fields for structured diagnostics
- **6 new optional fields** added to SpanResult (context, semantic_kind, language, checksum_before, file_checksum_before, error_code)
- **8 builder methods** added for fluent API (with_context, with_semantic_kind, with_language, with_checksum_before, with_file_checksum_before, with_error_code, with_semantic_info, with_both_checksums)
- **All 311 existing tests pass** - zero breaking changes verified

## Task Commits

Each task was committed atomically:

1. **Task 1: Create SpanContext and ErrorCode types in output.rs** - `17c0ad7` (feat)
2. **Task 2: Add rich span fields to SpanResult struct** - `27000d0` (feat)
3. **Task 3: Update from_byte_span and add builder methods** - `e9f88c5` (feat)
4. **Task 4: Run existing tests to verify backward compatibility** - `6826d25` (test)

**Plan metadata:** (no separate metadata commit - all work in task commits)

## Files Created/Modified

- `src/output.rs` - Extended SpanResult struct with 6 new optional fields, added SpanContext and ErrorCode types, added 8 builder methods

## Decisions Made

- **All new fields are optional** - guarantees zero breaking changes to existing JSON output
- **#[serde(skip_serializing_if = "Option::is_none")]** on all new fields - ensures backward compatibility with old JSON
- **Builder pattern for optional fields** - provides fluent API while keeping fields optional
- **Convenience methods for related fields** - `with_semantic_info()` combines kind+language, `with_both_checksums()` combines span+file checksums
- **No new dependencies** - uses existing serde and ropey infrastructure

## Deviations from Plan

None - plan executed exactly as written.

## Issues Encountered

- **Integration test failures in integration_refactor.rs** (3 tests failing) - These tests require a Magellan database (graph.db) which doesn't exist. Verified these tests were already failing before our changes by stashing and re-running. Not related to our schema changes.

## User Setup Required

None - no external service configuration required.

## Next Phase Readiness

**Ready for Phase 11-02 (Context Extraction):**
- SpanContext struct defined and ready for population
- Builder methods in place for attaching context to spans
- Zero breaking changes means existing functionality unaffected

**Ready for Phase 11-03 (Semantic Kind Detection):**
- semantic_kind and language fields added to SpanResult
- with_semantic_kind(), with_language(), and with_semantic_info() builder methods available
- Additive schema allows gradual rollout of semantic kind detection

**Ready for Phase 11-04 (Checksum Integration):**
- checksum_before and file_checksum_before fields added to SpanResult
- with_checksum_before(), with_file_checksum_before(), and with_both_checksums() builder methods available
- Existing checksum infrastructure in checksum.rs can now populate these fields

**Ready for Phase 11-05 (Error Codes):**
- ErrorCode struct defined with code, severity, location, hint fields
- error_code field added to SpanResult
- with_error_code() builder method available
- Error code registry can be built in next plan

**Blockers/Concerns:**
- None - all foundational types in place for subsequent plans

---
*Phase: 11-rich-span-core*
*Completed: 2026-01-22*
