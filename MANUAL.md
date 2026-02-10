# Splice v2.4.0 Manual

Comprehensive guide for Splice span-safe refactoring with Magellan integration.

---

## Splice v2.4.0 Overview

### What is Splice?

Splice is a span-safe code refactoring tool that provides AST-validated code modifications across 7 programming languages (Rust, Python, C, C++, Java, JavaScript, TypeScript). It uses tree-sitter for parsing and SQLiteGraph for code relationship storage, with multi-stage validation (tree-sitter + compiler) before applying changes.

### v2.4.0 New Features

- **Preview JSON Output**: Structured JSON for preview mode with full metrics (line/column, bytes added/removed)
- **Documentation Updates**: Condensed manual, improved README with toolset references

### Key Features

**Span-Safe Operations:**
- Byte-accurate span extraction and modification
- Multi-language support (Rust, Python, C, C++, Java, JavaScript, TypeScript)
- Tree-sitter based parsing for AST accuracy

**Validation Infrastructure:**
- Pre-verification: File state, workspace conditions, checksums
- Post-verification: Tree-sitter reparse, compiler validation
- Automatic rollback on validation failure

**Structured Output:**
- Versioned JSON schema with stable identifiers
- Span-aware metadata (byte offsets + line/column)
- Deterministic ordering for reproducibility

---

## Commands Reference

### splice patch

Replace a function body, class definition, or enum variant with validation.

```bash
splice patch --file <PATH> --symbol <NAME> --with <FILE> [OPTIONS]
```

**Required Arguments:**
- `--file <PATH>`: Path to source file
- `--symbol <NAME>`: Symbol name to patch
- `--with <FILE>`: File containing replacement code

**Optional Arguments:**
- `--kind <KIND>`: Symbol kind filter (function, method, class, struct, interface, enum, trait, impl, module, variable, constructor, type-alias)
- `--language <LANG>`: Language override (rust, python, c, cpp, java, java-script, type-script)
- `--preview`: Run in preview mode without modifying files
- `--json`: Output JSON format
- `--create-backup`: Create backup before patching
- `--operation-id <ID>`: Custom operation ID for auditing
- `--snapshot-before`: Capture graph snapshot before patching (requires --db)
- `--db <PATH>`: Path to codegraph.db (default: .codemcp/codegraph.db)

**Preview JSON Output:**
```json
{
  "status": "ok",
  "message": "Previewed patch 'foo' at bytes 0..29 (dry-run)",
  "data": {
    "symbol": "foo",
    "preview_report": {
      "file": "src/lib.rs",
      "line_start": 1,
      "line_end": 3,
      "lines_added": 3,
      "lines_removed": 3,
      "bytes_added": 31,
      "bytes_removed": 29
    },
    "files": [{"file": "src/lib.rs"}]
  }
}
```

### splice rename

Rename a symbol across all files using Magellan ReferenceFact data.

```bash
splice rename --symbol <id> --file <path> --to <new_name> [OPTIONS]
```

**Required Arguments:**
- `--symbol <id>`: Symbol ID (16-char hex V1 or 32-char hex V2) or symbol name
- `--file <path>`: File path containing the symbol definition
- `--to <new_name>`: New name for the symbol

**Optional Arguments:**
- `--db <path>`: Path to codegraph.db (default: `.codemcp/codegraph.db`)
- `--preview`: Show changes without applying (no file modifications)
- `--proof`: Generate refactoring proof for audit trail
- `--snapshot-before`: Capture graph snapshot before renaming (requires --db)
- `--impact-graph`: Generate DOT graph for impact visualization (requires --preview)

**Features:**
- Byte-accurate replacement at exact reference spans
- Automatic backup before modification
- Rollback on validation failures
- UTF-8 boundary validation for multi-byte characters

### splice reachable

Show all symbols reachable from a target symbol (impact analysis).

```bash
splice reachable --symbol <name> --path <file> [OPTIONS]
```

