# Changelog

Format based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/).
Project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [2.7.0] - 2026-05-28

### Added

- **`workspace` module** — Library-level workspace detection using magellan's manifest parsers. Three public functions:
  - `find_workspace_root(path)` — walks parents to find project root via manifest files (Cargo.toml, pyproject.toml, go.mod, package.json, tsconfig.json, pom.xml, CMakeLists.txt), with boundary logic (stops at /, /tmp, $TMPDIR, conditional $HOME).
  - `detect_project_language(root)` — returns the primary language ("rust", "python", "go", "javascript", "java", "c") from manifest detection.
  - `detect_include_paths(root)` — delegates to `magellan::manifest::detect_include_paths_from_root` for source directory auto-detection.
- **`SymbolNavigator` integration** — `MagellanIntegration::find_symbol_by_name_sqlite()` now uses magellan's `SymbolNavigator` for O(1) symbol resolution, falling back to O(N) file scan when unavailable.

### Changed

- **`find_workspace_root`** moved from `main.rs` to `splice::workspace` module. Now uses magellan manifest parsers (CargoManifest, PyprojectManifest, GoModuleManifest, etc.) instead of hardcoded marker file names. Added `tsconfig.json` and `CMakeLists.txt` support.
- **magellan dependency** updated from `3.3.9` to `4.2.0`. Major API update picking up `SymbolNavigator`, manifest detection, and service daemon improvements.
- **Geometric backend feature** is now a no-op (`geometric = []`). The `geometric-backend` was removed from magellan 4.x. Existing `#[cfg(feature = "geometric")]` code gates are preserved but inert.

## [2.6.13] - 2026-05-21

### Changed
- **sqlitegraph 3.0.2 → 3.0.3** — Picks up AVX-512 SIMD performance improvements.

## [2.6.12] - 2026-05-19

### Changed

- **sqlitegraph 3.0.1 → 3.0.2** — Picks up V3Backend flush-error handling, PersistentHeaderV3 panic fixes, CliClient stack optimization, and HNSW docstring corrections.

## [2.6.10] - 2026-05-18

### Changed
- Updated Magellan dependency to `3.3.9` (sqlitegraph 3.0.1).
- Updated sqlitegraph dependency to `3.0.1`.

## [2.6.9] - 2026-05-12

### Fixed

- **`splice search` returned 0 matches for all queries** — The `glob` crate does not support `{rs,py,ts,...}` brace expansion. When no `--language` was given, the single glob pattern `"**/*.{rs,py,ts,...}"` matched nothing. Search now iterates per file extension, running one glob per type when no language filter is specified.

- **`splice apply-files` applied immediately with no preview** — Added `--dry-run` flag. When set, `apply-files` reports the number of matches and affected files without writing changes or creating backups. JSON output includes `dry_run`, `files_would_patch`, and `matches_count` fields.

- **`splice snapshots cleanup` silently deleted hundreds of snapshots** — Added `--yes` confirmation flag. Bulk deletions above 50 snapshots now refuse to run unless `--yes` is passed, directing the user to use `--dry-run` first or confirm with `--yes`.

- **`error_code.location` was always `"<unknown>"` for errors without a file path** — `ErrorCode.location` changed from `String` to `Option<String>`. When no file/line/column is available, the `location` field is omitted from JSON output entirely instead of showing `"<unknown>"`.

- **`splice migrate-db` used `--db-path` instead of the project-wide `--db` convention** — The flag now accepts `--db` (short `-d`) matching all other DB-touching commands. `--db-path` still works as the underlying field name.

- **`splice create` returned `status:ok` on validation failure** — When the provided code fails validation, `create` now returns a proper error with `status: "error"` and the compiler diagnostics. Previously, the validation failure was printed to stdout/stderr but the JSON envelope reported success.

- **`splice rename --proof` help said "(requires --preview)" but `--preview` does not exist** — Help text corrected to say "(requires --dry-run)".

- **`splice rename --symbol + --file` returned vague "Invalid argument combination"** — Error message is now specific: `"--file is not needed with --symbol (the symbol ID uniquely identifies the target)"`.

- **SPL-E091 explain text referenced nonexistent `splice ingest` command** — The hint and `splice explain --code SPL-E091` output now reference `magellan watch --root ./src --db <db> --scan-initial` and `magellan refresh --db <db>` instead.

- **Clippy** — 3 `clone_on_copy` warnings in `main.rs` auto-fixed (zero warnings remaining).

## [2.6.8] - 2026-05-12

### Fixed

- **Misleading `"File name too long"` error from `splice patch`** — When the target file lived outside any project directory but a stray `Cargo.toml` existed in an ancestor of `/tmp` (e.g. left over from a debug session), `find_workspace_root` walked all the way up to that stray `Cargo.toml` and picked `/tmp` as the workspace root. `clone_workspace_for_preview` then tried to recursively copy all of `/tmp` into a temp directory, hitting some long path and surfacing the result as `"I/O error for path <unknown>: File name too long (os error 36)"`. Three independent issues compounded:
  - `find_workspace_root` (`src/main.rs`, and the duplicate in `src/resolve/references/rust.rs`) now stops walking before entering `/`, `/tmp`, `$TMPDIR`, or `$HOME` (when the file is outside `$HOME`). Both copies recognise additional project markers (`pyproject.toml`, `package.json`, `go.mod`, `pom.xml`, `build.gradle`, `setup.py`) — not just `Cargo.toml`.
  - `should_skip_entry` in `src/patch/mod.rs` now excludes more build/cache directories (`dist`, `build`, `__pycache__`, `.venv`, `venv`, `.pytest_cache`, `.mypy_cache`, `.tox`, `.next`, `.nuxt`, `.cache`, `.gradle`, `.idea`, `.vscode`) from the preview workspace clone.
  - New `src/io_ext.rs` provides `read`/`read_to_string`/`write` helpers that preserve the file path in `SpliceError::Io`. All bare `?` propagation sites of `std::fs` calls have been migrated (in `patch/mod.rs`, `main.rs`, `plan/mod.rs`, `create.rs`, `verify.rs`, `batch/executor.rs`, `patch/batch_loader.rs`, `resolve/references/rust.rs`, and `graph/router.rs`) — `splice patch`, `splice delete`, `splice plan`, `splice create`, `splice rename`, and `splice batch` now surface the real path in io error messages instead of the literal `<unknown>` placeholder.

- **Multi-language project detection** — `splice patch` no longer requires a Rust project. A file under `pyproject.toml`, `package.json`, `go.mod`, `pom.xml`, `build.gradle`, or `setup.py` is now correctly resolved to its workspace root.

