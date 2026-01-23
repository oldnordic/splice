# Coding Conventions

**Analysis Date:** 2026-01-23

## Naming Patterns

**Files:**
- snake_case for all files (e.g., `mod.rs`, `error_codes.rs`, `checksum.rs`)
- Module directories use the module name (e.g., `src/ingest/`, `src/resolve/`)
- Test files use `*_tests.rs` or `test_*.rs` naming

**Functions:**
- snake_case for all functions (e.g., `extract_rust_symbols`, `apply_patch_with_validation`)
- Public functions have descriptive names indicating action and target

**Variables:**
- snake_case for local variables and parameters (e.g., `file_path`, `byte_start`, `context_before`)
- Field names in structs use snake_case (e.g., `before_hash`, `after_hash`, `line_start`)

**Types:**
- UpperCamelCase for all types (structs, enums, traits) (e.g., `SpanReplacement`, `SpliceError`, `RustSymbol`)
- Enum variants use UpperCamelCase (e.g., `Visibility::Public`, `DiagnosticLevel::Error`)

**Modules:**
- snake_case for module names (e.g., `mod ingest`, `mod patch`)
- Use `mod.rs` files for directory modules

## Code Style

**Formatting:**
- Four-space indentation (no tabs)
- Rust 2021 edition defaults
- No explicit formatting config found (uses rustfmt defaults)

**Linting:**
- `#[warn(missing_docs)]` at `src/lib.rs` level enforces documentation
- `#[expect(unused_crate_dependencies)]` suppresses unused dependency warnings
- Uses Clippy with strict mode (as per AGENTS.md: `cargo clippy --all-targets --all-features -D warnings`)
- No explicit `.clippy.toml` configuration file

## Import Organization

**Order:**
1. Standard library imports (`std::path`, `std::fs`, `std::io`)
2. External crate imports (`ropey::Rope`, `tree_sitter`, `thiserror::Error`)
3. Internal module imports (`crate::error`, `crate::ingest`, `crate::validate`)

**Path Aliases:**
- No path aliases configured
- Uses full module paths or `use crate::*` style

**Re-exports:**
- Library-level re-exports in `src/lib.rs` using `pub use` pattern
- Example: `pub use error::{Result, SpliceError};`
- Commonly re-exported: error types, graph types, context functions, diff utilities

## Error Handling

**Patterns:**
- Uses `thiserror` crate for typed errors with `#[derive(Error)]`
- `Result<T>` type alias: `pub type Result<T> = std::result::Result<T, SpliceError>;`
- Error variants include structured context: file paths, line/column info, hints
- NO `unwrap()` in production code (per AGENTS.md)
- Early returns with `?` operator for error propagation
- Context can be added via `.with_context()` helper method on `SpliceError`

**Error Variant Structure:**
```rust
#[error("Message with {placeholder}")]
VariantName {
    field: Type,
    #[source]
    source: UnderlyingError,
}
```

**Helper Methods:**
- `SpliceError::symbol_not_found()` - for symbol lookup failures
- `SpliceError::symbol_not_found_with_suggestions()` - with fuzzy matching
- `SpliceError::parse_with_file()` - parse errors with context
- `SpliceError::with_context()` - chainable context addition
- `SpliceError::with_path()` - attach path information

## Logging

**Framework:** `log` crate with `env_logger` for CLI

**Patterns:**
- `log::info!()` for informational messages
- `log::warn!()` for warnings that don't block execution
- `log::error!()` for errors requiring attention
- `log::debug!()` for debugging output (verbose mode)
- Context-rich messages with format strings: `log::info!("Post-verification: syntax={}, compiler={}", syntax_ok, compiler_ok)`

## Comments

**When to Comment:**
- Module-level documentation explaining purpose (at top of every file)
- Function documentation explaining arguments, returns, and behavior
- Inline comments for non-obvious logic
- TODO comments for planned work (found in `src/ingest/mod.rs:55,63`)

**Documentation Style:**
- Module docs: `//! Module description.` at file top
- Function docs: `/// Brief description.` on function/struct definitions
- Comprehensive doc examples using `///` blocks with `# Examples`, `# Arguments`, `# Returns` sections
- Rustdoc-compliant examples (marked with `/// ```no_run` or `/// ````)

**Example:**
```rust
//! Context extraction for span surroundings.
//!
//! Provides line-based context extraction using ropey for efficient
//! UTF-8 aware line/column calculations.

/// Extract context lines for a byte span with asymmetric before/after counts.
///
/// Given a file path and byte range, extracts lines before, within, and after
/// span. Allows different amounts of context before vs after the match.
///
/// # Arguments
///
/// * `path` - File path to read
/// * `byte_start` - Start byte offset (must be <= byte_end)
/// * `byte_end` - End byte offset (must be <= file size)
///
/// # Returns
///
/// * `Ok(SpanContext)` - Extracted context with before/selected/after arrays
/// * `Err(SpliceError)` - If file cannot be read or span is invalid
pub fn extract_context_asymmetric(...) -> Result<SpanContext>
```

## Function Design

**Size:** Functions typically 20-50 lines, up to 100 lines for complex operations
- Large files: `src/main.rs` (3668 lines) is the CLI entry point (expected to be large)
- `src/error.rs` (854 lines) contains error definitions and helpers

**Parameters:**
- Pass paths as `&Path` or `PathBuf` for ownership control
- Pass strings as `&str` when ownership not needed
- Use `Option<T>` for optional parameters
- Config parameters grouped into structs (e.g., `SpanReplacement`, `SpanBatch`)

**Return Values:**
- `Result<T>` from `crate::error` for all fallible operations
- `Option<T>` for lookup operations that may not find results
- Tuple returns for multiple values (e.g., `(before_hash, after_hash)`)

## Module Design

**Exports:**
- `mod.rs` files re-export public items with `pub use`
- Library `src/lib.rs` re-exports commonly used items for convenience
- Pattern: `pub use module::{TypeA, TypeB, function_c};`

**Barrel Files:**
- Every directory has `mod.rs` that declares submodules
- `mod.rs` files aggregate exports from submodules
- Example: `src/ingest/mod.rs` declares `mod rust;`, `mod python;`, etc. and re-exports

**Module Organization:**
- `cli/` - Command-line interface (thin adapter only, no logic)
- `ingest/` - Tree-sitter parsing per language
- `resolve/` - Reference finding across files
- `patch/` - Span replacement with validation gates
- `validate/` - Compiler and analyzer integration
- `graph/` - SQLiteGraph integration and schema
- `error/` - Error types and Result alias
- `context/` - Context extraction utilities
- `output/` - Diff formatting and display

---

*Convention analysis: 2026-01-23*
