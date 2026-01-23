# Technology Stack

**Analysis Date:** 2026-01-23

## Languages

**Primary:**
- Rust 2021 - Core implementation language (entire project)

**Secondary:**
- Python - For test fixtures and cross-language validation
- JavaScript/TypeScript - Code generation targets (via tree-sitter)
- C/C++ - Code generation targets (via tree-sitter)
- Java - Code generation targets (via tree-sitter)

## Runtime

**Environment:**
- Rust 1.70+ (minimum for building)
- Target: Linux, macOS, Windows (via std)

**Package Manager:**
- Cargo - Rust package manager
- Lockfile: `Cargo.lock` (present)

## Frameworks

**Core:**
- tree-sitter 0.22 - AST parsing for all supported languages
- SQLiteGraph 1.0 - Code graph storage backend (native-v2 features)
- Magellan 0.5.3 - Code indexing and label-based symbol discovery

**Testing:**
- Rust built-in test framework - Unit and integration testing
- tempfile 3.10 - Test file management

**Build/Dev:**
- clap 4.5 (derive) - CLI argument parsing
- ropey 1.6 - Safe byte-level text editing

## Key Dependencies

**Critical:**
- sqlitegraph 1.0 - Code graph database backend, required for symbol storage and queries
- magellan 0.5.3 - Multi-language code indexing, required for symbol discovery
- rusqlite 0.31 (bundled) - Direct SQLite access for execution logging

**Infrastructure:**
- tree-sitter 0.22 + language parsers - AST validation for Rust, Python, C, C++, Java, JavaScript, TypeScript
- which 6 - Locating compiler binaries on PATH
- std::process::Command - External compiler execution for validation gates

**Data Structures:**
- serde 1.0 + serde_json - JSON serialization for configuration, plans, and output
- uuid 1.10 (v4) - Unique identifier generation for operations
- chrono 0.4 - Timestamp handling for execution logging
- sha2 0.10 - File checksum computation for validation

## Configuration

**Environment:**
- `SPLICE_EXECUTION_LOG` - Enable/disable execution logging (default: true)
- `NO_COLOR` - Disable colored diff output
- `RUST_LOG` - Standard Rust logging configuration

**Key configs required:**
- Language compilers in PATH (cargo, python, gcc, g++, javac, node, tsc)
- Graph database path (auto-detected from workspace: `.splice/codegraph.db`)
- Execution log database path (auto-detected: `.splice/operations.db`)

**Build:**
- No separate build config files
- Standard Cargo configuration via `Cargo.toml`
- Release optimization: Use `cargo build --release`

## Platform Requirements

**Development:**
- Rust 1.70+ toolchain
- Git repository (for version control and backups)
- File system with write permissions for `.splice/` directory
- Language-specific compilers for target languages

**Production:**
- CLI binary deployment (no runtime dependencies)
- Local filesystem access for code graph databases
- Optional: Language compilers for validation gates

---

*Stack analysis: 2026-01-23*
