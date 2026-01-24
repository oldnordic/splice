# Splice

Span-safe refactoring kernel for 7 languages using tree-sitter and SQLiteGraph.

**Version**: 2.0.0
**License**: GPL-3.0-or-later

## What This Is

Splice is a command-line tool that performs byte-accurate, AST-validated refactoring operations on code in 7 languages: Rust, Python, C, C++, Java, JavaScript, and TypeScript. It can replace function bodies, delete symbols, apply batch changes, perform pattern replacements, and undo operations.

## v2.0 Features

Splice v2.0 introduces comprehensive improvements across safety, output structure, validation, and observability:

- **Structured JSON Output**: All operations return structured JSON with explicit fields (execution_id, match_id, span_id) for programmatic consumption
- **Span-Aware Metadata**: Line and column coordinates in all output for precise code location tracking
- **Deterministic Ordering**: All operations (spans, matches, errors, steps) return sorted results for reproducibility
- **Validation Hooks**: Pre/post-operation checksums verify code integrity at every step
- **Execution Logging**: Complete audit trail in `.splice/operations.db` with timestamps, durations, and command-line capture
- **SQLiteGraph v1.0**: Native V2 backend for improved performance and reliability
- **Magellan Integration**: Code indexing and label-based symbol discovery for all 7 languages
- **Enhanced Safety**: Eliminated unwrap() calls, comprehensive error handling, atomic rollback on failures

## What This Is NOT

- An IDE or LSP server - Use Rust Analyzer, IntelliJ, PyCharm, VSCode, or your editor
- A general-purpose refactoring tool - Focused on specific operations
- A complete solution - It's a focused tool for specific jobs
- Production-hardened - Use with version control

## What It Does

- **patch**: Replace function bodies, class definitions, enum variants with validation (single or batch)
- **delete**: Remove symbol definitions and all references (cross-file, Rust-only)
- **apply-files**: Multi-file pattern replacement with AST confirmation
- **query**: Query symbols by labels using Magellan integration (NEW)
- **get**: Get code chunks from the database without re-reading files (NEW)
- **undo**: Restore files from backup manifest
- **plan**: Orchestrate multi-step refactors via JSON plans
- **preview**: Inspect changes before applying (dry-run mode)
- **backup**: Create backups with automatic restore capability
- Validates syntax with tree-sitter after every operation
- Validates compilation with language-specific compilers
- Rolls back atomically on any failure

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

**Delete modes:**
- **Full**: Finds all references across files (Rust only)
- **Basic**: Deletes definition only, no reference finding (other languages)

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

## Quick Start

### Delete a Symbol (Rust)

Delete a function and all its references:

```bash
splice delete --file src/lib.rs --symbol helper --kind function
```

Output (structured JSON):
```json
{
  "version": "2.0.0",
  "operation_id": "20250118_123456_abc123",
  "operation_type": "delete",
  "status": "success",
  "message": "Deleted 'helper' (3 references + definition) across 2 file(s).",
  "timestamp": "2025-01-18T12:34:56.789Z",
  "result": {
    "symbol": "helper",
    "kind": "function",
    "spans_deleted": 4,
    "files_affected": 2,
    "spans": [
      {
        "file_path": "src/lib.rs",
        "symbol": "helper",
        "kind": "function",
        "byte_start": 120,
        "byte_end": 189,
        "line_start": 5,
        "line_end": 7,
        "col_start": 0,
        "col_end": 1,
        "span_checksum_before": "a1b2c3d4..."
      }
    ]
  }
}
```

### Patch a Symbol (Rust)

Replace a function body:

```bash
cat > new_greet.rs << 'EOF'
pub fn greet(name: &str) -> String {
    format!("Hi, {}!", name)
}
EOF

splice patch --file src/lib.rs --symbol greet --kind function --with new_greet.rs
```

### Patch a Symbol (Python)

```bash
cat > new_calc.py << 'EOF'
def calculate(x: int, y: int) -> int:
    return x * y
EOF

splice patch --file utils.py --symbol calculate --language python --with new_calc.py
```

### Patch a Symbol (TypeScript)

```bash
cat > new_fn.ts << 'EOF'
function calculate(x: number, y: number): number {
    return x * y;
}
EOF

splice patch --file src/math.ts --symbol calculate --language type-script --with new_fn.ts
```

### Multi-Step Plan

