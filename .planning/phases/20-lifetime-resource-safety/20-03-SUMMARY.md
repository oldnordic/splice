# Phase 20 Plan 03: Replace to_string_lossy() in Pattern Tests Summary

**One-liner:** Replaced 33 instances of to_string_lossy() in pattern.rs test code with explicit UTF-8 handling using to_str().expect() and to_str().unwrap_or().

---

## Metadata

- **Phase:** 20 - Lifetime & Resource Safety
- **Plan:** 03 - Replace to_string_lossy() in pattern.rs
- **Type:** Code Quality Refactoring
- **Autonomous:** Yes
- **Completed:** 2026-01-24
- **Duration:** ~2 minutes

## Tech Stack

- **Rust:** 2021 edition
- **Patterns:** UTF-8 explicit handling, expect() for test code
- **Testing:** cargo test --lib patch::pattern (24 tests pass)

---

## What Was Done

### Task 1: Replace to_string_lossy() for glob_pattern in test code

**Problem:** Pattern test code used `to_string_lossy()` which silently replaces invalid UTF-8 with replacement characters, potentially hiding data corruption issues.

**Solution:** Replace with explicit UTF-8 handling:
- **Glob pattern assignments:** Use `to_str().expect("Invalid UTF-8 path")`
- **File name extraction:** Use `and_then(|n| n.to_str()).unwrap_or_default()`
- **JSON serialization:** Use `to_str().unwrap_or("<invalid-utf-8>")`

**Changes made:**

1. **Glob pattern assignments (27 instances):**
   ```rust
   // Before:
   glob_pattern: workspace_root.join("*.py").to_string_lossy().to_string(),

   // After:
   glob_pattern: workspace_root.join("*.py").to_str().expect("Invalid UTF-8 path").to_string(),
   ```

2. **File name mapping (1 instance - line 802):**
   ```rust
   // Before:
   .map(|m| m.file.file_name().unwrap().to_string_lossy().to_string())

   // After:
   .map(|m| m.file.file_name().and_then(|n| n.to_str()).unwrap_or_default().to_string())
   ```

3. **JSON file path serialization (5 instances):**
   ```rust
   // Before:
   "file": m.file.to_string_lossy().to_string(),

   // After:
   "file": m.file.to_str().unwrap_or("<invalid-utf-8>").to_string(),
   ```

**Total replacements:** 33 instances across 24 test functions

**Verification:**
```bash
$ grep -n "to_string_lossy" src/patch/pattern.rs
# (empty - no matches)

$ cargo test --lib patch::pattern
test result: ok. 24 passed; 0 failed; 0 ignored
```

---

## Deviations from Plan

None - plan executed exactly as written.

---

## Key Files

### Created
- `.planning/phases/20-lifetime-resource-safety/20-03-SUMMARY.md`

### Modified
- `src/patch/pattern.rs` - Replaced all 33 to_string_lossy() calls with explicit UTF-8 handling

---

## Decisions Made

### Decision 1: Use expect() for glob patterns in test code

**Context:** Test code uses TempDir which always creates valid UTF-8 paths on modern systems.

**Decision:** Use `to_str().expect("Invalid UTF-8 path")` for glob pattern construction.

**Rationale:**
- TempDir paths are always valid UTF-8 in practice
- expect() provides clear error messages if the impossible happens
- Better than silent data loss from to_string_lossy()
- More explicit about UTF-8 expectations

**Trade-offs:**
- **Pro:** Fails fast with clear message if UTF-8 validation fails
- **Pro:** Makes test expectations explicit
- **Con:** Will panic on invalid UTF-8 (but this never happens in practice)
- **Mitigation:** TempDir guarantees valid paths on all supported platforms

---

### Decision 2: Use unwrap_or_default() for file name extraction

**Context:** Line 802 extracts file names from PathBuf for display in test assertions.

**Decision:** Use `file_name().and_then(|n| n.to_str()).unwrap_or_default()`

**Rationale:**
- File name might not exist (empty path) or might not be valid UTF-8
- Default to empty string in error cases
- Prevents test panics on edge cases
- Better than unwrap() which would panic on None

