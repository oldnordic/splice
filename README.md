# Splice

Span-safe refactoring kernel for 7 languages using tree-sitter and SQLiteGraph.

**Version**: 0.5.0
**License**: GPL-3.0-or-later

## What This Is

Splice is a command-line tool that performs byte-accurate, AST-validated refactoring operations on code in 7 languages: Rust, Python, C, C++, Java, JavaScript, and TypeScript. It can replace function bodies, delete symbols, apply batch changes, perform pattern replacements, and undo operations.

**NEW in v0.5.0**: Magellan integration for code indexing and label-based symbol discovery.

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

Output:
```
Deleted 'helper' (3 references + definition) across 2 file(s).
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

## Documentation

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
- **src/graph/** - SQLiteGraph integration
- **src/resolve/** - Symbol resolution and reference finding
- **src/patch/** - Span-safe replacement + validation + batch operations + pattern replace + backup
- **src/validate/** - Tree-sitter + compiler validation gates
- **src/plan/** - JSON plan orchestration

## Validation Gates

Every operation passes:
1. UTF-8 boundary validation
2. Tree-sitter reparse (syntax check)
3. Language-specific compiler check (cargo check, python -m py_compile, etc.)

## Testing

```bash
cargo test
```

Test Coverage: 334/334 tests passing

## Feedback

If you use this tool and find a bug or miss a feature that would be useful, drop a line. It's appreciated.

## License

GPL-3.0-or-later

## Disclaimer

This software modifies source code. Always commit your changes before running Splice.
