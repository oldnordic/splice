# Splice Multi-Language Support Plan

**Date**: 2025-12-30
**Status**: PLANNING
**Purpose**: Add multi-language support to Splice's CLI while preserving existing Rust functionality

---

## Current State

### What Works
| Language | Parser | Symbols | Imports | CLI | Notes |
|----------|--------|---------|---------|-----|-------|
| Rust | ✅ | ✅ | ✅ | ✅ | Fully functional |
| Python | ✅ | ✅ | ✅ | ❌ | Parser exists, CLI not wired |
| C | ✅ | ✅ | ✅ | ❌ | Via cpp.rs |
| C++ | ✅ | ✅ | ✅ | ❌ | Parser exists, CLI not wired |
| Java | ✅ | ✅ | ✅ | ❌ | Parser exists, CLI not wired |
| JavaScript | ✅ | ✅ | ✅ | ❌ | Parser exists, CLI not wired |
| TypeScript | ✅ | ✅ | ✅ | ❌ | Parser exists, CLI not wired |

### What Doesn't Work
- CLI commands (`delete`, `patch`) are hardcoded to Rust only
- `SymbolKind` enum only includes Rust types (Function, Struct, Enum, Trait, Impl)
- No language detection from file extension
- Graph schema uses Rust-specific labels
- No multi-language validation (only `cargo check` wired up)

---

## Problem Summary

**Splice has excellent multi-language parsing infrastructure, but the CLI layer is Rust-only.**

The architecture is sound - each language has:
- Dedicated parser in `src/ingest/{lang}.rs`
- Import extraction in `src/ingest/imports/`
- Symbol type definitions

But `main.rs` hardcodes:
```rust
use splice::ingest::rust::{extract_rust_symbols, RustSymbolKind};
let symbols = extract_rust_symbols(file_path, &source)?;
```

---

## Implementation Strategy

### Phase 1: Common Abstraction Layer

**Goal**: Create language-agnostic interfaces

**Deliverables**:
1. `src/symbol/common.rs` - Common `Symbol` trait and `Language` enum
2. `src/ingest/dispatch.rs` - Language dispatcher for symbol extraction
3. Update each parser to implement common trait

### Phase 2: CLI Language Detection

**Goal**: Auto-detect language from file extension

**Deliverables**:
1. Add `--language` flag to CLI (optional, auto-detect by default)
2. Update `cli::SymbolKind` to include all language types
3. Route to appropriate parser based on language

### Phase 3: Multi-Language Graph Storage

**Goal**: Store symbols without Rust bias

**Deliverables**:
1. Add `language` column to graph schema
2. Use generic labels (`symbol_function` instead of `rust_function`)
3. Store language-specific metadata

### Phase 4: Multi-Language Validation

**Goal**: Validate refactors for each language

**Deliverables**:
1. Python: `mypy` or `python -m py_compile`
2. Java: `javac`
3. JS/TS: `tsc` or `eslint`
4. C/C++: `clang` or compiler
5. Keep existing `cargo check` for Rust

---

## File Modifications

| File | Changes |
|------|---------|
| `src/main.rs` | Add language detection and dispatch |
| `src/cli/mod.rs` | Add `--language` flag, update `SymbolKind` enum |
| `src/ingest/mod.rs` | Add dispatcher module |
| `src/ingest/dispatch.rs` | NEW - Language-aware symbol extraction |
| `src/symbol/common.rs` | NEW - Common `Symbol` trait |
| `src/graph/mod.rs` | Add language support to graph storage |
| `src/validate/mod.rs` | Add multi-language validators |

---

## CLI Changes

### Before (Rust-only)
```bash
splice delete --file src/main.rs --name foo --kind Function
```

### After (Multi-language)
```bash
# Auto-detect from extension
splice delete --file src/main.rs --name foo --kind Function
splice delete --file src/lib.py --name foo --kind Function

# Explicit language
splice delete --file src/main.rs --name foo --kind Function --language rust
splice delete --file src/lib.py --name foo --kind Function --language python
```

---

## SymbolKind Unification

### Current (Rust-only)
```rust
pub enum SymbolKind {
    Function,
    Struct,
    Enum,
    Trait,
    Impl,
}
```

### Target (Multi-language)
```rust
pub enum SymbolKind {
    // Common
    Function,
    Method,
    Class,
    Struct,
    Interface,
    Enum,
    Module,
    Namespace,
    Variable,
    Field,

    // Language-specific (optional)
    Trait,       // Rust
    Impl,        // Rust
    Constructor, // Java
    TypeAlias,   // TypeScript, Python
    Decorator,   // Python
    // ...
}
```

---

## Success Criteria

1. ✅ All 7 languages work with `delete` command
2. ✅ All 7 languages work with `patch` command
3. ✅ Language auto-detection works from file extension
4. ✅ Validation works for each language
5. ✅ All existing Rust functionality preserved
6. ✅ Zero breaking changes to existing CLI usage

---

*Created: 2025-12-30*
