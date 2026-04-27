# Code Completion

Splice provides grounded code completion using the Magellan database for fast, accurate symbol suggestions.

## Overview

The completion feature analyzes your codebase to suggest relevant symbols based on:
- **Database grounding**: All suggestions proven to exist in your code (includes database IDs)
- **Import awareness**: Suggests symbols from imported modules across file boundaries
- **Context understanding**: Knows your current function, module, and what you're typing
- **Fast performance**: 2-5ms query time on large codebases

## Quick Start

### Basic Usage

```bash
# Get completion suggestions at cursor position
splice complete --file src/lib.rs --line 27 --column 8 --db .magellan/splice.db --max-results 10

# JSON output for tooling integration
splice complete --file src/lib.rs --line 27 --column 8 --db .magellan/splice.db --max-results 5 --output json

# Use specific database
splice complete --file src/main.rs --line 15 --column 4 --db .magellan/splice.db --max-results 20
```

### Output Format

**Human-readable (default):**
```
✓ CompletionSuggestions (15 results, query: 2.3ms)

1. process_request (Function) [score: 0.95]
   src/api/handler.rs:42:8

2. RequestHandler (Struct) [score: 0.87]
   Via: use crate::api::RequestHandler

3. handle_response (Function) [score: 0.82]
   Via: use crate::api::handle_response
```

**JSON output:**
```json
{
  "suggestions": [
    {
      "label": "process_request",
      "insert_text": "process_request",
      "detail": "Function",
      "kind": "Function",
      "score": 0.95,
      "source": "Database",
      "grounded_in": ["12345"],
      "source_file": null,
      "via_import": null
    },
    {
      "label": "RequestHandler",
      "insert_text": "RequestHandler",
      "detail": "Struct",
      "kind": "Struct",
      "score": 0.87,
      "source": "Imported",
      "grounded_in": ["67890"],
      "source_file": "src/api/types.rs",
      "via_import": "use crate::api::RequestHandler"
    }
  ],
  "metadata": {
    "query_time_ms": 2.3,
    "total_symbols": 15234,
    "filtered_symbols": 847
  }
}
```

## Features

### 1. Grounded Suggestions

Every completion suggestion is backed by actual symbols in your codebase:

- **Database IDs**: Each suggestion includes `grounded_in` field with Magellan entity IDs
- **Verification**: Suggestions are pulled from indexed code, not guessed
- **Traceability**: Know exactly where each symbol comes from

### 2. Import-Aware Completion

Splice resolves imports to suggest symbols from other modules:

**Example:**
```rust
// In src/main.rs
use crate::api::{RequestHandler, process_request};

fn main() {
    process_ // ← Type here, get completions including:
             // - process_request (from import)
             // - RequestHandler (from import)
             // - Any local symbols starting with "process_"
}
```

**How it works:**
1. Queries Magellan database for Import entities
2. Resolves import paths to target files
3. Extracts public symbols from target files
4. Merges imported symbols with local symbols
5. Ranks imported symbols appropriately

### 3. Token Filtering

Completions are filtered based on what you're currently typing:

```bash
# Typing "req" at line 27, column 31
splice complete --file src/main.rs --line 27 --column 31 --db .magellan/splice.db --max-results 10

# Results only include symbols starting with "req":
# - request
# - RequestHandler
# - request_url
```

### 4. Fast Performance

Optimized for speed on large codebases:

- **2-5ms query time** on 8,600+ symbols
- **Incremental indexing** via Magellan watcher
- **Efficient ranking** using proximity signals

### 5. Context-Aware Ranking

Suggestions are ranked by relevance:

**Proximity signals (highest to lowest):**
1. **Same function** (1.0) - Symbols in current function
2. **Imported** (0.85) - Explicitly imported symbols
3. **Same file** (0.6) - Local symbols in current file
4. **Other** (0.3) - Everything else

**Example ranking:**
```
Typing in handle_request() function:

1. validate_token [score: 0.95]  ← Same function, high text match
2. RequestParser [score: 0.87]   ← Imported, good text match
3. parse_url [score: 0.72]       ← Same file, partial match
4. global_config [score: 0.35]   ← Different file, low relevance
```

## Architecture

### Database Schema

Completion uses Magellan's graph database:

```sql
-- Symbols (8,600+ in splice codebase)
SELECT id, name, kind, file_path, data
FROM graph_entities
WHERE kind = 'Symbol';

-- Imports (4,440+ in splice codebase)
SELECT id, file_path, data
FROM graph_entities
WHERE kind = 'Import';

-- Import relationships
SELECT from_id, to_id, edge_type
FROM graph_edges
WHERE edge_type = 'IMPORTS';
```

### Module Structure

```
src/completion/
├── engine.rs         # Main completion orchestration
├── context.rs        # Context analysis (function, module, scope)
├── types.rs          # Request/response types
├── ranking.rs        # Suggestion ranking algorithms
├── imports.rs        # Import resolution (cross-file)
├── module_index.rs   # Module path → file path mapping
└── tokenizer.rs      # Token extraction for filtering
```

### Query Flow

