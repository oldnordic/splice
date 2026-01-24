# Magellan Integration

Splice provides a unified CLI interface combining Magellan's code graph querying capabilities with Splice's span-safe refactoring operations. This integration allows LLMs and users to discover, query, and modify code using a single tool.

## Quick Reference

Copy-paste examples for common workflows.

### Get Database Statistics

```bash
# Human-readable output
splice status --db code-graph.db

# JSON output for parsing
splice status --db code-graph.db --output json
```

**Expected output (human):**
```
Files:        42
Symbols:      1,234
References:   3,456
Calls:        789
Code chunks:  1,234
Database:     code-graph.db
```

**Expected output (JSON):**
```json
{
  "files": 42,
  "symbols": 1234,
  "references": 3456,
  "calls": 789,
  "code_chunks": 1234,
  "db_path": "code-graph.db"
}
```

### Find All Functions in a File

```bash
# Query by language and kind labels
splice query --db code-graph.db --label rust --label fn

# With code snippets shown
splice query --db code-graph.db --label rust --label fn --show-code
```

**Expected output (JSON):**
```json
{
  "labels": ["rust", "fn"],
  "count": 15,
  "symbols": [
    {
      "name": "process",
      "kind": "fn",
      "file_path": "/project/src/lib.rs",
      "byte_start": 120,
      "byte_end": 245,
      "start_line": 5,
      "end_line": 12,
      "start_col": 4,
      "end_col": 1
    }
  ]
}
```

### Find Symbol by Name

```bash
# Find by name (first match)
splice find --db code-graph.db --name my_function

# Find by 16-character symbol ID
splice find --db code-graph.db --symbol-id 1a2b3c4d5e6f7a8b

# Return all matches (ambiguous names)
splice find --db code-graph.db --name new --ambiguous
```

**Expected output:**
```json
{
  "symbols": [
    {
      "symbol_id": "1a2b3c4d5e6f7a8b",
      "name": "my_function",
      "kind": "fn",
      "file_path": "/project/src/lib.rs",
      "byte_start": 120,
      "byte_end": 245,
      "start_line": 5,
      "end_line": 12,
      "start_col": 4,
      "end_col": 1
    }
  ],
  "count": 1
}
```

### Show Call Relationships

```bash
# Show callers (what calls this symbol)
splice refs --db code-graph.db --path src/lib.rs --name helper --direction in

# Show callees (what this symbol calls)
splice refs --db code-graph.db --path src/lib.rs --name helper --direction out

# Show both callers and callees
splice refs --db code-graph.db --path src/lib.rs --name helper --direction both
```

**Expected output:**
```json
{
  "symbol": {
    "name": "helper",
    "kind": "fn",
    "file_path": "/project/src/lib.rs",
    "byte_start": 50,
    "byte_end": 100,
    "start_line": 2,
    "end_line": 5,
    "start_col": 0,
    "end_col": 1
  },
  "callers": [
    {
      "symbol": {
        "name": "main",
        "kind": "fn",
        "file_path": "/project/src/main.rs"
      },
      "call_site": {
        "file_path": "/project/src/main.rs",
        "byte_start": 75,
        "byte_end": 83,
        "start_line": 10,
        "start_col": 4,
        "end_line": 10,
        "end_col": 12
      }
    }
  ],
  "callees": [
    {
      "symbol": {
        "name": "process",
        "kind": "fn",
        "file_path": "/project/src/lib.rs"
      },
      "call_site": {
        "file_path": "/project/src/lib.rs",
        "byte_start": 60,
        "byte_end": 68,
        "start_line": 3,
        "start_col": 8,
        "end_line": 3,
        "end_col": 16
      }
    }
  ]
}
```

### Export Graph Data

```bash
# Export as JSON to file
splice export --db code-graph.db --format json --file export.json

# Export as JSONL to stdout (pipe to jq)
splice export --db code-graph.db --format jsonl | jq '.type'

# Export as CSV for spreadsheet analysis
splice export --db code-graph.db --format csv --file export.csv
```

