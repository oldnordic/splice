# Phase 29-01: Rename Command with Symbol Lookup - SUMMARY

**Plan:** 29-01
**Date:** 2026-02-04
**Status:** COMPLETED
**Autonomous:** Yes

## Objective

Add `splice rename` CLI command with symbol lookup supporting ID-first with name+path fallback.

## Tasks Completed

### Task 1: Add Rename command to CLI (COMPLETED)
**File:** `src/cli/mod.rs`
- Added `Rename` subcommand to `Commands` enum after `MigrateDb` (display_order = 108)
- Implemented all required flags:
  - `--symbol <id>`: 32-char BLAKE3 or 16-char SHA-256 symbol ID
  - `--name <name>`: Symbol name (requires --file)
  - `--file <path>`: File path for symbol name resolution
  - `--to <new_name>`: New name for the symbol (required)
  - `--db <path>`: Path to Magellan database (default: .codemcp/codegraph.db)
  - `--preview` / `--dry-run`: Preview changes without applying
  - `--backup-dir`: Override backup directory
  - `--no-backup`: Skip backup creation
  - `--create-backup`: Create backup (default: true)

### Task 2: Add RenameFailed error variant (COMPLETED)
**Files:**
- `src/error.rs`: Added `RenameFailed` variant with `reason` and `symbol` fields
- `src/error.rs`: Added `RenameFailed` to `kind()` method returning "RenameFailed"
- `src/error_codes.rs`: Added `SPL-E040` error code for rename operations
- `src/error_codes.rs`: Added error code mapping in `from_splice_error()`

### Task 3: Add get_all_references method to MagellanIntegration (COMPLETED)
**File:** `src/graph/magellan_integration.rs`
- Added `get_all_references(&mut self, entity_id: i64)` method
- Returns `Vec<magellan::references::ReferenceFact>` with byte spans
- Uses Magellan's `references_to_symbol()` API

### Task 4: Implement execute_rename handler stub (COMPLETED)
**File:** `src/main.rs`
- Added `execute_rename()` function after `execute_migrate_db()`
- Added handler in main() command match arm
- Implemented stub behavior:
  - Validates input (either --symbol or --name+--file)
  - Opens Magellan database
  - Resolves symbol (ID-first with name+path fallback)
  - Validates symbol exists and has references
  - Reports preview mode or stub status
- Full implementation deferred to plan 29-02

### Additional Changes
**File:** `src/graph/mod.rs`
- Re-exported `MagellanIntegration` for convenient use

## Verification

1. **Cargo check passes:** Code compiles without errors
2. **CLI help works:** `splice rename --help` displays correct usage
3. **Command structure correct:** All flags are properly configured
4. **Error handling:** SPL-E040 error code mapped correctly

## Success Criteria Met

- [x] User can invoke `splice rename --symbol <id> --to <new_name>`
- [x] User can invoke `splice rename --name <name> --file <path> --to <new_name>`
- [x] Symbol lookup resolves both 32-char V2 and 16-char V1 IDs (via existing `find_symbol_by_id()`)
- [x] Ambiguous symbols produce error listing all matches
- [x] Pre-flight validation: symbol must exist and have references

## Files Modified

1. `src/cli/mod.rs` - Added Rename command
2. `src/error.rs` - Added RenameFailed variant
3. `src/error_codes.rs` - Added SPL-E040 error code
4. `src/graph/magellan_integration.rs` - Added get_all_references method
5. `src/graph/mod.rs` - Re-exported MagellanIntegration

## Next Steps

Plan 29-02 will implement the full rename execution logic:
- Create backup before renaming
- Apply byte-accurate replacements using ReferenceFact spans
- Validate after rename (parse check, optional compiler check)
- Rollback on validation failure
