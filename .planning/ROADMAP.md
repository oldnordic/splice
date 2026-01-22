# Roadmap: Splice Refactoring Tool

## Overview

Splice is a span-safe refactoring tool for 7 programming languages (Rust, Python, C, C++, Java, JavaScript, TypeScript) using tree-sitter for AST parsing and SQLiteGraph for code relationship storage. The v2.0 overhaul established production safety and structured output. The v2.2 milestone implements the Unified JSON Schema with rich span extensions optimized for LLM consumption and human usability improvements.

## Milestones

- ✅ **v2.0 Overhaul** - Phases 1-10 (shipped 2026-01-18)
- 🚧 **v2.2 Unified JSON & LLM Optimization** - Phases 11-17 (in progress)

## Phases

<details>
<summary>✅ v2.0 Overhaul (Phases 1-10) - SHIPPED 2026-01-18</summary>

**Milestone Goal:** Comprehensive overhaul for production safety, SQLiteGraph v1.0 upgrade, and structured output

See `.planning/milestones/v2.0-ROADMAP.md` for complete details of phases 1-10.

**Key Features Delivered:**
- Safety Foundation: Eliminated all unwrap() calls in production paths
- SQLiteGraph v1.0: Migrated to Native V2 backend
- Structured Output: Explicit field schema with stable identifiers
- Span-Aware Metadata: Line/column coordinates in all output
- Deterministic Ordering: Sorted output across all operations
- Validation Hooks: Pre/post verification with checksums
- Execution Logging: Complete audit trail with query capabilities
- Integration Testing: 75+ new tests for 7 languages
- Documentation: Full v2.0 documentation (README, manual, API)

</details>

### 🚧 v2.2 Unified JSON & LLM Optimization (In Progress)

**Milestone Goal:** Implement the Unified JSON Schema across all LLM tools with rich span extensions optimized for AI agent consumption and human-friendly CLI improvements.

#### Phase 11: Rich Span Core

**Goal**: Spans include rich metadata (context, semantic kind, language, checksums, error codes) for LLM consumption

**Depends on**: Phase 10 (v2.0 Documentation)
**Requirements**: RICHSPAN-01 through RICHSPAN-13

**Success Criteria** (what must be TRUE):
1. User receives span output with `context` field containing `before`, `selected`, `after` arrays (default 3 lines, configurable via `--context-lines`)
2. User receives span output with `semantic_kind` field (function, variable, parameter, etc.) and `language` field detected from file extension
3. User receives span output with `checksum_before` and `file_checksum_before` fields for race condition protection
4. User receives span output with `error_code` field including severity (error/warning/note), precise location (file:line:column), and "what to do" hint
5. All rich span fields use UTF-8 byte offsets consistent with existing span coordinates

**Plans**: TBD (refined during planning)

Plans:
- [ ] 11-01: Extend SpanResult structure with context, semantic_kind, language, checksums, and error_code fields
- [ ] 11-02: Implement context extraction module using ropey for efficient line calculations
- [ ] 11-03: Implement semantic kind detection mapping tree-sitter node types to standardized kinds
- [ ] 11-04: Add language detection from file extension/tree-sitter parser
- [ ] 11-05: Expose checksum_before and file_checksum_before in JSON output (reuse existing SHA-256 implementation)
- [ ] 11-06: Implement error code field with SPL-E### format, severity level, location, and hints
- [ ] 11-07: Ensure all new fields are optional with `#[serde(skip_serializing_if = "Option::is_none")]` for backward compatibility

#### Phase 12: Rich Span Advanced

**Goal**: Spans include relationships, tool hints, and suggested actions for advanced LLM workflows

**Depends on**: Phase 11
**Requirements**: RICHSPAN-14 through RICHSPAN-21

**Success Criteria** (what must be TRUE):
1. User receives span output with `relationships` object containing callers, callees, imports, exports (when `--relationships` flag used)
2. User receives span output with `tool_hints` object containing requires_full_context, apply_atomically, may_break_tests, requires_compilation
3. User receives span output with `suggested_action` object containing action_type (delete, replace, expand) and params
4. Relationship queries are lazy (only executed with `--relationships` flag) and span full codebase via codegraph.db
5. All advanced fields are optional and don't impact performance when not requested

**Plans**: TBD