- **`<unknown>` placeholder in `validate_with_cargo` spawn failure** — `validate::validate_with_cargo()` previously used bare `?` on `Command::new("cargo").output()`, so a spawn failure (missing binary or non-existent `project_dir`) surfaced as `"I/O error for path <unknown>: …"`. The error now mentions the real `project_dir`.

- **`<unknown>` placeholder in `patch::validate_utf8_span`** — `patch::validate_utf8_span` previously hard-coded `<unknown>` for the file path in `SpliceError::InvalidSpan`. The function now takes a `file_path: &Path` parameter (breaking API change — no internal callers; external callers must pass the originating file path) and surfaces the real path in the error.

- **`<unknown>` placeholder in `symbol::parser_for_language`** — `parser_for_language()` previously hard-coded `<unknown>` for the file path in `SpliceError::Parse` when `tree_sitter::Parser::set_language` failed. The function now takes a `file_path: &Path` parameter (breaking API change; 3 internal callers updated: `patch::pattern::find_pattern_in_file`, `expand::expand_to_body_with_docs`, `expand::expand_symbol_impl`).

### Changed

- **Removed `From<std::io::Error> for SpliceError`** — The catch-all `From` impl produced `SpliceError::Io { path: PathBuf::from("<unknown>"), source }` for any bare `?` propagation of `io::Error`. The impl is gone, so a bare `?` on `io::Error` no longer compiles in code returning `Result<_, SpliceError>` — every `io::Error` wrap site must now use explicit `map_err` with the originating path. Six call sites were migrated: `commands.rs` (stdin), `patch/mod.rs` (cargo check spawn + `TempDir::new`), `verify.rs` (statvfs CString + last_os_error), and `main.rs` (`env::current_dir`).

- **Removed dead `BatchResult.spec_path` field** — The field was never read anywhere in the codebase; the comment "Set by caller" was inaccurate. Removing it eliminated a stray `PathBuf::from("<unknown>")` placeholder. Breaking change for hypothetical external Rust consumers of `BatchResult`.

- **Clippy cleanup** — Resolved all 338 `cargo clippy --all-targets` warnings (mix of auto-fix and manual): boxed large `Err` variants in `relationships`, hoisted regex compilation out of loops in test data, replaced `sort_by` with `sort_by_key`, replaced `&PathBuf` with `&Path` in several signatures, deleted dead test helper functions, and added `#[allow(..., reason = "...")]` (project-required reason field) for CLI handler argument counts and `Arc<MagellanIntegration>` shared ownership.

## [2.6.7] - 2026-05-12

### Fixed
- **`status` on missing database** — `splice status --db <missing>` previously silently created an empty database and reported 0 files/symbols. Now it returns a JSON error with `SPL-E091` and exits non-zero.
- **Test reliability** — `test_magellan_query_error_preserves_context` now overwrites SQLite header magic bytes instead of truncating, since SQLite is resilient to truncation.

## [2.6.4] - 2026-05-08

### Fixed
- **splice rename definition bug** — `execute_rename()` now injects the definition site (name-only span) into the reference list before grouping. Previously, only call sites were renamed, leaving `fn old_name()` unchanged while callers referenced `fn new_name()`, causing compile errors. The fix computes the name-only byte offset by searching for `symbol_info.name` within the declaration span, avoiding full-declaration replacement that would corrupt the function body. Also removed the `references.is_empty()` guard so zero-caller symbols can still be renamed.
- **splice undo manifest format mismatch** — `restore_from_manifest()` in `src/patch/backup.rs` now supports both `BackupManifest` (patch format, `files: Vec<BackupEntry>`) and `RenameBackupManifest` (rename format, `files: HashMap<String, String>`). Previously, undo could only parse patch-format manifests, causing "expected a sequence" errors when restoring rename backups.

### Changed
- Updated `MANUAL.md` version header to v2.6.4.

## [2.6.3] - 2026-05-06

### Fixed
- Eliminated all compiler warnings (102 → 0).
- Added 85 missing documentation comments across 13 files.
- Removed 3 dead code items (`is_geometric_db` methods, `build_success_payload`).
- Fixed 8 unused imports via `cargo fix`.

## [2.6.2] - 2026-05-04

### Changed
- Updated Magellan dependency to `3.1.9`.
- Removed local path patch for Magellan; now uses published crate from crates.io.

## [2.6.1] - 2026-04-27

### Changed
- Updated Magellan dependency to `3.1.7`.
- Updated sqlitegraph dependency to use the published crate instead of a local path.
- Sanitized public documentation to avoid local benchmark overclaims and stale command flags.

## [2.6.0] - 2026-04-24

### Added

**Import-Aware Code Completion**
Added grounded code completion that suggests symbols from imported modules across files, using Magellan database for zero-hallucination completions.

**Core Features:**
- **Cross-File Symbol Resolution** - Queries Magellan Import entities to resolve imported symbols
- **Source Tracking** - Distinguishes local (`Database`) vs imported (`Imported`) symbols
- **Enhanced Ranking** - Import proximity signal boosts relevance of imported symbols (15% weight)
- **Token Filtering** - Filters suggestions based on partial token being typed
- **Performance Optimized** - Current smoke tests show 3-13ms internal completion query time on the Splice database

**New Modules:**
- `src/completion/mod.rs` (7 files) - Complete import-aware completion system
  - `types.rs` - Completion types with `SuggestionSource::Imported` variant
  - `context.rs` - Context analysis with `get_imported_symbols()` for cross-file resolution
  - `imports.rs` - Import resolution via `ImportResolver` (queries Import entities)
  - `module_index.rs` - Module path to file path mapping
  - `ranking.rs` - Enhanced ranking with `import_proximity` signal
  - `tokenizer.rs` - Token extraction for filtering
  - `engine.rs` - Completion orchestration with source detection

**Documentation:**
- `docs/completion.md` (9.8KB) - Comprehensive user and developer documentation
  - Quick start guide with CLI examples
  - Architecture overview with database schema
  - Performance benchmarks and CLI examples
  - Integration examples (CLI, editor, LSP)
  - Troubleshooting guide

**Testing:**
- `tests/completion_integration.rs` - Integration tests validating import-aware completion
  - Imported symbol suggestions validate cross-file resolution
  - Token filtering validated at different cursor positions
- `tests/completion_bench.rs` - Performance benchmark (100 iterations)
  - Internal query time validated in low milliseconds
  - Symbol count caching avoids repeated `COUNT(*)` queries

**Usage Examples:**
```bash
# Basic completion
splice complete --file src/completion/engine.rs --line 30 --column 8 --max-results 10

# JSON output with metadata
splice complete --file src/lib.rs --line 27 --column 8 --max-results 5 --output json

# Output includes:
# - source: "Imported" (cross-file) or "Database" (same file)
# - source_file: absolute path to defining file
# - via_import: import statement (e.g., "use crate::completion::types")
# - grounded_in: database IDs proving symbol exists
```

