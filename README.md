# splice

[![Crates.io](https://img.shields.io/crates/v/splice)](https://crates.io/crates/splice)
[![Documentation](https://docs.rs/splice/badge.svg)](https://docs.rs/splice)

**Version:** 3.0.0

Span-safe refactoring kernel for 7 languages with agent-focused surface. Byte-accurate code editing with graph algorithm analysis.

**Positioning:** Precision editing tool for codebases indexed by Magellan. Use for cross-file rename, impact analysis, and verified refactoring.

## Purpose

**splice edits code with graph-aware safety.** Built for:

- **AI Agents** — Agent-friendly edit/suggest commands with validation
- **Developers** — Cross-file rename, safe refactoring with rollback
- **Grounded Assistants** — Byte-accurate edits with proof-based verification
- **Tooling** — Scriptable transformations with AST validation

## Features

- **Agent surface (v3.0)** — New commands for AI agent workflows
  - `splice edit` — Text-replace with validation (alternative to Write tool)
  - `splice suggest` — Intent-based scaffold generation with imports
  - Auto-discovery — `--db` flag optional (git root inference)
- **Span-safe editing** — Byte-accurate replacements with tree-sitter validation
- **Cross-file rename** — Symbol rename across entire codebase
- **Graph algorithms** — Reachability, dead-code, cycles, condense, slice
- **Impact analysis** — Blast zone detection before editing
- **Proof-based refactoring** — Machine-checkable behavioral equivalence
- **Code completion** — Grounded, import-aware suggestions using Magellan database
  - Cross-file symbol resolution via import tracking
  - Distinguishes local vs imported symbols with source tracking
  - Token filtering and enhanced ranking for imported symbols
  - Performance: low-millisecond internal query time on indexed project databases
- **7 languages** — Rust, Python, C, C++, Java, JavaScript, TypeScript
- **Library API** — Use as a Rust crate for programmatic refactoring (forge, agents)

## Quick Start

```bash
# Install
cargo install splice

# Requires Magellan database (create first)
magellan watch --root ./src --db .magellan/splice.db

# Check status (auto-discovers DB from git root)
splice status

# Agent surface: edit (text-replace with validation)
splice edit --file src/lib.rs --replace-old "old_function()" --replace-new "new_function()" --preview

# Agent surface: suggest (intent-based scaffolding)
splice suggest --fn-name "process_data" --desc "Parse JSON and validate schema" --output json

# Cross-file rename
splice rename --symbol old_name --to new_name --path src/lib.rs

# Code completion (import-aware, for IDE/LSP integrations)
splice complete --file src/lib.rs --line 27 --column 8 --max-results 10
```

## Library Usage

Splice exposes a library API for programmatic use. Add to `Cargo.toml`:

```toml
[dependencies]
splice = { version = "3.0", default-features = false, features = ["sqlite"] }
```

### High-Level API (forge integration)

```rust
use splice::forge;

// Patch a symbol with new content (resolves via SymbolNavigator)
let result = forge::patch_symbol_in_file(
    Path::new("src/lib.rs"),
    "my_function",
    "pub fn my_function() -> i32 { 42 }",
    Path::new(".magellan/myproject.db"),
)?;

// Rename across all files
let result = forge::rename_symbol_across_files("old_name", "new_name", db_path)?;

// Preview without writing
let preview = forge::preview_patch_symbol(file, symbol, content, db)?;

// Resolve symbol to byte span
let span = forge::resolve_symbol_span(file, symbol, db)?;
```

### Low-Level API

```rust
use splice::patch::{replace_span, apply_patch_with_validation, SpanReplacement};
use splice::graph::rename::{apply_replacements_in_file, group_references_by_file};
use splice::workspace::{find_workspace_root, detect_project_language};
use splice::verify::{pre_verify_patch, post_verify_patch};
```

## Installation

```bash
cargo install splice
```

## Backends

| Feature | Description | File | Best For |
|---------|-------------|------|----------|
| (default) | SQLite backend | `.db` | Compatibility |
| `geometric` | Geometric analysis backend (no-op) | `.geo` | Reserved for future use |

## Requirements

- **[Magellan](https://github.com/oldnordic/magellan)** 4.2+ — Required for code graph
- **[sqlitegraph](https://crates.io/crates/sqlitegraph)** 3.0.3+ — Included automatically

## Documentation

- **[docs/ARCHITECTURE.md](docs/ARCHITECTURE.md)** — Architecture and design (v3.0 agent surface)
- **[CHANGELOG.md](CHANGELOG.md)** — Version history
- **[docs/API.md](docs/API.md)** — Library API reference
- **[docs/completion.md](docs/completion.md)** — Code completion feature documentation

## What splice Does NOT Do

- Index code (use [Magellan](https://github.com/oldnordic/magellan))
- Search code (use [llmgrep](https://github.com/oldnordic/llmgrep))
- CFG analysis (use [Mirage](https://github.com/oldnordic/mirage))
- Type checking (use Compiler/LSP)

## License

GPL-3.0. See [LICENSE](LICENSE).
