# Requirements: Splice v2.1 - LLM & Human Usability

**Defined:** 2026-01-22
**Core Value:** Span-safe refactoring with validation
**Milestone:** v2.1 - LLM & Human Usability

## v2.1 Requirements

Requirements for improving LLM and human usability. Each maps to roadmap phases.

### Dry-run & Diff Output

- [ ] **DRYRUN-01:** Tool supports `-n, --dry-run` flag as alias to existing `--preview`
- [ ] **DRYRUN-02:** Dry-run outputs unified diff format with `---`/`+++`, `-`/`+` notation
- [ ] **DRYRUN-03:** Dry-run shows summary header with files affected and line counts
- [ ] **DRYRUN-04:** Tool uses red for deletions, green for additions (Git convention)
- [ ] **DRYRUN-05:** Tool respects `NO_COLOR` environment variable and auto-detects TTY
- [ ] **DRYRUN-06:** Tool supports `--unified <n>` flag for configurable context in diff
- [ ] **DRYRUN-07:** Dry-run returns exit code 1 if changes would be made, 0 if no changes

### Context Flags

- [ ] **CONTEXT-01:** Tool supports `-A <lines>` flag for lines after match
- [ ] **CONTEXT-02:** Tool supports `-B <lines>` flag for lines before match
- [ ] **CONTEXT-03:** Tool supports `-C <lines>` flag for context on both sides
- [ ] **CONTEXT-04:** Default context when `-C` specified is 3 lines (git diff convention)
- [ ] **CONTEXT-05:** Context lines included in JSON output under `context_before` and `context_after` keys
- [ ] **CONTEXT-06:** Context flags work with patch, delete, query, and get commands
- [ ] **CONTEXT-07:** Context respects expanded symbol boundaries when used with `--expand`

### Enhanced Error Messages

- [ ] **ERRORS-01:** Every error includes severity level (error/warning/note)
- [ ] **ERRORS-02:** Every error includes precise location (file:line:column)
- [ ] **ERRORS-03:** Every error includes stable error code (SPLICE_E001 format)
- [ ] **ERRORS-04:** Every error includes "what to do" hint or suggestion
- [ ] **ERRORS-05:** SymbolNotFound errors include Levenshtein distance suggestions
- [ ] **ERRORS-06:** Tool extracts error codes from compiler output (Rust E0XXX, TypeScript TSXXXX)
- [ ] **ERRORS-07:** Tool provides `splice explain <code>` command for detailed error documentation

### Symbol Expansion

- [ ] **EXPAND-01:** Tool supports `--expand` flag to get full symbol body
- [ ] **EXPAND-02:** Expansion uses AST-aware tree-sitter parent chain walking
- [ ] **EXPAND-03:** First expansion gets name, second gets full body, third gets containing block
- [ ] **EXPAND-04:** Expansion includes leading doc comments
- [ ] **EXPAND-05:** Tool supports `--expand-level <N>` flag for multi-level expansion
- [ ] **EXPAND-06:** Expansion works across all 7 supported languages

### Search & Patch Workflow

- [ ] **SEARCH-01:** Tool provides `splice search --pattern <text>` command
- [ ] **SEARCH-02:** Search shows matches with file paths, line numbers, and context
- [ ] **SEARCH-03:** Tool supports `splice search --apply` for atomic find-and-replace
- [ ] **SEARCH-04:** Tool supports `--glob` flag for file pattern filtering in search
- [ ] **SEARCH-05:** Search + apply workflow supports rollback on partial failure
- [ ] **SEARCH-06:** Search results available in JSON format for LLM consumption

## v2.2 Requirements

Deferred to future release. Tracked but not in current roadmap.

### Error Documentation

- **ERRORS-08:** Online hosted error documentation at splice.dev/errors/
- **ERRORS-09:** IDE/hyperlink integration for error codes

### Advanced Expansion

- **EXPAND-07:** Configurable expansion behavior (include/exclude trailing comments)
- **EXPAND-08:** Symbol-at-file-scope handling strategy

## Out of Scope

| Feature | Reason |
|---------|--------|
| Web UI or IDE integration | CLI-only tool, LLM-friendly CLI is priority |
| New programming languages | Current 7 languages are sufficient |
| Parallel batch processing | Existing single-threaded is fine for v2.1 |

## Traceability

Which phases cover which requirements. Updated during roadmap creation.

| Requirement | Phase | Status |
|-------------|-------|--------|
| DRYRUN-01 through DRYRUN-07 | Phase 11 | Pending |
| CONTEXT-01 through CONTEXT-07 | Phase 12 | Pending |
| ERRORS-01 through ERRORS-07 | Phase 13 | Pending |
| EXPAND-01 through EXPAND-06 | Phase 14 | Pending |
| SEARCH-01 through SEARCH-06 | Phase 15 | Pending |

**Coverage:**
- v2.1 requirements: 33 total
- Mapped to phases: Pending (roadmap next)
- Unmapped: 0 ✓

---
*Requirements defined: 2026-01-22*
*Last updated: 2026-01-22 after initial definition*
