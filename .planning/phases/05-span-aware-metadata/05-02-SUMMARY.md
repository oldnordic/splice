# Plan 05-02 Summary: Pass Computed Line/Col to Storage

**Status:** ✅ Complete
**Commits:** `138e12e`

## Objective
Update all language ingest module call sites to pass computed line/col to graph storage.

## Context
All 7 language ingest modules (rust, python, cpp, java, javascript, typescript) already compute line/col using ropey. Their Symbol structs already have these fields. The Symbol trait already provides accessor methods. This plan simply updates the call sites to actually pass these values instead of 0 placeholders.

## Changes Made

### 1. Updated src/main.rs
- Changed `symbol.line_start(), symbol.line_end(), symbol.col_start(), symbol.col_end()` calls
- Replaced 0 placeholders with actual values from Symbol trait accessors

### 2. Updated src/plan/mod.rs
- Changed RustSymbol direct field access
- Replaced 0 placeholders with actual computed values

### 3. Updated all test files
- tests/python_patch_tests.rs
- tests/javascript_patch_tests.rs
- tests/typescript_patch_tests.rs
- tests/cpp_patch_tests.rs
- tests/java_patch_tests.rs
- tests/patch_tests.rs
- tests/integration_refactor.rs

## Verification
- ✅ cargo check passes
- ✅ All 118 unit tests pass
- ✅ Line/col values now flow through entire pipeline

## Notes
- Ingest modules didn't need changes - they already computed line/col
- Symbol trait already had line_start/end/col_start/end accessors
- This was purely about updating call sites
