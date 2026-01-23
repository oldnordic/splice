# Technology Stack

**Analysis Date:** 2026-01-23

## Languages

**Primary:**
- Rust 2021 Edition - Core language for the entire codebase
- CLI binary entry point in `/home/feanor/Projects/splice/src/main.rs`

**Secondary:**
- Shell/bash - Test scripts and integration validation
  - `/home/feanor/Projects/splice/test_splice_magellan_integration.sh`
  - `/home/feanor/Projects/splice/validate_integration.sh`

## Runtime

**Environment:**
- Rust 2021 Edition (stable)
- Cargo 1.0 - Package manager and build tool
- SQLite 3.x - Embedded database for operations logging

**Package Manager:**
- Cargo 1.0 - Native Rust package manager
- Lockfile: `Cargo.lock` (versioned dependencies)

## Frameworks

**Core:**
- Magellan 0.5.3 - Multi-language code indexing and graph database
  - Purpose: Code graph storage, symbol queries, code chunk retrieval
  - Features: native-v2 for enhanced performance
  - Integration: `/home/feanor/Projects/splice/src/graph/magellan_integration.rs`

**Testing:**
- Rust built-in test framework - Unit and integration tests
  - Pattern: `#[test]` functions alongside production code
  - Test files: Co-located with source files in `src/` and `tests/`

**Build/Dev:**
- Cargo build system - Native compilation and dependency management
- clap 4.5 - Command line interface parsing
- env_logger 0.11 - Structured logging
- tempfile 3.10 - Temporary file handling for tests

## Key Dependencies

**Critical:**
- magellan 0.5.3 - Core code indexing and graph functionality
- sqlitegraph 1.0 - SQLiteGraph re-export from Magellan
- rusqlite 0.31 - Direct SQLite access for execution logs
- tree-sitter 0.22 - AST parsing for multiple languages
- ropey 1.6 - Safe byte-level text editing
- serde 1.0 - JSON serialization
- thiserror 1.0 - Error handling

**Infrastructure:**
- uuid 1.10 - UUID generation for unique identifiers
- chrono 0.4 - Date/time handling
- sha2 0.10 - File hashing for validation
- glob 0.3 - Pattern matching for file discovery
- similar 2.6 - Diff generation
- strsim 0.11 - String similarity for suggestions

## Configuration

**Environment:**
- Configuration via environment variables (e.g., `SPLICE_EXECUTION_LOG`)
- No external configuration files required
- `.codemcp/config.toml` - External tool integration config

**Build:**
- Single `Cargo.toml` for entire project
- Feature flags: native-v2 for enhanced performance
- No separate build configuration files

## Platform Requirements

**Development:**
- Rust toolchain (stable/nightly)
- SQLite development headers (for rusqlite bundled build)
- Optional: Magellan binary for integration testing

**Production:**
- Compiled static binary (no external runtime dependencies)
- SQLite library (bundled with rusqlite)
- File system access for database and code manipulation

---

*Stack analysis: 2026-01-23*