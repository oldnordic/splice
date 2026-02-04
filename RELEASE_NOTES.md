# Splice v2.3.0 Release Notes

**Release Date:** 2026-02-04

Splice v2.3.0 introduces Magellan v2 integration with advanced semantic program transformation capabilities including cross-file rename, impact analysis, dead code detection, cycle detection, graph condensation, program slicing, and proof-based refactoring.

---

## What's New

### Cross-File Rename

Rename symbols across files using Magellan ReferenceFact byte offsets:

```bash
# Ingest your codebase
splice ingest --root ./src --db .codemcp/codegraph.db

# Find symbol ID
splice find --name "my_function" --path "src/my_file.rs"

# Preview rename (no changes)
splice rename --symbol <id> --file "src/my_file.rs" --to "new_name" --preview

# Perform rename with backup
splice rename --symbol <id> --file "src/my_file.rs" --to "new_name"
```

**Features:**
- Byte-accurate replacement at exact reference spans
- Automatic backup and rollback on validation failures
- Preview mode with colored diff output
- UTF-8 boundary validation for safe multi-byte character handling
- Multi-language support (C, C++, Java, JavaScript, Python, TypeScript)

### Impact Analysis

Analyze call graph impact before refactoring:

```bash
# See what code is reachable from a symbol
splice reachable --symbol "main" --path "src/main.rs" --max-depth 3

# Slice forward from a target (what it affects)
splice slice --target <id> --direction forward --max-distance 5

# Slice backward to a target (what affects it)
splice slice --target <id> --direction backward --max-distance 5
```

### Dead Code Detection

Find unused symbols from entry points:

```bash
# Find dead code from main entry point
splice dead-code --entry "main" --path "src/main.rs"
```

### Cycle Detection

Find circular dependencies in the call graph:

```bash
# Detect circular dependencies
splice cycles

# Show all members of each cycle
splice cycles --show-members
```

### Graph Condensation

Collapse SCCs to DAG for dependency analysis:

```bash
# Show topological levels
splice condense --show-levels

# Show SCC members
splice condense --show-members
```

### Proof-Based Refactoring

Generate machine-checkable behavioral equivalence proofs:

```bash
# Generate proof for audit trail
splice rename --symbol <id> --file "src/my_file.rs" --to "new_name" --proof

# Validate proof file
splice validate-proof --proof proof.json
```

**Proof includes:**
- Before/after graph snapshots
- Invariant validation (reference counts, orphan detection, ID stability, entry points)
- SHA-256 checksums for audit trail integrity

---

## Migration Guide

### Magellan Database Migration

Magellan 2.0.0 uses BLAKE3 for symbol IDs (V2 format). Existing databases auto-migrate on open:

```bash
# Check migration status (dry-run)
splice migrate --db .codemcp/codegraph.db --dry-run

# Migrate with backup
splice migrate --db .codemcp/codegraph.db --backup
```

**Symbol ID Formats:**
- V1: 16-character hex (SHA-256, first 8 bytes) — legacy
- V2: 32-character hex (BLAKE3, first 16 bytes) — new default

The `find_symbol_by_id()` function tries V2 first, then V1 for backward compatibility.

### Dependency Updates

This release upgrades Magellan to 2.1.0 and SQLiteGraph to 1.4.2. Run:

```bash
cargo update
cargo build --release
```

---

## Performance

Graph algorithm performance (1K symbols):

| Algorithm | Time |
|-----------|------|
| Reachability | 5-15ms |
| Dead Code Detection | 10-25ms |
| Cycle Detection | 20-40ms |
| Graph Condensation | 15-35ms |
| Program Slicing | 25-60ms |

All algorithms meet the <1s target for 1K symbols.

---

## Documentation

- **[docs/magellan_integration.md](docs/magellan_integration.md)** - Magellan integration guide
- **[docs/manual.md](docs/manual.md)** - Complete CLI reference manual
- **[docs/examples/rename_examples.md](docs/examples/rename_examples.md)** - Cross-file rename usage examples
- **[docs/examples/graph_algorithm_examples.md](docs/examples/graph_algorithm_examples.md)** - Graph algorithm usage examples
- **[docs/examples/proof_examples.md](docs/examples/proof_examples.md)** - Proof-based refactoring examples

---

## Installation

```bash
cargo install splice
```

Or from source:

```bash
git clone https://github.com/oldnordic/splice.git
cd splice
cargo build --release
cp target/release/splice ~/.local/bin/
```

---

## Full Changelog

See [CHANGELOG.md](CHANGELOG.md) for the complete list of changes.

---

## License

GPL-3.0-or-later
