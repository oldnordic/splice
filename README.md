# splice

[![Crates.io](https://img.shields.io/crates/v/splice)](https://crates.io/crates/splice)
[![Documentation](https://docs.rs/splice/badge.svg)](https://docs.rs/splice)

**Version:** 2.5.2

Span-safe refactoring kernel for 7 languages. Byte-accurate code editing with graph algorithm analysis.

**Positioning:** Precision editing tool for codebases indexed by Magellan. Use for cross-file rename, impact analysis, and verified refactoring.

## Purpose

**splice edits code with graph-aware safety.** Built for:

- **Developers** — Cross-file rename, safe refactoring with rollback
- **AI Assistants** — Byte-accurate edits with proof-based verification
- **Tooling** — Scriptable transformations with AST validation

## Features

- **Span-safe editing** — Byte-accurate replacements with tree-sitter validation
- **Cross-file rename** — Symbol rename across entire codebase
- **Graph algorithms** — Reachability, dead-code, cycles, condense, slice
- **Impact analysis** — Blast zone detection before editing
- **Proof-based refactoring** — Machine-checkable behavioral equivalence
- **7 languages** — Rust, Python, C, C++, Java, JavaScript, TypeScript

## Quick Start

```bash
# Install
cargo install splice

# Requires Magellan database (create first)
magellan watch --root ./src --db code.v3

# Check status
splice status --db code.v3

# Find reachable code from main
splice reachable --symbol main --path src/main.rs --db code.v3

# Cross-file rename
splice rename --symbol old_name --to new_name --path src/lib.rs --db code.v3
```

## Installation

```bash
cargo install splice
```

Or build from source with Native-V3 backend:

```bash
# Native-V3 backend (recommended - fastest)
cargo install splice --features native-v3 --no-default-features
```

## Backends

| Feature | Description | File | Best For |
|---------|-------------|------|----------|
| `native-v3` | **High-performance binary backend** | `.v3` | Production (recommended) |
| (default) | SQLite backend | `.db` | Compatibility |

Both backends provide identical refactoring capabilities.

## Requirements

- **[Magellan](https://github.com/oldnordic/magellan)** 2.4.3+ — Required for code graph
- **[sqlitegraph](https://crates.io/crates/sqlitegraph)** 2.0.3+ — Included automatically

## Documentation

- **[MANUAL.md](MANUAL.md)** — Complete command reference and examples
- **[CHANGELOG.md](CHANGELOG.md)** — Version history

## What splice Does NOT Do

- ❌ Index code (use [Magellan](https://github.com/oldnordic/magellan))
- ❌ Search code (use [llmgrep](https://github.com/oldnordic/llmgrep))
- ❌ CFG analysis (use [Mirage](https://github.com/oldnordic/mirage))
- ❌ Type checking (use Compiler/LSP)

## License

GPL-3.0-or-later. See [LICENSE](LICENSE).
