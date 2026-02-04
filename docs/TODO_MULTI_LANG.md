# Multi-Language Support TODO

**Linked Document**: [MULTI_LANGUAGE_ROADMAP.md](./MULTI_LANGUAGE_ROADMAP.md)
**Last Updated**: 2025-12-28

---

## Status Legend

- [ ] TODO - Not started
- [~] IN PROGRESS - Started but not complete
- [x] DONE - Completed
- [!] BLOCKED - Waiting on dependency

---

## Phase 1: Foundation (Weeks 1-2)

### 1.1 Create Language Abstraction Module
- [ ] Create `src/languages/mod.rs`
- [ ] Define `Language` enum (Rust, Python, Go, Java, JS, TS, Cpp, C, etc.)
- [ ] Define `SymbolKind` enum (Function, Method, Class, Struct, etc.)
- [ ] Define `SymbolSpan` struct
- [ ] Implement `Language::from_extension()`
- [ ] Implement `Language::extensions()`
- [ ] Implement `Language::tree_sitter_language()`

### 1.2 Create Language Registry
- [ ] Create `src/languages/registry.rs`
- [ ] Define `LanguageRegistry` struct
- [ ] Define `LanguageHandler` trait
- [ ] Implement extension-to-language mapping
- [ ] Add methods: `register()`, `get_handler()`, `detect_from_path()`

### 1.3 Refactor Existing Rust Implementation
- [ ] Create `src/languages/rust.rs` from `src/ingest/rust.rs`
- [ ] Implement `LanguageHandler` for Rust
- [ ] Update `src/ingest/mod.rs` to use new abstraction
- [ ] Add backward compatibility shims for existing API
- [ ] Update imports across codebase

### 1.4 Tests
- [ ] Test language detection from extensions
- [ ] Test registry registration
- [ ] Verify all 22 existing tests still pass
- [ ] Add tests for new language abstraction types
- [ ] Test backward compatibility

**Phase 1 Completion Criteria**:
- All existing tests pass
- `Language::from_extension("rs")` returns `Language::Rust`
- No behavioral changes to existing functionality

---

## Phase 2: Python Support (Weeks 3-4)

### 2.1 Dependencies
- [ ] Add `tree-sitter-python = "0.21"` to Cargo.toml
- [ ] Update dependency documentation
- [ ] Verify Python 3.x available for validation

### 2.2 Python Language Handler
- [ ] Create `src/languages/python.rs`
- [ ] Implement `LanguageHandler` trait for Python
- [ ] Extract: functions, classes, methods, imports
- [ ] Map Python AST nodes to `SymbolKind`:
  - `function_definition` → `Function`
  - `class_definition` → `Class`
  - `decorated_definition` → handle decorators
  - `import` / `import_from` → track as `Import` symbols
- [ ] Handle Python-specific features:
  - Indentation-based structure
  - Decorators
  - Context managers (`with` statements)
  - List/dict/set comprehensions

### 2.3 Python Validation
- [ ] Implement `validate_patch()` using `python -m py_compile`
- [ ] Detect Python syntax errors correctly
- [ ] Handle Python 2 vs 3 differences (Python 3 only)
- [ ] Test rollback on validation failure

### 2.4 CLI Updates
- [ ] Magellan: Detect `.py` extension, use Python handler
- [ ] Splice: Detect `.py` extension, use Python handler
- [ ] Add `--language python` explicit flag support
- [ ] Update help text

### 2.5 Python Integration Tests
- [ ] Index single Python file
- [ ] Index Python Flask/Django project
- [ ] Patch Python function successfully
- [ ] Verify rollback on Python syntax error
- [ ] Test Python class modification
- [ ] Test Python method patching

**Phase 2 Completion Criteria**:
- Python file indexed correctly
- Python symbols stored in SQLiteGraph
- Python patch validated with `python -m py_compile`
- Integration tests pass

---

## Phase 3: Go Support (Weeks 5-6)

### 3.1 Dependencies
- [ ] Add `tree-sitter-go = "0.21"` to Cargo.toml
- [ ] Verify Go compiler available for validation

### 3.2 Go Language Handler
- [ ] Create `src/languages/go.rs`
- [ ] Implement `LanguageHandler` trait for Go
- [ ] Extract: functions, methods, structs, interfaces, packages
- [ ] Map Go AST nodes to `SymbolKind`:
  - `function_declaration` → `Function`
  - `method_declaration` → `Method`
  - `type_spec` (struct) → `Struct`
  - `type_spec` (interface) → `Interface`
  - `package_clause` → track package name
