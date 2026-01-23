# Architecture

**Analysis Date:** 2026-01-23

## Pattern Overview

**Overall:** Layered architecture with graph-based symbol resolution and validation gates

**Key Characteristics:**
- Multi-language code graph with exact byte spans
- Three validation gates (UTF-8, tree-sitter, compiler)
- Atomic file operations with rollback capability
- Language-agnostic symbol storage with SQLiteGraph backend
- Execution logging for audit trail

## Layers

**CLI Layer:**
- Purpose: Thin adapter for user-facing command-line interface
- Location: `src/main.rs`, `src/cli/mod.rs`
- Contains: Argument parsing, command dispatch, result formatting
- Depends on: Library modules (patch, resolve, validate, graph, output)
- Used by: End users via CLI, automation systems

**Graph Layer:**
- Purpose: Persistent code graph storing symbols, spans, and relationships
- Location: `src/graph/mod.rs`
- Contains: CodeGraph wrapper, symbol storage, node/edge queries
- Depends on: Magellan (sqlitegraph crate)
- Used by: resolve, patch (for validation), query commands

**Ingest Layer:**
- Purpose: Parse source files with tree-sitter and extract symbol metadata
- Location: `src/ingest/mod.rs` with language subdirectories
- Contains: Language-specific parsers (rust, python, cpp, java, javascript, typescript)
- Depends on: tree-sitter language parsers
- Used by: CLI commands, resolution (during indexing)

**Resolution Layer:**
- Purpose: Resolve symbol names to exact byte spans with ambiguity detection
- Location: `src/resolve/mod.rs`
- Contains: Symbol resolution, cross-file reference finding, "did you mean" suggestions
- Depends on: Graph layer for symbol lookup
- Used by: CLI delete/patch commands, query operations

**Patch Layer:**
- Purpose: Apply span-safe replacements with validation gates
- Location: `src/patch/mod.rs`
- Contains: Span replacement, atomic writes, validation gates, backup/restore
- Depends on: validate module, ropey for byte manipulation, verify for pre-checks
- Used by: CLI delete/patch commands

**Validation Layer:**
- Purpose: Compiler and AST validation gates
- Location: `src/validate/mod.rs`, `src/validate/gates.rs`
- Contains: cargo check, rust-analyzer, tree-sitter validation, language-specific compilers
- Depends on: External tools (cargo, rust-analyzer, python, tsc, gcc, etc.)
- Used by: Patch layer (gates), CLI commands

**Expansion Layer:**
- Purpose: Retrieve full symbol bodies by walking AST parent chains
- Location: `src/expand/mod.rs`, `src/expand/tree_walker.rs`
- Contains: Language-specific expanders, tree-walker utilities, expansion levels (None/Body/ContainingBlock)
- Depends on: tree-sitter parsers, ingest for language detection
- Used by: Query/get commands for full symbol content

**Relationship Layer:**
- Purpose: Query code relationships (callers, callees, imports, exports)
- Location: `src/relationships/mod.rs`
- Contains: Relationship queries, session-based caching, graph traversal
- Depends on: Graph layer for neighbor queries
- Used by: CLI commands with --relationships flag, delete operations

**Output Layer:**
- Purpose: Structured JSON output with schema versioning
- Location: `src/output.rs`, `src/checksum.rs`, `src/context.rs`
- Contains: OperationResult, SpanResult, context extraction, unified diff formatting
- Depends on: serde for serialization, checksum module for hashing
- Used by: CLI commands, LLM consumption

**Execution Log Layer:**
- Purpose: Persistent audit trail of all operations
- Location: `src/execution/mod.rs`, `src/execution/base.rs`, `src/execution/log.rs`, `src/execution/query.rs`
- Contains: Execution log database, query API, statistics aggregation
- Depends on: rusqlite for storage
- Used by: CLI log command, all mutation operations

**Tool Hints Layer:**
- Purpose: Behavioral metadata to guide LLM refactoring decisions
- Location: `src/hints/mod.rs`
- Contains: Operation types (DeleteBody, ChangeSignature, ReplaceBody), semantic flag derivation
- Depends on: Semantic kind detection from ingest module
- Used by: CLI commands for rich span metadata

**Action Layer:**
- Purpose: Suggested refactoring actions based on symbol analysis
- Location: `src/action/mod.rs`
- Contains: SuggestedAction with confidence levels, action types
- Depends on: Tool hints and context analysis
- Used by: CLI commands for rich JSON output