**Benefits:**
- LLMs can provide grounded completions (no hallucinations)
- Suggests symbols from any imported module
- Fast internal query performance suitable for interactive tooling
- Every suggestion includes database IDs for verification

**Performance Optimization:**
- Cached symbol count in `CompletionEngine` (avoids repeated `COUNT(*)` queries)
- Avoids repeated symbol count queries during completion
- Single database query per completion request
- Graceful degradation if import resolution fails

**Database Integration:**
- Uses Magellan `graph_entities` table for imports and symbols
- Uses Magellan `graph_edges` table (edge_type='IMPORTS')
- Parses JSON `data` field for import metadata
- Resolves cross-file symbol relationships

## [2.6.0] - 2026-04-13

### Added

**File Creation Command**
Added `splice create` command for creating new Rust files with validation, enabling safer automated code generation.

**Core Features:**
- **rustc Snippet Validation** - Validates Rust code with `rustc --emit=metadata` before writing to disk
- **Atomic File Operations** - Either file exists with correct content, or not at all (no partial writes)
- **Validate-Only Mode** - Check code validity without creating files (`--validate-only`)
- **Module Declaration Insertion** - Automatically add `mod name;` to parent modules (`--with-mod`)
- **Overwrite Protection** - Prevents accidental file overwrites
- **Automatic Directory Creation** - Creates parent directories as needed
- **Multiple Output Formats** - Support for human, json, and pretty output

**New Modules:**
- `src/validate/mod.rs` — Standalone Rust snippet validation using rustc (for `create` command)
- `src/write.rs` (179 lines) - Atomic file writing utilities
- `src/create.rs` (331 lines) - File creation logic with validation
- `src/commands.rs` (179 lines) - Command handlers for create operation
- `src/cli/tests.rs` (89 lines) - CLI integration tests

**Usage Examples:**
```bash
# Create a new file with validation
echo 'pub fn hello() -> String { "Hello".to_string() }' | splice create --file src/hello.rs

# Validate without writing
echo 'pub fn test() -> i32 { 42 }' | splice create --file src/test.rs --validate-only

# Create with module declaration
echo 'pub fn handler() -> i32 { 200 }' | splice create --file src/api/handler.rs --with-mod

# JSON output
echo 'pub fn test() -> i32 { 42 }' | splice create --file test.rs --output json
```

**Benefits:**
- LLMs can validate code before writing (no build-test-fail cycles)
- Atomic operations prevent file corruption
- Immediate feedback from rustc
- Graceful degradation (works without rustc installed)

**Testing:**
- Added 32 new tests (all passing)
- Total test suite: 491 tests passing (32 new + 459 existing)
- 100% test coverage on new code

### Changed

**Database Path Updates**
Updated default database paths from `.codemcp/codegraph.db` to `.magellan/magellan.db` for consistency with Magellan/Mirage toolchain.

**Command Dependencies**
- splice now integrates with Magellan (code indexing)
- splice now integrates with Mirage (CFG analysis)
- Uses shared `.magellan/magellan.db` database

## [2.5.5] - 2026-03-19

### Fixed

**Patch Command SQLite Backend Fix**
Fixed two critical issues that prevented the `splice patch` command from working with SQLite backend:

1. **Missing `kind` Property in Symbol Data** (`graph/sqlite_impl.rs`)
   - `store_symbol()` and `store_symbol_with_file_and_language()` now include `"kind": kind` in the data JSON
   - Previously `kind` was only stored in the SQL-level `node.kind` field, but `resolve_symbol_in_file()` looked for it in `node.data`
   - This caused "Missing kind property" error when patching any symbol

2. **Cargo Check Timeout** (`patch/mod.rs`)
   - `gate_cargo_check()` now uses `thread::spawn()` with `recv_timeout(120s)` instead of blocking indefinitely
   - Prevents `splice patch` from hanging when cargo check takes too long on large projects
   - Returns clear error message: "cargo check timed out after 120 seconds"

## [2.5.4] - 2026-02-27

### Fixed

**All TODOs Completed**
This release completes all 7 pending TODO items from previous versions:

1. **Max Complexity Computation** (`proof/generation.rs`)
   - Implemented `max_complexity` calculation in graph snapshots
   - Computed as `fan_out + 1` for each symbol, taking the maximum
   - Fallback logic: 1 for non-empty graphs, 0 for empty

2. **Symbol ID → Node ID Mapping** (`proof/storage.rs`)
   - Implemented edge restoration during snapshot import
   - Tracks mapping from snapshot symbol IDs to database node IDs
   - Edges restored using `EDGE_CALLS` type
   - `edges_restored` count now reflects actual inserted edges

3. **Import Relationship Queries** (`relationships/mod.rs`)
   - Implemented `get_imports()` using language-specific import extraction
   - Supports Rust, Python, C/C++, JavaScript, TypeScript, Java
   - Returns empty results for unknown file types
   - Uses tree-sitter parsers for on-demand extraction

4. **Export Relationship Queries** (`relationships/mod.rs`)
   - Added placeholder with documented architectural limitations
   - Requires CodeGraph API additions for full implementation

5. **Python Import Resolution** (`resolve/cross_file.rs`)
   - Implemented Python module path resolution
   - Handles: `import module`, `from module import name`, relative imports
   - Resolves to `module.py` or `module/__init__.py`
   - Supports `.` (current), `..` (parent), `...` (ancestor) relative imports

6. **C/C++ Include Resolution** (`resolve/cross_file.rs`)
   - Implemented `#include "header.h"` local include resolution
   - Searches in same directory as current file
   - System headers (`#include <header.h>`) return None

7. **TypeScript-Specific Tests** (`ingest/javascript.rs`)
   - Added 7 tests using tree-sitter-typescript parser
   - Tests: interfaces, type aliases, enums, namespaces, typed functions, classes with implements, interface extends

### Test Coverage

- Previous: 415 tests passing
- Current: 422 tests passing (+7 TypeScript tests)

## [Unreleased]

### Fixed

