# Requirements: Splice v2.2 - Unified JSON & LLM Optimization

**Defined:** 2026-01-22
**Core Value:** Span-safe refactoring with validation
**Milestone:** v2.2 - Unified JSON & LLM Optimization

## v2.2 Requirements

Requirements for Unified JSON Schema implementation with LLM-first UX improvements. Each maps to roadmap phases.

### Rich Span Extensions - Core

- [ ] **RICHSPAN-01:** Spans include `context` field with `before`, `selected`, `after` arrays
- [ ] **RICHSPAN-02:** Default context is 3 lines (configurable via `--context-lines`)
- [ ] **RICHSPAN-03:** Context uses UTF-8 byte offsets consistent with span coordinates
- [ ] **RICHSPAN-04:** Spans include `semantic_kind` field (function, variable, parameter, etc.)
- [ ] **RICHSPAN-05:** Spans include `language` field detected from file extension/tree-sitter
- [ ] **RICHSPAN-06:** Semantic kind mappings cover all 7 supported languages
- [ ] **RICHSPAN-07:** Spans include `checksum_before` for race condition protection
- [ ] **RICHSPAN-08:** Spans include `file_checksum_before` for file-level verification
- [ ] **RICHSPAN-09:** Checksums use SHA-256 consistent with existing v2.0 implementation
- [ ] **RICHSPAN-10:** Spans include `error_code` field with `SPL-E001` format for errors
- [ ] **RICHSPAN-11:** Error codes include severity level (error/warning/note)
- [ ] **RICHSPAN-12:** Error codes include precise location (file:line:column)
- [ ] **RICHSPAN-13:** Error codes include "what to do" hint or suggestion

### Rich Span Extensions - Advanced

- [ ] **RICHSPAN-14:** Spans include `relationships` object with `callers`, `callees`, `imports`, `exports`
- [ ] **RICHSPAN-15:** Relationships span full codebase (cross-file queries via codegraph.db)
- [ ] **RICHSPAN-16:** Relationship queries are lazy (only with `--relationships` flag)
- [ ] **RICHSPAN-17:** Spans include `tool_hints` object with `requires_full_context`, `apply_atomically`
- [ ] **RICHSPAN-18:** Tool hints include `may_break_tests`, `requires_compilation` flags
- [ ] **RICHSPAN-19:** Spans include `suggested_action` object with `action_type` and `params`
- [ ] **RICHSPAN-20:** Suggested actions support primitives: delete, replace, expand
- [ ] **RICHSPAN-21:** All rich span fields are optional (use `#[serde(skip_serializing_if = "Option::is_none")]`)

### CLI Improvements - Dry-run & Diff

- [ ] **CLI-01:** Tool supports `-n, --dry-run` flag as alias to existing `--preview`
- [ ] **CLI-02:** Dry-run outputs unified diff format with `---`/`+++`, `-`/`+` notation
- [ ] **CLI-03:** Dry-run shows summary header with files affected and line counts
- [ ] **CLI-04:** Tool uses red for deletions, green for additions (Git convention)
- [ ] **CLI-05:** Tool respects `NO_COLOR` environment variable and auto-detects TTY
- [ ] **CLI-06:** Tool supports `--unified <n>` flag for configurable context in diff
- [ ] **CLI-07:** Dry-run returns exit code 1 if changes would be made, 0 if no changes

### CLI Improvements - Context Flags

- [ ] **CLI-08:** Tool supports `-A <lines>` flag for lines after match
- [ ] **CLI-09:** Tool supports `-B <lines>` flag for lines before match
- [ ] **CLI-10:** Tool supports `-C <lines>` flag for context on both sides
- [ ] **CLI-11:** Default context when `-C` specified is 3 lines (git diff convention)
- [ ] **CLI-12:** Context lines included in JSON output under `context_before` and `context_after` keys
- [ ] **CLI-13:** Context flags work with patch, delete, query, and get commands
- [ ] **CLI-14:** Context respects expanded symbol boundaries when used with `--expand`

### CLI Improvements - Enhanced Errors

- [ ] **CLI-15:** Every error includes severity level (error/warning/note)
- [ ] **CLI-16:** Every error includes precise location (file:line:column)
- [ ] **CLI-17:** Every error includes stable error code (SPL-E001 format)
- [ ] **CLI-18:** Every error includes "what to do" hint or suggestion
- [ ] **CLI-19:** SymbolNotFound errors include Levenshtein distance suggestions
- [ ] **CLI-20:** Tool extracts error codes from compiler output (Rust E0XXX, TypeScript TSXXXX)
- [ ] **CLI-21:** Tool provides `splice explain <code>` command for detailed error documentation

### CLI Improvements - Symbol Expansion

- [ ] **CLI-22:** Tool supports `--expand` flag to get full symbol body
- [ ] **CLI-23:** Expansion uses AST-aware tree-sitter parent chain walking
- [ ] **CLI-24:** First expansion gets name, second gets full body, third gets containing block
- [ ] **CLI-25:** Expansion includes leading doc comments
- [ ] **CLI-26:** Tool supports `--expand-level <N>` flag for multi-level expansion
- [ ] **CLI-27:** Expansion works across all 7 supported languages

### CLI Improvements - Search & Patch