**Expected output (JSON):**
```json
{
  "schema_version": "1.0.0",
  "timestamp": "2025-01-24T15:30:00Z",
  "db_path": "code-graph.db",
  "data": {
    "files": [
      {
        "path": "/project/src/lib.rs",
        "hash": "a1b2c3d4e5f6a7b8c9d0e1f2a3b4c5d6",
        "last_indexed_at": 1706140800,
        "last_modified": 1706140800
      }
    ],
    "symbols": [
      {
        "symbol_id": "a1b2c3d4e5f6a7b8",
        "name": "process",
        "kind": "fn",
        "file_path": "/project/src/lib.rs",
        "byte_start": 120,
        "byte_end": 245,
        "start_line": 5,
        "end_line": 12,
        "start_col": 0,
        "end_col": 1
      }
    ],
    "references": [],
    "calls": []
  }
}
```

### LLM Workflow Example

Complete LLM-driven refactoring workflow:

```bash
# Step 1: Discover codebase structure
splice status --db code.db --output json

# Step 2: Find functions by language and kind
splice query --db code.db --label rust --label fn --output json

# Step 3: Get specific symbol details with span information
splice find --db code.db --name process --output json

# Step 4: Understand call relationships before editing
splice refs --db code.db --name process --path src/lib.rs --direction in --output json

# Step 5: Edit with span safety (using byte spans from find)
splice patch --file src/lib.rs --symbol process --with new_content.rs

# Step 6: Verify changes didn't break callers
splice refs --db code.db --name process --path src/lib.rs --direction in --output json
```

**Workflow notes:**
- Step 1 returns database statistics (confirms indexing is complete)
- Step 2 returns all matching symbols (for exploring available functions)
- Step 3 returns exact byte spans for the target symbol
- Step 4 shows all callers (impact analysis before editing)
- Step 5 applies the change using span-safe operations
- Step 6 verifies callers are still intact after the change

## Overview

### What is Magellan Integration?

Magellan is a code graph database that indexes symbols, references, and calls across multiple programming languages. Splice integrates Magellan v0.5.0 as an in-process library, providing:

- **Multi-language code indexing**: Rust, Python, C, C++, Java, JavaScript, TypeScript
- **Label-based symbol discovery**: Query by language (rust, python) and kind (fn, struct, class)
- **Call relationship traversal**: Find callers and callees for any symbol
- **Code chunk retrieval**: Access source code without re-reading files

### Why Unify Splice and Magellan?

Before this integration, users needed two separate tools:

- **Magellan**: For code discovery and querying
- **Splice**: For span-safe editing operations

The unified interface provides:

1. **Single-tool workflow**: LLMs can use one tool for both discovery and editing
2 **Consistent output format**: All commands return structured JSON with the same schema
3. **Aligned CLI conventions**: Matching `--output`, `--db` flags across all commands
4. **Span-safe editing**: Query discovers exact byte spans, patch operates on those spans

### Architecture

```
                    User / LLM
                         |
        +----------------+----------------+
        |                                |
   Query Commands                 Edit Commands
   (Magellan-delegated)            (Splice-native)
        |                                |
        +----------------+----------------+
                         |
                     splice CLI
                         |
        +----------------+----------------+
        |                                |
   MagellanIntegration              Splice Core
   (in-process library)          (refactoring engine)
        |                                |
        +----------------+----------------+
                         |
                    SQLiteGraph
                 (code graph database)
```

**Key architectural decisions:**

- **Library delegation**: Magellan is linked as a Rust library, not a subprocess
- **Field translation layer**: Splice fields (`line_start`) map to Magellan fields (`start_line`)
- **16-char symbol IDs**: SHA-256(name:path:byte_start)[0..8] for stable identifiers
- **Unified exit codes**: Magellan-compatible exit codes (0-5) across all commands

## Quick Reference

### Common Workflows

```bash
# 1. Get database statistics
splice status --db code-graph.db
splice status --db code-graph.db --output json

# 2. Find all functions in a file
splice query --db code-graph.db --file src/lib.rs --kind fn

# 3. Find symbol by name
splice find --db code-graph.db --name my_function
splice find --db code-graph.db --symbol-id 1a2b3c4d5e6f7a8b

# 4. Show call relationships
splice refs --db code-graph.db --name my_function --direction out
splice refs --db code-graph.db --path src/lib.rs --name helper --direction both

# 5. Export graph data
splice export --db code-graph.db --format json --file export.json
splice export --db code-graph.db --format jsonl | jq

# 6. LLM workflow example
# 1. Discover structure
splice status --db code.db --output json
# 2. Find functions
splice query --db code.db --label rust --label fn --output json
# 3. Get symbol details
splice find --db code.db --name process --output json
# 4. Edit with span safety
splice patch --file src/lib.rs --symbol process --with new_content.rs
```

