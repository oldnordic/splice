---
phase: 19-critical-error-handling
plan: 06
title: "UTF-8 safe string handling consolidation"
subsystem: "import extraction"
tags: ["utf-8", "string-safety", "helper-functions", "imports"]
completed: 2026-01-23
duration: 12s
status: complete
commits:
  - edd7ed0
  - 317d8bb
tech-stack:
  added: []
  patterns: ["UTF-8 character-based iteration", "Helper function extraction"]
key-files:
  created: []
  modified:
    - src/ingest/imports/python.rs
    - src/ingest/imports/cpp.rs
    - src/ingest/imports/javascript.rs
    - src/ingest/imports/typescript.rs
dependency-graph:
  requires: [19-02, 19-03, 19-05]
  provides: "UTF-8 safe import extraction across all languages"
  affects: []
---

# Phase 19 Plan 06: UTF-8 safe string handling consolidation Summary

**One-liner:** Consolidated UTF-8 safe string handling across import modules with helper functions and fixed last remaining unwrap() in Python test code.

## Changes Made

### Task 1: Replaced unwrap() in Python test (line 312)

**Before:**
```rust
#[test]
fn test_extract_simple_import_basic() {
    let source = b"import os\n";
    let path = Path::new("test.py");
    let result = extract_python_imports(path, source);
    assert!(result.is_ok());
    let imports = result.unwrap();
    assert_eq!(imports.len(), 1);
    assert_eq!(imports[0].import_kind, ImportKind::PythonImport);
}
```

**After:**
```rust
#[test]
fn test_extract_simple_import_basic() -> std::result::Result<(), Box<dyn std::error::Error>> {
    let source = b"import os\n";
    let path = Path::new("test.py");
    let result = extract_python_imports(path, source)?;
    assert_eq!(result.len(), 1);
    assert_eq!(result[0].import_kind, ImportKind::PythonImport);
    Ok(())
}
```

**Commit:** edd7ed0

### Task 2: Added helper functions for safe quote stripping

Created three helper functions to consolidate UTF-8 safe quote stripping logic:

#### cpp.rs - `strip_quotes_or_angle_brackets()`

Handles `<...>`, `"..."`, and `'...'` patterns for C/C++ include directives.

```rust
fn strip_quotes_or_angle_brackets(text: &str) -> String {
    let chars: Vec<char> = text.chars().collect();
    if chars.len() >= 2 {
        match (chars.first(), chars.last()) {
            (Some('<'), Some('>')) | (Some('"'), Some('"')) | (Some('\''), Some('\'')) => {
                chars[1..chars.len() - 1].iter().collect()
            }
            _ => text.to_string(),
        }
    } else {
        text.to_string()
    }
}
```

#### javascript.rs and typescript.rs - `strip_quotes()`

Handles `"..."` and `'...'` patterns for JS/TS require() calls.

```rust
fn strip_quotes(text: &str) -> String {
    let chars: Vec<char> = text.chars().collect();
    if chars.len() >= 2 {
        match (chars.first(), chars.last()) {
            (Some('"'), Some('"')) | (Some('\''), Some('\'')) => {
                chars[1..chars.len() - 1].iter().collect()
            }
            _ => text.to_string(),
        }
    } else {
        text.to_string()
    }
}
```

**Commit:** 317d8bb

## UTF-8 Safety

All string manipulation uses **character-based iteration** (`Vec<char>`) rather than byte-based slicing:

- **Safe:** `chars[1..chars.len() - 1]` - operates on `char` indices
- **Unsafe:** `text[1..text.len() - 1]` - operates on byte indices, panics on multi-byte UTF-8

This pattern correctly handles Unicode characters like emojis and accented letters.

## Deviations from Plan

None - plan executed exactly as written.

## Test Results

All 26 import tests pass:
```
running 26 tests
test ingest::imports::cpp::tests::test_extract_local_include ... ok
test ingest::imports::cpp::tests::test_extract_system_include ... ok
test ingest::imports::java::tests::test_extract_simple_import ... ok
test ingest::imports::java::tests::test_extract_static_import ... ok
test ingest::imports::java::tests::test_extract_multiple_imports ... ok
test ingest::imports::java::tests::test_extract_static_wildcard_import ... ok
test ingest::imports::java::tests::test_extract_wildcard_import ... ok
test ingest::imports::java::tests::test_import_has_byte_span ... ok
test ingest::imports::javascript::tests::test_extract_default_import ... ok
test ingest::imports::javascript::tests::test_extract_nested_path_import ... ok
test ingest::imports::javascript::tests::test_extract_named_import ... ok
test ingest::imports::javascript::tests::test_extract_multiple_imports ... ok
test ingest::imports::javascript::tests::test_extract_require_call ... ok
test ingest::imports::javascript::tests::test_extract_namespace_import ... ok
test ingest::imports::javascript::tests::test_extract_side_effect_import ... ok
test ingest::imports::python::tests::test_empty_path_handling ... ok
test ingest::imports::python::tests::test_extract_simple_import_basic ... ok
test ingest::imports::typescript::tests::test_extract_default_import ... ok
test ingest::imports::typescript::tests::test_extract_import_with_alias ... ok
test ingest::imports::typescript::tests::test_extract_named_import ... ok
test ingest::imports::typescript::tests::test_extract_namespace_import ... ok
test ingest::imports::typescript::tests::test_extract_side_effect_import ... ok
test ingest::imports::typescript::tests::test_typescript_import_has_byte_span ... ok
test ingest::imports::typescript::tests::test_extract_type_only_default_import ... ok
test ingest::imports::typescript::tests::test_extract_type_only_named_import ... ok
test ingest::imports::typescript::tests::test_extract_multiple_imports ... ok

test result: ok. 26 passed; 0 failed; 0 ignored
```

## Verification

1. **No unwrap() in test code:** All import module tests now use `?` operator or explicit error handling
2. **No byte-based string slicing:** Verified with `grep -n "\\[1\\..*len()" src/ingest/imports/*.rs | grep -v "chars"` - returns nothing
3. **Helper functions exist:** Each module has its appropriate helper function documented and inlined
4. **All tests pass:** 26/26 import tests passing

## Decisions Made

| Decision | Reasoning |
|----------|-----------|
| Keep helpers in each module | The functions are small (10 LOC) and module-specific; not worth creating a shared module |
| Use `chars.len() >= 2` check | Safer than `> 2` - handles edge case of empty string and single character strings |
| Match on first/last char | More explicit than iterating, handles all quote types uniformly |

## Next Phase Readiness

This plan completes the UTF-8 safety work across import modules. The codebase is now ready for:

1. **19-07:** Data lifetime issues (if any remain)
2. **Remaining Phase 19 plans:** Error handling improvements in other subsystems

No blockers identified.
