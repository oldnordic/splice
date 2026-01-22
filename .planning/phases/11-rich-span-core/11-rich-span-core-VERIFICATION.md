---
phase: 11-rich-span-core
verified: 2026-01-22T09:42:59Z
status: gaps_found
score: 2/5 must-haves verified
gaps:
  - truth: "User receives span output with context field containing before, selected, after arrays (default 3 lines, configurable via --context-lines)"
    status: partial
    reason: "Context infrastructure exists (SpanContext struct, extract_context function, tests pass) but is NOT integrated into CLI output. No --context-lines flag exists. The fields are defined but never populated in actual CLI commands."
    artifacts:
      - path: "src/context.rs"
        issue: "Module exists and is well-tested (8 tests), but extract_context() is never called from CLI operations"
      - path: "src/output.rs"
        issue: "SpanContext struct defined and builder method with_context() exists, but never called in production code"
      - path: "src/main.rs"
        issue: "CLI commands (delete, patch, query, get) do NOT populate context field in SpanResult JSON output"
      - path: "src/cli/mod.rs"
        issue: "Missing --context-lines CLI argument for all commands (delete, patch, query, get)"
    missing:
      - "CLI integration: Call extract_context() in JSON output paths for delete, patch, query, get commands"
      - "CLI flag: Add --context-lines <n> argument to relevant commands (default 3 lines)"
      - "Population logic: Wire context extraction into SpanResult creation in main.rs JSON output sections"
  - truth: "User receives span output with semantic_kind field (function, variable, parameter, etc.) and language field detected from file extension"
    status: partial
    reason: "Semantic kind infrastructure exists (SemanticKind enum with 10 variants, detect_semantic_kind function, tests pass for all 7 languages) but is NOT integrated into CLI output. The fields are defined but never populated in actual CLI commands."
    artifacts:
      - path: "src/ingest/semantic_kind.rs"
        issue: "Module exists with comprehensive language mappings (497 lines, 9 tests), but detect_semantic_kind() is never called from CLI operations"
      - path: "src/ingest/detect.rs"
        issue: "detect_language() function exists and is exported, but language field is never populated in CLI JSON output"
      - path: "src/main.rs"
        issue: "CLI commands do NOT call with_semantic_kind() or with_language() builders in JSON output paths"
      - path: "src/output.rs"
        issue: "Builder methods exist (with_semantic_kind, with_language, with_semantic_info) but are never called in production code"
    missing:
      - "CLI integration: Call detect_semantic_kind() and detect_language() in JSON output paths"
      - "Population logic: Wire semantic info into SpanResult creation using with_semantic_info() builder"
      - "Integration point: Extract node type from tree-sitter parse results during symbol extraction"
  - truth: "User receives span output with checksum_before and file_checksum_before fields for race condition protection"
    status: failed
    reason: "Checksum infrastructure exists (SHA-256 implementation verified in checksum.rs) but rich span checksum fields (checksum_before, file_checksum_before) are NOT populated in CLI output. Only legacy fields (span_checksum_before, span_checksum_after) are used."
    artifacts:
      - path: "src/checksum.rs"
        issue: "checksum_file() and checksum_span() functions exist and are tested, but rich span checksum fields are not populated"
      - path: "src/output.rs"
        issue: "Builder methods with_checksum_before(), with_file_checksum_before(), with_both_checksums() exist but are never called"
      - path: "src/main.rs"
        issue: "CLI JSON output only populates legacy span_checksum_before/span_checksum_after fields (line 769-770), not new checksum_before/file_checksum_before fields"
    missing:
      - "CLI integration: Call with_both_checksums() builder in JSON output paths for delete, patch, apply-files commands"
      - "Field mapping: Either migrate to new checksum_before field or document relationship with legacy span_checksum_before field"
      - "Checksum calculation: Ensure file_checksum_before is computed and populated for all span operations"
  - truth: "User receives span output with error_code field including severity (error/warning/note), precise location (file:line:column), and what to do hint"
    status: failed
    reason: "Error code infrastructure exists (ErrorCode struct, SpliceErrorCode enum with 26 variants, comprehensive hints) but is NOT integrated into CLI output. Error codes are never attached to SpanResult in CLI operations."
    artifacts:
      - path: "src/error_codes.rs"
        issue: "Module exists with 26 error code variants and ErrorCode construction, but from_splice_error() is never called in CLI"
      - path: "src/error.rs"
        issue: "SpliceError enum exists but errors are returned directly, not converted to ErrorCode for JSON output"
      - path: "src/main.rs"
        issue: "Error handling returns early with ? operator, never converting SpliceError to ErrorCode for span output"
      - path: "src/output.rs"
        issue: "error_code field exists on SpanResult with with_error_code() builder, but never called in production code"
    missing:
      - "Error conversion: Convert SpliceError to ErrorCode in error handling paths"
      - "Span attachment: Attach ErrorCode to SpanResult when errors occur during span operations"
      - "Integration: Wire ErrorCode::from_splice_code() into CLI error handling for JSON output"
  - truth: "All rich span fields use UTF-8 byte offsets consistent with existing span coordinates"
    status: verified
    reason: "All rich span infrastructure correctly uses UTF-8 byte offsets. Context extraction verified with multi-byte UTF-8 tests (emoji), ropey ensures byte-aware operations, no character offsets used."
    artifacts:
      - path: "src/context.rs"
        status: "SUBSTANTIVE - 255 lines, 8 tests, UTF-8 verified with emoji test"
      - path: "src/output.rs"
        status: "SUBSTANTIVE - 718 lines, all fields use byte_start/byte_end (no character offsets)"
    evidence:
      - "test_extract_context_utf8_multibyte passes (line 193-211 in context.rs)"
      - "ropey.byte_to_line() used for byte-to-line conversion (line 92-93 in context.rs)"
      - "All span coordinates use byte offsets (byte_start, byte_end) not character offsets"

