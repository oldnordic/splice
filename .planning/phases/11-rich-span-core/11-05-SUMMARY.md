---
phase: 11-rich-span-core
plan: 05
type: execution
wave: 3
completed: 2026-01-22
duration: 3 minutes

title: Checksum Integration in SpanResult
summary: Exposed checksum_before and file_checksum_before in JSON output using existing SHA-256 implementation with builder methods and integration tests
tags: [checksums, sha256, json-output, spanresult, race-condition-protection]
tech-stack: []
tech-stack: []

# Dependency Graph
requires:
  - 11-01 (SpanResult with checksum fields already defined)
provides:
  - Checksum integration documentation
  - Builder methods for checksum population
  - Integration tests for checksum behavior
affects:
  - Future operations that need race condition protection

# Key Files
created:
  - tests/checksum_integration_tests.rs (integration tests)
modified:
  - src/output.rs (added documentation example)

# Decisions Made

## Decision 1: Use Existing SHA-256 Implementation
**Why:** The checksum module from v2.0 already provides SHA-256 checksums via `checksum_file()` and `checksum_span()` functions. No new implementation needed.

**Alternative considered:** Creating new checksum functions specifically for SpanResult.

**Trade-off:** Reusing existing code is faster and ensures consistency, but required adding builder methods to populate the new fields.

## Decision 2: Independent Checksum Fields
**Why:** `checksum_before` and `file_checksum_before` are independent convenience fields, separate from `span_checksum_before`/`span_checksum_after` from earlier work. This allows for flexible usage patterns.

**Alternative considered:** Making checksum_before an alias that sets span_checksum_before.

**Trade-off:** Independent fields provide more flexibility but require users to choose which fields to populate. Documentation should clarify the relationship.

# Deviations from Plan

None - plan executed exactly as written. The checksum functions and fields already existed from v2.0, so this plan was primarily about documentation and integration testing.

# Metrics

**Tests Added:** 4 integration tests
- test_span_result_with_checksums: Verifies with_both_checksums builder
- test_span_result_checksums_serialize_to_json: Confirms JSON serialization
- test_span_result_without_checksums_omits_fields: Tests skip_serializing_if
- test_checksum_fields_are_independent: Verifies field independence

**Tests Verified:** 13 existing checksum module tests all passing

**Commits:** 3
- docs(11-05): Add checksum integration example to output.rs
- test(11-05): Add checksum integration tests for SpanResult
- verify(11-05): Confirm all checksum module tests pass

# Next Phase Readiness

**Phase 11-06 (Language detection):** Ready to proceed. No blockers.

**Remaining Phase 11 plans:**
- 11-03: Semantic kind detection
- 11-04: Language detection (this plan)
- 11-06: Error code integration
- 11-07: Unified JSON output integration

**State:** Phase 11 progressing well. Checksums are now fully integrated and documented.
