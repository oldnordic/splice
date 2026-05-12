# Splice v2.6.9 Manual

Comprehensive guide for Splice span-safe refactoring with Magellan integration.

---

## Splice v2.6.9 Overview

### What is Splice?

Splice is a span-safe code refactoring tool that provides AST-validated code modifications across 7 programming languages (Rust, Python, C, C++, Java, JavaScript, TypeScript). It uses tree-sitter for parsing and SQLiteGraph for code relationship storage, with multi-stage validation (tree-sitter + compiler) before applying changes.

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
- `--dry-run`: Run in preview mode without modifying files
- `--json`: Output JSON format
- `--create-backup`: Create backup before patching
- `--operation-id <ID>`: Custom operation ID for auditing
- `--snapshot-before`: Capture graph snapshot before patching (requires --db)
- `--db <PATH>`: Path to codegraph.db (default: .magellan/splice.db)

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
- `--db <path>`: Path to codegraph.db (default: `.magellan/splice.db`)
- `--dry-run`: Show changes without applying (no file modifications)
- `--proof`: Generate refactoring proof for audit trail (requires --dry-run)
- `--snapshot-before`: Capture graph snapshot before renaming (requires --db)
- `--impact-graph`: Generate DOT graph for impact visualization (requires --dry-run)

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
- `--db <path>`: Path to codegraph.db (default: `.magellan/splice.db`)
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

### splice complete

Provide grounded, import-aware code completions using the Magellan database.

```bash
splice complete --file <PATH> --line <LINE> --column <COLUMN> [OPTIONS]
```

**Required Arguments:**
- `--file <PATH>`: Path to source file
- `--line <LINE>`: Cursor line number (1-based)
- `--column <COLUMN>`: Cursor column number (1-based)

**Optional Arguments:**
- `--max-results <n>`: Maximum suggestions to return (default: 10)
- `--db <path>`: Path to Magellan database (default: .magellan/splice.db)
- `--output <format>`: Output format (human, json, pretty)

**Behavior:**
1. Queries Magellan database for visible symbols (local + imported)
2. Analyzes cursor position to extract partial token being typed
3. Filters suggestions by token prefix/contains match
4. Ranks suggestions using multiple signals (frequency, proximity, kind, text match, import proximity)
5. Returns grounded suggestions with database IDs proving symbol existence

**Features:**
- **Import-Aware**: Suggests symbols from imported modules across files
- **Grounded**: Every suggestion includes database IDs (no hallucinations)
- **Token Filtering**: Shows only symbols matching what you're typing
- **Source Tracking**: Distinguishes local (Database) vs imported (Imported) symbols
- **Performance**: low-millisecond internal query time on indexed project databases

**Output Fields (JSON format):**
- `label`: Suggested symbol name
- `source`: "Database" (local) or "Imported" (cross-file)
- `source_file`: Absolute path to defining file
- `via_import`: Import statement (e.g., "use crate::completion::types")
- `grounded_in`: Database IDs proving symbol exists
- `score`: Relevance score (0.0-1.0)
- `kind`: Symbol kind (Function, Struct, etc.)
- `metadata.query_time_ms`: Query execution time

**Examples:**

```bash
# Basic completion
splice complete --file src/lib.rs --line 27 --column 8 --db .magellan/splice.db --max-results 10

# JSON output with metadata
splice complete --file src/main.rs --line 100 --column 10 --db .magellan/splice.db --max-results 5 --output json

# Import-aware completion
splice complete --file src/completion/engine.rs --line 30 --column 8 --db .magellan/splice.db
```

**Performance:**
- Current Splice database smoke test: 3-13ms internal completion query time
- CLI wall time is typically around 0.30s because each command starts a new process and opens the database
- Test database: 15,266 indexed graph entities reported by completion metadata

**See Also:** [docs/completion.md](docs/completion.md) for detailed documentation

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
- `--dry-run`: Preview matches without applying changes. Reports affected files and match count in JSON.

### splice create

Create a new file with validation, enabling safer code generation and editor integration.

```bash
cat <<'EOF' | splice create --file <PATH> [OPTIONS]
pub fn my_function() -> i32 {
    42
}
EOF
```

**Required Arguments:**
- `--file <PATH>`: Path where file should be created

**Optional Arguments:**
- `--validate-only`: Validate code without creating file
- `--with-mod`: Add `mod filename;` to parent module
- `--workspace <PATH>`: Workspace directory (default: current directory)
- `--output <format>`: Output format (human, json, pretty)

**Behavior:**
1. Reads Rust code from stdin
2. Validates with rustc --emit=metadata (catches syntax and basic type errors)
3. Creates file atomically (if validation passes)
4. Prevents overwriting existing files
5. Creates parent directories as needed
6. Optionally adds module declarations

**Examples:**

Create a new file:
```bash
echo 'pub fn hello() -> String { "Hello".to_string() }' | \
  splice create --file src/hello.rs
```

Validate without writing:
```bash
echo 'pub fn test() -> i32 { 42 }' | \
  splice create --file src/test.rs --validate-only
```

Create with module declaration:
```bash
echo 'pub fn handler() -> i32 { 200 }' | \
  splice create --file src/commands/handler.rs --with-mod
```

