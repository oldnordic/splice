# Project State

## Project Reference

See: .planning/PROJECT.md (updated 2026-01-23)

**Core value:** Span-safe refactoring with validation
**Current focus:** v2.2.1 Code Quality & Bug Fixes — Fixing 67 identified issues

## Current Position

Phase: 21 of 21 (API Consolidation)
Plan: 04 of 10
Status: In progress
Last activity: 2026-01-24 — Completed Plan 21-04: Execution Log Testability

Progress: [████████░] 90% (100/106 plans total: 31 v2.0 + 55 v2.2 + 10 v2.2.1 + 4 in Phase 21)

## Current Milestone: v2.2.1 Code Quality & Bug Fixes

**Goal:** Fix all 67 issues identified in comprehensive bug analysis

**Bug Analysis:** docs/BUG_ANALYSIS.md

| Category | Count | Priority |
|----------|-------|----------|
| Error-Handling Amnesia | 45+ | High |
| Data Lifetime Issues | 5 | High |
| Boundary Bugs | 6 | Critical |
| Concurrency Issues | 3 | Medium |
| API Fragmentation | 3 | Medium |
| State Drift | 2 | Medium |
| Resource Cleanup | 1 | Low |
| Math Issues | 1 | Medium |
| Global State | 1 | Low |

## Next Steps

**Plan Phase 20:** `/gsd:plan-phase 20`

Phase 20 focuses on:
- Fixing unwrap() on parent() in backup.rs
- Replacing to_string_lossy() in cli/mod.rs
- Replacing to_string_lossy() in patch/pattern.rs
- Fixing execution log error handling in log.rs
- Improving main.rs execution logging error handling
- Fixing test environment variable race condition
- Improving temp directory and resource cleanup

## Tech Stack

**Core:**
- Rust 2021 edition
- tree-sitter 0.21+ (parsers for 7 languages)
- SQLiteGraph 1.0 (code graph storage)
- ropey (rope data structure for text)
- serde (JSON serialization)

**Added in v2.2:**
- similar (unified diff generation)
- nu-ansi-term (terminal colors)
- is-terminal (TTY detection)

## Design Principles

1. **Additive schema evolution** — All new fields optional with skip_serializing_if
2. **Builder pattern** — Fluent API for optional field population
3. **CLI conventions** — Follow Unix/Git standards (`-n`, `-A`/`-B`/`-C`, exit codes)
4. **AST-aware operations** — Use tree-sitter for accurate symbol boundaries
5. **Lazy evaluation** — Optional features only execute when requested
6. **Zero breaking changes** — Maintain backward compatibility at all times

## Open Blockers

None

## Resolved Blockers (v2.2)

All blockers from v2.2 were resolved during gap closure:
- Error codes fully wired through CLI (Phase 18)
- Rich span fields integrated into JSON output (Phases 11-14)
- Context flags respect expanded boundaries (Phase 16)
- All cross-phase integration verified working (Phase 17)

## Decisions from Phase 19

| Plan | Decision |
|------|----------|
| 19-01 | Use if-let pattern instead of unwrap() on last() calls |
| 19-01 | Use cloned().expect() with descriptive message instead of unwrap() on first() |
| 19-02 | Replace unwrap() in test code with ? operator returning std::result::Result |
| 19-02 | Use char-based iteration (Vec<char>) instead of byte slicing for UTF-8 string manipulation |
| 19-06 | Keep UTF-8 helper functions in each module rather than creating shared module |
| 19-06 | Use chars.len() >= 2 check instead of > 2 for safer boundary handling |
| 19-07 | Use 3x multiplier for disk space estimation to account for CoW filesystem overhead |
| 19-07 | Add 4KB per-file overhead for filesystem metadata and journaling |

## Decisions from Phase 20

| Plan | Decision |
|------|----------|
| 20-01 | Use if-let pattern instead of unwrap() on Path::parent() to handle None case safely |
| 20-01 | Add descriptive messages to test expect() calls for better failure debugging |
| 20-02 | Use and_then(to_str()) instead of map(to_string_lossy()) for path serialization |
| 20-02 | Invalid UTF-8 paths should be omitted from JSON (None) rather than corrupted |
| 20-03 | Use to_str().expect() for glob patterns in test code (TempDir paths always valid UTF-8) |
| 20-03 | Use and_then/to_str with unwrap_or_default() for file name extraction |
| 20-03 | Use to_str().unwrap_or("<invalid-utf-8>") for JSON serialization of file paths |
| 20-04 | Execution log .err() patterns in test code are correct (used for assertion error messages) |
| 20-04 | No code changes needed - documentation added to explain error handling philosophy |
| 20-05 | Use helper function log_execution_error() for consistent error message formatting with operation context |
| 20-05 | Error messages include operation type (delete, patch, batch, plan, apply-files, query) for debugging |
| 20-06 | env_lock() Mutex must be held for entire test duration to prevent race conditions |
| 20-06 | Add comprehensive documentation to env_lock() to prevent future regressions |
| 20-07 | Use HashSet for test command filtering instead of sorting/de-duplicating in shell |
| 20-07 | Document TempDir's Drop trait behavior explicitly for future maintainers |
| 20-07 | Clean up stale test files before write test to prevent accumulation from crashes |
| 20-07 | Rope mutation state tracking (before_hash, replaced, after_hash) already exists and is documented |

## Decisions from Phase 21

| Plan | Decision |
|------|----------|
| 21-01 | Place parser_for_language in symbol module alongside Language enum for single source of truth |
| 21-01 | Use crate::symbol::parser_for_language as centralized parser creation API |
| 21-01 | Make parser_for_language public for cross-module reuse while maintaining encapsulation |
| 21-03 | resolve_symbol is the PRIMARY API for symbol resolution (documented explicitly) |
| 21-03 | find_symbol_or_suggest is specifically for user-facing commands needing suggestions |
| 21-03 | resolve_symbol_with_rust_kind deprecated with migration guide to string-based kinds (removed in v3.0) |
| 21-03 | Module-level documentation should include API comparison table for quick reference |
| 21-03 | Deprecation notices should include since version, removal timeline, and migration guide |
| 21-04 | Use Option<bool> for config.enabled field to distinguish explicit vs environment-based control |
| 21-04 | Environment feature toggles should support dependency injection for testability |
| 21-04 | Config structs provide enabled()/disabled()/from_env() constructors for flexibility |
| 21-04 | New functions accept Option<Config> parameter pattern for backward compatibility |

## Session Continuity

Last session: 2026-01-24
Stopped at: Completed Plan 21-04: Execution Log Testability
Resume file: None

---
*Last updated: 2026-01-24 — v2.2.1 milestone in progress*