# Phase 11: Rich Span Core Verification Report

**Phase Goal:** Spans include rich metadata (context, semantic kind, language, checksums, error codes) for LLM consumption
**Verified:** 2026-01-22T09:42:59Z
**Status:** gaps_found
**Re-verification:** No - initial verification

## Goal Achievement

### Observable Truths

| # | Truth | Status | Evidence |
|---|-------|--------|----------|
| 1 | User receives span output with `context` field containing `before`, `selected`, `after` arrays (default 3 lines, configurable via `--context-lines`) | ⚠️ PARTIAL | Infrastructure exists (`src/context.rs` with 8 passing tests, `SpanContext` struct, `extract_context()` function) but NOT integrated into CLI. No `--context-lines` flag exists. Builder method `with_context()` never called in production code. |
| 2 | User receives span output with `semantic_kind` field (function, variable, parameter, etc.) and `language` field detected from file extension | ⚠️ PARTIAL | Infrastructure exists (`src/ingest/semantic_kind.rs` with 9 tests covering all 7 languages, `detect_semantic_kind()` function, `detect_language()` function) but NOT integrated into CLI output. Builder methods `with_semantic_kind()`, `with_language()` never called. |
| 3 | User receives span output with `checksum_before` and `file_checksum_before` fields for race condition protection | ✗ FAILED | Infrastructure exists (SHA-256 in `src/checksum.rs`, builder methods `with_checksum_before()`, `with_file_checksum_before()`, `with_both_checksums()`) but NOT populated in CLI. Only legacy fields `span_checksum_before`/`span_checksum_after` used. |
| 4 | User receives span output with `error_code` field including severity (error/warning/note), precise location (file:line:column), and "what to do" hint | ✗ FAILED | Infrastructure exists (`src/error_codes.rs` with 26 error variants, `ErrorCode` struct, `SpliceErrorCode` enum, comprehensive hints) but NOT integrated into CLI. Error codes never attached to `SpanResult`. |
| 5 | All rich span fields use UTF-8 byte offsets consistent with existing span coordinates | ✓ VERIFIED | All infrastructure correctly uses UTF-8 byte offsets. Context extraction verified with multi-byte UTF-8 test (emoji), ropey ensures byte-aware operations, no character offsets used. |

**Score:** 1/5 truths verified (20%), 2/5 partial (40%), 2/5 failed (40%)

### Required Artifacts