**Error Layer:**
- Purpose: Centralized error types with structured error codes
- Location: `src/error.rs`, `src/error_codes.rs`
- Contains: SpliceError enum with variants, ErrorCode mappings, remediation links
- Depends on: thiserror for error derivation
- Used by: All layers for error handling

## Data Flow

**Symbol Resolution Flow:**

1. CLI receives symbol name (+ optional file, kind filters)
2. Resolve layer queries CodeGraph for matching symbols
3. Ambiguity detection: if file=None and multiple matches → error with suggestions
4. File-specific resolution: if file provided → resolves within that file only
5. Returns ResolvedSpan with exact byte span, line/col, language metadata

**Delete Operation Flow:**

1. Ingest file with language-aware tree-sitter parser
2. Extract symbols and store in CodeGraph
3. Resolve target symbol to get exact byte span
4. Find all references to symbol (same-file and cross-file via references module)
5. Group references by file, sort by byte offset descending
6. For each reference: apply deletion with validation gates
7. Delete definition last
8. Record execution in log database
9. Return structured output with checksums, relationships, tool hints

**Patch Operation Flow:**

1. Ingest file with language-aware tree-sitter parser
2. Extract symbols and store in CodeGraph
3. Resolve target symbol to get exact byte span
4. Read replacement content from file
5. Pre-verification: check file state, workspace resources
6. Compute before hash
7. Validate UTF-8 boundary (span splits correctly)
8. Apply byte-exact replacement using ropey
9. Write to temp file, fsync, atomic rename
10. Gate 1: tree-sitter reparse (language-specific parser)
11. Gate 2: compiler validation (cargo check for Rust, language-specific compiler for others)
12. Gate 3: rust-analyzer (Rust only, optional)
13. On any failure: automatic rollback to original content
14. Compute after hash
15. Post-verification: localized change check
16. Record execution in log database
17. Return structured output with checksums, relationships, tool hints

**Query Operation Flow:**

1. Open or create CodeGraph database
2. For label-based query: use graph schema to find matching nodes
3. For symbol query: resolve symbol name with ambiguity detection
4. Extract context around symbol using byte spans
5. Expand symbol body (if --expand flag)
6. Query relationships (if --relationships flag)
7. Return structured JSON with schema versioning

**Batch Pattern Replace Flow:**

1. Load patch specifications from file (JSON format)
2. Validate patch files and parameters
3. Group replacements by file, sort by byte offset
4. Apply all replacements atomically across files
5. Run validation gates (tree-sitter + compiler)
6. On any failure: rollback all modified files
7. Return structured output with per-file summaries

**Execution Logging Flow:**

1. All mutation operations (delete, patch, apply_files) generate execution_id
2. Record operation type, parameters, success/failure status
3. Store in operations.db (separate from codegraph.db)
4. Enable post-hoc querying, statistics, audit trail
5. CLI log command queries operations.db for history

**State Management:**

**CodeGraph:**
- Persistent SQLite database (`.splice_graph.db` or `.codemcp/codegraph.db`)
- Stores: File nodes, Symbol nodes with byte spans and metadata, DEFINES edges
- Backend: Magellan's SQLiteGraph (can detect native vs SQLite format)
- Caching: In-memory symbol cache (HashMap name → Vec<NodeId>), file cache

**Execution Log:**
- Persistent SQLite database (`.splice/operations.db` or `.codemcp/execution_log.db`)
- Stores: execution_id, operation_type, timestamp, status, parameters, result
- Enables: Independent operation history from code graph

**Workspace Detection:**
- Cargo workspace: Searches up directory tree for `Cargo.toml`
- Backup location: `.splice-backup/` at workspace root
- Workspace root: Used for compiler validation (cargo check, rust-analyzer)

## Key Abstractions

**Span (Byte Range):**
- Purpose: Exact location in file using byte offsets (start, end)
- Examples: Byte spans in patch/replacement operations, graph node storage
- Pattern: Always inclusive start, exclusive end, validated for UTF-8 boundaries
- Safety: Ropey library ensures UTF-8 safe byte-to-char conversion

