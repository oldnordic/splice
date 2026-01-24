---
phase: 21-api-consolidation
plan: 02
subsystem: api-refactoring
tags: [tree-sitter, trait-based-api, import-extraction, code-consolidation]

# Dependency graph
requires:
  - phase: 01-03
    provides: Import extraction modules for Python and C++
provides:
  - ImportExtractor trait for language-agnostic import extraction interface
  - Trait-based pattern for adding new language import extractors
  - Reduced code duplication across language modules
affects: [future-language-support, api-consistency]

# Tech tracking
tech-stack:
  added: []
  patterns: [trait-based extraction, default implementation pattern, unit struct for types]

key-files:
  created: []
  modified:
    - src/ingest/imports/mod.rs - Added ImportExtractor trait
    - src/ingest/imports/python.rs - Implements trait for Python
    - src/ingest/imports/cpp.rs - Implements trait for C/C++

key-decisions:
  - "Use trait with default implementation for common extraction flow"
  - "Unit struct pattern for language extractors (PythonExtractor, CppExtractor)"
  - "Preserve existing public API functions for backward compatibility"

patterns-established:
  - "ImportExtractor trait: language() provides tree-sitter language, language_enum() provides Language enum, extract_from_node() handles AST walking"
  - "Default extract() method encapsulates parser creation and source parsing"
  - "Language-specific implementations only need to implement extract_from_node()"

# Metrics
duration: 3min
completed: 2026-01-24
---

# Phase 21 Plan 02: Import Extraction Trait-Based API Summary

**Trait-based import extraction API with default implementation reducing duplication and enabling consistent language support**

## Performance

- **Duration:** 3 min
- **Started:** 2026-01-24T00:15:26Z
- **Completed:** 2026-01-24T00:18:15Z
- **Tasks:** 3
- **Files modified:** 3

## Accomplishments

- **ImportExtractor trait** defining language-agnostic interface with default extract() implementation
- **Python and C++ modules** refactored to use trait-based approach
- **Code duplication eliminated** - parser creation and AST walking now shared via trait
- **Backward compatibility maintained** - all existing extract_*_imports functions still work

## Task Commits

Each task was committed atomically:

1. **Task 1: Define ImportExtractor trait in mod.rs** - `a79885d` (feat)
2. **Task 2: Implement ImportExtractor for Python** - `069b07d` (feat)
3. **Task 3: Implement ImportExtractor for C/C++** - `d910230` (feat)

**Plan metadata:** Not yet committed

## Files Created/Modified

- `src/ingest/imports/mod.rs` - Added ImportExtractor trait with language(), language_enum(), extract_from_node(), and default extract() method
- `src/ingest/imports/python.rs` - Added PythonExtractor struct implementing ImportExtractor trait, refactored extract_python_imports to use trait
- `src/ingest/imports/cpp.rs` - Added CppExtractor struct implementing ImportExtractor trait, refactored extract_cpp_imports to use trait

## Decisions Made

- **Trait with default implementation**: Common parsing logic (parser creation, source parsing, AST walking) lives in trait's default extract() method, eliminating duplication
- **Unit struct pattern**: Language extractors use unit struct (PythonExtractor, CppExtractor) rather than carrying state
- **Backward compatibility**: Public extract_*_imports functions preserved as thin wrappers around trait methods

## Deviations from Plan

None - plan executed exactly as written.

## Issues Encountered

- Unused import warning (SpliceError) in python.rs after refactoring - removed unused import
- Pre-existing test failure in cli_tests::test_cli_patch_preview unrelated to our changes

## Authentication Gates

None - no external authentication required for this plan.

## Next Phase Readiness

- ImportExtractor trait provides template for implementing Java, JavaScript, and TypeScript extractors
- Pattern established for adding new language support: implement trait with language(), language_enum(), and extract_from_node()
- No blockers or concerns

---
*Phase: 21-api-consolidation*
*Completed: 2026-01-24*