```bash
cat > plan.json << 'EOF'
{
  "steps": [
    {
      "file": "src/lib.rs",
      "symbol": "foo",
      "kind": "function",
      "with": "patches/foo.rs"
    },
    {
      "file": "src/lib.rs",
      "symbol": "bar",
      "kind": "function",
      "with": "patches/bar.rs"
    }
  ]
}
EOF

splice plan --file plan.json
```

### Batch Patch

Apply multiple patches at once from a JSON file:

```bash
cat > batch.json << 'EOF'
{
  "patches": [
    {
      "file": "src/lib.rs",
      "symbol": "foo",
      "kind": "function",
      "with": "patches/foo.rs"
    },
    {
      "file": "src/lib.rs",
      "symbol": "bar",
      "kind": "function",
      "with": "patches/bar.rs"
    }
  ]
}
EOF

splice patch --batch batch.json --language rust
```

### Pattern Replace

Replace a pattern across multiple files:

```bash
# Replace "42" with "99" in all Python files
splice apply-files --glob "*.py" --find "42" --replace "99"

# With validation and backup
splice apply-files --glob "tests/**/*.rs" --find "old_func" --replace "new_func" --create-backup
```

### Preview Mode

Inspect changes before applying:

```bash
splice patch --file src/lib.rs --symbol foo --with new_foo.rs --preview
```

### Backup and Undo

Create a backup before changes:

```bash
splice patch --file src/lib.rs --symbol foo --with new_foo.rs --create-backup --operation-id "my-change"
```

Restore from backup:

```bash
splice undo --manifest .splice-backup/my-change/manifest.json
```

### Query Symbols by Label (Magellan Integration)

```bash
# List all available labels
splice query --db code.db --list

# Find all Rust functions
splice query --db code.db --label rust --label fn

# Show code for each result
splice query --db code.db --label struct --show-code
```

### Get Code Chunks (Magellan Integration)

```bash
# Get code by byte span without re-reading the file
splice get --db code.db --file src/lib.rs --start 0 --end 100
```

## Commands

### splice delete

Remove a symbol definition and all its references.

```bash
splice delete --file <PATH> --symbol <NAME> [--kind <KIND>] [--language <LANG>]
```

**Optional Arguments:**
- `--kind <KIND>`: Symbol kind filter
- `--language <LANG>`: Language override
- `--analyzer <MODE>`: Validation mode (off, os, path)
- `--create-backup`: Create backup before deleting
- `--operation-id <ID>`: Custom operation ID for auditing
- `--metadata <JSON>`: Optional metadata attachment

**Rust-specific features:**
- Finds references across the entire workspace
- Tracks imports and re-exports
- Handles shadowing correctly
- Cross-file reference resolution

**Other languages:**
- Deletes the symbol definition only
- Use with `--language` flag or auto-detection from file extension

### splice patch

Apply a patch to a symbol's span.

```bash
splice patch --file <PATH> --symbol <NAME> --with <FILE> [--kind <KIND>] [--language <LANG>]
```

**Optional Arguments:**
- `--kind <KIND>`: Symbol kind filter (function, method, class, struct, interface, enum, trait, impl, module, variable, constructor, type-alias)
- `--language <LANG>`: Language override (rust, python, c, cpp, java, java-script, type-script)
- `--analyzer <MODE>`: Validation mode (off, os, path)
- `--preview`: Run in preview mode without modifying files
- `--batch <FILE>`: JSON file describing batch replacements
- `--create-backup`: Create backup before patching
- `--operation-id <ID>`: Custom operation ID for auditing
- `--metadata <JSON>`: Optional metadata attachment

### splice apply-files

Apply a pattern replacement to multiple files.

```bash
splice apply-files --glob <GLOB> --find <PATTERN> --replace <REPLACEMENT>
```

**Required Arguments:**
- `--glob <GLOB>`: Glob pattern for matching files (e.g., `tests/**/*.rs`, `src/**/*.py`)
- `--find <PATTERN>`: Text pattern to find
- `--replace <REPLACEMENT>`: Replacement text

**Optional Arguments:**
- `--language <LANG>`: Language override (auto-detected from extension by default)
- `--no-validate`: Skip validation gates
- `--create-backup`: Create backup before applying
- `--operation-id <ID>`: Custom operation ID for auditing
- `--metadata <JSON>`: Optional metadata attachment

### splice undo

Undo a previous operation by restoring from a backup manifest.

```bash
splice undo --manifest <PATH>
```

### splice plan

Execute a multi-step refactoring plan.

```bash
splice plan --file <PLAN.json>
```

### splice query

Query symbols by labels using Magellan integration.

```bash
splice query --db <FILE> [--label <LABEL>]... [--list] [--count] [--show-code]
```