**Trade-offs:**
- **Pro:** Graceful handling of edge cases
- **Pro:** Test continues with empty string rather than panicking
- **Con:** Silent failure if file name has invalid UTF-8
- **Mitigation:** Test code uses TempDir with valid paths, so this is defensive programming

---

### Decision 3: Use unwrap_or("<invalid-utf-8>") for JSON serialization

**Context:** Lines 1041, 1100, 1168, 1221, 1270, 1317 serialize file paths to JSON output.

**Decision:** Use `to_str().unwrap_or("<invalid-utf-8>")`

**Rationale:**
- JSON requires valid UTF-8 strings
- Must have a fallback for invalid UTF-8 paths
- "<invalid-utf-8>" clearly indicates the problem
- Better than replacement characters from to_string_lossy()

**Trade-offs:**
- **Pro:** Explicit error indicator in JSON output
- **Pro:** Makes debugging easier when invalid paths appear
- **Pro:** No silent data loss
- **Con:** String is not the actual path (but neither is to_string_lossy output)

---

## Verification Results

### Test Results

```bash
$ cargo test --lib patch::pattern

running 24 tests
test patch::pattern::tests::test_apply_replace_multiple_files ... ok
test patch::pattern::tests::test_apply_pattern_replace ... ok
test patch::pattern::tests::test_apply_replace_single_file ... ok
test patch::pattern::tests::test_search_command_rust ... ok
test patch::pattern::tests::test_search_command_multiple_files ... ok
test patch::pattern::tests::test_find_pattern_in_file ... ok
test patch::pattern::tests::test_search_command_python ... ok
test patch::pattern::tests::test_apply_replace_with_validation ... ok
test patch::pattern::tests::test_apply_replace_rollback_on_error ... ok
test patch::pattern::tests::test_search_command_no_matches ... ok
test patch::pattern::tests::test_search_glob_no_matches ... ok
test patch::pattern::tests::test_search_context_asymmetric ... ok
test patch::pattern::tests::test_search_glob_rust_only ... ok
test patch::pattern::tests::test_search_glob_multi_extension ... ok
test patch::pattern::tests::test_search_glob_recursive ... ok
test patch::pattern::tests::test_search_json_output_format ... ok
test patch::pattern::tests::test_search_json_no_context ... ok
test patch::pattern::tests::test_search_glob_python_only ... ok
test patch::pattern::tests::test_search_json_all_metadata ... ok
test patch::pattern::tests::test_search_context_in_json ... ok
test patch::pattern::tests::test_search_json_parseable ... ok
test patch::pattern::tests::test_search_no_context ... ok
test patch::pattern::tests::test_search_with_context ... ok
test patch::pattern::tests::test_search_json_with_context ... ok

test result: ok. 24 passed; 0 failed; 0 ignored; 0 measured
```

### to_string_lossy() Verification

```bash
$ grep -n "to_string_lossy" src/patch/pattern.rs
# (no matches - all instances replaced)
```

---

## Success Criteria

- [x] All 33 to_string_lossy() instances in pattern.rs replaced
- [x] All 24 pattern tests pass
- [x] Explicit UTF-8 handling with expect() or unwrap_or()
- [x] No to_string_lossy() calls remain in src/patch/pattern.rs

---

## Next Phase Readiness

**Status:** Ready for Phase 20 Plan 04

**Blockers:** None

**Recommendations:**
- Continue with remaining Phase 20 plans for lifetime and resource safety improvements
- Pattern test code now has explicit UTF-8 handling consistent with production code standards

---

## Related Issues

- **v2.2.1 Bug Analysis:** Data Lifetime Issues (improves test code quality)
- **Phase 20 Focus:** Lifetime & Resource Safety

---

## References

- **Plan:** `.planning/phases/20-lifetime-resource-safety/20-03-PLAN.md`
- **Context:** `src/patch/pattern.rs` (test module)
- **Tests:** `cargo test --lib patch::pattern`
