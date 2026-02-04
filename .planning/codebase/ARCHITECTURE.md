# Architecture

**Analysis Date:** 2026-01-23

## Pattern Overview

**Overall:** Multi-language refactoring kernel with graph-based symbol resolution

**Key Characteristics:**
- Byte-accurate span-based patching using rope data structure
- SQLiteGraph code graph for cross-language symbol resolution
- Tree-sitter AST validation with language-specific parsers
- Compiler validation gates for each supported language
- Magellan integration for external tool interactions

## Layers

### Ingest Layer (`src/ingest/`)
**Purpose:** Filesystem → AST → SQLiteGraph ingestion pipeline
**Location:** `/home/feanor/Projects/splice/src/ingest/`
- **Magellan Integration:** `MagellanIngestor` for database-backed indexing (preferred API)
- **Language Modules:** Individual parsers for each language (rust.rs, python.rs, etc.)
- **Imports:** Cross-language import extraction and resolution
- **Detection:** Automatic language detection from file extensions
- **Dispatch:** Coordinated extraction across multiple languages
- **Direct Extraction:** `extract_symbols()` function for one-off symbol extraction

**Note:** The original `Ingestor` struct was removed in v2.2.4 as dead code. Use `MagellanIngestor` or `extract_symbols()` instead.

### Graph Layer (`src/graph/`)
**Purpose:** SQLiteGraph integration with Splice-specific operations
**Location:** `/home/feanor/Projects/splice/src/graph/`
- **CodeGraph:** Primary interface to graph database
- **Schema:** Graph node/edge definitions and type mappings
- **Magellan Integration:** External tool communication layer
- **Caching:** Symbol name → NodeId mapping for performance

### Symbol Layer (`src/symbol/`)
**Purpose:** Language-agnostic symbol abstraction
**Location:** `/home/feanor/Projects/splice/src/symbol/`
- **Symbol Trait:** Common interface for all language symbols
- **Language Enum:** Supported programming languages (Rust, Python, C/C++, Java, JS/TS)
- **AnySymbol:** Wrapper enum for heterogeneous symbol collections
- **Language-Specific Types:** Individual symbol implementations

### Patch Layer (`src/patch/`)
**Purpose:** Span-safe file replacement with validation
**Location:** `/home/feanor/Projects/splice/src/patch/`
- **Atomic Replacement:** Byte-exact patching with rollback capability
- **Validation Gates:** Tree-sitter, compiler, and rust-analyzer validation
- **Batch Operations:** Multi-file atomic patches
- **Preview Mode:** Safe change simulation with diff generation

### Validation Layer (`src/validate/`)
**Purpose:** Multi-language compilation and analysis validation
**Location:** `/home/feanor/Projects/splice/src/validate/`
- **Gates:** Language-specific validation checks
- **Compiler Integration:** Cross-language tool invocation
- **Diagnostics:** Error collection with remediation hints
- **Analyzer:** LSP integration (Rust-focused)

### Resolution Layer (`src/resolve/`)
**Purpose:** Symbol resolution and cross-reference tracking
**Location:** `/home/feanor/Projects/splice/src/resolve/`
- **Cross-File:** Multi-file symbol resolution
- **Module Resolver:** Language-aware module resolution
- **References:** Import/export relationship tracking
- **Context:** Symbol context extraction

### Execution Layer (`src/execution/`)
**Purpose:** Orchestrates Splice operations with logging
**Location:** `/home/feanor/Projects/splice/src/execution/`
- **Base:** Execution context management
- **Query:** Database query execution
- **Log:** Structured logging integration

## Data Flow

### Ingestion Flow:

1. **File Detection:** Language detected via file extension
2. **AST Parsing:** `MagellanIngestor` or `extract_symbols()` uses language-specific parser with tree-sitter
3. **Symbol Creation:** `Symbol` trait implementations wrap extracted data
4. **Graph Storage:** Symbols stored in SQLiteGraph database with byte spans and file associations
5. **Cache Update:** Symbol name → NodeId cache populated for fast lookups

### Patching Flow:

1. **Pre-Verification:** File state and workspace readiness checks
2. **Content Read:** Original file content with hash computation
3. **Byte Replacement:** Rope-based span manipulation
4. **Atomic Write:** Temp file → fsync → atomic rename
5. **Validation Gates:**
   - Tree-sitter reparse (syntax validation)
   - Compiler check (semantic validation)
   - rust-analyzer (optional LSP validation)
6. **Post-Verification:** Change localization and expected state confirmation

## Key Abstractions

### Symbol Interface
```rust
trait Symbol {
    fn name(&self) -> &str;
    fn kind(&self) -> &str;
    fn byte_start(&self) -> usize;
    fn byte_end(&self) -> usize;
    fn language(&self) -> Language;
}
```

### Language Registry
```rust
pub enum Language {
    Rust, Python, C, Cpp, Java, JavaScript, TypeScript
}
```

### Graph Schema
- **Nodes:** File, Symbol (language-agnostic kinds)
- **Edges:** DEFINES, REFERENCES, IMPORTS, etc.
- **Properties:** Byte spans, line/column, language metadata

### Span Replacement
```rust
pub struct SpanReplacement {
    pub file: PathBuf,
    pub start: usize,
    pub end: usize,
    pub content: String,
}
```

## Entry Points

### CLI Entry (`src/main.rs`)
**Location:** `/home/feanor/Projects/splice/main.rs`
- **Trigger:** Command line invocation
- **Responsibilities:** Parse CLI arguments, initialize components, orchestrate operations
- **Commands:** ingest, patch, query, validate, diff, etc.

### Library Entry (`src/lib.rs`)
**Location:** `/home/feanor/Projects/splice/src/lib.rs`
- **Purpose:** Public API for library usage
- **Re-exports:** Common types and functions for convenience
- **Core Types:** Result, CodeGraph, Symbol, Language

### Module Exports
Each module exports key types and functions:
- `ingest`: `extract_symbols`, `detect_language`
- `graph`: `CodeGraph`, symbol storage methods
- `patch`: `apply_patch_with_validation`
- `symbol`: `Symbol`, `Language`, `AnySymbol`
- `validate`: Language-specific validation functions

## Error Handling

**Strategy:** Comprehensive error hierarchy with source preservation

**Patterns:**
- `SpliceError`: Main error type with subtypes
- Diagnostic-level error categorization (Error, Warning, Info)
- Context preservation (file paths, spans, tool metadata)
- Automatic rollback on validation failures

## Cross-Cutting Concerns

**Logging:** Structured logging with log crate integration
**Validation:** Multi-language compiler and AST validation
**Atomicity:** File system operations with rollback capability
**Caching:** In-memory symbol cache for performance
**Byte Precision:** Rope-based byte-level operations

---

*Architecture analysis: 2026-01-23*