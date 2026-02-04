# Multi-Language Support Roadmap v2.0

**Project:** Splice + Magellan Unified Code Intelligence
**Version**: 2.0.0 (With Cross-File Resolution)
**Date**: 2025-12-28
**Status**: Planning Phase

---

## Executive Summary

Extend **Splice** and **Magellan** from Rust-only to support multiple programming languages with **95-98% precision cross-file symbol resolution** using import-aware AST analysis.

**Key Principle**: NO GUESSING - All operations grounded in AST facts. Import statements tell us where symbols come from.

---

## What Changed from v1.0

| Aspect | v1.0 | v2.0 |
|--------|------|------|
| Cross-file resolution | Not planned | **Core feature** |
| Precision target | ~50% (same-file only) | **95-98%** |
| Architecture | Simple name matching | **Import-aware resolution** |
| Timeline | 14 weeks | **20 weeks** (includes Magellan work) |

---

## Current State Analysis

### Magellan Current Limitation

From `src/references.rs`:
```rust
// Only matches symbols from SAME FILE
let referenced_symbol = symbols.iter().find(|s| {
    s.name.as_ref().map(|n| n == symbol_name).unwrap_or(false)
})?;
```

**Result**: Can't resolve references across files (~50% effective precision)

### Splice Current Limitation

From `src/graph/mod.rs`:
- Creates temporary `.splice_graph.db` per patch
- No persistent symbol database
- No cross-file tracking

---

## The Solution: Import-Aware Resolution

### Core Insight

**Import statements in the AST tell us exactly where symbols come from.**

```rust
// src/a.rs
pub fn foo() {}

// src/main.rs
use crate::a::foo;

fn main() {
    foo();  // ← We can now resolve this to src/a.rs!
}
```

### How It Works

```
1. Parse AST → Extract symbols WITH module_path
2. Parse AST → Extract imports (use, import, from)
3. Build import graph in SQLiteGraph
4. Resolve symbols by following import paths
```

---

## Architecture

### Language Abstraction

```rust
pub enum Language {
    Rust, Python, Go, JavaScript, TypeScript, Cpp, C, ...
}

pub trait LanguageHandler {
    fn extract_symbols(&self, source: &[u8]) -> Result<Vec<SymbolFact>>;
    fn extract_imports(&self, source: &[u8]) -> Result<Vec<ImportFact>>;
    fn resolve_module_path(&self, path: &str) -> Option<String>;
    fn validate_patch(&self, patched_source: &str) -> Result<(), ValidationError>;
}
```

### Enhanced Symbol Storage

```rust
pub struct SymbolFact {
    pub name: String,
    pub kind: SymbolKind,
    pub language: Language,

    // Cross-file resolution fields
    pub module_path: String,        // "crate::foo::bar::Baz"
    pub fully_qualified: String,    // "crate::foo::bar::Baz"
    pub visibility: Visibility,      // Public, Restricted, Private

    // Location
    pub byte_start: usize,
    pub byte_end: usize,
    pub file_path: PathBuf,
}
```

### Import Storage

```rust
pub struct ImportFact {
    pub file_path: PathBuf,
    pub language: Language,
    pub import_kind: ImportKind,
    pub import_path: Vec<String>,   // ["crate", "b"]
    pub imported_names: Vec<String>, // ["foo", "bar"]
    pub is_glob: bool,
}
```

---

## Implementation Timeline

| Phase | Duration | Focus | Tools |
|-------|----------|-------|-------|
| **Magellan Foundation** | | | |
| 0 | 1 week | Fix Splice warnings | clang |
| 1A | 1 week | Enhanced symbol storage | tree-sitter |
| 1B | 1 week | Import extraction | tree-sitter |
| 1C | 1 week | Module resolution | - |
| 1D | 1 week | Cross-file resolution | - |
| **Multi-Language** | | | |
| 2 | 2 weeks | Python support | tree-sitter-python |
| 3 | 2 weeks | Go support | tree-sitter-go |
| 4 | 2 weeks | JavaScript/TypeScript | tree-sitter-js/ts |
| 5 | 2 weeks | C/C++ | tree-sitter-c/cpp |
| **Integration** | | | |
| 6 | 2 weeks | Magellan multi-language | - |
| 7 | 2 weeks | Splice multi-language | - |
| 8 | 2 weeks | Validation & performance | - |

**Total**: 20 weeks (~5 months)

---

## Per-Language Import Patterns

### Rust

