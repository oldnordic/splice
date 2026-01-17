# Architecture

**Analysis Date:** 2026-01-17

## Pattern Overview

**Overall:** Library + CLI Binary

**Key Characteristics:**
- CLI as thin shell over core library
- Layered architecture with clear module boundaries
- Multi-language support via pluggable extractors
- Byte-accurate span-safe operations

## Layers

**CLI Layer:**
- Purpose: Command-line interface only, no business logic
- Contains: Command definitions, argument parsing, JSON I/O
- Location: `src/cli/mod.rs`
- Depends on: Service layer for all operations
- Used by: end users via CLI

**Service Layer:**
- Purpose: Core refactoring operations
- Contains:
  - Ingest (`src/ingest/`) - Symbol extraction and language detection
  - Resolve (`src/resolve/`) - Symbol resolution and reference finding
  - Patch (`src/patch/`) - Code modification with validation
  - Validate (`src/validate/`) - Compiler/analyzer integration
  - Plan (`src/plan/`) - Multi-step refactoring plans
- Depends on: Data layer, domain layer
- Used by: CLI layer

**Data Layer:**
- Purpose: Code graph storage and querying
- Contains: CodeGraph wrapper, Magellan integration, schema definitions
- Location: `src/graph/`
- Depends on: SQLite databases (magellan.db, codegraph.db)
- Used by: Service layer

**Domain Layer:**
- Purpose: Language-agnostic symbol abstraction
- Contains: Symbol trait, common types
- Location: `src/symbol/`
- Depends on: None (foundational types)
- Used by: All layers

## Data Flow

**CLI Command Execution:**

1. User runs: `splice <command> <args>`
2. CLI parses arguments in `src/main.rs` → `src/cli/mod.rs`
3. Command handler delegates to service function
4. Service extracts symbols via language dispatch (`src/ingest/dispatch.rs`)
5. Symbols stored in in-memory graph (`src/graph/mod.rs`)
6. References resolved using CodeGraph queries
7. Patches applied with byte-accurate spans
8. Validation via tree-sitter + compiler checks (`src/validate/gates.rs`)
9. Results returned as JSON to CLI

**State Management:**
- SQLite databases for persistent storage
- In-memory graph for operation duration
- Automatic backup creation before modifications
- No global mutable state

## Key Abstractions

**Symbol:**
- Purpose: Language-agnostic representation of code symbols
- Location: `src/symbol/mod.rs:19-49`
- Pattern: Trait with common properties across all languages
- Methods: name(), kind(), container(), defining_span()

**CodeGraph:**
- Purpose: Graph database wrapper for code relationships
- Location: `src/graph/mod.rs:20-29`
- Pattern: Wrapper over SQLiteGraph with caching
- Methods: add_symbol(), find_references(), get_file_symbols()

**SpliceError:**
- Purpose: Structured error handling with context
- Location: `src/error.rs:10-179`
- Pattern: Enum with variants for each error type
- Features: Diagnostic aggregation, file locations, hints

**Language Extractor:**
- Purpose: Pluggable per-language symbol extraction
- Location: `src/ingest/[language].rs`
- Pattern: Module with extract_symbols() function
- Supported: Rust, Python, C, C++, Java, JavaScript, TypeScript

## Entry Points

**CLI Entry:**
- Location: `src/main.rs`
- Triggers: User runs `splice` command
- Responsibilities: Parse args, delegate to library, format output

**Library Entry:**
- Location: `src/lib.rs`
- Triggers: Used as library by other tools
- Responsibilities: Export public API, initialize modules

**Tests:**
- Location: `tests/` directory
- Triggers: `cargo test`
- Responsibilities: Integration and unit tests

## Error Handling

**Strategy:** Structured errors with context, propagate to CLI

**Patterns:**
- `SpliceError` enum for all error types
- `Result<T>` type alias for consistent returns
- `thiserror` for error derivation
- Diagnostic aggregation for batch operations
- Rich error messages with file paths and line numbers

## Cross-Cutting Concerns

**Logging:**
- log crate with env_logger
- Structured logging with context
- Configurable via RUST_LOG environment variable

**Validation:**
- Multi-stage validation: tree-sitter AST + compiler checks
- Span validation before applying patches
- Language-specific validation gates

**Language Support:**
- Pluggable architecture via per-language modules
- Language dispatch in `src/ingest/dispatch.rs`
- Trait-based abstraction for language-agnostic operations

**Backup:**
- Automatic backup creation before modifications
- Manifest files for tracking
- Restore capability for rollbacks

---

*Architecture analysis: 2026-01-17*
*Update when major patterns change*