Plans:
- [ ] 12-01: Implement relationship builder traversing CodeGraph for callers, callees, imports, exports
- [ ] 12-02: Add lazy evaluation for relationships (only via `--relationships` flag)
- [ ] 12-03: Implement tool hints module with behavioral flags (requires_full_context, apply_atomically, etc.)
- [ ] 12-04: Implement suggested action engine with 3 primitives (delete, replace, expand)
- [ ] 12-05: Integrate all advanced fields into SpanResult with optional serialization
- [ ] 12-06: Performance test relationship queries on large codebases (>10K LOC)

#### Phase 13: Dry-run & Diff

**Goal**: Users can preview exact changes before applying them using standard CLI conventions

**Depends on**: Phase 12
**Requirements**: CLI-01 through CLI-07

**Success Criteria** (what must be TRUE):
1. User can run `splice --dry-run` or `splice -n` to preview changes without applying them
2. Dry-run output shows unified diff format with `---`/`+++` headers, file counts, and `-`/`+` notation
3. Diff output uses colors (red for deletions, green for additions) when TTY detected
4. User can control context lines with `--unified <n>` flag (default 3 lines)
5. Dry-run returns exit code 1 if changes would be made, 0 if no changes
6. Color output respects `NO_COLOR` environment variable

**Plans**: TBD

Plans:
- [ ] 13-01: Add `--dry-run` and `-n` flag aliases to existing preview functionality
- [ ] 13-02: Implement unified diff format generation with headers and +/- notation
- [ ] 13-03: Add color support with TTY detection and NO_COLOR respect
- [ ] 13-04: Implement `--unified <n>` flag for configurable context lines
- [ ] 13-05: Set proper exit codes (1 if changes pending, 0 if no changes)

#### Phase 14: Context Flags

**Goal**: Users can see surrounding code context around matches using standard Unix conventions

**Depends on**: Phase 13
**Requirements**: CLI-08 through CLI-14

**Success Criteria** (what must be TRUE):
1. User can use `-A <lines>` flag to show lines after a match
2. User can use `-B <lines>` flag to show lines before a match
3. User can use `-C <lines>` flag to show context on both sides (defaults to 3 lines, git diff convention)
4. Context appears in both human-readable output and JSON output (context_before/context_after keys)
5. Context flags work across patch, delete, query, and get commands
6. Context respects expanded symbol boundaries when used with `--expand` flag

**Plans**: TBD

Plans:
- [ ] 14-01: Implement `-A`, `-B`, `-C` flag parsing with defaults
- [ ] 14-02: Add context extraction logic for all output formats
- [ ] 14-03: Integrate context flags into patch, delete, query, and get commands
- [ ] 14-04: Ensure context respects symbol expansion boundaries
- [ ] 14-05: Test context extraction on large files (>32KB) for performance

#### Phase 15: Enhanced Errors

**Goal**: Users receive actionable, structured error messages with clear guidance on how to fix issues

**Depends on**: Phase 14
**Requirements**: CLI-15 through CLI-21

**Success Criteria** (what must be TRUE):
1. Every error includes severity level (error/warning/note)
2. Every error includes precise location (file:line:column)
3. Every error includes stable error code (SPL-E001 format)
4. Every error includes "what to do" hint or suggestion
5. SymbolNotFound errors show similar symbol suggestions using Levenshtein distance
6. Compiler errors are parsed to extract native error codes (Rust E0XXX, TypeScript TSXXXX)
7. User can run `splice explain <code>` to get detailed error documentation

**Plans**: TBD

Plans:
- [ ] 15-01: Define SPL-E001+ error code taxonomy and mapping (reuse from Phase 11)
- [ ] 15-02: Enhance all error types with severity, location, and hint fields
- [ ] 15-03: Implement Levenshtein distance suggestions for SymbolNotFound
- [ ] 15-04: Add compiler error code extraction (Rust, TypeScript)
- [ ] 15-05: Implement `splice explain` command with error documentation
- [ ] 15-06: Update all error sites to include hints and structured fields

#### Phase 16: Symbol Expansion & Search

**Goal**: Users can retrieve full symbol bodies and search code patterns with atomic apply workflow

**Depends on**: Phase 15
**Requirements**: CLI-22 through CLI-33