### Expected Output Examples

#### Status Command (Human Output)

```bash
$ splice status --db code-graph.db
Files:        42
Symbols:      1,234
References:   3,456
Calls:        789
Code chunks:  1,234
Database:     code-graph.db
```

#### Status Command (JSON Output)

```bash
$ splice status --db code-graph.db --output json
{
  "files": 42,
  "symbols": 1234,
  "references": 3456,
  "calls": 789,
  "code_chunks": 1234,
  "db_path": "code-graph.db"
}
```

#### Find Command Output

```bash
$ splice find --db code.db --name process --output json
{
  "symbols": [
    {
      "symbol_id": "1a2b3c4d5e6f7a8b",
      "name": "process",
      "kind": "fn",
      "file_path": "/path/to/src/lib.rs",
      "byte_start": 120,
      "byte_end": 245,
      "start_line": 5,
      "end_line": 12,
      "start_col": 0,
      "end_col": 1
    }
  ],
  "count": 1
}
```

## Query Commands Reference

### status

Display database statistics including file count, symbol count, references, calls, and code chunks.

**Syntax:**
```bash
splice status --db <PATH> [--output FORMAT]
```

**Required Arguments:**
- `--db <PATH>`: Path to the Magellan database

**Optional Arguments:**
- `-o, --output <FORMAT>`: Output format (human, json, pretty)

**Purpose:**
- Check database health and size
- Verify indexing completed successfully
- Get overview statistics before querying

**Exit Codes:**
- `0`: Success
- `3`: Database error (invalid or corrupt database)

**Example:**
```bash
$ splice status --db code.db --output json
{
  "files": 100,
  "symbols": 1234,
  "references": 5678,
  "calls": 890,
  "code_chunks": 1234,
  "db_path": "code.db"
}
```

### find

Find symbols by name or 16-character symbol ID across all indexed files.

**Syntax:**
```bash
splice find --db <PATH> (--name <NAME> | --symbol-id <ID>) [--ambiguous] [--output FORMAT]
```

**Required Arguments:**
- `--db <PATH>`: Path to the Magellan database
- `--name <NAME>`: Symbol name to search
- `--symbol-id <ID>`: 16-character hex symbol ID (mutually exclusive with `--name`)

**Optional Arguments:**
- `--ambiguous`: Return all matches (default: first match only)
- `-o, --output <FORMAT>`: Output format (human, json, pretty)

**Purpose:**
- Locate exact symbol definition across all files
- Get symbol span for editing operations
- Resolve symbol by stable ID

**Exit Codes:**
- `0`: Success (including no results found)
- `1`: Error
- `4`: File not found

**Example (by name):**
```bash
$ splice find --db code.db --name process --output json
{
  "symbols": [
    {
      "symbol_id": "a1b2c3d4e5f6a7b8",
      "name": "process",
      "kind": "fn",
      "file_path": "/project/src/lib.rs",
      "byte_start": 120,
      "byte_end": 245,
      "start_line": 5,
      "end_line": 12,
      "start_col": 4,
      "end_col": 1
    }
  ],
  "count": 1
}
```

**Example (by symbol ID):**
```bash
$ splice find --db code.db --symbol-id a1b2c3d4e5f6a7b8
{
  "symbols": [
    {
      "symbol_id": "a1b2c3d4e5f6a7b8",
      "name": "process",
      "kind": "fn",
      ...
    }
  ],
  "count": 1
}
```

### refs

Show call relationships for a symbol (callers and/or callees).

**Syntax:**
```bash
splice refs --db <PATH> --name <NAME> --path <PATH> [--direction DIR] [--output FORMAT]
```

**Required Arguments:**
- `--db <PATH>`: Path to the Magellan database
- `--name <NAME>`: Symbol name
- `--path <PATH>`: File path containing the symbol

**Optional Arguments:**
- `--direction <DIR>`: Call direction (in, out, both). Default: `both`
  - `in`: Show callers (what calls this symbol)
  - `out`: Show callees (what this symbol calls)
  - `both`: Show both callers and callees
- `-o, --output <FORMAT>`: Output format (human, json, pretty)

**Purpose:**
- Understand function call graph
- Find impact of changing a function
- Discover usage patterns

**Exit Codes:**
- `0`: Success
- `1`: Error
- `4`: Symbol not found

