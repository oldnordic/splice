# External Integrations

**Analysis Date:** 2026-01-22

## APIs & External Services

**Language Compilers:**
- Rust Compiler (cargo) - Compilation validation
  - Command: `cargo check`
  - Purpose: Validate Rust code syntax and semantics
- Python Compiler (python) - Python compilation validation
  - Command: `python -m py_compile`
  - Purpose: Validate Python syntax
- GCC/G++ - C/C++ compilation validation
  - Command: `gcc -fsyntax-only` / `g++ -fsyntax-only`
  - Purpose: Validate C/C++ syntax
- Java Compiler (javac) - Java compilation validation
  - Command: `javac`
  - Purpose: Validate Java syntax
- Node.js (node) - JavaScript syntax validation
  - Command: `node --check`
  - Purpose: Validate JavaScript syntax
- TypeScript Compiler (tsc) - TypeScript compilation validation
  - Command: `tsc --noEmit`
  - Purpose: Validate TypeScript syntax

**File System:**
- Native file system operations
  - Operations: Read/write/backup files
  - Purpose: Source code modification and backup

## Data Storage

**Databases:**
- SQLite (embedded)
  - Connection: Direct file access
  - Client: rusqlite 0.31
  - Purpose: Two databases:
    - `.splice_graph.db` - Code graph storage
    - `.splice/operations.db` - Execution audit trail

**File Storage:**
- Local filesystem only
  - Operations: File reading, writing, patching
  - Purpose: Source code manipulation

**Caching:**
- None
  - Operations: All data read fresh from database/filesystem

## Authentication & Identity

**Auth Provider:**
- None
  - Implementation: Not applicable

## Monitoring & Observability

**Error Tracking:**
- Built-in error handling with thiserror
  - Implementation: Custom error types with detailed context

**Logs:**
- env_logger with structured logging
  - Implementation: Log to stdout with timestamps and metadata
  - Audit trail in SQLite database (.splice/operations.db)

## CI/CD & Deployment

**Hosting:**
- Rust Cargo registry (crates.io)
  - Package name: "splice"
  - Version: 2.0.0

**CI Pipeline:**
- None (manual builds)
  - Implementation: Cargo build and test

## Environment Configuration

**Required env vars:**
- None (all configuration through CLI arguments)

**Secrets location:**
- None required

## Webhooks & Callbacks

**Incoming:**
- None

**Outgoing:**
- None

## Language-Specific Integrations

**Rust:**
- rust-analyzer integration (optional)
  - Purpose: LSP-based validation
  - Location: src/validate/
- Cross-file reference resolution
  - Implementation: Custom resolver in src/resolve/

**Multi-language:**
- Tree-sitter grammars for 7 languages
  - Languages: Rust, Python, C, C++, Java, JavaScript, TypeScript
  - Purpose: AST-based parsing and validation
- Magellan code indexing
  - Purpose: Symbol lookup and label-based queries
  - Integration: src/graph/magellan_integration.rs

## Database Schema

**Code Graph (.splice_graph.db):**
- graph_entities: Nodes in the code graph
- graph_edges: Relationships between entities
- graph_labels: Labels for categorization
- graph_properties: Additional metadata

**Audit Trail (.splice/operations.db):**
- Operations table: Execution log with timestamps
- Metadata: Command-line arguments, workspace, affected files

---

*Integration audit: 2026-01-22*