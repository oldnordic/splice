# splice

**Precision Code Editing with Graph Algorithm Analysis**

[![Crates.io](https://img.shields.io/crates/v/splice)](https://crates.io/crates/splice)
[![Documentation](https://docs.rs/splice/badge.svg)](https://docs.rs/splice)

> Span-safe refactoring kernel for 7 languages with Magellan code graph integration.

## ⚠️ Requires Magellan

**splice requires [Magellan](https://github.com/oldnordic/magellan) for full functionality.**

Magellan provides the code graph database that splice uses for cross-file operations, impact analysis, and graph algorithms. You must run `magellan watch` on your codebase before using splice's advanced features.

```bash
# Install Magellan first
cargo install magellan

# Watch your project (builds database)
magellan watch --root ./src --db .codemcp/codegraph.db

# Now splice can perform graph-aware operations
splice reachable --symbol main --path src/main.rs --db .codemcp/codegraph.db
```

## The Code Intelligence Toolset

splice is part of a coordinated toolset built on [sqlitegraph](https://github.com/oldnordic/sqlitegraph). All tools share a common SQLite graph database and are designed to work together for AI-assisted code understanding and transformation.

```
┌─────────────┐      ┌─────────────┐      ┌─────────────┐
│  Magellan   │ ───► │  llmgrep    │ ───► │   Mirage    │
│(Symbols &   │      │ (Semantic   │      │(CFG & Paths)│
│  Call Graph)│      │  Search)    │      │             │
└─────────────┘      └─────────────┘      └─────────────┘
       │                    │                     │
       └────────────────────┴─────────────────────┘
                     │
              ┌──────▼──────┐
              │ sqlitegraph │
              │  (Database) │
              └─────────────┘
                     │
              ┌──────▼──────┐
              │   splice    │
              │(Edit using  │
              │   spans)    │
              └─────────────┘
```

| Tool | Purpose | Repository | Install |
|------|---------|------------|---------|
| **sqlitegraph** | Graph database foundation | [github.com/oldnordic/sqlitegraph](https://github.com/oldnordic/sqlitegraph) | `cargo add sqlitegraph` |
| **Magellan** | Call graph indexing, symbol navigation | [github.com/oldnordic/magellan](https://github.com/oldnordic/magellan) | `cargo install magellan` |
| **llmgrep** | Semantic code search | [github.com/oldnordic/llmgrep](https://github.com/oldnordic/llmgrep) | `cargo install llmgrep` |
| **Mirage** | CFG analysis, path enumeration | [github.com/oldnordic/mirage](https://github.com/oldnordic/mirage) | `cargo install mirage-analyzer` |
| **splice** | Precision code editing | [github.com/oldnordic/splice](https://github.com/oldnordic/splice) | `cargo install splice` |

## What is splice?

splice performs byte-accurate, AST-validated refactoring operations on code in 7 languages: Rust, Python, C, C++, Java, JavaScript, and TypeScript. It can replace function bodies, delete symbols, perform cross-file rename, analyze code graphs, and generate refactoring proofs.

### What splice is NOT

- ❌ A code indexer (use [Magellan](https://github.com/oldnordic/magellan))
- ❌ A semantic search tool (use [llmgrep](https://github.com/oldnordic/llmgrep))
- ❌ A CFG analysis tool (use [Mirage](https://github.com/oldnordic/mirage))
- ❌ An IDE or LSP server (use rust-analyzer, IntelliJ, PyCharm, VSCode)

### What splice IS

- ✅ Span-safe code editing with byte-accurate replacements
- ✅ Cross-file symbol rename using Magellan ReferenceFact data
- ✅ Graph algorithm analysis (reachability, dead-code, cycles, condense, slice)
- ✅ Proof-based refactoring with machine-checkable behavioral equivalence
- ✅ Multi-language validation (tree-sitter + compiler gates)
- ✅ Backup and rollback with automatic checksum verification

## Installation

### Quick Install (SQLite Backend)

```bash
cargo install splice
```

This installs splice with the SQLite backend (default).

### Backend Selection

splice supports two database backends via compile-time feature flags. Choose your backend based on your needs (see [Which Backend Should I Use?](#which-backend-should-i-use) for guidance).

**SQLite Backend (Default)**

The SQLite backend is stable and recommended for most users.

```bash
# Install from crates.io (SQLite backend)
cargo install splice

# Build from source (SQLite backend)
git clone https://github.com/oldnordic/splice
cd splice
cargo build --release
cp target/release/splice ~/.local/bin/

# Explicit SQLite feature
cargo build --release --features sqlite
```

**Native-V2 Backend**

The native-v2 backend uses an embedded Rust database for improved performance and is ideal for large-scale projects.

```bash
# Install with native-v2 backend
cargo install splice --features native-v2 --no-default-features

# Build from source (native-v2 backend)
git clone https://github.com/oldnordic/splice
cd splice
cargo build --release --features native-v2 --no-default-features
cp target/release/splice ~/.local/bin/
```

**Note:** The `--no-default-features` flag is required for native-v2 builds to exclude the SQLite default.

### Platform Features

Platform support is controlled via feature flags:

| Platform | Feature | Default |
|----------|---------|---------|
| Linux/macOS | `unix` | Yes |
| Windows | `windows` | No |

Build for specific platforms:

```bash
# Build for Windows (with SQLite backend)
cargo build --release --features windows

# Build for Windows (with native-v2 backend)
cargo build --release --features native-v2,windows --no-default-features
```

For installation on Windows:

```bash
# Install on Windows (SQLite backend)
cargo install splice --features windows

# Install on Windows (native-v2 backend)
cargo install splice --features native-v2,windows --no-default-features
```

## Which Backend Should I Use?

splice supports two database backends for storing code graph data: **SQLite** (default) and **native-v2** (opt-in). Both backends provide full functionality, but they have different characteristics that may make one more suitable for your use case.

### Backend Comparison

| Aspect | SQLite Backend | Native-V2 Backend |
|--------|----------------|-------------------|
| **Feature flag** | `--features sqlite` (default) | `--features native-v2` |
| **Database format** | SQLite format 3 | Custom key-value format |
| **Default behavior** | Default backend | Opt-in (must enable feature) |
| **File size** | Larger (2-3x for large codebases) | ~70% smaller |
| **Symbol lookup** | Table scan (O(n)) | O(1) KV get |
| **File listing** | JOIN query | Prefix scan |
| **Graph traversal** | Row-by-row iteration | Clustered adjacency |
| **Performance** | Good for small/medium codebases | 2-100x faster for large codebases |
| **Snapshot support** | Manual export/import | Native snapshot/restore system |
| **Tools compatibility** | All SQLiteGraph tools | All SQLiteGraph tools |
| **Maturity** | Stable, battle-tested | Stable (v2.5.0+) |

### Feature Availability

| Feature | SQLite | Native-V2 |
|---------|--------|-----------|
| Basic operations (patch, delete, rename) | ✅ | ✅ |
| Graph algorithms (reachable, cycles, dead-code) | ✅ | ✅ |
| Snapshot capture | ✅ (v2.5.0, manual export) | ✅ (v2.5.0, native) |
| Database restore | ⚠️ Manual import | ✅ Native restore |
| Batch operations with rollback | ⚠️ Limited | ✅ Full support (v2.5.0) |
| Magellan compatibility | ✅ Full | ✅ Full |

### Choose SQLite If:

- ✅ You're just getting started with splice
- ✅ You need maximum compatibility with standard SQLite tools
- ✅ You work with small to medium codebases (<100K LOC)
- ✅ You prefer battle-tested, stable defaults
- ✅ You want to inspect database with standard SQLite tools (sqlite3, DB Browser)

### Choose Native-V2 If:

- ✅ You work with large codebases (100K+ LOC)
- ✅ You need faster graph operations and symbol lookups
- ✅ You want smaller database file sizes for CI/CD storage
- ✅ You need native snapshot/restore for rollback workflows
- ✅ You use batch operations with transaction safety
- ✅ You want the latest performance optimizations

### Migrating Between Backends

If you want to switch backends, see the [Native-V2 Migration Guide](docs/NATIVE-V2-MIGRATION.md) for detailed instructions. The migration process preserves all your graph data and ensures no loss of information.

**Note:** New databases created with native-v2 support enabled will use the native-v2 format by default, even if the sqlite feature is also enabled.

## Quick Start

### 1. Install the Toolset

```bash
# Install all tools for complete workflow
cargo install magellan        # Call graph & CFG extraction (REQUIRED)
cargo install llmgrep         # Semantic search
cargo install mirage-analyzer # Path-aware analysis
cargo install splice          # Precision editing
```

### 2. Index Your Project

```bash
# Magellan watches your source and builds database
magellan watch --root ./src --db .codemcp/codegraph.db
```

### 3. Edit with Splice

```bash
# Patch a function body
cat > new_func.rs << 'EOF'
pub fn process(data: &str) -> Result<String> {
    Ok(data.to_uppercase())
}
EOF

splice patch --file src/lib.rs --symbol process --with new_func.rs --db .codemcp/codegraph.db

# Preview before applying
splice patch --file src/lib.rs --symbol process --with new_func.rs --preview --json

# Cross-file rename
splice rename --symbol <id> --file src/lib.rs --to new_name

# Impact analysis
splice reachable --symbol main --path src/main.rs --max-depth 3
splice dead-code --entry main --path src/main.rs
splice cycles
```

## Commands

### Edit Commands

| Command | Description |
|---------|-------------|
| `patch` | Replace function bodies, class definitions with validation |
| `delete` | Remove symbol definitions and all references |
| `apply-files` | Multi-file pattern replacement with AST confirmation |
| `rename` | Cross-file symbol rename using Magellan ReferenceFact |

### Graph Algorithm Commands

| Command | Description |
|---------|-------------|
| `reachable` | Show caller/callee chains and affected files |
| `dead-code` | Find unused symbols from entry points |
| `cycles` | Find circular dependencies in call graph |
| `condense` | Collapse SCCs to DAG for dependency analysis |
| `slice` | Forward/backward program slicing |

### Query Commands (Magellan)

| Command | Description |
|---------|-------------|
| `status` | Display database statistics |
| `find` | Locate symbols by name or symbol_id |
| `refs` | Show callers/callees for a symbol |
| `files` | List indexed files |
| `export` | Export graph data (JSON, JSONL, CSV) |

### Utility Commands

| Command | Description |
|---------|-------------|
| `undo` | Restore files from backup manifest |
| `log` | Query execution audit trail |
| `explain` | Get detailed explanations for error codes |
| `validate-proof` | Validate refactoring proof files |

## Examples

### Cross-File Rename with Preview

```bash
# Find symbol ID first
splice find --name my_function --path src/lib.rs --db .codemcp/codegraph.db

# Preview rename (no changes)
splice rename --symbol <id> --file src/lib.rs --to new_name --preview --json

# Apply rename with backup
splice rename --symbol <id> --file src/lib.rs --to new_name

# Generate proof for audit trail
splice rename --symbol <id> --file src/lib.rs --to new_name --proof
```

### Impact Analysis Before Refactoring

```bash
# See what code is reachable from a symbol
splice reachable --symbol main --path src/main.rs --max-depth 3 --output json

# Find dead code from main entry point
splice dead-code --entry main --path src/main.rs --exclude-public

# Detect circular dependencies
splice cycles --show-members --max-cycles 20

# Slice forward from a target (what it affects)
splice slice --target <id> --direction forward --max-distance 5
```

### Patch with Preview

```bash
# Create replacement
cat > new_greet.rs << 'EOF'
pub fn greet(name: &str) -> String {
    format!("Hi, {}!", name)
}
EOF

# Preview with JSON output
splice patch --file src/lib.rs --symbol greet --with new_greet.rs --preview --json

# Output:
# {
#   "status": "ok",
#   "message": "Previewed patch 'greet' at bytes 0..42 (dry-run)",
#   "data": {
#     "symbol": "greet",
#     "preview_report": {
#       "file": "src/lib.rs",
#       "line_start": 1,
#       "line_end": 3,
#       "lines_added": 3,
#       "lines_removed": 3,
#       "bytes_added": 45,
#       "bytes_removed": 42
#     },
#     "files": [{"file": "src/lib.rs"}]
#   }
# }
```

### Batch Pattern Replacement

```bash
# Replace across multiple files
splice apply-files --glob "src/**/*.rs" --find "old_func" --replace "new_func"

# With validation and backup
splice apply-files --glob "tests/**/*.rs" --find "42" --replace "99" --create-backup
```

## Supported Languages

| Language | Extensions | Patch | Delete | Validation |
|----------|-----------|-------|--------|------------|
| Rust | `.rs` | Full | Full | `cargo check` |
| Python | `.py` | Full | Basic | `python -m py_compile` |
| C | `.c`, `.h` | Full | Basic | `gcc -fsyntax-only` |
| C++ | `.cpp`, `.hpp`, `.cc`, `.cxx` | Full | Basic | `g++ -fsyntax-only` |
| Java | `.java` | Full | Basic | `javac` |
| JavaScript | `.js`, `.mjs`, `.cjs` | Full | Basic | `node --check` |
| TypeScript | `.ts`, `.tsx` | Full | Basic | `tsc --noEmit` |

## Requirements

- **[Magellan](https://github.com/oldnordic/magellan)** 2.1.0+ — Required for code graph operations
  ```bash
  cargo install magellan
  magellan watch --root ./src --db .codemcp/codegraph.db
  ```
- **[sqlitegraph](https://crates.io/crates/sqlitegraph)** 1.4.2+ — Included automatically

## Documentation

- `splice <command> --help` — Built-in usage examples
- [MANUAL.md](MANUAL.md) — Complete CLI reference manual
- [CHANGELOG.md](CHANGELOG.md) — Version history
- [docs/examples/rename_examples.md](docs/examples/rename_examples.md) — Cross-file rename examples
- [docs/examples/graph_algorithm_examples.md](docs/examples/graph_algorithm_examples.md) — Graph algorithm usage
- [docs/examples/proof_examples.md](docs/examples/proof_examples.md) — Proof-based refactoring

## License

GPL-3.0-only. See [LICENSE.md](LICENSE.md).

## Related Projects

- [sqlitegraph](https://github.com/oldnordic/sqlitegraph) — Graph database library
- [Magellan](https://github.com/oldnordic/magellan) — Call graph indexer
- [llmgrep](https://github.com/oldnordic/llmgrep) — Semantic code search
- [Mirage](https://github.com/oldnordic/mirage) — CFG analysis
