# Codebase Structure

**Analysis Date:** 2026-01-23

## Directory Layout

```
splice/
├── src/                     # Core library and business logic
│   ├── cli/              # Command-line interface (thin adapter)
│   ├── ingest/           # Multi-language parsing (tree-sitter)
│   │   ├── imports/      # Import extraction per language
│   ├── graph/            # SQLiteGraph wrapper and code graph
│   ├── resolve/           # Symbol resolution and reference finding
│   │   └── references/ # Cross-file reference queries
│   ├── patch/             # Span-safe replacement engine
│   ├── validate/          # Compiler and AST validation gates
│   ├── execution/          # Execution log infrastructure
│   ├── expand/            # Symbol expansion and AST walking
│   ├── relationships/      # Code relationship queries
│   ├── hints/             # LLM guidance and tool hints
│   ├── action/            # Suggested refactoring actions
│   ├── output/            # Structured JSON output types
│   ├── checksum/          # File and span hashing
│   ├── context/           # Context extraction around spans
│   ├── verify/            # Pre/post verification checks
│   ├── suggestions/       # "Did you mean" functionality
│   ├── symbol/            # Symbol and language enums
│   ├── error/             # Error types and thiserror wrappers
│   ├── error_codes/       # Machine-readable error code mappings
│   ├── diff/              # Unified diff generation
│   └── plan/             # Plan execution (placeholder)
├── tests/                 # Integration and unit tests
├── docs/                  # Documentation (manual, specs, plans)
├── .splice/               # Splice internal state (exec logs)
├── .codemcp/             # CodeMCP internal state (code graph, operations db)
├── .planning/             # GSD planning artifacts (phases, this directory)
└── target/                # Rust build artifacts
```

## Directory Purposes

**src/cli:**
- Purpose: Command-line argument parsing and command dispatch
- Contains: CLI struct, Commands enum, flag definitions (verbose, json, strict)
- Key files: `mod.rs`, `parse_args.rs` (if separate)
- Re-exports: Command definitions for library consumers (rarely used)

**src/ingest:**
- Purpose: Parse source files with tree-sitter and extract symbol metadata
- Contains: Language-specific symbol extractors, import extractors, dispatcher, semantic kind detection
- Key files: `mod.rs`, `dispatch.rs`, `detect.rs`, `magellan.rs`
  - `rust.rs`: Rust symbol extraction with visibility
  - `python.rs`: Python symbol extraction
  - `cpp.rs`: C/C++ symbol extraction
  - `java.rs`: Java symbol extraction
  - `javascript.rs`: JavaScript symbol extraction
  - `typescript.rs`: TypeScript symbol extraction
- Re-exports: All language-specific types and `extract_symbols`, `detect_language` functions

**src/ingest/imports:**
- Purpose: Extract import/module dependencies from parsed AST
- Contains: Language-specific import extractors
- Key files: `mod.rs`, `rust.rs`, `python.rs`, `cpp.rs`, `java.rs`, `javascript.rs`, `typescript.rs`
- Re-exports: `ImportFact`, `ImportKind` enums for each language

**src/graph:**
- Purpose: SQLiteGraph integration layer for code graph operations
- Contains: CodeGraph wrapper, schema definitions, Magellan integration
- Key files: `mod.rs`, `schema.rs`, `magellan_integration.rs`
- Provides: Symbol storage, span queries, node/edge API
- Backend: Magellan's SQLiteGraph (auto-detects SQLite vs native format)

**src/resolve:**
- Purpose: Deterministic symbol resolution with ambiguity detection
- Contains: Symbol resolution, cross-file reference finding
- Key files: `mod.rs`, `module_resolver.rs`, `cross_file.rs`
- Re-exports: `resolve_symbol`, `ResolvedSpan`, `find_symbol_or_suggest` functions

**src/resolve/references:**
- Purpose: Cross-file reference finding (currently Rust-only)
- Contains: Reference queries using DEFINES edges
- Key files: `mod.rs`, `rust.rs`
- Limitations: Multi-language reference finding not yet implemented

**src/patch:**
- Purpose: Span-safe replacement engine with atomic writes and validation gates
- Contains: Patch application, validation gates, backup/restore, pattern replacement, batch loading
- Key files: `mod.rs`, `backup.rs`, `pattern.rs`, `batch_loader.rs`
- Dependencies: validate module, ropey for byte manipulation, verify module
- Core functions: `apply_patch_with_validation`, `apply_batch_with_validation`, `preview_patch`

**src/validate:**
- Purpose: Compiler and AST validation gates
- Contains: Validation logic, gate implementations, error parsing
- Key files: `mod.rs`, `gates.rs`
- Supports: cargo check, rust-analyzer, tree-sitter, python, tsc, gcc, javac, node
- Modes: AnalyzerMode enum (Off, Path, Explicit)