- [ ] Handle Go-specific features:
  - Go routines (`go func()`)
  - Channels (`chan`, `<-`)
  - Deferred statements (`defer`)
  - Interface satisfaction (duck typing)
  - Embedded structs

### 3.3 Go Package System
- [ ] Map Go package declarations to modules
- [ ] Use `package/path.File` as fully qualified name
- [ ] Track `import` statements
- [ ] Handle vendored packages

### 3.4 Go Validation
- [ ] Implement `validate_patch()` using `go build`
- [ ] Detect Go workspace (go.mod)
- [ ] Handle module-aware validation
- [ ] Test rollback on validation failure

### 3.5 Go Integration Tests
- [ ] Index single Go file
- [ ] Index Go standard library subset
- [ ] Index Go project with multiple packages
- [ ] Patch Go function successfully
- [ ] Test Go interface modification
- [ ] Test Go struct modification

**Phase 3 Completion Criteria**:
- Go file indexed correctly
- Go package boundaries respected
- Go patch validated with `go build`
- Integration tests pass

---

## Phase 4: JavaScript/TypeScript Support (Weeks 7-8)

### 4.1 Dependencies
- [ ] Add `tree-sitter-javascript = "0.21"` to Cargo.toml
- [ ] Add `tree-sitter-typescript = "0.21"` to Cargo.toml
- [ ] Verify Node.js + TypeScript compiler available

### 4.2 JavaScript Language Handler
- [ ] Create `src/languages/javascript.rs`
- [ ] Implement `LanguageHandler` trait for JavaScript
- [ ] Extract: functions, classes, variables, imports
- [ ] Map JS AST nodes to `SymbolKind`:
  - `function_declaration`, `function_expression` → `Function`
  - `class_declaration`, `class_expression` → `Class`
  - `variable_declaration` → `Variable`
  - `method_definition` → `Method`
- [ ] Handle JS-specific features:
  - CommonJS (`require`, `module.exports`)
  - ES6 modules (`import`, `export`)
  - Arrow functions
  - Template literals
  - Async/await

### 4.3 TypeScript Language Handler
- [ ] Create `src/languages/typescript.rs`
- [ ] Implement `LanguageHandler` trait for TypeScript
- [ ] Extract all JS symbols plus TypeScript-specific:
  - `interface_declaration` → `Interface`
  - `type_alias_declaration` → `TypeAlias`
  - `enum_declaration` → `Enum`
  - Generic type parameters
- [ ] Handle TS-specific features:
  - Type annotations
  - Decorators
  - Namespace declarations
  - Ambient declarations

### 4.4 JS/TS Validation
- [ ] Implement JS validation (optional `eslint` or node syntax check)
- [ ] Implement TS validation using `tsc --noEmit`
- [ ] Detect Node.js project (package.json)
- [ ] Detect TypeScript project (tsconfig.json)
- [ ] Test rollback on validation failure

### 4.5 JS/TS Integration Tests
- [ ] Index JavaScript file
- [ ] Index TypeScript file
- [ ] Index popular JS/TS project (e.g., small Express app)
- [ ] Patch JS function successfully
- [ ] Patch TS interface successfully
- [ ] Test TS type modification

**Phase 4 Completion Criteria**:
- JS/TS files indexed correctly
- TypeScript type symbols captured
- Validation via TypeScript compiler
- Integration tests pass

---

## Phase 5: C/C++ Support (Weeks 9-10)

### 5.1 Dependencies
- [ ] Add `tree-sitter-c = "0.21"` to Cargo.toml
- [ ] Add `tree-sitter-cpp = "0.21"` to Cargo.toml
- [ ] Verify GCC/Clang available for validation

### 5.2 C Language Handler
- [ ] Create `src/languages/c.rs`
- [ ] Implement `LanguageHandler` trait for C
- [ ] Extract: functions, structs, enums, variables
- [ ] Handle C-specific features:
  - Preprocessor directives (`#include`, `#define`)
  - Function pointers
  - Typedefs

### 5.3 C++ Language Handler
- [ ] Create `src/languages/cpp.rs`
- [ ] Implement `LanguageHandler` trait for C++
- [ ] Extract all C symbols plus C++-specific:
  - Classes, templates
  - Namespaces
  - Member functions
- [ ] Handle C++-specific features:
  - Template declarations
  - Namespace declarations
  - Constructor/destructor syntax
  - Operator overloads

### 5.4 Header File Handling
- [ ] Track header/source relationships
- [ ] Handle `.h` and `.hpp` files
- [ ] Include directory resolution
- [ ] Avoid duplicate symbol indexing from includes