**Required Arguments:**
- `--symbol <name>`: Symbol name
- `--path <file>`: File path for disambiguation

**Optional Arguments:**
- `--db <path>`: Path to codegraph.db (default: `.codemcp/codegraph.db`)
- `--direction <forward|backward>`: Traversal direction (default: forward)
- `--max-depth <n>`: Maximum traversal depth (default: 10)
- `--output <format>`: Output format (human, json, pretty)
- `--impact-graph`: Generate DOT graph for impact visualization

### splice dead-code

Find unused symbols from entry points.

```bash
splice dead-code --entry <symbol> --path <file> [OPTIONS]
```

**Required Arguments:**
- `--entry <symbol>`: Entry point symbol name
- `--path <file>`: File path for disambiguation

**Optional Arguments:**
- `--db <path>`: Path to codegraph.db
- `--exclude-public`: Exclude public symbols from analysis
- `--output <format>`: Output format (human, json, pretty)

### splice cycles

Find circular dependencies in the call graph.

```bash
splice cycles [OPTIONS]
```

**Optional Arguments:**
- `--db <path>`: Path to codegraph.db
- `--symbol <name>`: Find cycles containing a specific symbol
- `--path <file>`: File path for symbol disambiguation
- `--max-cycles <n>`: Maximum cycles to display (default: 100)
- `--show-members`: Show all cycle members
- `--output <format>`: Output format (human, json, pretty)

### splice condense

Collapse SCCs to DAG for dependency analysis.

```bash
splice condense [OPTIONS]
```

**Optional Arguments:**
- `--db <path>`: Path to codegraph.db
- `--show-levels`: Show topological levels
- `--show-members`: Show SCC members
- `--output <format>`: Output format (human, json, pretty)

### splice slice

Perform forward or backward program slicing.

```bash
splice slice --target <id> --direction <forward|backward> [OPTIONS]
```

**Required Arguments:**
- `--target <id>`: Symbol ID
- `--direction <forward|backward>`: Slice direction

**Optional Arguments:**
- `--db <path>`: Path to codegraph.db
- `--max-distance <n>`: Maximum slice distance (default: 10)
- `--output <format>`: Output format (human, json, pretty)

### splice delete

Remove a symbol definition and all its references.

```bash
splice delete --file <PATH> --symbol <NAME> [OPTIONS]
```

**Optional Arguments:**
- `--kind <KIND>`: Symbol kind filter
- `--language <LANG>`: Language override
- `--create-backup`: Create backup before deleting
- `--operation-id <ID>`: Custom operation ID

### splice apply-files

Apply a pattern replacement to multiple files.

```bash
splice apply-files --glob <GLOB> --find <PATTERN> --replace <REPLACEMENT>
```

**Required Arguments:**
- `--glob <GLOB>`: Glob pattern (e.g., `tests/**/*.rs`, `src/**/*.py`)
- `--find <PATTERN>`: Text pattern to find
- `--replace <REPLACEMENT>`: Replacement text

**Optional Arguments:**
- `--language <LANG>`: Language override
- `--create-backup`: Create backup before applying

### splice undo

Undo a previous operation by restoring from backup.

```bash
splice undo --manifest <PATH>
```

### splice validate-proof

Validate a refactoring proof file.

```bash
splice validate-proof --proof <path> [OPTIONS]
```

**Optional Arguments:**
- `--output <format>`: Output format (human, json, pretty)

### splice verify

Compare two snapshots to detect changes in symbols and edges.

```bash
splice verify --before <SNAPSHOT> --after <SNAPSHOT> [OPTIONS]
```

**Required Arguments:**
- `--before <SNAPSHOT>`: Path to before snapshot (JSON file)
- `--after <SNAPSHOT>`: Path to after snapshot (JSON file)

**Optional Arguments:**
- `--detailed`: Show per-symbol changes with before/after comparison
- `--output <format>`: Output format (human, json, pretty)
- `--json`: Output JSON format