**src/execution:**
- Purpose: Persistent audit trail for all Splice operations
- Contains: Execution log database, query API, statistics
- Key files: `mod.rs`, `base.rs`, `log.rs`, `query.rs`
- Database: `operations.db` at `.splice/` directory

**src/expand:**
- Purpose: Retrieve full symbol bodies by walking AST parent chains
- Contains: Language-specific expanders, tree-walker utilities
- Key files: `mod.rs`, `tree_walker.rs`, per-language expanders (rust, python, cpp, java, js, ts)
- Levels: ExpansionLevel enum (None=0, Body=1, ContainingBlock=2)

**src/relationships:**
- Purpose: Query code relationships (callers, callees, imports, exports)
- Contains: Relationship queries with session-based caching
- Key files: `mod.rs` (main module)
- Limitations: Imports and exports currently return empty (DEFINES edge traversal not yet available)

**src/hints:**
- Purpose: Derive behavioral metadata for LLM guidance
- Contains: Tool hints based on semantic kind and operation type
- Key files: `mod.rs`
- Flags: `requires_full_context`, `apply_atomically`, `may_break_tests`, `requires_compilation`

**src/action:**
- Purpose: Suggested refactoring actions with confidence levels
- Contains: SuggestedAction enum with action types
- Key files: `mod.rs`
- Action types: `DeleteBody`, `ChangeSignature`, `ChangeType`, `ReplaceBody`

**src/output:**
- Purpose: Structured JSON output with schema versioning
- Contains: OperationResult, SpanResult, JsonResponse types, diff formatting
- Key files: `mod.rs` (main), plus checksum, context, diff modules
- Schema versions: `OPERATION_SCHEMA_VERSION = "2.0.0"`, `QUERY_SCHEMA_VERSION = "1.0.0"`

**src/checksum:**
- Purpose: SHA-256 hashing for file and span verification
- Contains: Hash computation utilities
- Key files: `checksum.rs`

**src/context:**
- Purpose: Extract context lines before/after spans
- Contains: Asymmetric and symmetric context extraction
- Key files: `context.rs`
- Supports: grep-style flags (-A, -B, -C)

**src/verify:**
- Purpose: Pre and post verification checks for file state and changes
- Contains: File readiness checks, localized change detection, workspace validation
- Key files: `verify.rs`

**src/suggestions:**
- Purpose: "Did you mean" functionality using string similarity
- Contains: Suggestion algorithms
- Key files: `suggestions.rs`
- Uses: strsim for fuzzy matching

**src/symbol:**
- Purpose: Symbol and language enums
- Contains: Language enum, SymbolKind enum, AnySymbol union type
- Key files: `mod.rs`

**src/error:**
- Purpose: Centralized error handling with thiserror
- Contains: SpliceError enum with 60+ variants, Diagnostic struct
- Key files: `error.rs`

**src/error_codes:**
- Purpose: Machine-readable error code mappings with remediation links
- Contains: ErrorCode enum, error code constants, explanation functions
- Key files: `error_codes.rs`

**src/diff:**
- Purpose: Unified diff generation with color support
- Contains: Diff formatting using similar crate
- Key files: `mod.rs`

**tests:**
- Purpose: Integration and unit tests for all functionality
- Contains: Test suites organized by feature/language
- Key test files:
  - `cli_tests.rs`: CLI command handling
  - `patch_tests.rs`: Patch operations
  - `resolve_tests.rs`: Symbol resolution
  - `validation_gates_tests.rs`: Validation logic
  - `ingest_tests.rs`: Ingestion
  - `cross_language_tests.rs`: Multi-language support
  - `cross_file_tests.rs`: Cross-file resolution
  - `relationship_performance.rs`: Relationship queries
  - And more: 40+ test files covering all modules

**docs:**
- Purpose: User documentation, architectural decisions, API specs, development plans
- Contains: Manual, QUICKSTART, CHANGELOG, and rendered documentation
- Key files: `manual.md`, `QUICKSTART.md`, `CHANGELOG.md`, plus various ADRs and specs

## Key File Locations

**Entry Points:**
- `src/main.rs`: CLI binary entry point, command dispatch
- `src/lib.rs`: Library entry point, module re-exports

**Configuration:**
- `Cargo.toml`: Package metadata, dependencies, features
- `.codemcp/`: CodeMCP state directory (codegraph.db, operations.db, etc.)
- `.splice/`: Splice state directory (exec log database)

