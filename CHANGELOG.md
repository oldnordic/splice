# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added
- Initial release of Splice v0.1.0
- Span-safe byte replacement for Rust code
- Tree-sitter AST validation
- Cargo check compilation validation
- Atomic rollback on validation failures
- Multi-step JSON plan orchestration
- CLI interface with `patch` and `plan` commands

### Features
- **Single Patch Mode**: Replace function bodies, struct definitions, enum variants with byte-accurate precision
- **Validation Gates**: UTF-8 boundary check, tree-sitter reparse, cargo check
- **Plan Execution**: JSON-based multi-step refactoring orchestration
- **Error Handling**: Typed error hierarchy with helpful messages

### Known Limitations
- No cross-file reference tracking
- No persistent symbol database
- No resume mode for failed plans
- No dry-run mode
- Single-file symbol resolution only

## [0.1.0] - 2025-12-23

### Added
- Initial MVP release
- Complete feature set for single-file refactoring
- Comprehensive test suite (22 tests)
- Documentation (README.md, manual.md, QUICKSTART.md)
- GPL-3.0-or-later licensing

### Technical
- Built on tree-sitter-rust 0.21
- SQLiteGraph 0.2.4 integration
- ropey 1.6 for safe text editing
- clap 4.5 for CLI parsing

### Documentation
- User manual with examples
- Quick start guide
- Architecture overview
- Troubleshooting guide

---

**Note**: This is a proof-of-concept/MVP release. The feature set is intentionally limited and feature-frozen. No new features are planned for v0.1.x.