| Pattern | Example | Resolution |
|---------|---------|------------|
| `use crate::X::Y` | `use crate::b::foo;` | Absolute path → file |
| `use super::X` | `use super::foo;` | Parent module → file |
| `use self::X` | `use self::foo;` | Current module → file |
| `use extern_crate::X` | `use std::collections::HashMap;` | External crate → metadata |
| Glob | `use crate::module::*;` | Search target (85% precision) |

### Python

| Pattern | Example | Resolution |
|---------|---------|------------|
| Import module | `import foo` | foo.py → file |
| From import | `from foo import bar` | foo.py → symbol bar |
| Relative | `from . import bar` | sibling/bar.py → file |
| Parent | `from .. import bar` | parent/bar.py → file |
| Glob | `from foo import *` | Search foo.py (85% precision) |

### Go

| Pattern | Example | Resolution |
|---------|---------|------------|
| Package path | `import "github.com/user/repo"` | Go module → file |
| Relative | `import "./local"` | local/ → file |
| Dot import | `import . "package"` | Search package (85% precision) |

### JavaScript/TypeScript

| Pattern | Example | Resolution |
|---------|---------|------------|
| Named import | `import { foo } from 'bar'` | bar.js → symbol foo |
| Namespace | `import * as baz from 'qux'` | qux.js → namespace baz |
| Relative | `import './local.js'` | sibling/local.js → file |

---

## Precision Targets

### Per-Language Precision

| Language | Same-File | Cross-File | Overall |
|----------|-----------|------------|---------|
| Rust | 95% | **95-98%** | **95-98%** |
| Python | 95% | **90-95%** | **92-95%** |
| Go | 95% | **95-98%** | **95-98%** |
| JavaScript | 90% | **85-90%** | **87-92%** |
| TypeScript | 95% | **90-95%** | **92-95%** |
| C/C++ | 95% | **85-90%** | **87-92%** |

**Weighted Average**: **93-96%** (up from ~50% current)

### Why Precision Varies

| Factor | Impact |
|--------|--------|
| Strong typing | Higher precision |
| Explicit imports | Higher precision |
| Dynamic features | Lower precision |
| Glob/star imports | Lower precision |
| Macros/template expansion | Lower precision |

---

## Phase 0: Splice Cleanup (Week 1)

### Tasks

#### High Priority (49 warnings)
- [ ] Remove unused imports (10+ instances)
  - [ ] `PropertyValue` in `src/graph/schema.rs`
  - [ ] Multiple imports in `src/plan/mod.rs`
  - [ ] Unused imports in test files
- [ ] Remove unused dependencies
  - [ ] `env_logger` (use statement only)
  - [ ] `tempfile` (tests use tempdir directly)
- [ ] Fix unused variables (5 instances)
  - [ ] `cache_key` in `src/resolve/mod.rs`
  - [ ] `path` in `src/ingest/mod.rs`
  - [ ] `line` in `src/validate/mod.rs`
- [ ] Add missing documentation
  - [ ] Enum variants in `cli.rs`
  - [ ] Error struct fields in `error.rs`

#### Code Quality
- [x] Remove dead `Ingestor.graph` field (completed in v2.2.4)
- [ ] Remove unused functions in `validate/mod.rs`
- [ ] Implement `parse_cargo_line()` (currently stub)
- [ ] Deduplicate `execute_patch()` and `execute_single_step()`

**Completion Criteria**:
- Zero compiler warnings
- No dead code
- All documentation present

---

## Phase 1A: Enhanced Symbol Storage (Week 2)

### File: `src/ingest/mod.rs` (modify)

**Tasks**:
- [ ] Add `module_path: String` to `SymbolFact`
- [ ] Add `fully_qualified: String` to `SymbolFact`
- [ ] Add `visibility: Visibility` to `SymbolFact`
- [ ] Extract module declarations from AST
- [ ] Build fully qualified names during extraction
- [ ] Detect visibility (pub, pub(crate), etc.)

**Tests**:
- [ ] Module path extraction
- [ ] Visibility detection
- [ ] Fully qualified names

---

## Phase 1B: Import Extraction (Week 3)

### File: `src/ingest/imports.rs` (new)

**Tasks**:
- [ ] Create `ImportFact` struct
- [ ] Create `ImportKind` enum
- [ ] Create `ImportExtractor` (tree-sitter based)
- [ ] Extract all Rust import patterns
- [ ] Store imports in SQLiteGraph
- [ ] Write comprehensive tests

**Tests**:
- [ ] All import patterns
- [ ] Nested imports
- [ ] Glob imports
- [ ] Re-exports