**Example:**
```bash
$ splice refs --db code.db --name helper --path src/lib.rs --direction out --output json
{
  "symbol": {
    "name": "helper",
    "kind": "fn",
    "file_path": "/project/src/lib.rs",
    "byte_start": 50,
    "byte_end": 100,
    "start_line": 2,
    "end_line": 5,
    "start_col": 0,
    "end_col": 1
  },
  "callees": [
    {
      "symbol": {
        "name": "process",
        "kind": "fn",
        "file_path": "/project/src/lib.rs",
        ...
      },
      "call_site": {
        "file_path": "/project/src/lib.rs",
        "byte_start": 75,
        "byte_end": 83,
        "start_line": 4,
        "start_col": 4,
        "end_line": 4,
        "end_col": 12
      }
    }
  ],
  "callers": []
}
```

### files

List all indexed files with optional symbol counts.

**Syntax:**
```bash
splice files --db <PATH> [--symbols] [--output FORMAT]
```

**Required Arguments:**
- `--db <PATH>`: Path to the Magellan database

**Optional Arguments:**
- `--symbols`: Include symbol count per file
- `-o, --output <FORMAT>`: Output format (human, json, pretty)

**Purpose:**
- See which files are indexed
- Find files with most symbols
- Verify indexing coverage

**Exit Codes:**
- `0`: Success
- `3`: Database error

**Example:**
```bash
$ splice files --db code.db --symbols --output json
{
  "files": [
    {
      "path": "/project/src/lib.rs",
      "hash": "a1b2c3d4...",
      "last_indexed_at": 1706140800,
      "last_modified": 1706140800,
      "symbol_count": 42
    },
    {
      "path": "/project/src/main.rs",
      "hash": "e5f6a7b8...",
      "last_indexed_at": 1706140800,
      "last_modified": 1706140800,
      "symbol_count": 15
    }
  ],
  "count": 2
}
```

### query

Query symbols by labels using Magellan's label-based discovery.

**Syntax:**
```bash
splice query --db <PATH> [--label <LABEL>]... [--list] [--count] [--show-code]
```

**Required Arguments:**
- `--db <PATH>`: Path to the Magellan database

**Optional Arguments:**
- `--label <LABEL>`: Label to query (can be specified multiple times for AND semantics)
- `--list`: List all available labels with counts
- `--count`: Count entities with specified label(s)
- `--show-code`: Show source code for each result

**Purpose:**
- Discover symbols by language and kind
- Find all functions in a language
- List available labels for exploration

**Exit Codes:**
- `0`: Success
- `3`: Database error

**Available Labels:**
- **Language labels**: `rust`, `python`, `javascript`, `typescript`, `c`, `cpp`, `java`
- **Symbol kind labels**: `fn`, `method`, `struct`, `class`, `enum`, `interface`, `module`, `union`, `namespace`, `typealias`

**Example (find Rust functions):**
```bash
$ splice query --db code.db --label rust --label fn
```

**Example (list all labels):**
```bash
$ splice query --db code.db --list
rust: 150
python: 80
fn: 200
struct: 50
class: 30
...
```

## Export Command Reference

### export

Export graph data in JSON, JSONL, or CSV format for analysis or migration.

**Syntax:**
```bash
splice export --db <PATH> [--format FORMAT] [--file PATH]
```

**Required Arguments:**
- `--db <PATH>`: Path to the Magellan database

**Optional Arguments:**
- `--format <FORMAT>`: Export format (json, jsonl, csv). Default: `json`
- `--file <PATH>`: Output file path (writes to stdout if not specified)

**Purpose:**
- Backup code graph data
- Analyze code structure offline
- Migrate to other tools
- Generate documentation

**Exit Codes:**
- `0`: Success
- `1`: Export error
- `3`: Database error

### Format Comparison

| Format | Description | Best For |
|--------|-------------|----------|
| **json** | Single JSON file with all data | API consumption, complete backups |
| **jsonl** | Newline-delimited JSON records | Streaming processing, large datasets |
| **csv** | CSV with section headers | Spreadsheet analysis, human review |

### JSON Format

```bash
$ splice export --db code.db --format json --file export.json
```