### 5.5 C/C++ Validation
- [ ] Implement C validation using `gcc -fsyntax-only` or `clang -fsyntax-only`
- [ ] Implement C++ validation using `g++ -fsyntax-only` or `clang++ -fsyntax-only`
- [ ] Detect build system (CMake, Makefile)
- [ ] Handle include path dependencies

### 5.6 C/C++ Integration Tests
- [ ] Index C file
- [ ] Index C++ file
- [ ] Index small C project with headers
- [ ] Patch C function successfully
- [ ] Patch C++ class successfully
- [ ] Test template modification

**Phase 5 Completion Criteria**:
- C/C++ files indexed correctly
- Header/source relationships tracked
- Validation via compiler
- Integration tests pass

---

## Phase 6: Enhanced Magellan (Weeks 11-12)

### 6.1 Remove Hardcoded Rust Filters
- [ ] Remove hardcoded `.rs` filter in watcher
- [ ] Use `LanguageRegistry::detect_from_path()`
- [ ] Update `watch` command for multi-language

### 6.2 Multi-Language Workspace Indexing
- [ ] Index polyglot codebases (Rust + Python + JS)
- [ ] Track language distribution in database
- [ ] Update `status` command to show per-language counts

### 6.3 Cross-Language Queries
- [ ] Allow querying across all languages
- [ ] Add `--language` filter to queries
- [ ] Support language-qualified symbol names (e.g., `python:main`)

### 6.4 Magellan Integration Tests
- [ ] Test polyglot codebase indexing
- [ ] Test cross-language queries
- [ ] Test per-language filtering
- [ ] Performance test (100 files per language)

**Phase 6 Completion Criteria**:
- Magellan indexes polyglot codebase
- Status reports per-language counts
- Cross-language queries work

---

## Phase 7: Enhanced Splice (Weeks 13-14)

### 7.1 Per-Language Validation Strategies
- [ ] Python: `python -m py_compile`
- [ ] Go: `go build`
- [ ] JavaScript: optional `eslint --no-eslintrc` or node check
- [ ] TypeScript: `tsc --noEmit`
- [ ] C: `gcc -fsyntax-only`
- [ ] C++: `g++ -fsyntax-only`
- [ ] Rust: `cargo check` (existing)

### 7.2 Language-Aware Patching
- [ ] Respect Python indentation
- [ ] Preserve language-specific comment styles
- [ ] Handle brace styles correctly (C vs Go vs Rust)
- [ ] Test indentation preservation for Python

### 7.3 Cross-Language Refactoring
- [ ] Implement name-only symbol rename across languages
- [ ] Update import statements per language
- [ ] Document limitations (no semantic cross-language refs)

### 7.4 Splice Integration Tests
- [ ] Patch Python file with rollback
- [ ] Patch Go file with rollback
- [ ] Patch JS/TS file with rollback
- [ ] Patch C/C++ file with rollback
- [ ] Test cross-language rename (name-only)

**Phase 7 Completion Criteria**:
- Splice patches all supported languages
- Validation works per-language
- Rollback on any validation failure

---

## Immediate Splice Improvements (Pre-Phase 1)

These are the issues identified in the current codebase that should be fixed first:

### High Priority
- [ ] Fix compiler warnings (49 warnings)
  - [ ] Remove unused imports
  - [ ] Remove unused dependencies (`env_logger`, `tempfile`)
  - [ ] Prefix unused variables with underscore
  - [ ] Add missing documentation
- [x] Remove unused `Ingestor.graph` field (removed in v2.2.4)
- [ ] Remove dead code in `validate/mod.rs` (unused functions)
- [ ] Implement `parse_cargo_line()` (currently returns `None`)

### Medium Priority
- [ ] Deduplicate `execute_patch()` and `execute_single_step()`
- [ ] Add missing docs to error types and enum variants
- [ ] Clean up unused `env_logger` dependency

### Low Priority
- [ ] Line/col tracking in `ResolvedSpan` (currently returns zeros)
- [ ] Resume mode for failed plans

---

## Research Tasks

- [ ] Benchmark tree-sitter parsing performance per language
- [ ] Research FFI mechanisms (PyO3, cgo, JNI, node-ffi)
- [ ] Design cross-language reference tracking (Phase 8+)
- [ ] Investigate LSIF import/export for LSP compatibility

---

## Documentation Tasks

- [ ] Update README.md with multi-language support
- [ ] Update manual.md with per-language examples
- [ ] Add language-specific quick start guides
- [ ] Document configuration file format (`.splice.toml`)
- [ ] Document FFI limitations

---

## Completed

*None yet - this is the initial planning document.*

---

**Last Updated**: 2025-12-28
**Total Tasks**: ~150 (excluding subtasks)
**Completed**: 0
**In Progress**: 0
**Pending**: 150
