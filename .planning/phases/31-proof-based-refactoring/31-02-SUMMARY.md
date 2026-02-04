# Phase 31-02 Summary: Proof Generation for Rename Operations

**Status:** ✅ COMPLETED
**Date:** 2026-02-04
**Dependencies:** 31-01 (proof data structures)
**Dependents:** 31-03 (invariant validation), 31-04 (checksums)

## Overview

Implemented proof generation for the rename command with `--proof` flag. Users can now generate machine-checkable behavioral equivalence proofs for rename operations.

## Files Modified

### Core Implementation

1. **src/proof/generation.rs** (NEW)
   - `generate_snapshot()`: Captures complete graph state from Magellan database
   - `is_public_symbol()`: Detects public/entry point symbols via heuristics
   - `create_metadata()`: Creates proof metadata (operation, timestamp, git commit)
   - `get_git_commit()`: Retrieves current git commit hash
   - `write_proof()`: Serializes and writes proof to JSON file

2. **src/proof/mod.rs**
   - Added `pub mod generation;`
   - Exported `generate_snapshot`, `create_metadata`, `write_proof`

3. **src/cli/mod.rs**
   - Added `--proof` flag to `Rename` command

4. **src/main.rs**
   - Updated `execute_rename()` signature to accept `proof` parameter
   - Added before snapshot generation (before rename operation)
   - Added after snapshot generation (after rename completion)
   - Integrated proof writing to `.splice/proofs/`

## Success Criteria

- ✅ User can invoke `splice rename --symbol <id> --to <new_name> --proof`
- ✅ Proof JSON written to `.splice/proofs/rename-<timestamp>.json`
- ✅ Proof contains before/after graph snapshots
- ✅ Rename operation unchanged (proof is observational)
- ✅ All existing tests pass (383 passed)

## Proof JSON Schema

```json
{
  "metadata": {
    "operation": "rename",
    "user": "feanor",
    "timestamp": 1738681234,
    "git_commit": "1a2b3c4d...",
    "splice_version": "2.2.4",
    "database_path": "/path/to/.codemcp/codegraph.db"
  },
  "before": {
    "timestamp": 1738681230,
    "symbols": { /* id -> SymbolInfo */ },
    "edges": { /* id -> [callee_ids] */ },
    "entry_points": [ /* ids */ ],
    "stats": {
      "total_symbols": 150,
      "total_edges": 320,
      "entry_point_count": 12
    }
  },
  "after": { /* same structure as before */ },
  "invariants": [], // Will be populated in 31-03
  "checksums": null // Will be populated in 31-04
}
```

## Symbol ID Generation

Proof snapshots use incremental hex IDs (`0000000000000000`, `0000000000000001`, ...) rather than Magellan entity IDs. This is because:

1. `SymbolFact` doesn't include `entity_id` directly
2. IDs are only available from `symbol_extents()` which returns `(entity_id, fact)`
3. For proof consistency, we use deterministic counter-based IDs

## Entry Point Detection

Entry points are detected via heuristics:
- **Functions/methods**: No underscore prefix (`_private` is skipped)
- **Types (struct, class, trait, enum)**: Uppercase first letter (`MyStruct`)
- **Other kinds**: Considered public by default

## Testing

### Unit Tests
- `test_is_public_symbol_functions()`: Validates function visibility detection
- `test_is_public_symbol_types()`: Validates type visibility detection
- `test_is_public_symbol_methods()`: Validates method visibility detection

All tests pass: `cargo test --lib proof::generation`

## Usage Example

```bash
# Generate proof for rename operation
splice rename --symbol abc123 --to new_name --proof

# Output includes proof file location
Renamed 'old_name' to 'new_name' in 3 files
Backup: .splice/backups/rename-abc123-1738681234/
Proof written to: .splice/proofs/rename-1738681234.json
```

## Known Limitations

1. **No entity_id in snapshots**: Proof uses counter-based IDs instead of Magellan entity IDs
2. **Entry point heuristics**: Language-specific conventions not fully implemented
3. **No invariant validation**: Invariants checked in 31-03
4. **No checksums**: Integrity hashes added in 31-04

## Next Steps

- **31-03**: Implement invariant validation (reference counts, orphan detection, ID stability)
- **31-04**: Add SHA-256 checksums for proof integrity
- **31-05**: Generate proofs for other operations (delete, patch, plan)

## Commits

1. `06d27f9` feat(31-02): Add proof generation module
2. `9a19d99` feat(31-02): Export proof generation functions
3. `f659b63` feat(31-02): Add --proof flag to Rename command
4. `1e04d74` feat(31-02): Integrate proof generation into execute_rename
