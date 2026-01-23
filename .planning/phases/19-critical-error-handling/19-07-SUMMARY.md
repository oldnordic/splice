# Phase 19 Plan 07: Boundary Checks and Disk Space Summary

**One-liner:** Improved disk space calculation using 3x multiplier with filesystem overhead, and documented boundary safety in Python import extraction.

---

## Meta

**Phase:** 19-critical-error-handling
**Plan:** 07
**Subsystem:** Error Handling
**Tags:** disk-space, boundary-safety, verification

**Dependency Graph:**
- **requires:** Phase 19 Wave 1 (unwrap() fixes completed)
- **provides:** Improved resource verification and boundary safety documentation
- **affects:** Future plans requiring disk space validation

**Tech Stack Changes:**
- **Added:** None
- **Patterns:** Constants for configurable multipliers, safety invariant documentation

**File Tracking:**
- **Created:** None
- **Modified:** `src/verify.rs`, `src/ingest/imports/python.rs`

---

## Objective

Add boundary checks and improve disk space calculation to fix potential off-by-one boundary issues (BOUNDARY-01 through BOUNDARY-06) and improve disk space heuristic (MATH-01).

---

## Changes Made

### 1. Disk Space Calculation Improvement (src/verify.rs)

**Constants added at top of file:**
```rust
/// Multiplier for disk space estimation to account for filesystem overhead.
/// Atomic writes need space for both original and new file simultaneously,
/// plus additional buffer for journaling and metadata.
const DISK_SPACE_MULTIPLIER: usize = 3;

/// Additional overhead per file for metadata and filesystem structures (in bytes).
/// Accounts for typical filesystem block size (4KB) and inode/table overhead.
const DISK_OVERHEAD_PER_FILE: u64 = 4096;
```

**Updated calculation:**
```rust
let needed = (estimated_size * DISK_SPACE_MULTIPLIER) as u64 + DISK_OVERHEAD_PER_FILE;
if available < needed {
    return PreVerificationResult::blocking(
        "disk_space",
        format!(
            "Insufficient disk space: need {} bytes ({}x file size + {} overhead), available {} bytes",
            needed, DISK_SPACE_MULTIPLIER, DISK_OVERHEAD_PER_FILE, available
        ),
    );
}
```

**Rationale for 3x multiplier:**
- Atomic writes need space for: original file + new file + journaling buffer
- The 2x multiplier was insufficient for filesystems with copy-on-write (Btrfs, ZFS, APFS)
- 3x provides a safety margin for metadata updates and block alignment overhead
- 4KB overhead accounts for typical filesystem block size and inode table entries

### 2. Boundary Safety Documentation (src/ingest/imports/python.rs)

**Added Safety Invariants to `extract_import_statement`:**
```rust
/// # Safety Invariants
/// - All collection access (first/last) is guarded by is_empty() checks
/// - All Option unwrapping uses ? or or_else patterns
/// - No unwrap() or expect() in production code paths
```

**Improved `extract_aliased_import` fallback pattern:**
```rust
// Before:
path.first()?.clone()

// After:
path.first().cloned().unwrap_or_default()
```

This change provides:
- Empty string fallback instead of None (preventing extraction failure)
- More graceful handling of edge cases
- No change to overall Option return (function still returns Option<ImportFact>)

### 3. Edge Case Test (src/ingest/imports/python.rs)

**Added `test_empty_path_handling`:**
```rust
#[test]
fn test_empty_path_handling() {
    // Test that malformed input doesn't cause panics
    let source = b"import\n"; // Malformed but shouldn't panic
    let path = Path::new("test.py");
    let result = extract_python_imports(path, source);

    match result {
        Ok(imports) => {
            // Verify structure is valid
            for imp in imports {
                if imp.path.is_empty() {
                    assert!(imp.imported_names.is_empty() || imp.imported_names.iter().all(|n| n.is_empty()));
                }
            }
        }
        Err(_) => {
            // Parse errors are acceptable for malformed input
        }
    }
}
```

---

## Deviations from Plan

**None** - plan executed exactly as written.

---

## Verification Results

### Test Results
```bash
cargo test --lib verify
# test result: ok. 19 passed; 0 failed; 0 ignored

cargo test --lib ingest::imports::python
# test result: ok. 2 passed; 0 failed; 0 ignored
```

### unwrap() Audit
- **src/verify.rs:** No unwrap() in production code (all in test module)
- **src/ingest/imports/python.rs:** No unwrap() anywhere (only in comments)

---

## Decisions Made

### Decision 1: 3x Multiplier for Disk Space
**Context:** Previous 2x multiplier was insufficient for copy-on-write filesystems.

**Rationale:**
- Atomic file operations need: original + new + journal buffer
- CoW filesystems (Btrfs, ZFS, APFS) temporarily keep both copies
- Metadata updates (inodes, directory entries) add overhead
- 3x provides buffer while not being overly conservative

**Alternatives considered:**
- 2.5x: Non-integer, awkward to express
- 4x: More conservative, but 3x is sufficient for most cases
- Dynamic detection: Would require filesystem-specific logic

**Trade-off:** 3x may reject some operations that would succeed, but prevents out-of-space errors mid-write.

---

## Metrics

**Duration:** ~5 minutes
**Completed:** 2026-01-23
**Commits:** 3
- `2aa812a`: feat(19-07): improve disk space calculation
- `28ae143`: docs(19-07): add boundary safety documentation
- `0105b56`: test(19-07): add edge case test

---

## Next Phase Readiness

**Status:** Ready for next plan

**Blockers:** None

**Known Issues:** None