---

## Phase 1C: Module Resolution (Week 4)

### File: `src/resolve/module.rs` (new)

**Tasks**:
- [ ] Create `ModuleResolver`
- [ ] Build `module_path → file_id` index
- [ ] Resolve `crate::*` paths
- [ ] Resolve `super::*` paths
- [ ] Resolve `self::*` paths
- [ ] Handle relative paths

**Tests**:
- [ ] Absolute resolution
- [ ] Relative resolution
- [ ] Nested modules
- [ ] Edge cases

---

## Phase 1D: Cross-File Resolution (Week 5)

### File: `src/resolve/cross_file.rs` (new)

**Tasks**:
- [ ] Implement `resolve_symbol_cross_file()`
- [ ] Query import edges
- [ ] Follow import paths
- [ ] Return resolved symbol
- [ ] Handle name collisions
- [ ] Handle glob imports

**Tests**:
- [ ] Cross-file resolution
- [ ] Import chain following
- [ ] Glob imports
- [ ] Name collisions

---

## Phase 2: Python Support (Weeks 6-7)

### Dependencies
```toml
tree-sitter-python = "0.21"
```

### File: `src/languages/python.rs` (new)

**Tasks**:
- [ ] Implement `LanguageHandler` for Python
- [ ] Extract symbols (functions, classes, methods)
- [ ] Extract imports (`import`, `from ... import`)
- [ ] Extract module structure
- [ ] Handle relative imports
- [ ] Build module paths
- [ ] Validate with `python -m py_compile`

**Python Import Patterns**:
- `import foo` → module `foo`
- `from foo import bar` → symbol `bar` from `foo`
- `from . import foo` → relative import
- `from ..bar import baz` → parent-relative
- `from foo import *` → glob import

**Tests**:
- [ ] Python file indexing
- [ ] Python import extraction
- [ ] Cross-file resolution
- [ ] Patching with validation

---

## Phase 3: Go Support (Weeks 8-9)

### Dependencies
```toml
tree-sitter-go = "0.21"
```

### File: `src/languages/go.rs` (new)

**Tasks**:
- [ ] Implement `LanguageHandler` for Go
- [ ] Extract symbols (functions, methods, structs, interfaces)
- [ ] Extract imports (`import "..."`)
- [ ] Handle Go packages
- [ ] Build package paths
- [ ] Validate with `go build`

**Go Import Patterns**:
- `import "github.com/user/repo"` → package path
- `import "./local"` → relative import
- `import . "package"` → dot import (glob)

**Tests**:
- [ ] Go file indexing
- [ ] Go import extraction
- [ ] Package resolution
- [ ] Patching with validation

---

## Phase 4: JavaScript/TypeScript (Weeks 10-11)

### Dependencies
```toml
tree-sitter-javascript = "0.21"
tree-sitter-typescript = "0.21"
```

### Files: `src/languages/javascript.rs`, `src/languages/typescript.rs` (new)

**Tasks**:
- [ ] Implement `LanguageHandler` for JavaScript
- [ ] Implement `LanguageHandler` for TypeScript
- [ ] Extract symbols (functions, classes, variables)
- [ ] Extract imports (`import`, `require`)
- [ ] Handle CommonJS vs ES modules
- [ ] Validate with `tsc --noEmit` or node

**JS/TS Import Patterns**:
- `import { foo } from 'bar'` → named import
- `import * as baz from 'qux'` → namespace
- `require('./foo')` → CommonJS
- `import './foo.js'` → relative

**Tests**:
- [ ] JS/TS file indexing
- [ ] JS/TS import extraction
- [ ] Module resolution
- [ ] Patching with validation

---

## Phase 5: C/C++ (Weeks 12-13)

### Dependencies
```toml
tree-sitter-c = "0.21"
tree-sitter-cpp = "0.21"
```

### Files: `src/languages/c.rs`, `src/languages/cpp.rs` (new)

**Tasks**:
- [ ] Implement `LanguageHandler` for C
- [ ] Implement `LanguageHandler` for C++
- [ ] Extract symbols (functions, structs, classes)
- [ ] Extract `#include` directives
- [ ] Handle header files
- [ ] Validate with `gcc -fsyntax-only` / `g++`

**C/C++ Import Patterns**:
- `#include "foo.h"` → local header
- `#include <foo.h>` → system header
- `#include "../bar.h"` → relative header

**Tests**:
- [ ] C/C++ file indexing
- [ ] Include extraction
- [ ] Header resolution
- [ ] Patching with validation