**Output structure:**
```json
{
  "schema_version": "1.0.0",
  "timestamp": "2025-01-24T15:30:00Z",
  "db_path": "code.db",
  "data": {
    "files": [
      {
        "path": "/project/src/lib.rs",
        "hash": "a1b2c3d4...",
        "last_indexed_at": 1706140800,
        "last_modified": 1706140800
      }
    ],
    "symbols": [
      {
        "symbol_id": "a1b2c3d4e5f6a7b8",
        "name": "process",
        "kind": "fn",
        "file_path": "/project/src/lib.rs",
        "byte_start": 120,
        "byte_end": 245,
        "start_line": 5,
        "end_line": 12,
        "start_col": 0,
        "end_col": 1
      }
    ],
    "references": [],
    "calls": []
  }
}
```

### JSONL Format

```bash
$ splice export --db code.db --format jsonl --file export.jsonl
```

**Output structure (one JSON object per line):**
```
{"type":"header","schema_version":"1.0.0","timestamp":"2025-01-24T15:30:00Z","db_path":"code.db"}
{"type":"file","data":{"path":"/project/src/lib.rs","hash":"a1b2c3d4...","last_indexed_at":1706140800,"last_modified":1706140800}}
{"type":"symbol","data":{"symbol_id":"a1b2c3d4e5f6a7b8","name":"process","kind":"fn",...}}
{"type":"call","data":{"caller_symbol_id":"...","callee_symbol_id":"...","call_site_file":"...","call_site_line":42}}
```

**Record types:**
- `header`: Export metadata
- `file`: File metadata
- `symbol`: Symbol definition
- `reference`: Reference between symbols
- `call`: Function call relationship

### CSV Format

```bash
$ splice export --db code.db --format csv --file export.csv
```

**Output structure:**
```
# Files
path,hash,last_indexed_at,last_modified
/project/src/lib.rs,a1b2c3d4...,1706140800,1706140800

# Symbols
symbol_id,name,kind,file_path,byte_start,byte_end,start_line,end_line,start_col,end_col
a1b2c3d4e5f6a7b8,process,fn,/project/src/lib.rs,120,245,5,12,0,1

# References
from_symbol_id,to_symbol_id,reference_kind

# Calls
caller_symbol_id,callee_symbol_id,call_site_file,call_site_line
```

**Use cases:**
- Import into spreadsheet for analysis
- Generate documentation reports
- Custom data processing with standard tools

## Error Handling

### Exit Code Mapping

Splice uses Magellan-compatible exit codes:

| Exit Code | Meaning | Description |
|-----------|---------|-------------|
| `0` | Success | Operation completed successfully |
| `1` | Error | General error (invalid input, operation failed) |
| `2` | Usage | Invalid command-line arguments |
| `3` | Database | Database error (corrupt, invalid, or missing) |
| `4` | File Not Found | Requested file does not exist |
| `5` | Validation | Validation gate failed (syntax, compilation) |

### SPL-E091 Magellan Error Code

When Magellan operations fail, Splice returns error code `SPL-E091` with exit code `3`.

**Error response structure:**
```json
{
  "status": "error",
  "error": {
    "kind": "MagellanError",
    "message": "Failed to open Magellan graph at /path/to/db: database is locked",
    "error_code": {
      "code": "SPL-E091",
      "severity": "error",
      "message": "Magellan database operation failed"
    },
    "explain_command": "splice explain --code SPL-E091"
  }
}
```

**Common Magellan error scenarios:**

| Scenario | Error Code | Exit Code | Remedy |
|----------|------------|-----------|--------|
| Database not found | SPL-E091 | 3 | Create database with `magellan init` |
| Database corrupt | SPL-E091 | 3 | Re-index from source files |
| Database locked | SPL-E091 | 3 | Close other processes using the database |
| Invalid query | SPL-E001 | 1 | Check symbol name and file path |

### Getting Error Help

Use the `explain` command to get detailed error information:

```bash
$ splice explain --code SPL-E091
SPL-E091: Magellan database operation failed

Severity: error
Exit code: 3

Description:
A Magellan database operation failed. This may indicate:
- The database file does not exist
- The database is corrupted
- The database is locked by another process
- Insufficient permissions to access the database

Remediation:
1. Verify the database path is correct
2. Check file permissions on the database
3. Ensure no other processes are using the database
4. Re-index the codebase if the database is corrupt
```

## LLM Usage Patterns

### Discovery Workflow

The discovery workflow helps LLMs understand code structure before making changes.

**Step 1: Get database overview**
```bash
splice status --db code.db --output json
```

