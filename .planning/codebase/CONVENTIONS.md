# Coding Conventions

**Analysis Date:** 2026-01-23

## Naming Patterns

**Files:**
- All lowercase with underscores: `snake_case`
- Modules: `mod.rs` for module files, named after directory
- Test files: `*_tests.rs` pattern
- Main source: `src/lib.rs` as library entry point

**Functions:**
- Public functions: `pub fn descriptive_name()` (snake_case)
- Private functions: `fn private_function()` (snake_case)
- Constructor functions: `new()` or `from_*()`
- Builder methods: `with_*()` for chaining

**Variables:**
- All lowercase snake_case: `let variable_name`
- Constants: `SCREAMING_SNAKE_CASE`
- Parameters: `parameter_name` (descriptive)
- References: `&ref` or `ref_mut`

**Types:**
- Structs: `PascalCase` (e.g., `SpanBatch`, `CodeGraph`)
- Enums: `PascalCase` (e.g., `Language`, `AnalyzerMode`)
- Traits: `PascalCase` (e.g., `Symbol`)
- Error Types: `SpliceError` as main enum

## Code Style

**Formatting:**
- Tool: Rustfmt (standard Rust formatter)
- Line length: ~100 characters (some exceptions for long spans)
- Indentation: 4 spaces (no tabs)
- Trailing commas: Always in multi-line structures
- Braces: K&R style (opening brace on same line)

**Linting:**
- Tool: Rust compiler warnings + Clippy
- Key rules enforced:
  - `#![warn(missing_docs)]` - All public items must have docs
  - `#![expect(unused_crate_dependencies)]` - Unused dependencies OK
  - `dead_code` warnings allowed selectively
  - `clippy::pedantic` enabled

## Import Organization

**Order:**
1. Standard library imports: `use std::...`
2. External crate imports: `use crate::...` (ordered alphabetically)
3. Local module imports: `use self::...` or `use super::...`
4. Re-exports grouped by module

**Path Aliases:**
- `crate::` prefix for intra-crate references
- No `use` pollution - import only what's needed
- Re-exports in `lib.rs` for public API

## Error Handling

**Patterns:**
- Main error type: `SpliceError` enum with variants
- Result type: `pub type Result<T> = std::result::Result<T, SpliceError>`
- Error context: Use `.with_context()` for chainable context
- Location metadata: `.with_path()` for file-specific errors
- Structured diagnostics: `Diagnostic` type with location, level, message

**Error Creation:**
```rust
// Standard error with context
let err = SpliceError::SymbolNotFound { ... }.with_context("while resolving symbol");

// I/O error with path
let err = SpliceError::io_with_path(path, source);

// Parse error with file
let err = SpliceError::parse_with_file(file, message);
```

**Error Handling in Functions:**
- Early returns with `?` operator
- Map errors with additional context
- Preserve original error sources with `#[source]`
- Provide actionable hints for CLI users

## Logging

**Framework:** `log` crate + `env_logger` for CLI binary
**Patterns:**
- Debug logging for detailed tracing
- Error logging only for user-facing errors
- Structured error logging with kind() and location()
- No logging in pure library code (CLI handles display)

**When to Log:**
- External tool execution (cargo check, rust-analyzer)
- File operations with meaningful context
- Performance-critical operations (minimal logging)

## Comments

**When to Comment:**
- Complex algorithm logic (multi-step operations)
- Non-obvious error handling decisions
- Performance optimization reasoning
- TODO items with clear next steps

**JSDoc/TSDoc:**
- All public functions have documentation
- Examples in Rust doc format: `/// # Examples`
- Argument descriptions with `///` prefix
- Return value documentation
- Error conditions documented

## Function Design

**Size:** Generally under 50 lines, some exceptions up to 100
- Small functions (5-20 lines): Focus on single responsibility
- Medium functions (20-50 lines): Clear logical flow
- Large functions (50-100 lines): Well-documented multi-step process

**Parameters:**
- 3-5 parameters ideal
- Use structs for complex parameter sets
- Optional parameters with `Option<T>`
- Builder pattern for optional parameters

**Return Values:**
- Use `Result<T, SpliceError>` for fallible operations
- Return tuples for multiple related values
- Use `Option<T>` for nullable values
- Structured output types for complex results

## Module Design

**Exports:**
- Public API re-exports in `lib.rs`
- Clear module boundaries with `pub mod`
- Private implementation details not exposed
- Wrapper types for external dependencies

**Barrel Files:**
- No barrel files (avoid unnecessary indirection)
- Direct imports preferred for clarity
- Re-exports only in top-level `lib.rs`

**Structure:**
- Feature-based module organization
- Each module has clear responsibility
- Cross-cutting concerns in dedicated modules
- Hierarchical organization from core to specific

## Special Conventions

**Spans and Byte Offsets:**
- All spans use byte offsets (not line/column for core logic)
- Line numbers 1-based, columns 0-based (compiler convention)
- Context extraction uses UTF-8 aware line counting

**Validation Gates:**
- Atomic operations with rollback on failure
- Hash validation before/after operations
- Multi-language validation (tree-sitter + compiler)
- Optional rust-analyzer for additional safety

**Database Operations:**
- SQLite through Magellan wrapper
- Transaction-based operations
- Prepared statements for repeated queries
- Connection pooling where appropriate

**Async Patterns:**
- Minimal async usage (mostly CLI, not core library)
- File operations blocking but efficient
- Tree-sitter parsing uses owned data, not references

## Testing Conventions

**Test Organization:**
- Integration tests in `tests/` directory
- Unit tests alongside production code
- E2E tests via CLI invocation
- Mock external tools in tests

**Test Data:**
- Temporary files for workspace creation
- Checksums for deterministic validation
- Fixed examples for predictable test behavior
- Performance tests with meaningful datasets

---

*Convention analysis: 2026-01-23*