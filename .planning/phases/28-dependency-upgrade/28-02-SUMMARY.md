---
phase: 28-dependency-upgrade
plan: 02
subsystem: symbol-identification
tags: [blake3, sha256, symbol-id, dual-format, magellan-v2]

# Dependency graph
requires:
  - phase: 28-01
    provides: Magellan v2.1.0 and SQLiteGraph v1.4.2 dependency upgrade
provides:
  - Dual-format SymbolId enum (V1: 16-char SHA-256, V2: 32-char BLAKE3)
  - BLAKE3 dependency (blake3 v1.5)
  - Backward-compatible SymbolId parsing (auto-detects 16 or 32 char format)
  - generate_v1() for legacy SHA-256 ID generation
  - generate_v2() for new BLAKE3 ID generation
  - generate_symbol_id() defaulting to V2 format
affects:
  - Phase 28-03: Update graph ingestion to use SymbolId enum
  - Phase 29: Cross-file rename using BLAKE3 SymbolId
  - Phase 30: Graph algorithm queries with SymbolId enum

# Tech tracking
tech-stack:
  added:
    - blake3 v1.5 (BLAKE3 hashing for 32-char SymbolId)
  patterns:
    - Dual-format enum pattern for backward compatibility during migration
    - Auto-detection by length (16 vs 32 characters)
    - Unchecked constructors for trusted internal generation

key-files:
  created: []
  modified:
    - Cargo.toml (added blake3 = "1.5" dependency)
    - src/symbol_id.rs (converted to enum with V1/V2 variants)

key-decisions:
  - "Use BLAKE3 full 16-byte output as 32 hex chars (not 64 chars from to_hex())"
  - "Rename generate_symbol_id() to generate_v1() to preserve SHA-256 logic exactly"
  - "Default generate_symbol_id() to V2 BLAKE3 for new code"
  - "Add parse() method instead of new() for dual-format detection"

patterns-established:
  - "Enum-based format migration: V1(String) / V2(String) with auto-detection"
  - "Length-based parsing: 16 chars -> V1, 32 chars -> V2, else error"
  - "Preserve legacy code: rename don't rewrite when adding new format"

# Metrics
duration: 15min
completed: 2026-02-04
---

# Phase 28: Dependency Upgrade Summary

**BLAKE3-based 32-character SymbolId enum with dual-format V1/V2 support for backward-compatible migration from SHA-256**

## Performance

- **Duration:** 15 min
- **Started:** 2026-02-04T09:41:58Z
- **Completed:** 2026-02-04T09:56:58Z
- **Tasks:** 3
- **Files modified:** 2

## Accomplishments

- Added blake3 v1.5 dependency for 32-char SymbolId generation
- Converted SymbolId from newtype struct to dual-format enum (V1/V2)
- Implemented generate_v1() for legacy 16-char SHA-256 IDs
- Implemented generate_v2() for new 32-char BLAKE3 IDs
- Added SymbolId::parse() with auto-detection (16 or 32 chars)
- All 21 symbol_id tests pass plus 343 total lib tests

## Task Commits

Each task was committed atomically:

1. **Task 1: Add blake3 dependency** - `1f57a0d` (chore)
2. **Task 2: Implement dual-format SymbolId enum** - `e90742f` (feat)
3. **Task 3: Verify lib.rs exports** - No changes needed (already had `pub mod symbol_id;`)

**Plan metadata:** (to be committed after SUMMARY.md)

_Note: TDD tasks may have multiple commits (test → feat → refactor)_

## Files Created/Modified

- `Cargo.toml` - Added blake3 = "1.5" dependency after sha2 = "0.10"
- `src/symbol_id.rs` - Converted from newtype struct to enum with dual-format support:
  - Changed from `pub struct SymbolId(String)` to `pub enum SymbolId { V1(String), V2(String) }`
  - Renamed `generate_symbol_id()` to `generate_v1()` preserving exact SHA-256 logic
  - Added `generate_v2()` using BLAKE3 with 16-byte hash -> 32 hex chars
  - Added `generate_symbol_id()` wrapper defaulting to V2
  - Added `parse()` method for dual-format detection
  - Added `is_v1()` and `is_v2()` helper methods
  - Updated all trait implementations (Display, AsRef, TryFrom, Hash)
  - Updated module documentation

## Decisions Made

1. **Use BLAKE3's first 16 bytes as 32 hex chars** - BLAKE3's `to_hex()` returns 64 chars (full 32-byte hash), but we only need first 16 bytes (32 hex chars) to match Magellan v2.1.0's format. Used `hash.as_bytes()[0..16]` manually formatted.

2. **Rename don't rewrite SHA-256 logic** - The existing `generate_symbol_id()` had working SHA-256 code. Renamed to `generate_v1()` to preserve exact behavior rather than risk bugs by rewriting.

3. **Use enum not struct with version field** - Dual-format as `enum SymbolId { V1(String), V2(String) }` provides compile-time guarantees that format-specific code is correct, better than `struct SymbolId { version: u8, id: String }`.

4. **Parse method not new method** - `SymbolId::new()` typically implies construction from validated parts. `SymbolId::parse()` better conveys "parse string and detect format." Kept `new_v1_unchecked()` and `new_v2_unchecked()` for internal use.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Bug] Fixed BLAKE3 update() type error**
- **Found during:** Task 2 (generate_v2 implementation)
- **Issue:** `hasher.update(byte_start.to_be_bytes())` failed - BLAKE3 expects `&[u8]` not `[u8; 8]`
- **Fix:** Changed to `hasher.update(&byte_start.to_be_bytes())` to pass reference
- **Files modified:** src/symbol_id.rs
- **Verification:** Compiled successfully, tests pass
- **Committed in:** e90742f (Task 2 commit)

**2. [Rule 1 - Bug] Fixed BLAKE3 hash length (64 chars -> 32 chars)**
- **Found during:** Task 2 (generate_v2 testing)
- **Issue:** `hash.to_hex()` returns 64 hex characters (full 32-byte hash), but tests expect 32 chars
- **Fix:** Manually format first 16 bytes as 32 hex chars using `format!("{:02x}...")` with `hash.as_bytes()[0..16]`
- **Files modified:** src/symbol_id.rs
- **Verification:** All 21 symbol_id tests pass
- **Committed in:** e90742f (Task 2 commit)

---

**Total deviations:** 2 auto-fixed (both Rule 1 - Bug fixes)
**Impact on plan:** Both were necessary fixes for BLAKE3 API compatibility. No scope creep.

## Issues Encountered

- **BLAKE3 API differences from SHA-256:** BLAKE3's `update()` method requires `&[u8]` while SHA-256 accepts both. Fixed by adding `&` reference operator.
- **BLAKE3 hex output length:** `to_hex()` returns 64 chars (full hash), not 32. Fixed by manually formatting first 16 bytes.
- **Magellan database schema mismatch:** Database was created with Magellan v0.5.3 (schema 4) but Magellan v2.1.0 expects schema 6 with BLAKE3 IDs. This is expected and will be addressed by re-indexing in later phases.

## Authentication Gates

None - no external authentication required.

## User Setup Required

None - all changes are internal to the codebase.

## Next Phase Readiness

- BLAKE3 SymbolId generation complete and tested
- Dual-format parsing (V1/V2) enables gradual migration
- Ready for Phase 28-03: Update graph ingestion to use SymbolId enum
- Ready for Phase 29: Cross-file rename with BLAKE3-based symbol references
- Note: Magellan database needs re-indexing to use new BLAKE3 format (will be addressed in subsequent phases)

---
*Phase: 28-dependency-upgrade*
*Completed: 2026-02-04*