| Artifact | Expected | Status | Details |
|----------|----------|--------|---------|
| `src/context.rs` | Context extraction using ropey | ✓ SUBSTANTIVE | 255 lines, 8 tests passing, UTF-8 verified with emoji test. NOT called from CLI. |
| `src/ingest/semantic_kind.rs` | Semantic kind detection for 7 languages | ✓ SUBSTANTIVE | 497 lines, 9 tests covering all languages (Rust, Python, JS, TS, Java, C, C++). NOT called from CLI. |
| `src/error_codes.rs` | Error code registry with SPL-E### format | ✓ SUBSTANTIVE | 431+ lines, 26 error variants, 8 tests passing. NOT integrated into CLI output. |
| `src/output.rs` (SpanResult extensions) | 6 new optional fields with builder methods | ✓ SUBSTANTIVE | Lines 322-339 have all 6 fields (context, semantic_kind, language, checksum_before, file_checksum_before, error_code). Lines 407-455 have all 8 builder methods. All fields use `#[serde(skip_serializing_if = "Option::is_none")]`. |
| `src/cli/mod.rs` | --context-lines flag for CLI commands | ✗ MISSING | No `--context-lines` argument exists in any CLI command (delete, patch, query, get). |
| `src/main.rs` (CLI integration) | Population of rich span fields in JSON output | ✗ STUB | Rich span builder methods NEVER called in CLI JSON output paths (lines 432-470 for delete, 748-790 for patch). Only legacy fields populated. |

### Key Link Verification

| From | To | Via | Status | Details |
|------|-----|-----|--------|---------|
| `extract_context()` | CLI JSON output | Function call in main.rs | ✗ NOT_WIRED | Function exists in `src/context.rs` but never called from CLI operations. Context field always None in output. |
| `detect_semantic_kind()` | CLI JSON output | Function call in main.rs | ✗ NOT_WIRED | Function exists in `src/ingest/semantic_kind.rs` but never called from CLI. semantic_kind field always None. |
| `detect_language()` | CLI JSON output | Function call in main.rs | ✗ NOT_WIRED | Function exists in `src/ingest/detect.rs` but never called from CLI. language field always None. |
| `with_context()` | SpanResult creation | Builder method call | ✗ NOT_WIRED | Builder method exists (output.rs:408) but never called in production code (only tests). |
| `with_semantic_info()` | SpanResult creation | Builder method call | ✗ NOT_WIRED | Builder method exists (output.rs:444) but never called in production code (only tests). |
| `with_both_checksums()` | SpanResult creation | Builder method call | ✗ NOT_WIRED | Builder method exists (output.rs:451) but never called. Only legacy `with_span_checksums()` used. |
| `with_error_code()` | SpanResult creation | Builder method call | ✗ NOT_WIRED | Builder method exists (output.rs:438) but never called. Error codes never attached to spans. |
| CLI --context-lines flag | context_lines parameter | clap argument | ✗ NOT_WIRED | Flag does not exist in CLI definition (src/cli/mod.rs). |

### Requirements Coverage

| Requirement | Phase | Status | Blocking Issue |
|-------------|-------|--------|----------------|
| RICHSPAN-01: Spans include `context` field with `before`, `selected`, `after` arrays | 11 | ✗ BLOCKED | Context field exists but never populated in CLI output |
| RICHSPAN-02: Default context is 3 lines (configurable via `--context-lines`) | 11 | ✗ BLOCKED | No `--context-lines` CLI flag exists |
| RICHSPAN-03: Context uses UTF-8 byte offsets | 11 | ✓ SATISFIED | Verified with emoji test in context.rs |
| RICHSPAN-04: Spans include `semantic_kind` field | 11 | ✗ BLOCKED | Field exists but never populated in CLI output |
| RICHSPAN-05: Spans include `language` field | 11 | ✗ BLOCKED | Field exists but never populated in CLI output |
| RICHSPAN-06: Semantic kind mappings cover all 7 supported languages | 11 | ✓ SATISFIED | All 7 languages have mappings in semantic_kind.rs |
| RICHSPAN-07: Spans include `checksum_before` for race condition protection | 11 | ✗ BLOCKED | Field exists but never populated (only legacy span_checksum_before used) |
| RICHSPAN-08: Spans include `file_checksum_before` for file-level verification | 11 | ✗ BLOCKED | Field exists but never populated in CLI output |
| RICHSPAN-09: Checksums use SHA-256 consistent with existing v2.0 implementation | 11 | ✓ SATISFIED | checksum.rs uses SHA-256 (verified) |
| RICHSPAN-10: Spans include `error_code` field with `SPL-E001` format | 11 | ✗ BLOCKED | Field exists but never populated in CLI output |
| RICHSPAN-11: Error codes include severity level (error/warning/note) | 11 | ✓ SATISFIED | ErrorSeverity enum exists with all 3 levels |
| RICHSPAN-12: Error codes include precise location (file:line:column) | 11 | ✓ SATISFIED | ErrorCode::from_splice_code() formats location correctly |
| RICHSPAN-13: Error codes include "what to do" hint | 11 | ✓ SATISFIED | All 26 SpliceErrorCode variants have hint() method |

