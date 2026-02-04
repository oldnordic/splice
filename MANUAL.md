# Splice v2.3 Manual

Comprehensive guide for Splice v2.3 span-safe refactoring with Magellan v2 integration.

---

## Splice v2.3 Overview

### What is Splice?

Splice is a span-safe code refactoring tool that provides AST-validated code modifications across 7 programming languages (Rust, Python, C, C++, Java, JavaScript, TypeScript). It uses tree-sitter for parsing and SQLiteGraph for code relationship storage, with multi-stage validation (tree-sitter + compiler) before applying changes.

### v2.3 New Features

- **Cross-File Rename**: Byte-accurate symbol renaming across all files using Magellan ReferenceFact
- **Graph Algorithms**: Reachability, dead-code detection, cycle detection, condensation, program slicing
- **Proof-Based Refactoring**: Machine-checkable behavioral equivalence proofs with SHA-256 checksums
- **Preview JSON Output**: Structured JSON for preview mode with full metrics

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

---

## Query Commands (Magellan)

### splice status

Display database statistics.

```bash
splice status --db <FILE>
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
