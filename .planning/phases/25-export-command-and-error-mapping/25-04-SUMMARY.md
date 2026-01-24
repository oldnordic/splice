---
phase: 25-export-command-and-error-mapping
plan: 04
subsystem: cli
tags: [export, tests, integration, csv, jsonl]

# Dependency graph
requires:
  - phase: 25-export-command-and-error-mapping
    plan: 03
    provides: Export data types and execution function
provides:
  - Export command integration tests covering all three formats (json, jsonl, csv)
  - Tests for --file flag behavior (file vs stdout)
affects: []

# Tech tracking
tech-stack:
  added: []
  patterns:
    - TempDir for isolated test environments
    - Command integration tests via std::process::Command
    - get_splice_binary() helper for test binary location

key-files:
  created: []
  modified:
    - src/cli/mod.rs - Fixed Export command field name conflict (output->file)
    - src/main.rs - Updated Export command match arm to use renamed field
    - tests/cli_output_tests.rs - Added 5 export command integration tests

key-decisions:
  - Export command uses --file for output path (not --output to avoid conflict with global --output flag)
  - Export command uses --long-only option (no -o short option) to avoid conflict with global -o
  - Export tests verify actual file output via Command subprocess
  - JSONL format validated with type-tagged records (header, file, symbol)

patterns-established:
  - Integration tests use TempDir for isolated test environments
  - get_splice_binary() helper locates test binary across different build configurations
  - Export tests create actual Magellan databases via MagellanIntegration::open()
  - File-based output validation via std::fs::read_to_string()

# Metrics
duration: 15min
completed: 2026-01-24
---

# Phase 25: Plan 04 - Export Command Tests Summary

**Integration tests for export command covering all three formats (json, jsonl, csv) and output modes**

## Performance

- **Duration:** 15 minutes
- **Started:** 2026-01-24
- **Completed:** 2026-01-24
- **Tasks:** 1
- **Files created:** 0
- **Files modified:** 3

## Accomplishments

- Added 5 integration tests for the export command in tests/cli_output_tests.rs
- Fixed Export command field name conflict (output->file) that prevented CLI from working
- All export tests pass (5/5)

## Task Commits

1. **Task 1: Add export command integration tests** - `6f9014b` (test)

## Export Tests Added

1. **test_export_json_format** - Validates JSON structure:
   - schema_version field present
   - timestamp field present
   - db_path field present
   - data object with files, symbols, references, calls arrays

2. **test_export_jsonl_format** - Validates JSON Lines format:
   - Each line is valid JSON
   - Type-tagged records: header, file, symbol
   - Correct type field values

3. **test_export_csv_format** - Validates CSV format:
   - Section headers (# Files, # Symbols)
   - Column headers (path, hash)

4. **test_export_defaults_to_json** - Validates default format:
   - Without --format flag, defaults to JSON
   - Output file contains valid JSON

5. **test_export_stdout_output** - Validates stdout output:
   - Without --file flag, writes to stdout
   - Contains expected fields (schema_version, files, symbols, status)

## Deviations from Plan

### Rule 3 - Blocking Issue Fixed: Export Command Field Name Conflict

**Found during:** Task 1 - Running export command tests

**Issue:** Export command used `output` as field name, which conflicted with the global `Cli::output` field (OutputFormat type). This caused clap to panic with:
```
Mismatch between definition and access of `output`. Could not downcast to splice::cli::OutputFormat, need to downcast to std::path::PathBuf
```

**Fix:** Renamed Export command field from `output` to `file` and removed the short option `-o` to avoid conflict with global `-o/--output`.

**Files modified:**
- src/cli/mod.rs - Changed Export::output to Export::file, removed short option
- src/main.rs - Updated Commands::Export match arm pattern
- tests/cli_output_tests.rs - Updated test commands to use --file instead of --output

**Commit:** Part of `6f9014b`

## Test Results

All export tests pass:

```
running 6 tests
test export_tests::test_export_csv_format ... ok
test export_tests::test_export_defaults_to_json ... ok
test export_tests::test_export_json_format ... ok
test export_tests::test_export_jsonl_format ... ok
test export_tests::test_export_stdout_output ... ok
test test_response_types_reexported ... ok

test result: ok. 6 passed; 0 failed
```

## Files Created/Modified

- `src/cli/mod.rs` - Fixed Export command field name conflict (output->file, removed -o short option)
- `src/main.rs` - Updated Export command match arm to use renamed field
- `tests/cli_output_tests.rs` - Added export_tests module with 5 integration tests

## Export Command Usage

After this fix, the export command is invoked as:

```bash
# Export to file (JSON format by default)
splice export --db path/to/graph.db --file output.json

# Export to file with specific format
splice export --db path/to/graph.db --format csv --file output.csv
splice export --db path/to/graph.db --format jsonl --file output.jsonl

# Export to stdout
splice export --db path/to/graph.db
```

## Pre-existing Issues

One pre-existing test failure was identified but is unrelated to this plan:
- `tests::test_cli_patch_preview` - Fails with JSON parsing error (existed before this plan)

## Next Phase Readiness

- Phase 25-04 complete - export command fully tested with all three formats
- Export command ready for use
- Field naming conflict resolved
- All success criteria met

## Success Criteria

All Phase 25-04 requirements verified:

- Export command integration tests cover all three formats - Verified (json, jsonl, csv tests pass)
- JSON output produces valid ExportResponse with schema_version - Verified in test_export_json_format
- JSONL output produces type-tagged records - Verified in test_export_jsonl_format
- CSV output produces valid CSV with proper headers - Verified in test_export_csv_format
- Export with --file flag writes to file - Verified in all format tests
- Export without --file writes to stdout - Verified in test_export_stdout_output
- All tests pass without regressing existing functionality - Verified (22/22 cli_output_tests pass)

---
*Phase: 25-export-command-and-error-mapping*
*Plan: 04*
*Completed: 2026-01-24*
