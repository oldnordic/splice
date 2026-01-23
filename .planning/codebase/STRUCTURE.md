# Codebase Structure

**Analysis Date:** 2026-01-23

## Directory Layout

```
splice/
├── src/                     # Core library source code
│   ├── action/             # Suggested actions and confidence scoring
│   ├── cli/                # Command line interface components
│   ├── context/           # Context extraction and analysis
│   ├── diff/               # Diff generation and formatting
│   ├── error.rs            # Error type definitions
│   ├── error_codes.rs     # Error code registry
│   ├── execution/         # Operation orchestration
│   ├── expand/             # Symbol expansion utilities
│   ├── graph/              # SQLiteGraph integration
│   │   ├── magellan_integration.rs  # External tool integration
│   │   └── schema.rs       # Graph node/edge definitions
│   ├── hints/              # Tool hint generation
│   ├── ingest/             # File parsing and symbol extraction
│   │   ├── cpp/            # C/C++ symbol extraction
│   │   ├── imports/        # Cross-language import handling
│   │   ├── java/           # Java symbol extraction
│   │   ├── javascript/    # JavaScript symbol extraction
│   │   ├── python/         # Python symbol extraction
│   │   ├── rust/           # Rust symbol extraction
│   │   ├── typescript/    # TypeScript symbol extraction
│   │   ├── detect.rs       # Language detection
│   │   ├── dispatch.rs     # Multi-language dispatch
│   │   ├── magellan.rs     # Magellan integration
│   │   └── semantic_kind.rs  # Semantic type detection
│   ├── lib.rs              # Library entry point and re-exports
│   ├── main.rs             # CLI application entry
│   ├── output.rs           # Output formatting and display
│   ├── patch/              # File patching and validation
│   │   ├── backup.rs       # Backup and rollback functionality
│   │   ├── batch_loader.rs # Batch operation loading
│   │   └── pattern.rs      # Pattern matching and replacement
│   ├── plan/               # Operation planning and strategy
│   ├── relationships/      # Symbol relationship tracking
│   ├── resolve/            # Symbol resolution algorithms
│   │   ├── cross_file.rs   # Multi-file resolution
│   │   ├── module_resolver.rs  # Module path resolution
│   │   └── references/     # Reference tracking
│   ├── symbol/             # Language-agnostic symbol types
│   ├── suggestions.rs      # Suggestion generation
│   ├── validate/           # Multi-language validation
│   │   └── gates.rs        # Validation gate implementations
│   └── verify.rs           # Verification and pre-flight checks
├── tests/                  # Comprehensive test suite
│   ├── checksum_integration_tests.rs
│   ├── cli_tests.rs
│   ├── cross_language_tests.rs
│   ├── e2e_refactor_tests.rs
│   ├── ingest_tests.rs
│   ├── magellan_integration_tests.rs
│   ├── patch_tests.rs
│   ├── performance_tests.rs
│   ├── rich_span_tests.rs
│   ├── symbol_tests/
│   └── validation_tests.rs
├── docs/                  # Documentation and planning
│   ├── ADR_*.md          # Architecture Decision Records
│   ├── EXECUTIVE_SUMMARY.md
│   ├── MULTI_LANGUAGE_ROADMAP.md
│   └── PHILosophy.md
├── .planning/            # Phase planning documents
│   └── phases/           # Individual phase tracking
├── QUICKSTART.md         # Getting started guide
└── CLAUDE.md             # Development rules and constraints
```

## Directory Purposes

### **Source Directory (`src/`)**
**Purpose:** Core library implementation with clear separation of concerns

**Key Modules:**
- **Ingest:** File parsing → symbol extraction → graph storage
- **Graph:** SQLiteGraph database interface and caching
- **Patch:** Byte-accurate file replacement with validation
- **Symbol:** Language-agnostic symbol abstractions
- **Validate:** Multi-language compilation and analysis validation
- **Resolve:** Symbol resolution and cross-reference tracking

### **Test Directory (`tests/`)**
**Purpose:** Comprehensive test suite with integration, E2E, and performance tests

**Test Categories:**
- **Unit Tests:** Individual component testing
- **Integration Tests:** Multi-component interaction testing
- **E2E Tests:** End-to-end workflow validation
- **Performance Tests:** Benchmarking and optimization
- **Cross-Language Tests:** Multi-language functionality validation