JSON output:
```bash
echo 'pub fn test() -> i32 { 42 }' | \
  splice create --file test.rs --output json
```

**Validation Result (JSON):**
```json
{
  "is_valid": true,
  "errors": [],
  "warnings": []
}
```

**Error Handling:**
- Validation fails: File NOT created. Returns `status: "error"` with `SPL-E043` and compiler diagnostics (exit code 5).
- File exists: Returns error, preserves original
- rustc unavailable: Graceful degradation (assumes OK)

**Use Cases:**
- Interactive file creation with validation
- Batch file creation from templates
- Automated code generation pipelines

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
- `--db <PATH>`: Path to codegraph.db (default: .magellan/splice.db)
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
- `cleanup [--keep <N>] [--dry-run] [--yes]`: Remove old snapshots, keeping N most recent. Bulk deletes above 50 require `--yes` to confirm.

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

**Backend Detection:**

The `--detect-backend` flag reports which backend format a database file uses:

- `sqlite`: SQLite format 3 database (default backend)
- `unknown`: File doesn't exist or format unrecognized

**Examples:**

```bash
# Show database statistics with backend detection
splice status --db .magellan/splice.db --detect-backend

# Output:
# Database: .magellan/splice.db
# Backend: sqlite
# Symbols: 1,234
# Files: 42
```

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
      "location": "file.rs:10:5",
      "hint": "Remediation suggestion"
    }
  }
}
```

The `location` field is present only when a file path is available. Errors without a file association omit `location` entirely.
```

---

## Requirements

- **[Magellan](https://github.com/oldnordic/magellan)** 3.3+ — Required for code graph operations
- **[sqlitegraph](https://crates.io/crates/sqlitegraph)** 2.2+ — Included automatically

---

## Performance

| Operation | Typical Time | Notes |
|-----------|--------------|-------|
| Patch (single file) | 50-200ms | Parse + validate + compiler |
| Rename (cross-file) | 100-500ms | Depends on reference count |
| Delete (with cleanup) | 100-400ms | Cross-file removal |
| Graph algorithms | 10-60ms | Depends on database size and traversal depth |

## Best Practices

### For Editor Integration

1. **Always use `--dry-run`** before applying changes
2. **Check impact** with `reachable` or `refs` before refactoring
3. **Generate proofs** with `--proof` for audit trail

### For Interactive Use

1. **Use `--dry-run --output json`** to see exact changes
2. **Create backups** with `--create-backup` for critical operations
3. **Run tests** after each operation to verify correctness
4. **Use batch operations** for multiple related changes

### For Scripting

1. **Use `--output json`** for programmatic consumption
2. **Parse operation_id** for tracking and undo capability
3. **Validate proofs** after operations for verification
4. **Use snapshots** for before/after comparison

## Error Codes

| Code | Description | Exit Code |
|------|-------------|-----------|
| SPL-E001 | Symbol not found | 1 |
| SPL-E002 | Ambiguous symbol (multiple files) | 1 |
| SPL-E003 | Reference lookup failed | 1 |
| SPL-E004 | Ambiguous reference | 1 |
| SPL-E011 | Parse error | 1 |
| SPL-E012 | Invalid UTF-8 | 1 |
| SPL-E021 | Invalid byte span | 1 |
| SPL-E022 | Invalid line range | 1 |
| SPL-E031 | File read error | 4 |
| SPL-E033 | File not found | 4 |
| SPL-E034 | File externally modified | 4 |
| SPL-E040 | Rename failed | 1 |
| SPL-E041 | Pre-verification failed | 5 |
| SPL-E042 | Parse validation failed | 5 |
| SPL-E043 | Compiler validation failed | 5 |
| SPL-E051 | Invalid plan schema | 2 |
| SPL-E052 | Plan execution failed | 1 |
| SPL-E061 | Graph database error | 3 |
| SPL-E071 | Execution log error | 3 |
| SPL-E081 | Analyzer not available | 1 |
| SPL-E091 | Magellan integration error | 3 |

Use `splice explain --code <SPL-EXXX>` for detailed diagnostics.

## Exit Codes

| Code | Meaning |
|------|---------|
| 0 | Success |
| 1 | Generic error |
| 2 | Usage error (invalid arguments) |
| 3 | Database error |
| 4 | File not found |
| 5 | Validation error |

---

## License

GPL-3.0-only. See [LICENSE.md](../LICENSE.md).

## Further Documentation

- **[README.md](README.md)** — Quick start and overview
- **[CHANGELOG.md](CHANGELOG.md)** — Version history
- **[docs/completion.md](docs/completion.md)** — Code completion behavior and examples
- **[docs/API.md](docs/API.md)** — API and structured output reference
- **[docs/SCHEMA_REFERENCE.md](docs/SCHEMA_REFERENCE.md)** — Database/schema reference
- **[docs/JSON_EXPORT_FORMAT.md](docs/JSON_EXPORT_FORMAT.md)** — JSON export format
- **[docs/examples/rename_examples.md](docs/examples/rename_examples.md)** — Cross-file rename examples
- **[docs/examples/graph_algorithm_examples.md](docs/examples/graph_algorithm_examples.md)** — Graph algorithm usage
- **[docs/examples/proof_examples.md](docs/examples/proof_examples.md)** — Proof-based refactoring