---

## Phase 6: Magellan Multi-Language (Weeks 14-15)

### File: `src/main.rs`, `src/watcher.rs`

**Tasks**:
- [ ] Remove hardcoded `.rs` filter
- [ ] Detect language from file extension
- [ ] Use `LanguageRegistry` for all operations
- [ ] Index polyglot codebases
- [ ] Update `status` for per-language counts
- [ ] Update `export` for all languages

**Tests**:
- [ ] Polyglot indexing
- [ ] Per-language queries
- [ ] Cross-language queries
- [ ] Performance tests

---

## Phase 7: Splice Multi-Language (Weeks 16-17)

### File: `src/patch/mod.rs`, `src/plan/mod.rs`

**Tasks**:
- [ ] Per-language validation commands
- [ ] Language-aware patching
- [ ] Cross-language refactoring (name-only)
- [ ] Update CLI for multi-language
- [ ] Handle language-specific formatting

**Validation Commands**:
| Language | Command |
|----------|---------|
| Rust | `cargo check` |
| Python | `python -m py_compile` |
| Go | `go build` |
| TypeScript | `tsc --noEmit` |
| C | `gcc -fsyntax-only` |
| C++ | `g++ -fsyntax-only` |

**Tests**:
- [ ] Patch each language
- [ ] Validation per language
- [ ] Rollback on failure
- [ ] Integration tests

---

## Phase 8: Validation & Performance (Weeks 18-19)

### Precision Validation

**Tasks**:
- [ ] Create validation test suite
- [ ] Test on real projects (ripgrep, requests, gin, express)
- [ ] Sample 500 random symbol references
- [ ] Compare with LSP (rust-analyzer, pylsp, gopls)
- [ ] Calculate precision per language
- [ ] Target: >93% weighted average

### Performance

**Tasks**:
- [ ] Benchmark import extraction
- [ ] Benchmark cross-file resolution
- [ ] Benchmark multi-language indexing
- [ ] Add caching if needed
- [ ] Target: <100ms per query

### Real-World Testing

**Projects to test**:
| Language | Project | Size |
|----------|---------|------|
| Rust | ripgrep | ~50K LOC |
| Python | requests | ~10K LOC |
| Go | gin | ~10K LOC |
| TypeScript | tsx | ~20K LOC |
| C | redis | ~100K LOC |

**Completion Criteria**:
- Precision >93%
- Performance <100ms
- All real-world projects work

---

## Risk Mitigation

| Risk | Mitigation |
|------|------------|
| Tree-sitter grammar quality | Test each language thoroughly |
| Performance degradation | Benchmark each phase |
| Name collisions | Document behavior, return all matches |
| Maintenance burden | Keep handlers isolated |
| Python indentation | Use tree-sitter + compiler validation |

---

## Success Criteria

### Foundation (Phases 0-1)
- [ ] Splice warnings fixed
- [ ] Enhanced symbol storage
- [ ] Import extraction working
- [ ] Module resolution working
- [ ] Cross-file resolution working
- [ ] All existing tests pass

### Multi-Language (Phases 2-5)
- [ ] Python support complete
- [ ] Go support complete
- [ ] JS/TS support complete
- [ ] C/C++ support complete
- [ ] All languages can index symbols
- [ ] All languages can resolve cross-file

### Integration (Phases 6-7)
- [ ] Magellan indexes polyglot codebases
- [ ] Splice patches all languages
- [ ] Cross-language queries work
- [ ] No regression in Rust functionality

### Validation (Phase 8)
- [ ] Precision >93% overall
- [ ] Performance <100ms per query
- [ ] Real-world projects work
- [ ] Documentation complete

---

## Summary

| Metric | Current | Target |
|--------|---------|--------|
| Languages supported | 1 | 6 |
| Cross-file resolution | No | Yes |
| Same-file precision | 95% | 95% |
| Cross-file precision | 0% | 93-96% |
| Overall precision | ~50% | **93-96%** |
| Timeline | — | 20 weeks |

---

## Related Documents

- [CROSS_FILE_RESOLUTION.md](./CROSS_FILE_RESOLUTION.md) - Detailed design
- [EXECUTIVE_SUMMARY.md](./EXECUTIVE_SUMMARY.md) - High-level overview
- [TODO_MULTI_LANG.md](./TODO_MULTI_LANG.md) - Task checklist

---

**Document Version**: 2.0.0
**Last Updated**: 2025-12-28
**Status**: DRAFT - Pending Approval