**Max Complexity Computation (TODO #2)**
- Implemented `max_complexity` calculation in `proof/generation.rs`
- Computed as `fan_out + 1` for each symbol, taking the maximum across all symbols
- Falls back to 1 for non-empty graphs, 0 for empty graphs

**Symbol ID → Node ID Mapping (TODO #3)**
- Implemented edge restoration during snapshot import in `proof/storage.rs`
- Tracks mapping from snapshot symbol IDs (String) to database node IDs (i64)
- Edges are now properly restored using the mapped node IDs
- `edges_restored` count now reflects actual inserted edges

**Import Relationship Queries (TODO #4a)**
- Implemented `get_imports()` in `relationships/mod.rs`
- Uses language-specific import extraction from tree-sitter parsers
- Supports Rust, Python, C/C++, JavaScript, TypeScript, Java
- Returns empty results for unknown file types

**Export Relationship Queries (TODO #4b)**
- Added placeholder implementation for `get_exports()` in `relationships/mod.rs`
- Current limitation: CodeGraph doesn't expose public API for file/symbol queries
- Documented requirements for full implementation

**Python Import Resolution (TODO #5)**
- Implemented Python module path resolution in `resolve/cross_file.rs`
- Handles `import module`, `from module import name`, and relative imports
- Resolves to `module.py` or `module/__init__.py` paths
- Supports relative imports with `.` (current), `..` (parent), `...` (ancestor)

**C/C++ Include Resolution (TODO #6)**
- Implemented C/C++ local include resolution in `resolve/cross_file.rs`
- Handles `#include "header.h"` by looking in the same directory
- System headers `#include <header.h>` return None (not indexed)

**TypeScript-Specific Tests (TODO #7)**
- Added 7 TypeScript-specific tests using tree-sitter-typescript parser
- Tests cover: interfaces, type aliases, enums, namespaces, typed functions, classes with implements, interface extends
- All tests pass and validate TypeScript construct extraction

### Changed

**Dependencies**
- Added `libc = { version = "0.2", optional = true }` for Unix disk space API

### All TODOs Completed

All 7 pending TODOs have been addressed:
1. ✅ Max complexity computation
2. ✅ Symbol ID → node ID mapping
3. ✅ Import relationship queries
4. ✅ Export relationship queries (documented limitations)
5. ✅ Python import resolution
6. ✅ C/C++ include resolution
7. ✅ TypeScript-specific tests

## [2.5.3] - 2026-02-20

### Updated
- Update sqlitegraph from 2.0.3 to 2.0.7 (bug fixes)

## [2.5.2] - 2026-02-14

### Changed
- **Documentation:** Updated MANUAL version to 2.5.2

## [2.5.1] - 2026-02-14

### Changed
- **Backend:** Retired the experimental binary backend path in favor of SQLite
  - Feature flag removed; SQLite remains the supported default backend
  - Dependencies updated: magellan 2.2 → 2.4.3, sqlitegraph 1.5.7 → 2.0.3
  - Full feature parity with SQLite backend

### Fixed
- **Tests:** Fixed `cleanup_old_snapshots` to properly sort snapshots by timestamp
- All lib tests now pass (409 tests)

### Documentation
- Rewrote README in concise format (100 lines)

## [2.5.0] - 2026-02-10

### Experimental Binary Backend Support — Historical Dual Backend Architecture

This historical milestone explored dual backend support with SQLite as default and an opt-in binary backend, including snapshot-based rollback, impact visualization, and batch multi-file operations. The binary backend path has since been retired; current Splice uses SQLite plus optional geometric analysis.

### Added

**Backend Feature Flags (Phase 33)**
- Compile-time feature flags for backend selection: `--features sqlite` (default) or the now-retired binary backend experiment
- Mutual exclusion enforcement: build fails if both backends enabled simultaneously
- Public API stability across both backend variants
- Platform-specific builds: `--features windows` for Windows, `--features unix` (default)

**Backend Detection & Migration (Phase 34)**
- `splice status --detect-backend` flag to detect database backend format
- Historical `splice migrate` command to convert SQLite databases to the experimental binary format
- 5-step migration workflow with progress reporting
- Migration verification and rollback on failure
- Clear error messages when opening unsupported backend databases

**Snapshots & Verification (Phase 35)**
- `--snapshot-before` flag on `splice patch` and `splice rename` to capture graph state
- `splice verify` command to compare two snapshots and detect changes
- Snapshots stored in `.splice/snapshots/` with timestamp filenames
- `splice snapshots` subcommands: list, delete, cleanup
- Historical binary backend snapshot restore capability

**Advanced Features (Phase 36)**
- `--impact-graph` flag on `splice rename --preview`, `splice reachable`, `splice refs` for DOT graph visualization
- `splice batch` command for multi-file refactoring from YAML specification
- Batch operations with automatic rollback on failure
- Transaction-based execution with `--rollback` mode (on-failure, never, always)
- YAML spec schema supporting: patch, delete, rename, pattern-replace operations

**Testing Infrastructure (Phase 37)**
- `./scripts/test-all.sh` for backend testing (no CI required)
- Feature-gated tests for backend-specific functionality
- Migration integration tests with verification
- Sequential testing to avoid database conflicts

**Documentation (Phase 38)**
- "Which Backend Should I Use?" decision guide in README.md
- Backend comparison table: 11 aspects (feature flags, format, size, performance)
- Feature availability matrix: 6 features compared across backends
- Installation documentation for SQLite and the historical binary backend variant
- Migration workflow documentation in MANUAL.md

### Fixed

**Historical Binary Backend Persistence Query**
- `all_symbol_names()` now queries database directly via `backend.entity_ids()` and `backend.get_node()` instead of only reading in-memory cache
- `find_symbols_by_name()` now queries database directly instead of only reading in-memory cache
- File nodes (kind "File" and "file") are filtered out from symbol query results
- Added `backend.flush()` call after write operations for data safety
- Root cause: the experimental binary backend was correctly persisting data, but splice's query methods only searched in-memory `symbol_cache`

**Dependencies**
- Updated to sqlitegraph 1.5.7 (KV recovery and flush fixes)
- Updated to magellan 2.2 (alignment with sqlitegraph)

### Changed

- New databases created with the historical binary feature enabled used the binary format by default
- Snapshot format: SQLite uses `snapshot.json`; the historical binary backend used `export.manifest`
- Backend detection via file header magic

### Performance

- Historical binary backend: 2-100x faster for large codebases (100K+ LOC)
- Historical binary database size: ~70% smaller than SQLite
- Symbol lookup: O(1) KV get vs SQLite table scan O(n)

### Known Issues

- Migration snapshot format incompatibility: SQLite's `snapshot_export()` creates `snapshot.json` but the historical binary import path expected `export.manifest`

**One sentence for the docs:**
> v2.5.0 explored a binary backend alongside SQLite; current Splice has retired that path and uses SQLite databases with optional geometric analysis.

## [2.4.1] - 2026-02-04

### Added
- **Windows Support:** Full cross-platform compatibility via explicit feature flag
  - Use `--features windows` to enable Windows builds
  - Default: `--features unix` (Linux/macOS)
  - Platform detection centralized in `platform.rs` module
  - splice is fully functional on Windows (all features work identically)

### Changed
- Feature model: `default = ["unix"]`, `windows` opt-in
- Updated magellan dependency to 2.1 (path for local development)

**One sentence for the docs:**
> Windows support is opt-in via `--features windows`. Fully supported — all features work identically across platforms.

## [2.4.0] - 2026-02-04

### Fixed

**Preview Mode JSON Output**
- `splice patch --preview --json` now returns structured JSON response
  - `data.preview_report` with full PreviewReport struct (file, line_start, line_end, lines_added, lines_removed, bytes_added, bytes_removed)
  - `data.files` array with affected file entries
  - `data.symbol` at top level for symbol name
  - Consistent with CliSuccessPayload pattern used by other commands

### Changed

**Documentation**
- Updated README.md following llmgrep format with badges, toolset diagram, and references
- Rewrote MANUAL.md (condensed from 1654 to 342 lines)
- Added comprehensive command reference and examples
- Updated .gitignore to only track README.md, CHANGELOG.md, MANUAL.md

## [2.3.0] - Unreleased

### Magellan v2 Integration — Cross-File Rename and Semantic Program Transformation

This release delivers Magellan v2 integration with cross-file rename, impact analysis, dead code detection, cycle detection, graph condensation, program slicing, and proof-based refactoring. Five phases with 25 plans bring advanced semantic program transformation capabilities to Splice.

### Added

**Preview Mode JSON Output**
- `splice patch --preview --json` now returns structured JSON response
  - `data.preview_report` with full PreviewReport struct (file, line_start, line_end, lines_added, lines_removed, bytes_added, bytes_removed)
  - `data.files` array with affected file entries
  - `data.symbol` at top level for symbol name
  - Consistent with CliSuccessPayload pattern used by other commands

This release delivers Magellan v2 integration with cross-file rename, impact analysis, dead code detection, cycle detection, graph condensation, program slicing, and proof-based refactoring. Five phases with 25 plans bring advanced semantic program transformation capabilities to Splice.

### Added

**Cross-File Rename**
- `splice rename` command with byte-accurate reference replacement across files
  - Uses Magellan ReferenceFact byte offsets for precise span targeting
  - `--symbol <id>` for symbol identification (16-char hex V1 or 32-char hex V2)
  - `--file <path>` for definition file location
  - `--to <new_name>` for the new symbol name
  - `--preview` flag for dry-run mode with colored diff output
  - `--proof` flag for generating refactoring proofs
  - Automatic backup and rollback on validation failures
  - UTF-8 boundary validation for safe multi-byte character handling
  - Multi-language support (C, C++, Java, JavaScript, Python, TypeScript)
  - SPL-E040 error code for ambiguous symbol detection
  - Sorted references by (file_path, byte_start descending) for safe in-order replacement

**Impact Analysis Commands**
- `splice reachable` — Show all symbols reachable from a target symbol
  - `--symbol <name>` and `--path <file>` for target specification
  - `--direction <forward|backward>` for traversal direction
  - `--max-depth <n>` for depth limiting (default: 10)
  - BFS traversal with visited set to avoid cycles
  - Affected file identification

- `splice slice` — Forward or backward program slicing
  - `--target <id>` for symbol ID
  - `--direction <forward|backward>` for slice direction
  - `--max-distance <n>` for maximum slice distance
  - Distance tracking and affected file analysis

**Dead Code Detection**
- `splice dead-code` command for finding unused symbols from entry points
  - `--entry <symbol>` and `--path <file>` for entry point specification
  - `--exclude-public` flag to exclude public symbols from analysis
  - BFS traversal from entry point with visited tracking
  - Public symbol detection heuristics (uppercase first char for Rust functions, kind-based for types)

**Cycle Detection**
- `splice cycles` command for finding circular dependencies in call graph
  - Tarjan's SCC algorithm implemented directly in MagellanIntegration
  - `--max-cycles <n>` for limiting output (default: 100)
  - `--show-members` flag for displaying all cycle members
  - Cycles defined as SCCs with size > 1 OR self-loops
  - Representative symbol selected as alphabetically first member

**Graph Condensation**
- `splice condense` command for collapsing SCCs to DAG
  - Kahn's algorithm for topological level assignment
  - Edge weight tracking between SCCs (coupling strength)
  - `--show-levels` flag for topological level display
  - `--show-members` flag for SCC member display

**Proof-Based Refactoring**
- `splice rename --proof` flag for generating machine-checkable proofs
  - Before/after graph snapshots with symbol counts
  - Invariant validation (reference counts, orphan detection, ID stability, entry points)
  - SHA-256 checksums for audit trail integrity
  - `splice validate-proof --proof <path>` command for proof validation
  - RefactoringProof, GraphSnapshot, InvariantCheck data structures
  - JSON proof file format with version "1.0.0"

**Dual-Format SymbolId Support**
- V1: 16-character hex (SHA-256, first 8 bytes) — backward compatible
- V2: 32-character hex (BLAKE3, first 16 bytes) — new default
- `SymbolId::parse()` for auto-detection (16 or 32 chars)
- `id_format` field in JSON output ("v1" or "v2")
- Backward compatibility: find_symbol_by_id tries V2 first, then V1

**Magellan Database Migration**
- Magellan 2.0.0 auto-migration support (v5 to v6 schema)
- BLAKE3-based symbol IDs for new databases
- `--backup` flag defaults to true for migration safety
- `--dry-run` mode for migration status checking

### Changed

**Dependencies**
- Magellan 2.1.0 (was 2.0.0) — BLAKE3 symbol IDs, ReferenceFact byte offsets
- SQLiteGraph 1.4.2 (was 1.2.7) — BLAKE3-backed symbol ID support
- sha2 0.10.9 retained for backward compatibility during migration
- blake3 1.5.0 for new symbol ID generation

**Test Coverage**
- 407+ passing tests (from 334 in v2.2.4)
- 18 cross-file rename integration tests with multi-language coverage
- 6 performance regression tests for graph algorithms (<1s target for 1K symbols)
- 3 integration tests for invariant validation

**Documentation**
- Updated README with v2.3 feature highlights
- Extended manual with cross-file rename and graph algorithm documentation
- Three new example files: rename_examples.md, graph_algorithm_examples.md, proof_examples.md
- CI/CD integration patterns and troubleshooting sections

### Performance

Graph algorithm performance benchmarks (Criterion):
- Reachability (1K symbols): 5-15ms
- Dead code detection (1K symbols): 10-25ms
- Cycle detection (1K symbols): 20-40ms
- Graph condensation (1K symbols): 15-35ms
- Program slicing (1K symbols): 25-60ms

All algorithms meet the <1s target for 1K symbols.

### Technical Notes

- **Cross-file rename**: Uses Magellan ReferenceFact byte offsets for exact span targeting. Rust reference extraction not yet available in Magellan 2.0.0 — tests use manual span detection as workaround.
- **Graph algorithms**: Implemented directly in MagellanIntegration using in-process library delegation (not subprocess).
- **Proof format**: Versioned schema "1.0.0" with before/after snapshots, invariant checks, and checksums.
- **Migration**: Magellan 2.0.0 databases auto-migrate on open. V5 databases use SHA-256, V6 use BLAKE3.

## [2.2.4] - 2026-02-04

### Removed

**Dead Code Cleanup**
- Removed unused `Ingestor` struct from `src/ingest/mod.rs`
  - This was an abandoned design for custom CodeGraph-based indexing
  - Replaced by Magellan integration (v0.5.0) which provides production-quality multi-language code graph
  - Use `MagellanIngestor` or `extract_symbols()` instead
  - No functional changes - the struct was never implemented

### Changed

**Documentation**
- Updated `.planning/codebase/ARCHITECTURE.md` to remove Ingestor references
- Clarified that `MagellanIngestor` is the preferred ingestion API
- Closed TODO items related to Ingestor cleanup

## [2.2.3] - 2026-02-02

### Fixed

**SQLiteGraph 1.2.7 MVCC API Compatibility**
- All `get_node()` calls now include `SnapshotId(0)` parameter for latest committed state
- All `neighbors()` calls now include `SnapshotId(0)` parameter for latest committed state
- Fixes compatibility with sqlitegraph 1.2.7's snapshot isolation system

### Changed

**Dependencies**
- sqlitegraph 1.2.7 (was 1.2.5) - requires explicit SnapshotId parameter for graph queries

### Technical Notes

- **Affected files**: `src/graph/mod.rs` (lines 277, 328) and `src/relationships/mod.rs` (lines 201, 267, 318, 340, 377)
- **SnapshotId(0)** represents the latest committed snapshot in sqlitegraph's MVCC system
- All graph queries now explicitly specify snapshot isolation level
- No functional changes to CLI output or behavior

## [2.2.2] - 2026-01-24

### Magellan Integration — Unified CLI Interface

This release delivers a unified CLI interface where Splice provides both Magellan query commands and span-safe editing through a single tool. Five phases with 24 plans bring comprehensive code graph query capabilities alongside Splice's refactoring features.

### Added

**Query Commands (Magellan Delegation)**
- `splice status` — Display database statistics (files, symbols, references, calls, code_chunks counts)
- `splice query` — List symbols in a file with optional context/relationship flags
- `splice find` — Locate symbols by name or symbol_id with disambiguation support
- `splice refs` — Show callers/callees for a symbol with bidirectional traversal
- `splice files` — List indexed files with optional symbol counts per file
- `splice export` — Export graph data in JSON, JSONL, or CSV format

**CLI Enhancements**
- `--output` flag supports human (default), json, and pretty formats
- `--db` flag specifies database path and delegates to Magellan
- Exit codes match Magellan conventions (0=success, 1=error, 2=usage, 3=database, 4=file not found, 5=validation)
- Command categories in help (Query, Edit, Export, Validation)

**Data Format Alignment**
- Symbol IDs use 16-character hex format (SHA-256 hash, first 8 bytes)
- Execution IDs use {timestamp_hex}-{pid_hex} format for delegated queries
- Field name translation between Magellan (start_line) and Splice (line_start) conventions
- Response types: StatusResponse, FindResponse, RefsResponse, FilesResponse

**Error Handling**
- SPL-E091 Magellan error code with original error preserved in chain
- anyhow::Error source preservation for complete error context

**Documentation**
- Comprehensive Magellan integration guide (docs/magellan_integration.md — 1030 lines)
- Editor/agent usage patterns and workflow examples
- Performance benchmarks and characteristics

### Changed

**Dependencies**
- Added `csv = "1.3"` for CSV export format
- Added `anyhow = "1.0"` for error chain preservation

**Test Coverage**
- 21 new integration tests covering commands, formats, errors, agent workflows, and performance
- All query commands validated end-to-end
- Export format validation tests for JSON, JSONL, CSV

### Technical Notes

- **Single-Tool Workflow**: LLMs can now discover and edit code using one tool
- **In-Process Delegation**: Uses Magellan as library, not subprocess
- **Export Schema**: Versioned schema (1.0.0) with files, symbols, references, calls

## [2.2.1] - 2026-01-24

### Code Quality & Bug Fixes

This release addresses 67 issues identified in comprehensive bug analysis, improving code reliability and safety.

### Fixed

**Critical Error Handling**
- Eliminated unwrap() panic paths in symbol resolution, parser creation, and file loading
- Proper error propagation throughout all language modules

**Lifetime & Resource Safety**
- Fixed 'static lifetime abuses in parser creation
- Improved UTF-8 handling across all language modules
- Removed excessive clone() patterns

**API Consolidation**
- Merged duplicate parser creation APIs
- Unified import extraction implementations
- Consolidated resolve_symbol variants

### Changed

- 20 plans across 3 phases
- 70 files created/modified
- Improved testability with configurable execution logging

## [2.2.0] - 2026-01-23

### Unified JSON & Editor Optimization

This release introduces unified JSON schema across all code intelligence tools with rich span extensions optimized for agent consumption and human-friendly CLI improvements.

### Added

**Rich Span Extensions**
- Context fields for all operations (file_path, symbol_name, language)
- Semantic kind labels (function, method, class, struct, etc.)
- Checksums for change verification (SHA-256)
- Structured error codes with SPL-E### format

**Rich Span Advanced**
- Relationships (callers, callees, imports, exports)
- Tool hints for safe operations
- Suggested actions for common workflows

**CLI Conventions**
- `-n` dry-run flag for preview mode
- `-A`/`-B`/`-C` context flags (lines before/after/around)
- Unified diff output with TTY colors
- Git-style exit codes

**Enhanced Errors**
- Severity levels (error, warning, info)
- Precise locations (file, line, column)
- Fuzzy symbol suggestions
- `splice explain` command for error details

**Symbol Expansion**
- AST-aware parent chain walking
- Multi-level expansion with `--expand-level`
- 6 language expanders (Rust, Python, C, C++, Java, JavaScript, TypeScript)

**Search & Apply**
- `splice search --pattern` with glob filtering
- Atomic find-and-replace with rollback capability

### Changed

- 8 phases, 55 plans
- 340+ tests passing
- Magellan alignment for unified CLI

## [2.0.0] - 2026-01-18

### Major Release - Comprehensive Overhaul

This release represents a complete overhaul of Splice's internal architecture, safety, and observability. The codebase has been refactored across 10 phases with 31 individual plans, 133 commits, and significant improvements to code quality and test coverage.

### Added

**Safety Foundation**
- Eliminated all `unwrap()` calls from production code paths
- Established consistent error handling patterns using `?` operator
- Added context helpers for better error messages

**Structured Output & Identifiers**
- All operations now return structured JSON with explicit fields
- `execution_id` (UUID): Unique identifier for each operation run
- `match_id`: Unique identifier for each symbol match
- `span_id`: Unique identifier for each span operation
- Line and column coordinates included in all output (byte + line/col)
- Deterministic ordering across all operations (sorted results)

**Validation & Observability**
- SHA-256 checksums for pre/post-operation verification
- Execution logging audit trail in `.splice/operations.db`
- `splice log` command to query execution history
- Filters: operation_type, status, date_range, execution_id
- Output formats: table (human), JSON (machine)
- Statistics summary option

**Integration Tests**
- 26 Magellan integration tests (all 7 languages)
- 18 cross-language compatibility tests
- End-to-end refactoring workflow tests
- Total: 215+ passing tests

### Changed

**Dependencies**
- SQLiteGraph upgraded from 0.2.11 to 1.0
- Magellan upgraded to 0.5.3 with experimental binary backend features
- tree-sitter upgraded from 0.21 to 0.22
- Added `uuid` for execution ID generation
- Added `chrono` for timestamp handling
- Added `rusqlite` for execution log database access
- Added `sha2` for checksum computation

**Architecture**
- New modules:
  - `src/execution/` - Execution logging infrastructure
  - `src/output/` - Structured output types
  - `src/checksum.rs` - SHA-256 checksum computation
  - `src/verify.rs` - Validation hooks
- Refactored error handling throughout codebase
- Improved test organization and coverage

### Technical Notes

- **Breaking Change**: JSON output format now includes new required fields (execution_id, match_id, span_id)
- **Databases**: `.splice/operations.db` created for audit trail (separate from codegraph.db)
- **Test Status**: 215+ tests passing. Some existing tests have known failures due to hardcoded paths in test fixtures.
- **Backward Compatibility**: Existing codegraph.db files remain compatible

### Migration from v0.5.x

- JSON output now includes additional identifier fields
- Execution logging is enabled by default (set `SPLICE_EXECUTION_LOG=false` to disable)
- Validation hooks run automatically (can be skipped with `--no-validate` for apply-files)

## [0.5.3] - 2026-01-13

### Fixed

- Handle broken pipe when stdout closes early (e.g., piping `splice query` to `head`)

### Changed

- Bumped Magellan dependency to 0.5.3

## [0.5.0] - 2026-01-02

### Added

- **Magellan v0.5.0 integration**: Complete code indexing and label-based symbol discovery
  - `splice query --db <FILE>` command for querying symbols by labels
  - `splice get --db <FILE> --file <PATH> --start <N> --end <N>` command for retrieving code chunks
  - Multi-label queries with AND semantics (e.g., `--label rust --label fn` for all Rust functions)
  - `--list` flag to show all available labels with entity counts
  - `--count` flag to count entities with specified label(s)
  - `--show-code` flag to display source code without re-reading files
  - `src/graph/magellan_integration.rs` with MagellanIntegration wrapper
  - `src/ingest/magellan.rs` with MagellanIngestor for multi-language indexing
  - Code chunk storage during indexing for fast retrieval
  - Label queries: language labels (rust, python, etc.) and symbol kind labels (fn, struct, class, etc.)

### Changed

- **334 passing tests** (from 368 - test count adjustment)
- Updated sqlitegraph dependency to 0.2.11
- Ingest module now uses Magellan's parsers instead of "Not implemented yet"

### Technical

- New modules: `src/graph/magellan_integration.rs` (203 LOC), `src/ingest/magellan.rs` (76 LOC)
- New dependency: `magellan = "0.5.0"`
- Magellan provides working multi-language parsers (7 languages)
- Code chunks stored with byte spans eliminate need to re-read source files during refactoring
- Labels assigned automatically during indexing: language + symbol kind

## [0.4.1] - 2025-12-31

### Added

- **Batch patch API**: Apply multiple patches across multiple files in a single atomic operation
  - `splice patch --batch <file.json>` for JSON-based batch operations
  - Atomic rollback if any patch in the batch fails validation
  - Per-file hash tracking for audit trails
  - Batch specification documented in `docs/BATCH_PATCH_SPEC.md`

- **Preview mode**: Dry-run functionality to inspect changes before applying
  - `--preview` flag for `patch` command
  - Clones workspace, applies changes, runs validation, reports stats
  - Workspace remains untouched on preview
  - Preview flow documented in `docs/PREVIEW_FLAG.md`

- **Backup and undo support**: Create backups and restore from them
  - `--create-backup` flag for `patch`, `delete`, and `apply-files` commands
  - `--operation-id <ID>` for custom operation tracking (auto-generated UUID if not provided)
  - `splice undo --manifest <path>` command to restore from backup
  - Backups stored at `.splice-backup/<operation-id>/` with SHA-256 hash verification
  - `BackupWriter`, `BackupManifest`, and `restore_from_manifest` in `src/patch/backup.rs`

- **Operation metadata tracking**: Attach metadata to operations for auditing
  - `--metadata <JSON>` flag for optional metadata attachment
  - Response payloads include `operation_id`, `span_ids`, `metadata`, `files_modified`
  - File hash tracking (`before_hash`, `after_hash`) for all patch operations

- **Multi-file pattern replacement**: AST-confirmed find/replace across files
  - `splice apply-files --glob <pattern> --find <text> --replace <text>` command
  - Glob-based file discovery (e.g., `tests/**/*.rs`, `src/**/*.py`)
  - AST confirmation ensures replacements land in valid code locations
  - Comment filtering (skips matches in comments unless pattern starts with `//`)
  - `src/patch/pattern.rs` with `find_pattern_in_files()` and `apply_pattern_replace()`

- **Structured error responses**: Complete tool metadata in diagnostics
  - Tool path and version tracking (`cargo --version`, compiler versions)
  - Remediation links for common errors
  - Full diagnostic payload: `{tool, level, file, line, column, message, code, note, tool_path, tool_version, remediation}`
  - Documented in `docs/DIAGNOSTICS_OUTPUT.md`

### Changed

- **368 passing tests** (from 339)
- Updated all documentation for new features
- Patch module extended with batch, pattern, and backup capabilities
- CLI responses now include operation metadata for all commands

### Technical

- New modules: `src/patch/backup.rs` (437 LOC), `src/patch/batch_loader.rs`, `src/patch/pattern.rs` (347 LOC)
- New dependencies: `uuid = "1.10"`, `chrono = "0.4"`, `glob = "0.3"`
- Removed unnecessary `unsafe` block from tree-sitter language getter
- Response payloads standardized across all commands

## [0.4.0] - 2026-01-01

### Added
- Documentation: `docs/DIAGNOSTICS_HUMAN_LLM.md` explains the CLI diagnostics JSON contract, shows how rust-analyzer output is normalized, and references the per-language validator outputs so humans and LLMs use the same structured payload.

## [0.3.1] - 2025-12-31

### Fixed
- **Rust impl blocks now extract struct name** - `impl_item` nodes now properly extract the struct name
  - Previously: Used `child_by_field_name("name")` which doesn't exist for `impl_item`, causing impl blocks to be skipped
  - Now: Uses `child_by_field_name("type")` to extract the struct name being implemented
  - Works for both `impl StructName { }` and `impl Trait for StructName { }`

### Added
- `extract_impl_name()` helper function to Rust parser
- 3 new tests: `test_extract_impl_name_inherent`, `test_extract_impl_name_trait_impl`, `test_extract_impl_name_both`

## [0.3.0] - 2025-12-30

### Added

- **Multi-language patch support**: Full patch command now works on all 7 supported languages
- **Multi-language delete support**: Basic delete (definition-only) for non-Rust languages
- **Language auto-detection**: Automatic language detection from file extensions
- **--language flag**: Optional language override for CLI commands
- **Python validation**: `python -m py_compile` gate for Python files
- **C/C++ validation**: `gcc`/`g++ -fsyntax-only` gates for C/C++ files
- **Java validation**: `javac` compilation gate for Java files
- **JavaScript validation**: `node --check` gate for JavaScript files
- **TypeScript validation**: `tsc --noEmit` gate for TypeScript files
- **Extended symbol kinds**: Added method, class, interface, constructor, variable, type-alias kinds

### Changed

- **339 passing tests** (from 298)
- Updated all documentation for multi-language support
- Patch module now language-aware with language-specific compiler validation
- Graph schema uses language-agnostic labels (e.g., `symbol_function` instead of `rust_function`)

### Supported Languages

| Language | Extensions | Delete | Patch | Validation |
|----------|-----------|--------|-------|------------|
| Rust | `.rs` | Full | Full | `cargo check` |
| Python | `.py` | Basic | Full | `python -m py_compile` |
| C | `.c`, `.h` | Basic | Full | `gcc -fsyntax-only` |
| C++ | `.cpp`, `.hpp`, `.cc`, `.cxx` | Basic | Full | `g++ -fsyntax-only` |
| Java | `.java` | Basic | Full | `javac` |
| JavaScript | `.js`, `.mjs`, `.cjs` | Basic | Full | `node --check` |
| TypeScript | `.ts`, `.tsx` | Basic | Full | `tsc --noEmit` |

### Technical

- Language-specific tree-sitter parsers for all 7 languages
- Multi-language validation gates with compiler-specific error parsing
- Language detection from file extensions with manual override
- Symbol kind mapping across all languages

## [0.2.2] - 2025-12-30

### Changed

- Clarified documentation: CLI commands are Rust-only; parsers for other languages are library-use/future
- Fixed README: correctly identify rust-analyzer as LSP, not IDE

## [0.2.1] - 2025-12-30

### Changed

- Clarified delete command guarantees: workspace scope, exclusions, invariants
- Added Feedback section to README

## [0.2.0] - 2025-12-30

### Added

- **delete command**: Remove symbol definitions and all their references
- **Cross-file reference finding**: Tracks and removes references across multiple files
- **Shadowing detection**: Correctly handles local variables that shadow imported symbols
- **Re-export chain following**: Finds references through `pub use` re-exports (single-hop; chained not guaranteed)
- **Trait method reference detection**: Handles `value.method()`, `Trait::method()`, and `Type::method()` patterns
- **Multi-language support**: Import extraction for C/C++, Java, JavaScript, Python, TypeScript

### Behavior Guarantee (delete command)

For **public Rust functions** (those intended to be imported across modules), `delete` finds all references that reach the definition through:
- Direct imports: `use crate::foo::bar`
- Re-exports: `pub use crate::foo::bar as baz`
- Same-file unqualified calls: `bar()`

**Scope**: workspace = all `.rs` files under the current working directory

**Exclusions** (by design, not limitation):
- Private functions (no cross-file tracking)
- Fully-qualified paths: `crate::foo::bar()` (not tracked)
- Macro-generated references (not tracked)
- Shadowed names: local `fn bar` shadows import (correctly ignored)

**Deletion order**: References first (reverse byte order per file to preserve offsets), then definition

### What Did NOT Change

- No persistent database between runs
- No resume mode for failed plans
- No dry-run mode
- Atomic rollback on validation failure (still guaranteed)
- Byte spans remain source of truth (still guaranteed)
- Tree-sitter reparse before compiler gate (still guaranteed)

### Changed

- 298 passing tests (from 22)

### Technical

- tree-sitter parsers for C, C++, Java, JavaScript, Python, TypeScript
- Module path index for cross-language resolution
- Scope-based shadowing detection
- Re-export graph building

## [0.1.3] - 2025-12-28

### Fixed
- Added `readme = "README.md"` to Cargo.toml (was missing, causing no README on crates.io)

## [0.1.2] - 2025-12-28

### Changed
- Updated README to reflect MVP/POC status
- Removed emojis from documentation
- Clarified limitations in manual

## [0.1.1] - 2025-12-28

### Changed

- Updated documentation (removed emojis, clarified MVP status)
- README and manual simplified for clarity

## [0.1.0] - 2025-12-23

### Added

- Span-safe byte replacement for Rust code
- Tree-sitter AST validation
- Cargo check compilation validation
- Atomic rollback on validation failures
- Multi-step JSON plan orchestration
- CLI interface with `patch` and `plan` commands

### Features

- **Single Patch Mode**: Replace function bodies, struct definitions, enum variants
- **Validation Gates**: UTF-8 boundary check, tree-sitter reparse, cargo check
- **Plan Execution**: JSON-based multi-step refactoring
- **Error Handling**: Typed error hierarchy

### Known Limitations

- No cross-file reference tracking
- No persistent symbol database
- No resume mode for failed plans
- No dry-run mode
- Single-file symbol resolution only

### Technical

- Built on tree-sitter-rust 0.21
- SQLiteGraph 0.2.4 integration
- ropey 1.6 for text editing
- clap 4.5 for CLI parsing

### Testing

- 22/22 tests passing
- Complete feature set for single-file refactoring

---

**Note**: v0.1.x was an MVP/proof-of-concept release with intentionally limited features. v0.2.0 adds the delete command with cross-file reference finding. v0.3.0 adds multi-language support for patch and delete operations.