**Core Logic:**
- `src/graph/mod.rs`: CodeGraph API, symbol storage, span queries
- `src/patch/mod.rs`: Span-safe patching with validation gates
- `src/resolve/mod.rs`: Symbol resolution, ambiguity detection
- `src/validate/mod.rs`: Compiler and AST validation

**Testing:**
- `tests/`: Integration test suite
  - Test naming: `*_tests.rs` pattern
  - Coverage: All major modules have corresponding test files

**Documentation:**
- `docs/manual.md`: User manual
- `docs/QUICKSTART.md`: Quick start guide
- `docs/CHANGELOG.md`: Version history

## Naming Conventions

**Files:**
- Library modules: `mod.rs` in each directory (e.g., `src/patch/mod.rs`)
- Test files: `<module>_tests.rs` pattern (e.g., `patch_tests.rs`)
- Documentation: `.md` extension in docs directory

**Functions:**
- Public API: snake_case (e.g., `apply_patch_with_validation`, `resolve_symbol`)
- Internal helpers: snake_case (e.g., `compute_hash`, `gate_tree_sitter_reparse`)
- Boolean checks: `is_*` prefix (e.g., `is_sqlite_db`, `is_false`)

**Types:**
- Enums: CamelCase (e.g., `AnalyzerMode`, `ExpansionLevel`, `ToolHintOperation`)
- Structs: PascalCase (e.g., `ResolvedSpan`, `SpanReplacement`, `OperationResult`)
- Error types: PascalCase with `Error` suffix (e.g., `SpliceError`, `CompilerError`)

**Directories:**
- Lowercase snake_case for subdirectories (e.g., `src/ingest`, `src/resolve`)
- Uppercase for acronyms only (e.g., `src/cli`, `src/graph`)

**Constants:**
- SCREAMING_SNAKE_CASE (e.g., `OPERATION_SCHEMA_VERSION`, `QUERY_SCHEMA_VERSION`, `TOOL_NAME`)

**Traits:**
- CamelCase with trait purpose suffix (e.g., `SymbolExpander` for expanders)

## Where to Add New Code

**New Feature:**
- Primary code: Add module in `src/` with corresponding `mod.rs`
- Tests: Add `<feature>_tests.rs` in `tests/` directory
- Example: Adding new validation gate → `src/validate/new_gate.rs`, `tests/validation_new_gate_tests.rs`

**New Component/Module:**
- Implementation: Add files within existing `src/<module>/` directory
- Module re-export: Update `src/<module>/mod.rs` to re-export new public types
- Example: Adding new language support → `src/ingest/newlang.rs`, update `src/ingest/mod.rs`

**Utilities/Helpers:**
- Shared helpers: Add to existing module if logically related
- Standalone utilities: Create new module in `src/` if logically independent
- Example: File hashing utilities exist in `src/checksum.rs`, not separate module per helper

**Tests:**
- Unit tests: In `tests/<module>_tests.rs` for library module tests
- Integration tests: In `tests/` for end-to-end workflow tests
- Test naming: Follow `<module>_tests.rs` pattern for clarity
- Example: Testing new resolution logic → `tests/new_resolution_tests.rs`

**Documentation:**
- User docs: Add sections to `docs/manual.md` for new features
- API docs: Update `docs/API.md` if exists (or create)
- Architecture: Add ADR to `docs/` for major decisions
- Example: Adding new command → update manual with command reference

**Error Handling:**
- New errors: Add variant to `SpliceError` enum in `src/error.rs`
- Error codes: Add to `ErrorCode` enum in `src/error_codes.rs`
- Remediation: Add remediation link to `remediation_link_for_code` function in `src/error_codes.rs`

## Special Directories

**target/:**
- Purpose: Rust build artifacts (compiled binaries, dependencies)
- Generated: By `cargo build` or `cargo test`
- Committed: No (in `.gitignore`)
- Clean: Can be safely deleted with `cargo clean`

**.splice/:**
- Purpose: Splice's internal state directory
- Contains: `operations.db` (execution log database)
- Generated: By splice operations
- Safe to delete: No, contains audit trail

**.codemcp/:**
- Purpose: CodeMCP's internal state directory
- Contains: `codegraph.db` (code graph), `execution_log.db` (executions), possibly others
- Generated: By codemcp operations
- Safe to delete: Only if you want to reset CodeMCP state

**.planning/:**
- Purpose: GSD planning artifacts and codebase analysis
- Contains: `phases/` (planning phases), `codebase/` (this directory)
- Generated: By `/gsd/map-codebase` command
- Committed: Yes (planning state)
- Note: This is analysis output, not source code

**docs/:**
- Purpose: Documentation and specifications
- Contains: User manual, API docs, architecture decisions, plans
- Authored: Hand-written documentation
- Committed: Yes

---

*Structure analysis: 2026-01-23*
