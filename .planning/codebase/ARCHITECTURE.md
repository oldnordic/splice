# Architecture

**Analysis Date:** 2026-01-22

## Design Pattern

**Core Pattern:** Interpreter / Pipeline
- CLI commands are parsed and dispatched to operation handlers
- Each operation follows: ingest → resolve → validate → patch → log
- Validation gates form a pipeline with rollback on failure

**Secondary Patterns:**
- **Strategy Pattern**: Language-specific ingest modules (`rust.rs`, `python.rs`, etc.)
- **Repository Pattern**: `CodeGraph` wraps SQLiteGraph backend
- **Builder Pattern**: `SpanBatch` collects multiple replacements
- **Command Pattern**: CLI commands execute atomic refactoring operations

## System Layers

```
┌─────────────────────────────────────────────────────────────────┐
│                        CLI Layer (src/main.rs)                   │
│  Command parsing, JSON output orchestration, error reporting     │
└─────────────────────────────┬───────────────────────────────────┘
                              │
┌─────────────────────────────▼───────────────────────────────────┐
│                     Command Orchestration (src/cli/)             │
│  execute_delete, execute_patch, execute_plan, etc.               │
└─────────────────────────────┬───────────────────────────────────┘
                              │
        ┌─────────────────────┼─────────────────────┐
        │                     │                     │
┌───────▼────────┐   ┌────────▼─────────┐   ┌──────▼──────┐
│  Ingest Layer  │   │  Graph Layer     │   │  Patch      │
│  (ingest/)     │   │  (graph/)        │   │  Layer      │
│  - Parse AST   │   │  - CodeGraph     │   │  (patch/)   │
│  - Extract     │   │  - SQLiteGraph   │   │  - Replace  │
│    symbols     │   │  - Magellan      │   │  - Validate │
└────────────────┘   └──────────────────┘   └─────────────┘
        │                     │                     │
        └─────────────────────┼─────────────────────┘
                              │
┌─────────────────────────────▼───────────────────────────────────┐
│                     Core Services                                │
│  - resolve/ (reference finding)                                 │
│  - validate/ (syntax & compiler checks)                         │
│  - checksum/ (SHA-256 verification)                             │
│  - execution/ (audit logging)                                   │
└─────────────────────────────┬───────────────────────────────────┘
                              │
┌─────────────────────────────▼───────────────────────────────────┐
│                     Storage Layer                                │
│  - .splice_graph.db (SQLite code graph)                         │
│  - .splice/operations.db (audit trail)                          │
│  - File system (source files)                                   │
└─────────────────────────────────────────────────────────────────┘
```

## Data Flow

**Delete Operation Flow:**
```
1. CLI parses: splice delete --file src/lib.rs --symbol foo
2. Language detected from file extension
3. Ingest: Parse file with tree-sitter, extract symbols
4. Resolve: Find all references to symbol (cross-file for Rust)
5. Validate: Pre-check with compiler (optional)
6. Patch: Delete definition + all references (atomic batch)
7. Verify: Reparse, validate syntax, verify checksum
8. Log: Write operation record to .splice/operations.db
9. Output: JSON result with spans deleted
```

**Patch Operation Flow:**
```
1. CLI parses: splice patch --file src/lib.rs --symbol foo --with new.rs
2. Load replacement content from new.rs
3. Ingest: Parse file, locate symbol span
4. Validate: Check span bounds, UTF-8 boundaries
5. Pre-verify: Run validation gates (tree-sitter + compiler)
6. Patch: Atomic replace (write temp + fsync + rename)
7. Post-verify: Validate result, compute new checksum
8. Rollback: If validation fails, restore from backup
9. Log: Write operation record
10. Output: JSON result with span info
```

## Key Abstractions

**CodeGraph** (`src/graph/mod.rs:20`):
- Wraps SQLiteGraph backend
- Provides `store_symbol_with_file_and_language()` for indexing
- Caches symbol → NodeId mappings
- Methods: `open()`, `find_symbols_by_name()`, `get_span()`

**SpanReplacement** (`src/patch/mod.rs:33`):
- Represents a single byte-range replacement
- Contains: file path, start/end byte offsets, new content
- Applied atomically within a `SpanBatch`

**SpanBatch** (`src/patch/mod.rs:60`):
- Collection of replacements that succeed/fail together
- Atomic commit: all or nothing
- Used for multi-file operations

**ValidationGate** (`src/validate/gates.rs`):
- Enum of validation checks: `Utf8`, `TreeSitter`, `Compiler`, `RustAnalyzer`
- Each gate returns `Diagnostic` on failure
- Blocking vs warning distinction

## Entry Points

**CLI Entry Point** (`src/main.rs:12`):
```rust
fn main() -> ExitCode {
    let cli = splice::cli::parse_args();
    // Execute command...
}
```

**Library Entry Point** (`src/lib.rs:1`):
```rust
//! Splice: Span-safe refactoring kernel
pub mod checksum;
pub mod cli;
// ... other modules
```

**Command Handlers** (`src/main.rs:25-100`):
- `execute_delete()` - Handle delete command
- `execute_single_patch()` - Handle single patch
- `execute_patch_batch()` - Handle batch patches
- `execute_plan()` - Handle JSON plan files
- `execute_apply_files()` - Handle pattern replace
- `execute_query()` - Handle Magellan queries
- `execute_get()` - Handle code chunk retrieval

## Module Responsibilities

| Module | Responsibility |
|--------|----------------|
| `cli/` | Command-line parsing with clap |
| `ingest/` | Language-specific parsing (Rust, Python, C, C++, Java, JS, TS) |
| `graph/` | SQLiteGraph integration, symbol indexing |
| `resolve/` | Reference finding, cross-file resolution |
| `patch/` | Byte-exact replacement, atomic writes |
| `validate/` | Syntax and compiler validation gates |
| `checksum/` | SHA-256 hash computation |
| `execution/` | Audit trail logging to SQLite |
| `output/` | Structured JSON output formatting |
| `error/` | Centralized error types with thiserror |
| `symbol/` | Language and symbol kind enums |
| `plan/` | Multi-step plan orchestration |
| `verify/` | Pre-operation verification checks |

## Cross-Cutting Concerns

**Error Handling:**
- `SpliceError` enum in `src/error.rs`
- All functions return `Result<T>`
- Diagnostics for validation failures with levels

**Logging:**
- `env_logger` for debug output
- Audit trail in `.splice/operations.db`
- Execution time tracking

**Validation:**
- Pre-operation checks (optional strict mode)
- Post-operation verification
- Atomic rollback on failure

**Multi-Language Support:**
- Language detection via `ingest/detect.rs`
- Language-specific tree-sitter parsers
- Unified symbol model across languages

---

*Architecture analysis: 2026-01-22*