**CodeGraph (Symbol Database):**
- Purpose: Persistent, queryable representation of code structure
- Examples: `CodeGraph::open(&db_path)` in CLI commands
- Pattern: File nodes, Symbol nodes, DEFINES edges (File→Symbol)
- Backend: Magellan's SQLiteGraph with node/edge API
- Caching: In-memory HashMap for fast symbol name lookup

**ResolvedSpan (Symbol Location):**
- Purpose: Rich location metadata with disambiguation support
- Examples: Returned from `resolve::resolve_symbol()` with match_id for tracking
- Pattern: Includes node_id, name, kind, language, file_path, line_start, line_end, col_start, col_end
- Safety: File-scoped resolution prevents ambiguity errors

**Validation Gates:**
- Purpose: Three-tier safety net for patch operations
- Examples: `patch::apply_patch_with_validation()` runs all three gates
- Pattern: UTF-8 boundary check → tree-sitter reparse → compiler check → rust-analyzer (optional)
- Safety: Atomic rollback on any gate failure, pre-verification before patching

**SpanReplacement (Atomic Operation):**
- Purpose: Single file-span replacement unit
- Examples: Applied by patch layer, batched for multi-file operations
- Pattern: File path, start byte, end byte, replacement content
- Safety: Grouped by file, sorted descending for deletion, atomic application

**ExecutionId (Operation Tracking):**
- Purpose: Unique identifier for audit trail and operation linkage
- Examples: UUIDv4 generated per operation, passed to all sub-operations
- Pattern: Stored in operations.db, included in JSON output
- Safety: Enables dependency tracking and undo operations

## Entry Points

**CLI Entry Point:**
- Location: `src/main.rs`
- Triggers: User invokes `splice` binary with subcommand (delete, patch, plan, query, log, apply_files)
- Responsibilities:
  - Parse CLI arguments using clap
  - Dispatch to appropriate command handler
  - Initialize logger if verbose mode
  - Format and emit JSON/human-readable output
  - Handle broken pipe errors gracefully

**Library Entry Point:**
- Location: `src/lib.rs`
- Triggers: Other crates depend on splice library
- Responsibilities:
  - Re-export all public modules (cli, patch, resolve, validate, graph, etc.)
  - Re-export common types (Result, SpliceError, CodeGraph)
  - Provide version constant (VERSION)

**Ingest Dispatch Entry Point:**
- Location: `src/ingest/dispatch.rs`
- Triggers: Language detection and appropriate parser selection
- Responsibilities:
  - Auto-detect language from file extension
  - Call language-specific extract_symbols function
  - Return `AnySymbol` enum for unified symbol handling

**Validation Gate Entry Points:**
- Location: `src/patch/mod.rs` (gates are called internally), `src/validate/gates.rs`
- Triggers: After atomic write completes, before returning success
- Responsibilities:
  - Tree-sitter reparse: Verify patched file parses correctly
  - Compiler validation: Run language-specific compiler
  - rust-analyzer: Optional LSP-based semantic analysis (Rust only)

## Error Handling

**Strategy:** Structured error types with thiserror and error codes

**Patterns:**
- Centralized `SpliceError` enum in `src/error.rs`
- Each variant has context (file, line, column, message)
- Error codes in `src/error_codes.rs` provide machine-readable identifiers
- Remediation links point to official documentation (Rust error index, TypeScript errors)
- Error propagation uses `?` operator for Result chaining
- CLI formats errors with diagnostic level (Error, Warning, Note, Help)

**Structured Diagnostics:**
- `Diagnostic` struct in `src/error.rs` with level, file, position, code
- Used by: Compiler validation gates, rust-analyzer parsing
- Output: Included in error JSON responses with tool metadata
- Remediation: Links to official documentation based on error code

## Cross-Cutting Concerns

**Logging:** env_logger with log macros (info!, warn!, error!, debug!)

**Pattern:**
- CLI `--verbose` flag enables env_logger
- Log levels: info for normal operations, warn for warnings, error for failures, debug for diagnostics
- Contextual: Include file paths, operation types, span locations in messages

**Validation:**
- Three mandatory gates (UTF-8, tree-sitter, compiler)
- Language-specific: Different compilers per language (cargo, python, tsc, gcc, javac, node)
- Optional gate: rust-analyzer for Rust (off/path/explicit modes)
- Rollback: Automatic restore on any gate failure

**Authentication:** Not applicable (CLI tool, operates on local files only)

---

*Architecture analysis: 2026-01-23*