### **Documentation Directory (`docs/`)**
**Purpose:** Project documentation, ADRs, and strategic planning

**Document Types:**
- **Executive Summary:** High-level project overview and status
- **Roadmaps:** Multi-language support plans
- **ADR:** Architecture Decision Records
- **Technical Plans:** Detailed implementation strategies

### **Planning Directory (`.planning/`)**
**Purpose:** Phase-by-phase development tracking and progress monitoring

## Key File Locations

**Entry Points:**
- `/home/feanor/Projects/splice/src/lib.rs` - Library API entry point
- `/home/feanor/Projects/splice/main.rs` - CLI application entry point

**Core Components:**
- `/home/feanor/Projects/splice/src/ingest/mod.rs` - Ingestion pipeline orchestrator
- `/home/feanor/Projects/splice/src/graph/mod.rs` - Graph database interface
- `/home/feanor/Projects/splice/src/patch/mod.rs` - Patching engine with validation
- `/home/feanor/Projects/splice/src/symbol/mod.rs` - Symbol abstraction layer
- `/home/feanor/Projects/splice/src/validate/gates.rs` - Multi-language validation

**Language Support:**
- `/home/feanor/Projects/splice/src/ingest/rust.rs` - Rust symbol extraction
- `/home/feanor/Projects/splice/src/ingest/python.rs` - Python symbol extraction
- `/home/feanor/Projects/splice/src/ingest/cpp.rs` - C/C++ symbol extraction
- `/home/feanor/Projects/splice/src/ingest/java.rs` - Java symbol extraction
- `/home/feanor/Projects/splice/src/ingest/javascript.rs` - JavaScript symbol extraction
- `/home/feanor/Projects/splice/src/ingest/typescript.rs` - TypeScript symbol extraction

## Naming Conventions

**Files:**
- `snake_case.rs` for Rust modules
- `kebab-case.md` for documentation
- `PascalCase.rs` for test files when appropriate

**Directories:**
- `snake_case/` for module directories
- Mixed case for top-level directories (src, tests, docs, .planning)

**Rust Code:**
- `camelCase` for functions and methods
- `SCREAMING_SNAKE_CASE` for constants and enums
- `PascalCase` for types and structs
- `snake_case` for module-level functions

**Symbols:**
- Language-specific types: `RustSymbol`, `PythonSymbol`
- Common interface: `Symbol` trait
- Language enum: `Language::Rust`, `Language::Python`

## Where to Add New Code

**New Language Support:**
- **Implementation:** `/home/feanor/Projects/splice/src/ingest/[language].rs`
- **Symbol Type:** Add to `src/symbol/mod.rs` `AnySymbol` enum
- **Language Enum:** Extend `Language` enum in `src/symbol/mod.rs`
- **Tests:** `/home/feanor/Projects/splice/tests/[language]_tests.rs`

**New Core Feature:**
- **Module:** Create in appropriate directory (e.g., `src/[feature]/`)
- **Public API:** Export from `src/lib.rs`
- **Tests:** Add corresponding test file in `tests/`
- **Documentation:** Update in `docs/`

**New Validation Gate:**
- **Implementation:** Extend `/home/feanor/Projects/splice/src/validate/gates.rs`
- **Language Integration:** Add to `gate_compiler_validation` dispatch
- **Tests:** Add in `tests/validation_tests.rs`

**New CLI Command:**
- **Parsing:** Extend argument parsing in `main.rs`
- **Implementation:** Add to appropriate module or create new
- **Help Text:** Update CLI help documentation

## Special Directories

**`src/ingest/imports/`**
- **Purpose:** Language-specific import extraction
- **Generated:** No - manually maintained
- **Committed:** Yes - contains core language logic

**`tests/`**
- **Purpose:** Comprehensive test coverage
- **Generated:** No - manually maintained
- **Committed:** Yes - critical for regression testing

**`docs/`**
- **Purpose:** Project documentation
- **Generated:** No - manually maintained
- **Committed:** Yes - essential for understanding

**`.planning/`**
- **Purpose:** Development phase tracking
- **Generated:** Partially (phase summaries)
- **Committed:** Yes - important for project history

---

*Structure analysis: 2026-01-23*