**Coverage:** 6/13 requirements satisfied (46%), 7/13 blocked (54%)

### Anti-Patterns Found

| File | Line | Pattern | Severity | Impact |
|------|------|---------|----------|--------|
| src/main.rs | 432-470 | Rich span fields defined but never populated | 🛑 BLOCKER | Users do NOT receive rich metadata in JSON output despite infrastructure existing |
| src/main.rs | 748-790 | Rich span fields defined but never populated | 🛑 BLOCKER | CLI commands return spans with None for all new fields |
| src/cli/mod.rs | (missing) | Missing --context-lines flag | 🛑 BLOCKER | Users cannot configure context lines, goal criterion #1 unmet |
| src/main.rs | 769-770 | Using legacy checksum fields instead of new rich span fields | ⚠️ WARNING | Inconsistent field naming, checksum_before/file_checksum_before never populated |

### Human Verification Required

### 1. Rich Span JSON Output Test

**Test:** Run `cargo run -- delete --file src/test.rs --symbol test_function --json` on a real file
**Expected:** JSON output should contain:
```json
{
  "spans": [{
    "context": {
      "before": ["line 1", "line 2", "line 3"],
      "selected": ["line 4"],
      "after": ["line 5", "line 6", "line 7"]
    },
    "semantic_kind": "function",
    "language": "rust",
    "checksum_before": "<sha256>",
    "file_checksum_before": "<sha256>",
    "error_code": null
  }]
}
```
**Why human:** Cannot verify actual JSON output content programmatically without running real CLI commands. Need to confirm fields are populated in practice, not just defined.

### 2. --context-lines Flag Availability

**Test:** Run `cargo run -- delete --help` and check for `--context-lines` flag
**Expected:** Flag should exist with description like "Number of context lines before/after spans (default: 3)"
**Why human:** CLI help text inspection requires human verification. Automated grep found no trace of the flag.

### 3. Rich Span Integration in All Commands

**Test:** Test rich span fields appear in JSON output for all commands: delete, patch, query, get
**Expected:** All commands populate context, semantic_kind, language fields when --json flag is used
**Why human:** Requires running multiple CLI commands and inspecting JSON output manually.

### Gaps Summary

**Root Cause:** Phase 11 built comprehensive infrastructure for rich span metadata (types, functions, tests) but FAILED to integrate this infrastructure into the actual CLI output paths. The pattern is consistent across all 5 rich span features:

1. **Infrastructure exists and is well-tested** (context.rs, semantic_kind.rs, error_codes.rs)
2. **Builder methods exist** (with_context, with_semantic_info, with_both_checksums, with_error_code)
3. **CLI integration is MISSING** (no calls to builder methods in main.rs JSON output sections)
4. **CLI flags are MISSING** (no --context-lines flag)

**Impact:** Users do NOT receive rich span metadata in JSON output. The goal "Spans include rich metadata for LLM consumption" is NOT achieved despite infrastructure being complete.

**What Works:**
- UTF-8 byte offset handling (verified with emoji test)
- SHA-256 checksums (existing implementation reused)
- Semantic kind mappings (all 7 languages covered)
- Error code registry (26 variants with hints)
- Test coverage (340 tests passing, including 13 rich span integration tests)
- Backward compatibility (old JSON parses correctly)

**What's Missing:**
- CLI integration: Rich span builder methods never called in production code
- CLI flags: --context-lines flag does not exist
- Population logic: JSON output paths in main.rs don't populate new fields
- Error attachment: SpliceError not converted to ErrorCode for span output

**Estimated Effort to Close Gaps:** 2-3 hours
- Add --context-lines flag to CLI commands (30 min)
- Integrate context extraction in JSON output paths (45 min)
- Integrate semantic kind detection in JSON output paths (45 min)
- Integrate checksum_before/file_checksum_before in JSON output paths (30 min)
- Integrate error codes in error handling paths (30 min)

---

_Verified: 2026-01-22T09:42:59Z_
_Verifier: Claude (gsd-verifier)_
