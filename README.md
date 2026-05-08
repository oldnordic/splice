# splice

[![Crates.io](https://img.shields.io/crates/v/splice)](https://crates.io/crates/splice)
[![Documentation](https://docs.rs/splice/badge.svg)](https://docs.rs/splice)

**Version:** 2.6.4

Span-safe refactoring kernel for 7 languages. Byte-accurate code editing with graph algorithm analysis.

**Positioning:** Precision editing tool for codebases indexed by Magellan. Use for cross-file rename, impact analysis, and verified refactoring.

## Purpose

**splice edits code with graph-aware safety.** Built for:

- **Developers** — Cross-file rename, safe refactoring with rollback
- **Grounded Assistants** — Byte-accurate edits with proof-based verification
- **Tooling** — Scriptable transformations with AST validation

## Features

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

## Quick Start

```bash
# Install
cargo install splice

# Requires Magellan database (create first)
magellan watch --root ./src --db .magellan/splice.db

# Check status
splice status --db .magellan/splice.db

# Find reachable code from main
splice reachable --symbol main --path src/main.rs --db .magellan/splice.db

# Cross-file rename
splice rename --symbol old_name --to new_name --path src/lib.rs --db .magellan/splice.db

# Code completion (import-aware)
splice complete --file src/lib.rs --line 27 --column 8 --db .magellan/splice.db --max-results 10
```

## Installation

```bash
cargo install splice
```

## Backends

| Feature | Description | File | Best For |
|---------|-------------|------|----------|
| (default) | SQLite backend | `.db` | Compatibility |
| `geometric` | Geometric analysis backend | `.geo` | Spatial/CFG analysis experiments |

## Requirements

- **[Magellan](https://github.com/oldnordic/magellan)** 3.1.7+ — Required for code graph
- **[sqlitegraph](https://crates.io/crates/sqlitegraph)** 2.0.3+ — Included automatically

## Documentation

- **[MANUAL.md](MANUAL.md)** — Complete command reference and examples
- **[CHANGELOG.md](CHANGELOG.md)** — Version history
- **[docs/completion.md](docs/completion.md)** — Code completion feature documentation

## What splice Does NOT Do

- ❌ Index code (use [Magellan](https://github.com/oldnordic/magellan))
- ❌ Search code (use [llmgrep](https://github.com/oldnordic/llmgrep))
- ❌ CFG analysis (use [Mirage](https://github.com/oldnordic/mirage))
- ❌ Type checking (use Compiler/LSP)

## License

GPL-3.0. See [LICENSE](LICENSE).
