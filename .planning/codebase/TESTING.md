# Testing Patterns

**Analysis Date:** 2026-01-17

## Test Framework

**Runner:**
- Rust built-in #[test] framework
- cargo test for running tests

**Assertion Library:**
- Built-in assert! macros
- assert_eq!, assert_ne! for equality

**Run Commands:**
```bash
cargo test              # Run all tests
cargo test <name>       # Run specific test
cargo test -- --nocapture  # Show print output
```

## Test File Organization

**Location:**
- tests/ directory for integration tests
- #[cfg(test)] modules in source files for unit tests

**Naming:**
- [component]_tests.rs for integration tests
- test functions prefixed with test_

**Structure:**
```
tests/
├── cli_tests.rs
├── ingest_tests.rs
├── patch_tests.rs
├── validation_tests.rs
└── integration_refactor.rs

src/
├── lib.rs (contains #[cfg(test)] module)
└── *.rs (may contain #[cfg(test)] modules)
```

## Test Structure

**Suite Organization:**
```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_name() {
        // arrange
        let input = ...;

        // act
        let result = function(input);

        // assert
        assert_eq!(result, expected);
    }
}
```

**Patterns:**
- #[cfg(test)] for test modules
- Use super::* to import module items
- Descriptive test names with test_ prefix

## Mocking

**Framework:**
- No external mocking framework
- Uses Rust's trait system for mocking where needed

**Patterns:**
- Test fixtures defined in test functions
- Real implementations preferred over mocks
- Conditional compilation for test-specific code

## Fixtures and Factories

**Test Data:**
- Inline in test files
- Helper functions for creating test data
- No separate fixtures directory

**Location:**
- Test-specific helpers in test modules
- Shared test utilities in tests/ if needed

## Coverage

**Requirements:**
- No enforced coverage target
- Coverage tracked for awareness

**Configuration:**
- No coverage tool configured
- Manual coverage assessment via cargo test

**View Coverage:**
```bash
cargo test              # Run all tests
cargo tarpaulin         # Coverage (tarpaulin not configured)
```

## Test Types

**Unit Tests:**
- #[cfg(test)] modules in source files
- Test individual functions in isolation
- Fast execution

**Integration Tests:**
- tests/ directory
- Test multiple modules together
- Cross-language tests
- End-to-end refactoring tests

**Test Categories:**
- Language-specific tests (python_import_tests.rs, java_symbol_tests.rs, etc.)
- Validation tests (validation_tests.rs)
- CLI tests (cli_tests.rs)
- Integration refactoring (integration_refactor.rs)

## Common Patterns

**Async Testing:**
- Not applicable (Rust uses async/await, tests work similarly)

**Error Testing:**
```rust
#[test]
fn test_error_case() {
    let result = function_that_fails();
    assert!(result.is_err());
}
```

**File System Mocking:**
- Uses temporary files in tests/
- Cleanup in test teardown
- No explicit mocking framework

---

*Testing analysis: 2026-01-17*
*Update when test patterns change*