- [ ] **CLI-28:** Tool provides `splice search --pattern <text>` command
- [ ] **CLI-29:** Search shows matches with file paths, line numbers, and context
- [ ] **CLI-30:** Tool supports `splice search --apply` for atomic find-and-replace
- [ ] **CLI-31:** Tool supports `--glob` flag for file pattern filtering in search
- [ ] **CLI-32:** Search + apply workflow supports rollback on partial failure
- [ ] **CLI-33:** Search results available in JSON format for LLM consumption

### Integration & Testing

- [ ] **TEST-01:** All 334+ existing tests pass with new JSON schema
- [ ] **TEST-02:** New tests for rich span extensions across all 7 languages
- [ ] **TEST-03:** Performance tests for context extraction on large files (>32KB)
- [ ] **TEST-04:** Performance tests for relationship queries on large codebases (>10K LOC)
- [ ] **TEST-05:** Cross-tool alignment tests with Magellan format compatibility
- [ ] **TEST-06:** LLM consumption tests verify JSON fields are properly used by agents

## v2.3+ Requirements

Deferred to future release. Tracked but not in current roadmap.

### Error Documentation

- **ERRORS-08:** Online hosted error documentation at splice.dev/errors/
- **ERRORS-09:** IDE/hyperlink integration for error codes

### Advanced Expansion

- **EXPAND-07:** Configurable expansion behavior (include/exclude trailing comments)
- **EXPAND-08:** Symbol-at-file-scope handling strategy

### Relationship Optimizations

- **REL-01:** Relationship caching layer for faster repeated queries
- **REL-02:** Incremental relationship updates on file changes

## Out of Scope

| Feature | Reason |
|---------|--------|
| Web UI or IDE integration | CLI-only tool, LLM-friendly CLI is priority |
| New programming languages | Current 7 languages are sufficient |
| Parallel batch processing | Existing single-threaded is fine for v2.2 |
| Real-time relationship updates | Adds complexity, lazy evaluation sufficient |
| Macro expansion before semantic detection | Language-specific complexity, defer |

## Traceability

Which phases cover which requirements. All v2.2 requirements mapped to roadmap phases.

| Requirement | Phase | Status | Notes |
|-------------|-------|--------|-------|
| RICHSPAN-01 through RICHSPAN-13 | Phase 11 | In Progress | 7/11 plans executed, 4 gap closure plans created |
| RICHSPAN-14 through RICHSPAN-21 | Phase 12 | Pending | |
| CLI-01 through CLI-07 | Phase 13 | Pending | |
| CLI-08 through CLI-14 | Phase 14 | Pending | |
| CLI-15 through CLI-21 | Phase 15 | Pending | |
| CLI-22 through CLI-33 | Phase 16 | Pending | |
| TEST-01 through TEST-06 | Phase 17 | Pending | |

**Coverage:**
- v2.2 requirements: 45 total
- Mapped to phases: 45/45 (100%) ✓
- Unmapped: 0

**Phase Distribution:**
- Phase 11 (Rich Span Core): 13 requirements (RICHSPAN-01 to RICHSPAN-13)
  - Infrastructure complete (plans 11-01 through 11-07)
  - Gap closure plans created (11-08 through 11-11)
  - Status: 7/11 plans executed, 4 gap closure plans pending execution
- Phase 12 (Rich Span Advanced): 8 requirements (RICHSPAN-14 to RICHSPAN-21)
- Phase 13 (Dry-run & Diff): 7 requirements (CLI-01 to CLI-07)
- Phase 14 (Context Flags): 7 requirements (CLI-08 to CLI-14)
- Phase 15 (Enhanced Errors): 7 requirements (CLI-15 to CLI-21)
- Phase 16 (Symbol Expansion + Search): 12 requirements (CLI-22 to CLI-33)
- Phase 17 (Integration & Testing): 6 requirements (TEST-01 to TEST-06)

**Gap Status (Phase 11):**

| Requirement | Infrastructure | CLI Integration | Plan |
|-------------|---------------|-----------------|------|
| RICHSPAN-01: Context field | ✅ Complete | ❌ Not integrated | 11-08 |
| RICHSPAN-02: --context-lines flag | ❌ Missing | ❌ Missing | 11-08 |
| RICHSPAN-03: UTF-8 byte offsets | ✅ Complete | ✅ Verified | N/A |
| RICHSPAN-04: semantic_kind field | ✅ Complete | ❌ Not integrated | 11-09 |
| RICHSPAN-05: language field | ✅ Complete | ❌ Not integrated | 11-09 |
| RICHSPAN-06: All 7 languages mapped | ✅ Complete | ✅ Verified | N/A |
| RICHSPAN-07: checksum_before field | ✅ Complete | ❌ Not integrated | 11-10 |
| RICHSPAN-08: file_checksum_before field | ✅ Complete | ❌ Not integrated | 11-10 |
| RICHSPAN-09: SHA-256 checksums | ✅ Complete | ✅ Verified | N/A |
| RICHSPAN-10: error_code field | ✅ Complete | ❌ Not integrated | 11-11 |
| RICHSPAN-11: Severity levels | ✅ Complete | ✅ Verified | N/A |
| RICHSPAN-12: Precise location | ✅ Complete | ✅ Verified | N/A |
| RICHSPAN-13: What-to-do hints | ✅ Complete | ✅ Verified | N/A |

---
*Requirements defined: 2026-01-22*
*Traceability updated: 2026-01-22*