**Exit Codes:**
- `0`: Snapshots are identical
- `1`: Differences detected
- `2`: Error occurred

**Example:**

```bash
# Compare snapshots before and after a refactor
splice verify --before .splice/snapshots/snapshot-before-123456.json \
              --after .splice/snapshots/snapshot-after-789012.json \
              --detailed

# JSON output for programmatic use
splice verify --before before.json --after after.json --output json
```

### splice batch

Execute multi-file refactoring operations from a YAML specification.

```bash
splice batch --spec <FILE> [OPTIONS]
```

**Required Arguments:**
- `--spec <FILE>`: Path to YAML specification file

**Optional Arguments:**
- `--db <PATH>`: Path to codegraph.db (default: .codemcp/codegraph.db)
- `--dry-run`: Preview changes without applying
- `--continue-on-error`: Continue after failures (default: stop on error)
- `--rollback <MODE>`: Rollback mode (on-failure, never, always)
- `--json`: Output JSON format

**YAML Specification Format:**

```yaml
# Example batch spec for renaming across multiple files
operations:
  - type: rename
    symbol: old_function_name
    file: src/lib.rs
    to: new_function_name

  - type: patch
    symbol: process_data
    file: src/process.rs
    with: replacements/new_process.rs

  - type: delete
    symbol: deprecated_function
    file: src/utils.rs
```

**Rollback Modes:**
- `on-failure`: Automatic rollback if any operation fails (native-v2 only)
- `never`: No rollback (default)
- `always`: Always rollback after execution (for testing)

### splice snapshots

Manage graph snapshots for verification and rollback.

```bash
splice snapshots <subcommand> [OPTIONS]
```

**Subcommands:**

- `list [--operation <NAME>] [--limit <N>] [--disk-use]`: List snapshots
- `delete <id> [--force]`: Delete a snapshot by ID
- `cleanup [--keep <N>] [--dry-run]`: Remove old snapshots, keeping N most recent

**Examples:**

```bash
# List all snapshots
splice snapshots list

# Delete a specific snapshot
splice snapshots delete snapshot-1234567890 --force

# Clean up old snapshots (keep 10 most recent)
splice snapshots cleanup --keep 10
```

---

## Query Commands (Magellan)

### splice status

Display database statistics including backend detection.

```bash
splice status --db <FILE>
```

**Required Arguments:**
- `--db <FILE>`: Path to codegraph.db

**Optional Arguments:**
- `--detect-backend`: Show which backend the database uses (sqlite or native-v2)

**Backend Detection:**

The `--detect-backend` flag reports which backend format a database file uses:

- `sqlite`: SQLite format 3 database (default backend)
- `native-v2`: Native KV format database
- `unknown`: File doesn't exist or format unrecognized

**Examples:**

```bash
# Show database statistics with backend detection
splice status --db .codemcp/codegraph.db --detect-backend

# Output:
# Database: .codemcp/codegraph.db
# Backend: sqlite
# Symbols: 1,234
# Files: 42
```

### splice migrate

Migrate a database from SQLite to native-v2 format.

```bash
splice migrate --source <PATH> --dest <PATH> [OPTIONS]
```

**Required Arguments:**
- `--source <PATH>`: Path to source database (SQLite format)
- `--dest <PATH>`: Path to destination database (native-v2 format)

**Optional Arguments:**
- `--progress`: Show progress during migration (default: true)
- `--skip-verify`: Skip verification after migration (not recommended)
- `--json`: Output JSON format instead of human-readable

**Migration Workflow:**

1. **Check current backend:**
   ```bash
   splice status --db .codemcp/codegraph.db --detect-backend
   ```

2. **Build splice with native-v2 feature:**
   ```bash
   cargo build --release --features native-v2 --no-default-features
   ```

3. **Run migration:**
   ```bash
   splice migrate --source .codemcp/codegraph.db --dest .codemcp/codegraph-native.db
   ```

