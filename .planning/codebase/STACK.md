# Technology Stack

**Analysis Date:** 2026-01-22

## Languages

**Primary:**
- Rust 2021 Edition - Main implementation language
  - Version: 1.70+ (required for building)
  - Features: Edition 2021 with async support

**Secondary:**
- N/A - Pure Rust implementation with no language bindings

## Runtime

**Environment:**
- Rust Native
  - Version: 1.70+
  - Binary format: Static linking (no runtime dependencies)

**Package Manager:**
- Cargo (Rust package manager)
  - Version: Integrated with Rust toolchain
  - Lockfile: Cargo.lock (committed to version control)

## Frameworks

**Core:**
- Native Rust frameworks only
  - Custom CLI implementation with clap
  - No external application frameworks

**Testing:**
- Rust test framework (built-in)
  - Unit tests integrated into source files
  - Integration tests in tests/ directory

**Build/Dev:**
- Cargo Build System
  - Debug/Release profiles
  - Target-specific builds

## Key Dependencies

**Core:**
- magellan 0.5.3 - Multi-language code indexing and symbol queries
  - Features: native-v2
  - Purpose: Code graph construction and label-based queries
- sqlitegraph 1.0 - SQLiteGraph backend
  - Features: native-v2
  - Purpose: In-process graph database for code entities

**AST Parsing:**
- tree-sitter 0.22 - Framework for incremental parsing
  - Grammar-specific packages for each language:
    - tree-sitter-rust 0.21
    - tree-sitter-python 0.21
    - tree-sitter-c 0.21
    - tree-sitter-cpp 0.21
    - tree-sitter-javascript 0.21
    - tree-sitter-typescript 0.21
    - tree-sitter-java 0.21

**Text Processing:**
- ropey 1.6 - Safe byte-level text editing
  - Purpose: Precise code span manipulation
- glob 0.3 - File pattern matching
  - Purpose: Multi-file operations

**Serialization:**
- serde 1.0 - Serialization framework
  - Features: derive macros
- serde_json 1.0 - JSON serialization
  - Purpose: Configuration and structured output

**Error Handling:**
- thiserror 1.0 - Error type derivation
  - Purpose: Custom error types for the project
- rusqlite 0.31 - SQLite wrapper
  - Features: bundled (static linking)
  - Purpose: Direct database access for logging

**CLI & Logging:**
- clap 4.5 - Command line argument parsing
  - Features: derive macros
- log 0.4 - Logging framework
- env_logger 0.11 - Environment-based logging
- which 6 - Find executables
- tempfile 3.10 - Temporary file management

**Utilities:**
- sha2 0.10 - SHA-256 hashing
  - Purpose: Checksum verification
- uuid 1.10 - UUID generation
  - Features: v4 (random UUIDs)
- chrono 0.4 - Date/time handling
  - Features: std, clock (reduced featureset)
- strsim 0.10 - String similarity (for diffing)

## Configuration

**Environment:**
- No external environment configuration required
- Language-specific compilers must be available in PATH

**Build:**
- Cargo.toml - Main configuration
- Workspace: Single-crate project
- Profile-specific optimizations

## Platform Requirements

**Development:**
- Rust 1.70+ toolchain
- Language-specific compilers:
  - Rust: cargo
  - Python: python
  - C/C++: gcc/g++
  - Java: javac
  - JavaScript: node
  - TypeScript: tsc

**Production:**
- Rust runtime (statically linked)
- No external dependencies at runtime
- Cross-platform binaries (Linux, macOS, Windows)

---

*Stack analysis: 2026-01-22*