```
1. Parse cursor position (line, column)
   ↓
2. Analyze context (function, module, token)
   ↓
3. Query local symbols (same file)
   ↓
4. Resolve imports → get imported symbols
   ↓
5. Merge + deduplicate suggestions
   ↓
6. Rank by relevance
   ↓
7. Filter by current token
   ↓
8. Return top N results
```

## Suggestion Sources

Splice tracks where each suggestion comes from:

### Database (Local Symbols)

Symbols defined in the current file:

```json
{
  "label": "local_function",
  "source": "Database",
  "source_file": null,
  "via_import": null
}
```

### Imported (Cross-File Symbols)

Symbols from imported modules:

```json
{
  "label": "process_request",
  "source": "Imported",
  "source_file": "src/api/handler.rs",
  "via_import": "use crate::api::process_request"
}
```

## Performance Optimization

### Magellan Watcher

Keep your database updated for accurate completions:

```bash
# Start watcher (auto-updates on file changes)
magellan watch --root ./src --db .magellan/splice.db --debounce-ms 500 &
```

### Database Caching

Splice caches database connections for fast queries:

- **First query**: ~5ms (connection overhead)
- **Subsequent queries**: ~2ms (cached connection)

### Symbol Filtering

Performance optimized with incremental filtering:

1. **Database query**: Get all symbols in scope (~1000 symbols)
2. **Token filter**: Reduce to 50-100 matching symbols
3. **Ranking**: Sort and return top 10-20 results

## Integration Examples

### CLI Tools

```bash
# Get completions in a script
COMPLETIONS=$(splice complete --file src/main.rs --line 42 --column 8 --db .magellan/splice.db --max-results 5 --output json)

# Parse with jq
echo "$COMPLETIONS" | jq '.suggestions[].label'
```

### Editor Integration (Example)

```rust
// Vim/Neovim integration example
function! GetSpliceCompletion()
  let l = line('.')
  let c = col('.')
  let result = system('splice complete --file ' . expand('%') . ' --line ' . l . ' --column ' . c . ' --db .magellan/splice.db --max-results 10 --output json')
  return json_decode(result)
endfunction
```

### LSP Server (Future)

Completion data structure designed for LSP compatibility:

```rust
// Map to LSP CompletionItem
lsp_completion = CompletionItem {
    label: suggestion.label,
    kind: suggestion.kind.to_lsp_kind(),
    detail: suggestion.detail,
    score: Some(suggestion.score),
    ..()
}
```

## Limitations

### Language Support

Currently optimized for **Rust**:
- Full import resolution
- Module path mapping
- Symbol visibility (pub/private)

**Other languages** (Python, C++, etc.):
- Basic symbol completion
- Limited import resolution
- Future improvements planned

### Database Requirements

- Requires **Magellan v10+** database
- Must run `magellan watch` for live updates
- Import entities must be indexed (4,440+ in splice)

### Known Issues

1. **Macro expansion**: Macros not expanded in suggestions
2. **Generic types**: Type parameter resolution incomplete
3. **Dynamic imports**: Runtime imports not resolved

## Troubleshooting

### No Completions Returned

**Check database:**
```bash
magellan status --db .magellan/splice.db
```

**Verify symbol count:**
```bash
sqlite3 .magellan/splice.db "SELECT COUNT(*) FROM graph_entities WHERE kind = 'Symbol';"
```

**Check import entities:**
```bash
sqlite3 .magellan/splice.db "SELECT COUNT(*) FROM graph_entities WHERE kind = 'Import';"
```

### Slow Query Performance

**Rebuild database:**
```bash
rm .magellan/splice.db
magellan watch --root ./src --db .magellan/splice.db --scan-initial
```

**Check file count:**
```bash
magellan status --db .magellan/splice.db | grep "files indexed"
```

### Incorrect Suggestions

**Verify file path:**
```bash
# Use absolute path
splice complete --file /absolute/path/to/src/lib.rs --line 27 --column 8
```

**Check line/column:**
```bash
# Lines are 1-indexed, columns are 0-indexed
splice complete --file src/lib.rs --line 1 --column 0  # Start of file
```

## Performance Benchmarks

Tested on splice codebase (8,600+ symbols, 4,440+ imports):

| Metric | Value |
|--------|-------|
| Average query time | 2.3ms |
| 95th percentile | 4.1ms |
| 99th percentile | 6.8ms |
| Symbols analyzed | 8,600+ |
| Import entities | 4,440+ |
| Database size | ~15MB |

## Future Enhancements

### Planned Features

1. **Fuzzy matching**: Support typos and partial matches
2. **Type-aware completion**: Filter by expected type
3. **Documentation snippets**: Include doc comments
4. **Multi-language support**: Full Python, C++, Java support
5. **LSP integration**: Native Language Server Protocol
6. **Machine learning**: Learn user preferences

### Contributing

See [CONTRIBUTING.md](CONTRIBUTING.md) for development guidelines.

## See Also

- **[MANUAL.md](MANUAL.md)** — Complete command reference
- **[magellan_integration.md](magellan_integration.md)** — Database integration details
- **[CLAUDE.md](CLAUDE.md)** — Development workflow