4. **Verify migration:**
   ```bash
   splice status --db .codemcp/codegraph-native.db --detect-backend
   ```

5. **Update tooling to use new database:**
   ```bash
   # Use the new database path with splice commands
   splice reachable --symbol main --db .codemcp/codegraph-native.db
   ```

**Migration Report:**

The migration command reports:
- `nodes_migrated`: Number of symbols/entities migrated
- `edges_migrated`: Number of relationships migrated
- `snapshot_metadata`: Information about the migration snapshot

**Important Notes:**

- Migration requires native-v2 feature enabled at build time
- Source database must be SQLite format
- Migration creates a new database (does not modify source)
- Verify migration before replacing your original database

**See Also:**
- [Migration Guide](../docs/NATIVE-V2-MIGRATION.md) — Detailed migration documentation
- [Which Backend Should I Use?](../README.md#which-backend-should-i-use) — Backend selection guidance

### splice find

Find symbols by name or symbol_id.

```bash
splice find --db <FILE> (--name <NAME> | --symbol-id <ID>)
```

### splice refs

Show callers/callees for a symbol.

```bash
splice refs --db <FILE> --name <NAME> [--path <PATH>] [--direction <in|out>]
```

### splice files

List indexed files.

```bash
splice files --db <FILE> [--symbols]
```

### splice export

Export graph data.

```bash
splice export --db <FILE> --format FORMAT --file <PATH>
```

---

## Native-V2 Features

The following features require the native-v2 backend (build with `--features native-v2`):

- **Snapshots**: `--snapshot-before` flag captures graph state for rollback
- **Verify**: Compare snapshots to detect changes
- **Batch**: Multi-file operations with automatic rollback
- **Impact Graph**: DOT visualization of refactoring impact

See [Native-V2 Features](docs/NATIVE-V2-FEATURES.md) for detailed examples.

**Building with Native-V2:**

```bash
# Install with native-v2 backend
cargo install splice --features native-v2 --no-default-features

# Build from source (native-v2 backend)
cargo build --release --features native-v2 --no-default-features
```

**Native-V2 Advantages:**

- **Performance**: 2-100x faster for large codebases (100K+ LOC)
- **File Size**: ~70% smaller database files
- **Snapshot System**: Native snapshot/restore for rollback workflows
- **Batch Operations**: Transaction-safe multi-file refactors

---

## Supported Languages

| Language | Extensions | Delete | Patch | Validation |
|----------|-----------|--------|-------|------------|
| Rust | `.rs` | Full | Full | `cargo check` |
| Python | `.py` | Basic | Full | `python -m py_compile` |
| C | `.c`, `.h` | Basic | Full | `gcc -fsyntax-only` |
| C++ | `.cpp`, `.hpp`, `.cc`, `.cxx` | Basic | Full | `g++ -fsyntax-only` |
| Java | `.java` | Basic | Full | `javac` |
| JavaScript | `.js`, `.mjs`, `.cjs` | Basic | Full | `node --check` |
| TypeScript | `.ts`, `.tsx` | Basic | Full | `tsc --noEmit` |

---

## JSON Output Schema

### Success Response (CliSuccessPayload)

```json
{
  "status": "ok",
  "message": "Human-readable description",
  "data": {
    // Operation-specific fields
  }
}
```

### Error Response

```json
{
  "status": "error",
  "error": {
    "kind": "ErrorType",
    "message": "Error description",
    "error_code": {
      "code": "SPL-E###",
      "severity": "error|warning",
      "hint": "Remediation suggestion"
    }
  }
}
```

---

## Requirements

- **[Magellan](https://github.com/oldnordic/magellan)** 2.1.0+ — Required for code graph operations
- **[sqlitegraph](https://crates.io/crates/sqlitegraph)** 1.4.2+ — Included automatically

---

## License

GPL-3.0-only. See [LICENSE.md](../LICENSE.md).