**Optional Arguments:**
- `--db <FILE>`: Path to the Magellan database (required)
- `--label <LABEL>`: Label to query (can be specified multiple times for AND semantics)
- `--list`: List all available labels with counts
- `--count`: Count entities with specified label(s)
- `--show-code`: Show source code for each result

**Available labels:**
- Language labels: `rust`, `python`, `javascript`, `typescript`, `c`, `cpp`, `java`
- Symbol kind labels: `fn`, `method`, `struct`, `class`, `enum`, `interface`, `module`, `union`, `namespace`, `typealias`

### splice get

Get code chunks from the database using Magellan integration.

```bash
splice get --db <FILE> --file <PATH> --start <N> --end <N>
```

**Required Arguments:**
- `--db <FILE>`: Path to the Magellan database
- `--file <PATH>`: File path
- `--start <N>`: Start byte offset
- `--end <N>`: End byte offset

### splice log

Query the execution audit trail from `.splice/operations.db`.

```bash
# Show recent operations
splice log

# Filter by operation type
splice log --operation-type patch

# Filter by status
splice log --status success

# Filter by date range
splice log --after "2025-01-01" --before "2025-01-31"

# Query specific execution
splice log --execution-id 20250118_123456_abc123

# Show statistics
splice log --stats

# JSON output for programmatic access
splice log --format json
```

**Optional Arguments:**
- `--operation-type <TYPE>`: Filter by operation type (patch, delete, plan, apply-files)
- `--status <STATUS>`: Filter by status (success, failure, partial)
- `--after <DATE>`: Show operations after this date (ISO8601 format)
- `--before <DATE>`: Show operations before this date (ISO8601 format)
- `--execution-id <ID>`: Query specific execution by ID
- `--format <FORMAT>`: Output format (table, json) - default: table
- `--stats`: Show statistics summary instead of individual operations
- `--limit <N>`: Limit number of results (default: 50)

**Output Fields:**
- execution_id: Unique identifier for each operation
- operation_type: Type of operation performed
- status: Success/failure status
- timestamp: ISO8601 timestamp when operation started
- duration_ms: Operation duration in milliseconds
- command_line: Full command-line invocation
- workspace: Working directory path
- affected_files: Number of files modified

## Documentation

- **[docs/magellan_integration.md](docs/magellan_integration.md)** - Magellan integration guide (query commands, export formats, LLM usage)
- **manual.md** - Complete user manual
- **CHANGELOG.md** - Version history
- **docs/DIAGNOSTICS_HUMAN_LLM.md** - CLI diagnostics contract for humans and LLMs, including rust-analyzer and multi-language validation output

## Requirements

- Rust 1.70+ (for building)
- Language-specific compilers (for validation):
  - Rust: `cargo`
  - Python: `python`
  - C/C++: `gcc`/`g++`
  - Java: `javac`
  - JavaScript: `node`
  - TypeScript: `tsc`

## Architecture

- **src/cli/** - CLI argument parsing
- **src/ingest/** - Symbol parsing for 7 languages
- **src/graph/** - SQLiteGraph integration with Native V2 backend
- **src/resolve/** - Symbol resolution and reference finding
- **src/patch/** - Span-safe replacement + validation + batch operations + pattern replace + backup
- **src/validate/** - Tree-sitter + compiler validation gates with checksum verification
- **src/plan/** - JSON plan orchestration with deterministic ordering
- **src/execution/** - Execution logging audit trail with operations.db
- **src/output/** - Structured JSON output with v2.0 schema
- **src/checksum/** - SHA-256 checksum computation for verification hooks

## Validation Gates

Every operation passes:
1. UTF-8 boundary validation
2. Pre-operation checksum computation (SHA-256)
3. Tree-sitter reparse (syntax check)
4. Language-specific compiler check (cargo check, python -m py_compile, etc.)
5. Post-operation checksum verification
6. Atomic rollback on any failure

All operations are logged to `.splice/operations.db` with timestamps, durations, and command-line capture for complete auditability.

## Testing

```bash
cargo test
```

Test Coverage: 215+ tests passing, including:
- 171 unit tests for core functionality
- 26 Magellan integration tests for all 7 languages
- 18 cross-language compatibility tests
- End-to-end refactoring workflow tests

## Feedback

If you use this tool and find a bug or miss a feature that would be useful, drop a line. It's appreciated.

## License

GPL-3.0-or-later

## Disclaimer

This software modifies source code. Always commit your changes before running Splice.
