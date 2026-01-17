# Codebase Structure

**Analysis Date:** 2026-01-17

## Directory Layout

```
splice/
├── src/                    # Core library source
│   ├── cli/                # CLI interface (no logic)
│   ├── graph/              # Database layer
│   ├── ingest/             # Symbol extraction service
│   ├── patch/              # Code modification service
│   ├── plan/               # Multi-step operations
│   ├── resolve/            # Symbol resolution
│   ├── symbol/             # Domain models
│   ├── validate/           # Validation service
│   ├── cli.rs              # CLI types
│   ├── error.rs            # Error types
│   └── lib.rs              # Library entry
├── tests/                  # Integration tests
├── docs/                   # Documentation
├── .codemcp/               # CodeMCP configuration
├── Cargo.toml              # Dependencies and metadata
├── main.rs                 # CLI binary entry
└── README.md               # User documentation
```

## Directory Purposes

**src/**
- Purpose: Core library source code
- Contains: Rust modules for all functionality
- Key files: lib.rs, error.rs, cli.rs
- Subdirectories:
  - cli/ - Command definitions only
  - graph/ - CodeGraph wrapper and database integration
  - ingest/ - Symbol extraction for all supported languages
  - patch/ - Code modification and backup logic
  - plan/ - Multi-step refactoring plans
  - resolve/ - Symbol resolution and reference finding
  - symbol/ - Language-agnostic symbol types
  - validate/ - Compiler/analyzer validation gates

**tests/**
- Purpose: Integration and unit tests
- Contains: Test files for various components
- Key files: cli_tests.rs, patch_tests.rs, integration_refactor.rs
- Subdirectories: None (flat structure)

**docs/**
- Purpose: Project documentation
- Contains: ADRs, plans, TODO tracking
- Key files: ADR_*.md (architecture decisions), PLAN_*.md (feature plans)
- Subdirectories: None

**.codemcp/**
- Purpose: CodeMCP configuration for semantic indexing
- Contains: config.toml
- Subdirectories: None

## Key File Locations

**Entry Points:**
- `main.rs` - CLI binary entry point
- `src/lib.rs` - Library entry point and module exports

**Configuration:**
- `Cargo.toml` - Dependencies and crate metadata
- `.codemcp/config.toml` - CodeMCP configuration

**Core Logic:**
- `src/graph/mod.rs` - CodeGraph wrapper (database operations)
- `src/ingest/dispatch.rs` - Language dispatch for symbol extraction
- `src/resolve/mod.rs` - Symbol resolution logic
- `src/patch/mod.rs` - Patch application logic
- `src/validate/gates.rs` - Compiler validation

**Error Handling:**
- `src/error.rs` - All error types and diagnostics

**Testing:**
- `tests/` - Integration and unit tests

**Documentation:**
- `README.md` - User-facing documentation
- `docs/ADR_*.md` - Architecture decision records

## Naming Conventions

**Files:**
- snake_case.rs for modules (e.g., cli.rs, error.rs)
- mod.rs for module exports within directories

**Directories:**
- snake_case for all directories (e.g., src/ingest/, src/graph/)

**Special Patterns:**
- `src/ingest/[language].rs` - Per-language symbol extractors
- `src/resolve/references/[language].rs` - Per-language reference finders

## Where to Add New Code

**New CLI Command:**
- Definition: Extend `Commands` enum in `src/cli/mod.rs`
- Handler: Add handler function in `src/cli/mod.rs`
- Tests: Add in `tests/cli_tests.rs`

**New Language Support:**
- Implementation: `src/ingest/[language].rs` (symbol extraction)
- References: `src/resolve/references/[language].rs` (reference finding)
- Tests: `tests/[language]_tests.rs`

**New Validation Gate:**
- Implementation: `src/validate/gates.rs`
- Tests: `tests/validation_tests.rs`

**New Operation:**
- Implementation: Create new module in appropriate service directory
- Tests: `tests/` with descriptive name

**Utilities:**
- Implementation: `src/utils.rs` (doesn't exist yet, create if needed)
- Tests: Co-located in `#[cfg(test)]` modules

## Special Directories

**src/ingest/**
- Purpose: Language-specific symbol extraction
- Contains: rust.rs, python.rs, javascript.rs, typescript.rs, java.rs, c.rs, cpp.rs
- Subdirectories: imports/ (import resolution)

**src/resolve/references/**
- Purpose: Language-specific reference resolution
- Contains: Per-language reference finding modules

**.codemcp/**
- Purpose: CodeMCP semantic indexing configuration
- Source: Manually created configuration
- Committed: Yes

---

*Structure analysis: 2026-01-17*
*Update when directory structure changes*
