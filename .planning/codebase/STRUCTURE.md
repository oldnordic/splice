# Directory Structure

**Analysis Date:** 2026-01-22

## Directory Layout

```
splice/
├── src/
│   ├── main.rs                 # CLI entry point (~2140 lines)
│   ├── lib.rs                  # Library root, module re-exports
│   ├── cli/                    # Command-line interface
│   │   └── mod.rs              # Argument parsing with clap
│   ├── ingest/                 # Language-specific parsing
│   │   ├── mod.rs              # Module dispatch
│   │   ├── detect.rs           # Language detection
│   │   ├── magellan.rs         # Magellan integration
│   │   ├── rust.rs             # Rust symbol extraction
│   │   ├── python.rs           # Python symbol extraction
│   │   ├── java.rs             # Java symbol extraction
│   │   ├── javascript.rs       # JavaScript symbol extraction
│   │   ├── typescript.rs       # TypeScript symbol extraction
│   │   ├── cpp.rs              # C++ symbol extraction
│   │   └── imports/            # Import extraction submodules
│   │       ├── mod.rs
│   │       ├── rust.rs
│   │       ├── python.rs
│   │       ├── java.rs
│   │       ├── javascript.rs
│   │       └── typescript.rs
│   ├── graph/                  # Code graph integration
│   │   ├── mod.rs              # CodeGraph wrapper (~400 lines)
│   │   ├── schema.rs           # Graph labels and edge types
│   │   └── magellan_integration.rs  # Magellan API wrapper
│   ├── resolve/                # Reference resolution
│   │   ├── mod.rs
│   │   ├── cross_file.rs       # Cross-file reference logic
│   │   ├── module_resolver.rs  # Module path resolution
│   │   └── references/         # Reference extraction
│   │       ├── mod.rs
│   │       └── rust.rs         # Rust reference finding (~1395 lines)
│   ├── patch/                  # Span-safe replacement
│   │   ├── mod.rs              # Core patch engine
│   │   ├── backup.rs           # Backup and restore
│   │   ├── batch_loader.rs     # JSON batch loading
│   │   └── pattern.rs          # Pattern-based replacement
│   ├── validate/               # Validation gates
│   │   ├── mod.rs              # Validation orchestration
│   │   └── gates.rs            # Individual gate implementations
│   ├── execution/              # Audit logging
│   │   ├── mod.rs
│   │   ├── base.rs             # Execution tracking types
│   │   ├── query.rs            # Log querying
│   │   └── log.rs              # Database logging
│   ├── plan/                   # Multi-step plans
│   │   └── mod.rs              # Plan execution
│   ├── checksum.rs             # SHA-256 hashing
│   ├── verify.rs               # Pre-operation checks
│   ├── output.rs               # JSON output formatting
│   ├── error.rs                # Error types
│   └── symbol/                 # Symbol types
│       └── mod.rs              # Language and kind enums
├── tests/                      # Integration tests
│   ├── cli_tests.rs            # CLI command tests
│   ├── patch_tests.rs          # Patch operation tests
│   ├── cross_file_tests.rs     # Cross-file reference tests
│   ├── cross_language_tests.rs # Multi-language tests
│   ├── magellan_integration_tests.rs  # Magellan tests
│   ├── e2e_refactor_tests.rs   # End-to-end refactoring tests
│   ├── ingest_tests.rs         # Parsing tests
│   ├── language_detection_tests.rs
│   ├── module_path_tests.rs
│   ├── resolve_tests.rs        # Resolution tests
│   └── [language]_*_tests.rs   # Per-language tests
├── docs/                       # Documentation
│   ├── DATABASE_SCHEMA.md      # Database structure
│   └── DIAGNOSTICS_HUMAN_LLM.md # Diagnostics reference
├── .planning/                  # Planning artifacts
│   ├── codebase/               # Codebase analysis (this folder)
│   └── config.json             # GSD configuration
├── Cargo.toml                  # Package configuration
├── README.md                   # User documentation
├── CLAUDE.md                   # Development rules
└── CHANGELOG.md                # Version history
```

## Key Locations

| Path | Purpose | Key Types |
|------|---------|-----------|
| `src/main.rs` | CLI entry point, command execution | `main()`, `execute_*()` functions |
| `src/graph/mod.rs` | Code graph wrapper | `CodeGraph`, `NodeId` |
| `src/patch/mod.rs` | Core patching engine | `SpanReplacement`, `SpanBatch` |
| `src/resolve/references/rust.rs` | Rust reference finding | `ReferenceExtractor` |
| `src/error.rs` | Error definitions | `SpliceError`, `Result` |
| `src/symbol/mod.rs` | Symbol type enums | `Language`, `SymbolKind` |
| `.splice_graph.db` | Code graph database | SQLite (via SQLiteGraph) |
| `.splice/operations.db` | Audit trail | SQLite tables |

## Naming Conventions

**Files:**
- `snake_case.rs` for all Rust source files
- Module directories use `snake_case/`
- Test files: `{feature}_tests.rs`

**Modules:**
- `mod.rs` for module exports in subdirectories
- Feature-based grouping (e.g., `ingest/`, `resolve/`)

**Functions:**
- `snake_case` for all functions
- Verb prefixes: `extract_*`, `find_*`, `store_*`, `resolve_*`, `verify_*`
- Test functions: `test_*` or `test_*_specific_case`

**Types:**
- `PascalCase` for structs and enums: `CodeGraph`, `SpanReplacement`
- Enum variants: `PascalCase`: `Public`, `Private`, `Function`

**Constants:**
- `SCREAMING_SNAKE_CASE` for constants: `VERSION`, `EDGE_DEFINES`

## File Size Guidelines

Per `CLAUDE.md`:
- Max 300 LOC per file (600 with justification)
- Large files that exceed:
  - `src/main.rs`: 2140 lines (exceeds - needs splitting)
  - `src/resolve/references/rust.rs`: 1395 lines (exceeds - needs splitting)

## Configuration Files

| File | Purpose |
|------|---------|
| `Cargo.toml` | Dependencies, package metadata |
| `.planning/config.json` | GSD workflow configuration |
| `.splice/operations.db` | Execution audit trail (created at runtime) |
| `.splice_graph.db` | Code graph (created at runtime) |

## Generated Artifacts

| Location | Created By | Purpose |
|----------|-----------|---------|
| `.splice_graph.db` | `CodeGraph::open()` | Symbol storage, reference graph |
| `.splice/operations.db` | `execution::log` | Audit trail |
| `.splice-backup/*/` | `BackupWriter` | Restore points for undo |
| `target/` | Cargo build | Compiled binaries |

---

*Structure analysis: 2026-01-22*
