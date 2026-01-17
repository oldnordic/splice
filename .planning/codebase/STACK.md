# Technology Stack

**Analysis Date:** 2026-01-17

## Languages

**Primary:**
- Rust 2021 Edition - All application code

**Secondary:**
- Not detected

## Runtime

**Environment:**
- Rust 1.70+ (minimum version per `README.md`)
- Native binary compilation (no runtime VM required)

**Package Manager:**
- Cargo - Rust package manager
- Lockfile: `Cargo.lock` present

## Frameworks

**Core:**
- tree-sitter 0.21 - AST parsing for multiple languages
- SQLiteGraph 0.2.11 - Code graph database
- Magellan 0.5.3 - Multi-language code indexing

**Testing:**
- Rust built-in `#[test]` framework - Unit and integration tests
- `#[cfg(test)]` module pattern - Co-located tests

**Build/Dev:**
- rustc - Rust compiler
- cargo build - Build system
- No external build tools required

## Key Dependencies

**Critical:**
- magellan 0.5.3 - Multi-language symbol indexing and code analysis
- sqlitegraph 0.2.11 - Code graph persistence and querying
- tree-sitter (multiple) 0.21 - AST parsing for Rust, Python, C, C++, Java, JavaScript, TypeScript
- ropey 1.6 - Byte-safe text editing operations

**Infrastructure:**
- clap 4.5 - CLI argument parsing
- serde/serde_json 1.0 - JSON serialization
- uuid 1.10 - UUID generation
- chrono 0.4 - Time handling
- log/env_logger - Logging infrastructure
- thiserror 1.0 - Error handling

## Configuration

**Environment:**
- No environment variables required
- Configuration via CLI arguments only
- CodeMCP config in `.codemcp/config.toml`

**Build:**
- `Cargo.toml` - Dependency specification and crate metadata
- Rust 2021 edition defaults
- No custom build scripts detected

## Platform Requirements

**Development:**
- Linux/macOS/Windows (any platform with Rust toolchain)
- Rust 1.70+ compiler
- cargo package manager

**Production:**
- Distributed as cargo package
- Installed via `cargo install splice`
- Runs as native binary on user's system

---

*Stack analysis: 2026-01-17*
*Update after major dependency changes*
