# Changelog

Format based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/).
Project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.2.1] - 2025-12-30

### Changed

- Clarified delete command guarantees: workspace scope, exclusions, invariants
- Added Feedback section to README

## [0.2.0] - 2025-12-30

### Added

- **delete command**: Remove symbol definitions and all their references
- **Cross-file reference finding**: Tracks and removes references across multiple files
- **Shadowing detection**: Correctly handles local variables that shadow imported symbols
- **Re-export chain following**: Finds references through `pub use` re-exports (single-hop; chained not guaranteed)
- **Trait method reference detection**: Handles `value.method()`, `Trait::method()`, and `Type::method()` patterns
- **Multi-language support**: Import extraction for C/C++, Java, JavaScript, Python, TypeScript

### Behavior Guarantee (delete command)

For **public Rust functions** (those intended to be imported across modules), `delete` finds all references that reach the definition through:
- Direct imports: `use crate::foo::bar`
- Re-exports: `pub use crate::foo::bar as baz`
- Same-file unqualified calls: `bar()`

**Scope**: workspace = all `.rs` files under the current working directory

**Exclusions** (by design, not limitation):
- Private functions (no cross-file tracking)
- Fully-qualified paths: `crate::foo::bar()` (not tracked)
- Macro-generated references (not tracked)
- Shadowed names: local `fn bar` shadows import (correctly ignored)

**Deletion order**: References first (reverse byte order per file to preserve offsets), then definition

### What Did NOT Change

- No persistent database between runs
- No resume mode for failed plans
- No dry-run mode
- Atomic rollback on validation failure (still guaranteed)
- Byte spans remain source of truth (still guaranteed)
- Tree-sitter reparse before compiler gate (still guaranteed)

### Changed

- 298 passing tests (from 22)

### Technical

- tree-sitter parsers for C, C++, Java, JavaScript, Python, TypeScript
- Module path index for cross-language resolution
- Scope-based shadowing detection
- Re-export graph building

## [0.1.3] - 2025-12-28

### Fixed
- Added `readme = "README.md"` to Cargo.toml (was missing, causing no README on crates.io)

## [0.1.2] - 2025-12-28

### Changed
- Updated README to reflect MVP/POC status
- Removed emojis from documentation
- Clarified limitations in manual

## [0.1.1] - 2025-12-28

### Changed

- Updated documentation (removed emojis, clarified MVP status)
- README and manual simplified for clarity

## [0.1.0] - 2025-12-23

### Added

- Span-safe byte replacement for Rust code
- Tree-sitter AST validation
- Cargo check compilation validation
- Atomic rollback on validation failures
- Multi-step JSON plan orchestration
- CLI interface with `patch` and `plan` commands

### Features

- **Single Patch Mode**: Replace function bodies, struct definitions, enum variants
- **Validation Gates**: UTF-8 boundary check, tree-sitter reparse, cargo check
- **Plan Execution**: JSON-based multi-step refactoring
- **Error Handling**: Typed error hierarchy

### Known Limitations

- No cross-file reference tracking
- No persistent symbol database
- No resume mode for failed plans
- No dry-run mode
- Single-file symbol resolution only

### Technical

- Built on tree-sitter-rust 0.21
- SQLiteGraph 0.2.4 integration
- ropey 1.6 for text editing
- clap 4.5 for CLI parsing

### Testing

- 22/22 tests passing
- Complete feature set for single-file refactoring

---

**Note**: v0.1.x was an MVP/proof-of-concept release with intentionally limited features. v0.2.0 adds the delete command with cross-file reference finding.
