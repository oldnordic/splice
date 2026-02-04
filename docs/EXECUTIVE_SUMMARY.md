# Multi-Language Support - Executive Summary

**Date**: 2025-12-28
**Status**: Planning Complete - Awaiting Approval

---

## Overview

Plan to extend **Splice** and **Magellan** from Rust-only to support Python, Go, JavaScript/TypeScript, C/C++, and more.

**Key Principle**: NO GUESSING - All operations grounded in AST facts from tree-sitter.

---

## The Plan in Brief

### Timeline
- **MVP** (Rust + Python): 4 weeks
- **Full Implementation** (6 languages): 14 weeks (~3.5 months)

### Phases
| Phase | Duration | Focus |
|-------|----------|-------|
| 1 | 2 weeks | Language registry foundation |
| 2 | 2 weeks | Python support |
| 3 | 2 weeks | Go support |
| 4 | 2 weeks | JavaScript/TypeScript |
| 5 | 2 weeks | C/C++ |
| 6 | 2 weeks | Magellan multi-language |
| 7 | 2 weeks | Splice multi-language |

---

## Key Architecture Decisions

### 1. Language Registry Pattern
```rust
pub enum Language { Rust, Python, Go, Java, JavaScript, TypeScript, Cpp, C, ... }

pub trait LanguageHandler {
    fn extract_symbols(&self, source: &[u8]) -> Result<Vec<SymbolSpan>>;
    fn validate_patch(&self, patched_source: &str) -> Result<(), ValidationError>;
}
```

### 2. Unified Graph Schema
- SQLiteGraph already supports arbitrary JSON metadata
- Add `language` field to node data
- Language-agnostic `SymbolKind` enum (Function, Class, Struct, etc.)

### 3. Per-Language Validation
| Language | Validation Command |
|----------|-------------------|
| Rust | `cargo check` |
| Python | `python -m py_compile` |
| Go | `go build` |
| TypeScript | `tsc --noEmit` |
| C | `gcc -fsyntax-only` |
| C++ | `g++ -fsyntax-only` |

---

## Industry Research Findings

### Tools Analyzed
1. **ast-grep** - Tree-sitter based, 30+ languages, structural search/replace
2. **Sourcetrail** - Multi-language (C/C++, Java, Python), discontinued but good architecture
3. **Sourcegraph LSIF** - LSP-based, universal index format
4. **SemanticDB** - Language-agnostic data model

### Key Insights
- Tree-sitter supports 165+ languages
- Rust bindings allow dynamic language loading
- Per-language handlers behind shared trait is proven pattern
- Most tools use language-specific validation commands

---

## Immediate Splice Improvements

Before starting multi-language work, fix existing issues:

### High Priority (49 warnings)
- Remove unused imports (10+ instances)
- Remove unused dependencies (`env_logger`, `tempfile`)
- Fix unused variables (5 instances)
- Add missing documentation (enum variants, error fields)

### Code Quality
- Deduplicate `execute_patch()` and `execute_single_step()` (~145 LOC combined)
- Implement `parse_cargo_line()` (currently stub returning `None`)
- [x] Remove dead `Ingestor.graph` field (completed in v2.2.4)

---

## Success Criteria

### Foundation (Phase 1)
- All 22 existing tests pass
- Zero behavioral regression
- Language registry functional

### MVP (Phase 2: Python)
- Python files indexed correctly
- Python symbols in SQLiteGraph
- Python patching works with validation
- Integration tests pass

### Full (All Phases)
- Polyglot codebase indexed successfully
- Cross-language queries work
- Multi-language patching with rollback
- No Rust functionality regression

---

## Risk Assessment

| Risk | Mitigation |
|------|------------|
| Tree-sitter grammar quality varies | Test each language thoroughly |
| Language-specific validation is complex | Start simple, enhance incrementally |
| Performance degradation | Benchmark each phase, add regression tests |
| Maintenance burden increases | Keep handlers isolated, community contributions |

---

## Open Questions

1. **Symbol resolution across languages**: Namespace by language? (Recommend: Yes)
2. **Cross-language reference tracking**: Skip or do FFI-aware? (Recommend: Skip initially)
3. **Python indentation**: How to handle? (Recommend: tree-sitter byte spans + compiler validation)
4. **TypeScript types**: Index as symbols? (Recommend: Yes)
5. **Go packages**: How to represent? (Recommend: `package/path.File` format)

---

## Next Steps

1. Review this plan
2. Prioritize phases based on user demand
3. Approve Phase 1 start
4. Create tracking issues for tasks

---

## Documents Created

1. **MULTI_LANGUAGE_ROADMAP.md** - Detailed technical plan (this summary)
2. **TODO_MULTI_LANG.md** - Task checklist with ~150 items
3. **EXECUTIVE_SUMMARY.md** - This document

---

## Dependencies

```toml
# Phase 2 (Python)
tree-sitter-python = "0.21"

# Phase 3 (Go)
tree-sitter-go = "0.21"

# Phase 4 (JS/TS)
tree-sitter-javascript = "0.21"
tree-sitter-typescript = "0.21"

# Phase 5 (C/C++)
tree-sitter-c = "0.21"
tree-sitter-cpp = "0.21"
```

---

**Recommendation**: Start with Phase 1 + Phase 2 (MVP: Rust + Python) to validate architecture before full commitment.

---

*Document Version*: 1.0.0
*Last Updated*: 2025-12-28
