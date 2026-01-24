# Splice

Span-safe refactoring kernel for 7 languages using tree-sitter and SQLiteGraph.

**Version**: 2.2.2
**License**: GPL-3.0-or-later

## What This Is

Splice is a command-line tool that performs byte-accurate, AST-validated refactoring operations on code in 7 languages: Rust, Python, C, C++, Java, JavaScript, and TypeScript. It can replace function bodies, delete symbols, apply batch changes, perform pattern replacements, query code graphs, and undo operations.

## v2.2.2 Features

Splice v2.2.2 delivers a unified CLI interface with Magellan query commands for code discovery alongside span-safe editing:

- **Unified CLI**: Single tool for code graph queries (Magellan) and refactoring (Splice)
- **Query Commands**: `status`, `query`, `find`, `refs`, `files` for code graph navigation
- **Export Command**: Export graph data in JSON, JSONL, or CSV formats
- **Magellan Integration**: In-process library delegation for optimal performance
- **CLI Alignment**: `--output` (human/json/pretty), `--db` for database path, Magellan-compatible exit codes
- **Error Handling**: SPL-E091 Magellan error code with full error chain preservation

## v2.2 Features

- **Rich Span Extensions**: Context, semantic kind, language, checksums, error codes
- **Rich Span Advanced**: Relationships (callers, callees, imports, exports), tool hints
- **CLI Conventions**: `-n` dry-run, `-A`/`-B`/`-C` context, unified diff, git-style exit codes
- **Enhanced Errors**: SPL-E### codes, severity levels, fuzzy suggestions, `splice explain`

## v2.0 Features

- **Structured JSON Output**: All operations return structured JSON with explicit fields
- **Span-Aware Metadata**: Line and column coordinates in all output
- **Deterministic Ordering**: All operations return sorted results
- **Validation Hooks**: Pre/post-operation checksums verify code integrity
- **Execution Logging**: Complete audit trail in `.splice/operations.db`
- **SQLiteGraph v1.0**: Native V2 backend for improved performance
- **Enhanced Safety**: Eliminated unwrap() calls, comprehensive error handling

## What This Is NOT

- An IDE or LSP server - Use Rust Analyzer, IntelliJ, PyCharm, VSCode, or your editor
- A general-purpose refactoring tool - Focused on specific operations
- A complete solution - It's a focused tool for specific jobs
- Production-hardened - Use with version control

## What It Does

**Edit Commands:**
- **patch**: Replace function bodies, class definitions, enum variants with validation (single or batch)
- **delete**: Remove symbol definitions and all references (cross-file, Rust-only)
- **apply-files**: Multi-file pattern replacement with AST confirmation
- **search**: Pattern-based search with glob filtering

**Query Commands (Magellan Integration):**
- **status**: Display database statistics (files, symbols, references, calls, code_chunks)
- **query**: Query symbols by labels (language, kind) with optional context
- **find**: Locate symbols by name or symbol_id with disambiguation
- **refs**: Show callers/callees for a symbol (bidirectional traversal)
- **files**: List indexed files with optional symbol counts
- **export**: Export graph data (JSON, JSONL, CSV formats)

**Utility Commands:**
- **undo**: Restore files from backup manifest
- **plan**: Orchestrate multi-step refactors via JSON plans
- **log**: Query execution audit trail from operations.db
- **explain**: Get detailed explanations for error codes

**Validation:**
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

### Query Commands (Magellan Integration)

```bash
# Show database statistics
splice status --db code.db

# Find all functions in a file
splice query --db code.db --file src/lib.rs --kind fn

# Find symbol by name
splice find --db code.db --name my_function

# Show call relationships
splice refs --db code.db --name my_function --direction out

# List indexed files
splice files --db code.db --symbols

# Export graph data
splice export --db code.db --format json --file export.json
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

### splice status

Display database statistics.

```bash
splice status --db <FILE> [--output FORMAT]
```

**Required Arguments:**
- `--db <FILE>`: Path to the Magellan database

**Optional Arguments:**
- `--output FORMAT`: Output format (human, json, pretty) - default: human

### splice find

Find symbols by name or symbol_id.

```bash
splice find --db <FILE> (--name <NAME> | --symbol-id <ID>) [--ambiguous] [--output FORMAT]
```

**Required Arguments:**
- `--db <FILE>`: Path to the Magellan database
- `--name <NAME>`: Symbol name to find
- `--symbol-id <ID>`: 16-character hex symbol ID

**Optional Arguments:**
- `--ambiguous`: Show all matches for ambiguous names
- `--output FORMAT`: Output format (human, json, pretty)

### splice refs

Show callers/callees for a symbol.

```bash
splice refs --db <FILE> (--name <NAME> | --path <PATH> --name <NAME>) [--direction DIR] [--output FORMAT]
```

**Required Arguments:**
- `--db <FILE>`: Path to the Magellan database
- `--name <NAME>`: Symbol name
- `--path <PATH>`: File path (for disambiguation)

**Optional Arguments:**
- `--direction DIR`: Relationship direction (in, out, both) - default: both
- `--output FORMAT`: Output format (human, json, pretty)

### splice files

List indexed files.

```bash
splice files --db <FILE> [--symbols] [--output FORMAT]
```

**Required Arguments:**
- `--db <FILE>`: Path to the Magellan database

**Optional Arguments:**
- `--symbols`: Show symbol counts per file
- `--output FORMAT`: Output format (human, json, pretty)

### splice export

Export graph data.

```bash
splice export --db <FILE> --format FORMAT --file <PATH>
```

**Required Arguments:**
- `--db <FILE>`: Path to the Magellan database
- `--format FORMAT`: Export format (json, jsonl, csv)
- `--file <PATH>`: Output file path

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

---

## Integration with Magellan

**Splice works best with [Magellan](https://crates.io/crates/magellan)** for code graph indexing and querying.

Magellan is a code understanding and indexing library that provides:
- Multi-language parsing (Rust, Python, C, C++, Java, JavaScript, TypeScript)
- Symbol discovery and relationship tracking
- Call graph analysis
- Fast code graph queries

**Links:**
- [Magellan on crates.io](https://crates.io/crates/magellan)
- [Magellan on GitHub](https://github.com/oldnordic/magellan)

When you use Splice's query commands (`status`, `query`, `find`, `refs`, `files`, `export`), you're leveraging Magellan's powerful code graph capabilities through Splice's unified CLI interface.
