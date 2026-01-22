# Splice Project Summary

Splice is a span-safe refactoring kernel supporting 7 languages (Rust, Python, C, C++, Java, JavaScript, TypeScript) using tree-sitter and SQLiteGraph with Magellan integration.

## Core Architecture

### Primary Modules
- **graph/**: SQLiteGraph integration layer (CodeGraph struct)
- **ingest/**: Multi-language symbol extraction pipeline (currently incomplete)
- **patch/**: Byte-accurate replacement with validation
- **validate/**: Tree-sitter + compiler validation gates
- **plan/**: JSON plan orchestration for multi-step refactors
- **execution/**: Operation logging with audit trail
- **output/**: Structured JSON v2.0 output format

### Key Features
- **Span-accurate editing**: Byte-level precision using tree-sitter spans
- **Multi-language support**: 7 languages with appropriate validation
- **Atomic operations**: All changes succeed or rollback together
- **Audit trail**: Complete logging in `.splice/operations.db`
- **Validation gates**: Pre/post checksums + AST + compiler checks

## Current State

### Implemented Features
- ✅ CLI with full command set (delete, patch, apply-files, plan, query, get, undo, log)
- ✅ Magellan v0.5.3 integration with label queries and code chunks
- ✅ Tree-sitter validation for all supported languages
- ✅ Structured JSON output with v2.0 schema
- ✅ Atomic operations with rollback on failure
- ✅ 215+ passing tests

### Pending Implementation
- ❌ **Ingest module**: Currently returns "Not implemented yet" errors
  - Symbol extraction is delegated to individual language modules
  - Magellan integration wrapper exists but not fully utilized
  - Main Ingestor methods are placeholders

### Technical Approach
- Uses Magellan v0.5.3 for code indexing (native-v2 backend)
- Tree-sitter kept for patch validation (AST-based)
- sqlitegraph re-exported through Magellan
- Ropey for safe byte-level text editing
- Comprehensive validation with language-specific compilers

## Key Design Principles

1. **No guessing**: Explicit spans over heuristics
2. **Atomic operations**: Rollback on any failure
3. **Structured facts**: JSON output over unstructured text
4. **Multi-language support**: Consistent API across 7 languages
5. **Auditability**: Complete operation history with timestamps

## Command Examples

```bash
# Delete a Rust function with full reference resolution
splice delete --file src/lib.rs --symbol helper --kind function

# Patch a Python function
splice patch --file utils.py --symbol calculate --with new_calc.py --language python

# Query symbols by labels
splice query --db code.db --label rust --label fn

# Multi-step plan
splice plan --file plan.json
```

## Future Direction

The ingest module needs completion to fully utilize Magellan's capabilities:
- Replace placeholder methods with Magellan integration
- Remove redundant language-specific parsers
- Implement proper symbol ingestion using Magellan's API
- Add code chunk retrieval without file re-reading

The project demonstrates a robust approach to multi-language refactoring with safety, precision, and auditability as core principles.