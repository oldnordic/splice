# Changelog

Format based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/).
Project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

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