**Success Criteria** (what must be TRUE):
1. User can use `--expand` flag to get full symbol body instead of just name
2. Expansion uses AST-aware tree-sitter parent chain walking for accuracy
3. Multiple expansions work progressively: name → full body → containing block
4. Expansion includes leading doc comments and documentation
5. User can specify exact expansion level with `--expand-level <N>` flag
6. Expansion works consistently across all 7 supported languages
7. User can run `splice search --pattern <text>` to find code patterns
8. User can use `splice search --apply` for atomic find-and-replace with rollback on failure
9. User can filter search results with `--glob` flag for file patterns
10. Search results available in JSON format for LLM consumption

**Plans**: TBD

Plans:
- [ ] 16-01: Implement AST-aware parent chain walking for symbol expansion
- [ ] 16-02: Add `--expand` and `--expand-level <N>` flags to get/query commands
- [ ] 16-03: Implement progressive expansion (name → body → containing block)
- [ ] 16-04: Include doc comments in expanded output
- [ ] 16-05: Test expansion across all 7 languages for consistency
- [ ] 16-06: Implement `splice search --pattern <text>` command
- [ ] 16-07: Add file path, line number, and context to search output
- [ ] 16-08: Implement `--glob` flag for file pattern filtering
- [ ] 16-09: Add `--apply` flag for atomic find-and-replace with rollback
- [ ] 16-10: Add JSON output format for search results

#### Phase 17: Integration & Testing

**Goal**: All v2.2 features work correctly across 7 languages with comprehensive test coverage

**Depends on**: Phase 16
**Requirements**: TEST-01 through TEST-06

**Success Criteria** (what must be TRUE):
1. All 334+ existing tests pass with new JSON schema (no breaking changes)
2. New tests verify rich span extensions (context, semantic kind, checksums, error codes) across all 7 languages
3. Performance tests confirm context extraction works efficiently on large files (>32KB)
4. Performance tests confirm relationship queries scale on large codebases (>10K LOC)
5. Cross-tool alignment tests verify Magellan format compatibility
6. LLM consumption tests verify JSON fields are properly structured for agent use

**Plans**: TBD

Plans:
- [ ] 17-01: Run all 334+ existing tests and update golden files for new JSON schema
- [ ] 17-02: Add integration tests for rich span extensions across all 7 languages
- [ ] 17-03: Add performance tests for context extraction on large files (>32KB)
- [ ] 17-04: Add performance tests for relationship queries on large codebases (>10K LOC)
- [ ] 17-05: Add cross-tool alignment tests with Magellan format compatibility
- [ ] 17-06: Add LLM consumption tests verifying JSON fields are properly used by agents

## Progress

**Execution Order:**
Phases execute in numeric order: 11 → 12 → 13 → 14 → 15 → 16 → 17

| Phase | Milestone | Plans Complete | Status | Completed |
|-------|-----------|----------------|--------|-----------|
| 1. Safety Foundation | v2.0 | 3/3 | Complete | 2026-01-17 |
| 2. SQLiteGraph v1.0 | v2.0 | 4/4 | Complete | 2026-01-17 |
| 3. Structured Output | v2.0 | 3/3 | Complete | 2026-01-17 |
| 4. Stable Identifiers | v2.0 | 3/3 | Complete | 2026-01-17 |
| 5. Span-Aware Metadata | v2.0 | 3/3 | Complete | 2026-01-17 |
| 6. Deterministic Ordering | v2.0 | 3/3 | Complete | 2026-01-17 |
| 7. Validation Hooks | v2.0 | 3/3 | Complete | 2026-01-17 |
| 8. Execution Logging | v2.0 | 3/3 | Complete | 2026-01-17 |
| 9. Integration Testing | v2.0 | 3/3 | Complete | 2026-01-18 |
| 10. Documentation Update | v2.0 | 3/3 | Complete | 2026-01-18 |
| 11. Rich Span Core | v2.2 | 0/7 | Not started | - |
| 12. Rich Span Advanced | v2.2 | 0/6 | Not started | - |
| 13. Dry-run & Diff | v2.2 | 0/5 | Not started | - |
| 14. Context Flags | v2.2 | 0/5 | Not started | - |
| 15. Enhanced Errors | v2.2 | 0/6 | Not started | - |
| 16. Symbol Expansion & Search | v2.2 | 0/10 | Not started | - |
| 17. Integration & Testing | v2.2 | 0/6 | Not started | - |

**Milestone Progress:**
- v2.0: 31/31 plans complete (100%) ✅
- v2.2: 0/45 plans planned (0%) 🚧
