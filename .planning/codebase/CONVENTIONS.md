# Coding Conventions

**Analysis Date:** 2026-01-17

## Naming Patterns

**Files:**
- snake_case.rs for all module files (e.g., cli.rs, error.rs)
- mod.rs for directory exports

**Functions:**
- snake_case for all functions (e.g., extract_symbols, apply_patch)
- No special prefix for async functions (Rust uses async keyword)

**Variables:**
- snake_case for variables (e.g., byte_start, byte_end)
- SCREAMING_SNAKE_CASE for constants (e.g., VERSION)
- No underscore prefix (no private marker in Rust)

**Types:**
- PascalCase for structs (e.g., PythonSymbol, CodeGraph)
- PascalCase for enums (e.g., Language, SpliceError)
- PascalCase for type aliases (e.g., Result<T>)

## Code Style

**Formatting:**
- Rust 2021 edition defaults
- 4-space indentation (Rust standard)
- Double quotes for string literals
- Semicolons required
- No explicit config files (rustfmt defaults)

**Linting:**
- rustc built-in lints
- clippy for additional lints (not configured in repo)
- #![warn(missing_docs)] for documentation warnings
- No lint overrides or allow attributes in production code

## Import Organization

**Order:**
1. std library imports
2. External crate imports
3. Internal imports (crate::)
4. Module declarations (mod)

**Grouping:**
- Blank lines between groups
- Alphabetical within each group (convention, not enforced)

**Path Aliases:**
- None (uses crate:: prefix for internal imports)

## Error Handling

**Patterns:**
- thiserror for custom error types
- Result<T> type alias for consistent returns
- Structured error messages with context
- Error propagation using ? operator

**Error Types:**
- SpliceError enum in `src/error.rs`
- Variants for each error category
- Include file paths and line numbers in errors
- Diagnostic aggregation for batch operations

## Logging

**Framework:**
- log crate for facade
- env_logger for implementation in binary

**Patterns:**
- Structured logging with context
- Log at service boundaries
- No console.log in library code

## Comments

**When to Comment:**
- Module-level documentation with //!
- Function documentation with ///
- Explain non-obvious algorithms
- Note platform-specific behavior

**Rustdoc:**
- Required for public APIs
- Use /// for item docs
- Use //! for module docs
- Include examples where helpful

**TODO Comments:**
- TODO: for unimplemented features
- todo!() macro for unimplemented code paths

## Function Design

**Size:**
- Keep under 300 lines (project standard)
- Extract helpers for complex logic
- One level of abstraction per function

**Parameters:**
- Use struct for 3+ parameters
- Destructure in parameter list where appropriate

**Return Values:**
- Result<T> for fallible operations
- Explicit returns
- Return early for guard clauses

## Module Design

**Exports:**
- pub use for re-exports in lib.rs
- Private items by default
- pub for public API

**Barrel Files:**
- mod.rs for directory exports
- Re-export public API from mod.rs

---

*Convention analysis: 2026-01-17*
*Update when patterns change*