**Step 2: Find symbols by label**
```bash
splice query --db code.db --label rust --label fn --output json
```

**Step 3: Get symbol details**
```bash
splice find --db code.db --name process --output json
```

**Step 4: Understand call relationships**
```bash
splice refs --db code.db --name process --path src/lib.rs --direction both --output json
```

### Edit Workflow

The edit workflow combines discovery queries with span-safe editing.

**Step 1: Find target symbol**
```bash
splice find --db code.db --name refactor_me --output json
# Response includes byte_start, byte_end for precise editing
```

**Step 2: Preview changes (dry-run)**
```bash
splice patch --file src/lib.rs --symbol refactor_me --with new_impl.rs --preview
```

**Step 3: Apply changes**
```bash
splice patch --file src/lib.rs --symbol refactor_me --with new_impl.rs
```

**Step 4: Verify with relationships**
```bash
splice refs --db code.db --name refactor_me --path src/lib.rs --direction in --output json
# Check that callers still work correctly
```

### Example JSON Responses

**Status response:**
```json
{
  "files": 42,
  "symbols": 1234,
  "references": 5678,
  "calls": 890,
  "code_chunks": 1234,
  "db_path": "code.db"
}
```

**Find response:**
```json
{
  "symbols": [
    {
      "symbol_id": "a1b2c3d4e5f6a7b8",
      "name": "process",
      "kind": "fn",
      "file_path": "/project/src/lib.rs",
      "byte_start": 120,
      "byte_end": 245,
      "start_line": 5,
      "end_line": 12,
      "start_col": 4,
      "end_col": 1
    }
  ],
  "count": 1
}
```

**Refs response:**
```json
{
  "symbol": {
    "name": "process",
    "kind": "fn",
    "file_path": "/project/src/lib.rs",
    "byte_start": 120,
    "byte_end": 245,
    "start_line": 5,
    "end_line": 12,
    "start_col": 4,
    "end_col": 1
  },
  "callers": [
    {
      "symbol": {
        "name": "main",
        "kind": "fn",
        "file_path": "/project/src/main.rs",
        ...
      },
      "call_site": {
        "file_path": "/project/src/main.rs",
        "byte_start": 50,
        "byte_end": 58,
        "start_line": 10,
        "start_col": 4,
        "end_line": 10,
        "end_col": 12
      }
    }
  ],
  "callees": []
}
```

### Best Practices for LLM Consumption

1. **Always use `--output json`** for programmatic parsing
2. **Start with `status`** to verify database exists and is populated
3. **Use `refs --direction in`** before editing to understand impact
4. **Use `--symbol-id` when available** for stable references
5. **Parse `byte_start`/`byte_end`** for precise span-based editing
6. **Handle empty results gracefully** (e.g., no callers is valid, not an error)
7. **Check `error_code` field** in error responses for machine-readable diagnostics

## Performance Characteristics

### Benchmarks

Based on Phase 26 integration testing with typical codebases:

| Command | Operation | Performance | Complexity |
|---------|-----------|-------------|------------|
| `status` | Database statistics | < 500ms for 100 files | O(1) |
| `find --name` | Symbol name search | < 200ms by name | O(N) where N = files |
| `find --symbol-id` | Symbol ID lookup | < 100ms | O(N) where N = symbols |
| `refs` | Call relationships | < 300ms per symbol | O(degree) |
| `export` | Full export | < 1s for 500 symbols | O(total entities) |
| `query --label` | Label query | < 100ms indexed | O(log N) indexed |

### Scaling Recommendations

- **Small projects (< 100 files)**: All commands complete in < 1 second
- **Medium projects (100-1000 files)**: Most commands < 2 seconds
- **Large projects (> 1000 files)**: Consider incremental queries using `--label` filters

**Performance tips:**

1. Use `--name` for direct symbol lookup (faster than `--symbol-id`)
2. Use `--label` filters to reduce result set size
3. Use `--direction out` or `--direction in` instead of `--direction both` for large call graphs
4. Export to JSONL for streaming large datasets

## See Also

- **[README.md](../README.md)** - Main project documentation and installation
- **[splice explain](#)** - Error code reference
- **[ARCHITECTURE.md](ARCHITECTURE.md)** - Splice architecture details
- **[manual.md](manual.md)** - Complete user manual for all commands
- **[CLI_PATTERNS.md](CLI_PATTERNS.md)** - CLI design patterns and conventions
