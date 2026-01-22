# Coding Conventions

**Analysis Date:** 2026-01-22

## Naming Patterns

**Files:**
- All lowercase: `src/checksum.rs`, `src/error.rs`
- Modules in subdirectories: `src/ingest/rust.rs`, `src/ingest/imports/python.rs`
- Descriptive names: `verify.rs`, `magellan_integration.rs`

**Functions:**
- Snake_case for functions: `extract_rust_symbols`, `checksum_file`, `resolve_symbol`
- Verb-first naming: `extract_*`, `find_*`, `store_*`, `resolve_*`
- Public functions use `pub fn`, private functions use `fn`

**Variables:**
- Snake_case: `file_path`, `source`, `symbols`, `module_path`
- Descriptive names: `rope` for Rope, `cursor` for tree-sitter cursor
- Single character only for iterators: `i`, `j`, `c`, `n`

**Types:**
- PascalCase for structs and enums: `CodeGraph`, `RustSymbol`, `Visibility`, `SpliceError`
- Enum variants: `Public`, `Private`, `Function`, `Struct`

## Code Style

**Formatting:**
- Rust Edition 2021
- No explicit formatter configuration found, using defaults
- Max line length not enforced

**Linting:**
- `#![warn(missing_docs)]` enforces documentation
- `#![expect(unused_crate_dependencies)]` allows dev dependencies in lib.rs
- Custom error types with `thiserror`
- All public APIs documented with `///`

## Import Organization

**Order:**
1. Standard library: `use std::...`
2. External crates: `use sha2::...`, `use ropey::...`
3. Local modules: `use crate::...`

**Path Aliases:**
- No aliases found, using full paths
- Re-exports via `pub use` at module level

**Import Groups:**
- Blank lines separate import groups
- Related imports grouped together

## Error Handling

**Patterns:**
```rust
// Result<T> for fallible operations
pub fn extract_rust_symbols(path: &Path, source: &[u8]) -> Result<Vec<RustSymbol>> {
    // Error propagation with ?
    let tree = parser.parse(source, None)
        .ok_or_else(|| SpliceError::Parse {
            file: path.to_path_buf(),
            message: "Parse failed - no tree returned".to_string(),
        })?;
}

// Custom enum with thiserror
#[derive(Error, Debug)]
pub enum SpliceError {
    #[error("I/O error for path {path}: {source}")]
    Io {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
}
```

## Comments

**When to Comment:**
- Module headers with `//!` for documentation
- Public function documentation with `///`
- Complex algorithm explanations
- TODO items (minimal usage)

**JSDoc/TSDoc:**
- Rust standard doc comments
- Examples provided for key functions
- Documented all error variants

## Function Design

**Size:**
- Max 300 LOC per file (enforced by CLAUDE.md)
- Average function size: 10-50 lines
- Large functions broken into smaller helper functions

**Parameters:**
- Limited parameters (typically 3-5)
- Optional parameters using Option<T>
- Clear parameter names with type context

**Return Values:**
- Result<T> for fallible functions
- Vec<T> for multiple results
- Option<T> for optional results

## Module Design

**Exports:**
- Re-exports at module level for convenience
- Clear module boundaries
- Minimal public API per module

**Barrel Files:**
- Module-level `mod.rs` files organize functionality
- Re-exports for common types

## Logging

**Framework:** No logging framework configured

**Patterns:**
- No debug logging found in codebase
- Error logging via Result propagation
- CLI output via clap

## Documentation

**Module Headers:**
- Every module has `//!` documentation
- Explains purpose and scope

**Public APIs:**
- All public functions documented
- Documented error conditions
- Examples for complex operations

## Code Organization

**Directory Structure:**
- Feature-based organization (`ingest/`, `graph/`, `patch/`)
- Language-specific modules under `ingest/`
- Clear separation between core and utilities

**File Naming:**
- Descriptive names
- Consistent with feature names
- No abbreviations