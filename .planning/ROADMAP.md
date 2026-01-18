# Roadmap: Splice Refactoring Tool

## Overview

Splice is a span-safe refactoring tool for Rust, Python, C, C++, Java, JavaScript, and TypeScript. It uses tree-sitter for AST parsing and provides precise symbol manipulation with automatic validation and rollback.

## Milestones

### v2.0 Overhaul - COMPLETE ✅ (2026-01-18)

A comprehensive overhaul including safety foundation, SQLiteGraph v1.0 upgrade, structured output, stable identifiers, span-aware metadata, deterministic ordering, validation hooks, execution logging, integration testing, and documentation updates.

**Details:** See [milestones/v2.0-ROADMAP.md](milestones/v2.0-ROADMAP.md)

**Key Features:**
- No unwrap() panics in production paths
- SQLiteGraph v1.0 with Native V2 backend
- Structured JSON output with execution_id, match_id, span_id
- Line/column metadata in all output
- Deterministic ordering across operations
- Validation hooks with checksums
- Execution logging audit trail
- 75+ integration tests
- Comprehensive documentation

### Future Milestones

**Planned:** (TBD)
- Performance optimization for large codebases
- Additional language support
- Enhanced undo/redo capabilities
- IDE integration (LSP support)

## Current Status

**Version:** 2.0.0
**Status:** Stable
**Test Coverage:** 334+ tests passing
**Documentation:** Complete

## Contributing

See [PROJECT.md](PROJECT.md) for development guidelines.
