# Splice Architecture — v3.0 Agent Surface

**Version:** 3.0.0
**Last Updated:** 2026-06-26

## Overview

Splice v3.0 introduces an **agent-focused surface** for AI/LLM workflows while preserving existing developer tools. The new surface provides validated editing and intent-based scaffolding without requiring cursor positions or file state.

## Design Principles

1. **Agent-First Commands** — No cursor/LSP dependency for edit/suggest operations
2. **Validation Before Write** — All edits verified (syntax, compiler) before application
3. **Auto-Discovery** — DB path inferred from git root, `--db` flag optional
4. **Grounded Scaffolds** — Suggest queries Magellan DB for existing symbols/types
5. **Backward Compatible** — Existing CLI surface unchanged for editors/LSP integrations

## Architecture Layers

```
┌─────────────────────────────────────────────────────────┐
│                    Agent Surface Layer                  │
│  ┌────────────────┐  ┌─────────────────┐              │
│  │  splice edit  │  │ splice suggest  │              │
│  └────────┬───────┘  └────────┬────────┘              │
└───────────┼──────────────────┼──────────────────────────┘
            │                  │
┌───────────▼──────────────────▼──────────────────────────┐
│                 Command Layer (src/cli/)                  │
│  ┌──────────────────────────────────────────────────┐   │
│  │  commands.rs (edit, suggest, legacy commands)    │   │
│  └──────────────────────────────────────────────────┘   │
└──────────────────────────────────────────────────────────┘
            │                  │
┌───────────▼──────────────────▼──────────────────────────┐
│              Command Handlers (src/cmds/)                │
│  ┌──────────────┐  ┌──────────────┐  ┌──────────────┐  │
│  │  edit.rs     │  │  suggest.rs  │  │  [legacy]    │  │
│  │  (text       │  │  (intent-    │  │  handlers    │  │
│  │   replace)   │  │   based)     │  │              │  │
│  └──────┬───────┘  └──────┬───────┘  └──────────────┘  │
└─────────┼──────────────────┼────────────────────────────┘
          │                  │
┌─────────▼──────────────────▼────────────────────────────┐
│              Core Modules (src/)                         │
│  ┌──────────────┐  ┌──────────────┐  ┌──────────────┐  │
│  │  graph/      │  │  patch/      │  │  verify/     │  │
│  │  db_discovery│  │  text_       │  │  pre/post    │  │
│  │  .rs         │  │  replace.rs  │  │  validation  │  │
│  └──────────────┘  └──────────────┘  └──────────────┘  │
└──────────────────────────────────────────────────────────┘
            │                  │
┌───────────▼──────────────────▼──────────────────────────┐
│                 Graph Backend (magellan)                 │
│              Symbol resolution, cross-file refs          │
└──────────────────────────────────────────────────────────┘
```

## Component Deep-Dive

### 1. Agent Surface Commands

#### `splice edit` — Text-Replace with Validation

**File:** `src/cmds/edit.rs`
**Purpose:** Agent-friendly alternative to `Write` tool for validated edits

**Workflow:**
```rust
// 1. Parse CLI args (file, replace-old, replace-new, preview)
// 2. Call text_replace::find_replacement_span()
//    - Read file content
//    - Search for exact match of replace-old
//    - Return byte span or error (ambiguous/not found)
// 3. If preview mode: show diff and exit
// 4. If apply mode: call patch::apply_patch_with_validation()
//    - Validate UTF-8 span boundaries
//    - Apply replacement
//    - Run syntax check (tree-sitter)
//    - Run compiler check (cargo check, python -m py_compile, etc.)
//    - Rollback on validation failure
// 5. Report success with before/after checksums
```

**Key Features:**
- **Exact match only** — No regex, no partial matches (prevents ambiguous edits)
- **Ambiguity detection** — Error if multiple occurrences found
- **Context messages** — Show surrounding lines when not found
- **Preview mode** — `--preview` flag shows unified diff without modifying files
- **Auto-rollback** — Validation failure restores original bytes

**Error Cases:**
- `SPL-E071` — Text not found in file (with context preview)
- `SPL-E072` — Ambiguous match (multiple occurrences)
- `SPL-E042` — Syntax validation failed (auto-rollback)
- `SPL-E043` — Compiler validation failed (auto-rollback)

**Example:**
```bash
# Preview first
splice edit --file src/lib.rs \
  --replace-old "fn old_process() -> i32 { 0 }" \
  --replace-new "fn new_process() -> i32 { 42 }" \
  --preview

# Apply after validation
splice edit --file src/lib.rs \
  --replace-old "fn old_process() -> i32 { 0 }" \
  --replace-new "fn new_process() -> i32 { 42 }"
```

#### `splice suggest` — Intent-Based Scaffolding

**File:** `src/cmds/suggest.rs`
**Purpose:** Cursor-free scaffold generation with grounded type inference

**Workflow:**
```rust
// 1. Parse CLI args (fn-name, desc/d, output)
// 2. Extract keywords from description (simple heuristic: >3 chars)
// 3. Query Magellan DB for relevant symbols
//    - Search by name (fuzzy match on keywords)
//    - Search by kind (function, struct, enum)
//    - Retrieve return types and parameter types
// 4. Build scaffold from inferred types
//    - Function signature with placeholder params
//    - Return type from DB symbols
//    - Import suggestions from resolved types
// 5. Output text or JSON scaffold
```

**Key Features:**
- **Cursor-free** — No line/column needed (unlike `complete`)
- **Type inference** — Query DB for return types of similar functions
- **Import awareness** — Scaffold includes required imports
- **JSON mode** — `--output json` for structured agent consumption

**Query Strategy:**
```sql
-- Find functions with similar names
SELECT name, return_type 
FROM symbols 
WHERE kind = 'function' 
  AND name LIKE '%<keyword>%'
LIMIT 5

-- Find types mentioned in description
SELECT name, kind
FROM symbols
WHERE kind IN ('struct', 'enum')
  AND name LIKE '%<keyword>%'
```

**Example Output:**
```bash
# Text scaffold
$ splice suggest --fn-name "process_data" --desc "Parse JSON and validate"
// Suggested scaffold for process_data
// Inferred from: process_json, validate_schema, parse_data
use serde::Deserialize;
use std::path::Path;

pub fn process_data<T: Deserialize>(input: &Path) -> Result {
    todo!("Implement JSON parsing and validation")
}
```

```bash
# JSON scaffold (for agents)
$ splice suggest --fn-name "handler" --desc "HTTP request handler" --output json
{
  "function_name": "handler",
  "suggested_signature": "pub fn handler(req: Request) -> Response",
  "inferred_from": ["http_handle", "request_handler"],
  "imports": ["http::Request", "http::Response"],
  "keywords": ["handler", "http", "request"]
}
```

### 2. DB Auto-Discovery

**File:** `src/graph/db_discovery.rs`
**Purpose:** Resolve DB path without explicit `--db` flag

**Priority Chain:**
```
1. Explicit --db flag (highest priority)
2. SPLICE_DB environment variable
3. Git root inference: ~/.magellan/<project>/<project>.db
4. Legacy local path: .magellan/<project>.db
5. Error with helpful message listing all attempted paths
```

**Algorithm:**
```rust
pub fn discover_db_path(explicit_path: Option<PathBuf>) -> Result<DbResolution> {
    // 1. Check explicit flag
    if let Some(path) = explicit_path {
        return check_and_return(path, ResolutionSource::ExplicitFlag);
    }

    // 2. Check SPLICE_DB env var
    if let Ok(var_path) = env::var("SPLICE_DB") {
        if !var_path.is_empty() {
            if let Ok(res) = check_and_return(var_path, ResolutionSource::EnvVar) {
                return Ok(res);
            }
        }
    }

    // 3. Git root inference
    if let Ok(git_root) = get_git_root() {
        let project_name = git_root.file_name().and_then(|n| n.to_str())?;
        let inferred_path = PathBuf::from(format!(
            "{}/.magellan/{}/{}.db",
            env::var("HOME").unwrap_or_else(|_| ".".to_string()),
            project_name, project_name
        ));
        if let Ok(res) = check_and_return(inferred_path, ResolutionSource::GitRoot) {
            return Ok(res);
        }
    }

    // 4. Legacy local path
    if let Ok(current_dir) = env::current_dir() {
        let project_name = current_dir.file_name().and_then(|n| n.to_str())?;
        let legacy_path = current_dir.join(format!(".magellan/{}.db", project_name));
        if let Ok(res) = check_and_return(legacy_path, ResolutionSource::LegacyPath) {
            return Ok(res);
        }
    }

    // 5. Error with all attempted paths
    Err(SpliceError::IoContext { ... })
}
```

**Worktree Support:**
- Git worktrees automatically use same git root inference
- Bare repositories supported via `git rev-parse --show-toplevel`
- Error if not in a git repository

### 3. Text Replace Module

**File:** `src/patch/text_replace.rs`
**Purpose:** Find byte spans for exact text matches

**Core Function:**
```rust
pub fn find_replacement_span(
    file_path: &Path,
    search_text: &str,
) -> Result<(usize, usize)>
```

**Algorithm:**
```rust
// 1. Read file content
let content = fs::read_to_string(file_path)?;

// 2. Find all matches
let matches: Vec<_> = content.match_indices(search_text).collect();

// 3. Check for ambiguity
if matches.is_empty() {
    return Err(SpliceError::TextNotFound {
        context: extract_context(&content, search_text),
    });
}

if matches.len() > 1 {
    return Err(SpliceError::AmbiguousMatch {
        count: matches.len(),
        positions: matches.iter().map(|(i, _)| *i).collect(),
    });
}

// 4. Return single match as byte span
let (start, _) = matches[0];
let end = start + search_text.len();
Ok((start, end))
```

**UTF-8 Safety:**
- String matching ensures valid UTF-8 boundaries
- Span returned as byte offsets (not char indices)
- Validation in `apply_patch_with_validation` confirms UTF-8 before write

### 4. Validation Pipeline

**Files:** `src/verify/`, `src/patch/mod.rs`
**Purpose:** Multi-gate validation before/after edits

**Gates:**
1. **Pre-Verification** (before write)
   - File exists and writable
   - Workspace has disk space
   - Graph DB in sync (mtime check)

2. **Post-Verification** (after write, before commit)
   - **Syntax check** — Tree-sitter reparse
   - **Compiler check** — Language-specific compiler
   - **Checksum diff** — SHA-256 before/after

3. **Rollback on Failure**
   - Original bytes preserved in memory
   - Atomic write on validation success
   - Auto-restore on syntax/compiler failure

**Error Codes:**
- `SPL-E041` — Pre-verification failed
- `SPL-E042` — Syntax validation failed (post-edit)
- `SPL-E043` — Compiler validation failed (post-edit)

## Data Flow Examples

### Edit Command Flow

```
User: splice edit --file src/lib.rs --replace-old "fn foo()" --replace-new "fn bar()" --preview

CLI Parser (commands.rs)
  → edit::run() (src/cmds/edit.rs)
    → text_replace::find_replacement_span() (src/patch/text_replace.rs)
      → Read file content
      → Search for "fn foo()"
      → Return byte span (1024, 1032)
    → Preview mode: generate unified diff
    → Print diff and exit (no file modification)
```

```
User: splice edit --file src/lib.rs --replace-old "fn foo()" --replace-new "fn bar()" (apply)

CLI Parser (commands.rs)
  → edit::run() (src/cmds/edit.rs)
    → text_replace::find_replacement_span()
      → Return span (1024, 1032)
    → patch::apply_patch_with_validation() (src/patch/mod.rs)
      → pre_verify_patch() (src/verify/)
        → Check file writable
        → Check graph DB mtime
      → validate_utf8_span()
        → Confirm span boundaries valid UTF-8
      → Replace bytes (in-memory)
      → verify_after_patch() (src/verify/)
        → Tree-sitter reparse (syntax check)
        → Compiler check (cargo check / python -m py_compile)
      → If validation failed: rollback to original bytes
      → If validation passed: atomic write to file
    → Report success with before/after checksums
```

### Suggest Command Flow

```
User: splice suggest --fn-name "handler" --desc "HTTP request handler" --output json

CLI Parser (commands.rs)
  → suggest::run() (src/cmds/suggest.rs)
    → Extract keywords: ["handler", "http", "request"]
    → db_discovery::discover_db_path()
      → Check env var → not set
      → Git root → /home/user/project
      → Inferred DB → ~/.magellan/project/project.db
    → magellan query:
      → SELECT name, return_type FROM symbols WHERE name LIKE '%handler%'
      → Result: [(http_handle, Response), (request_handler, Request)]
    → Build scaffold:
      → Signature: "pub fn handler(req: Request) -> Response"
      → Imports: ["http::Request", "http::Response"]
      → Keywords: ["handler", "http", "request"]
    → Output JSON scaffold
```

## Error Handling

All commands use structured error types with error codes:

```rust
pub enum SpliceError {
    // Edit-specific errors
    TextNotFound { context: String },         // SPL-E071
    AmbiguousMatch { count: usize, positions: Vec<usize> }, // SPL-E072
    
    // Validation errors
    PreVerificationFailed { check: String, reason: String }, // SPL-E041
    ParseValidationFailed { file: String, errors: Vec<String> }, // SPL-E042
    CompilerValidationFailed { file: String, errors: Vec<String> }, // SPL-E043
    
    // DB discovery errors
    IoContext { context: String, source: std::io::Error },
    Magellan(anyhow::Error),  // SPL-E091
    
    // Legacy errors (unchanged)
    SymbolNotFound { ... },
    InvalidSpan { ... },
    // ...
}
```

**Error Envelope Format:**
```json
{
  "error": {
    "code": "SPL-E071",
    "severity": "error",
    "message": "Text not found in file: 'fn old_name()'",
    "hint": "Check the exact text including whitespace and newlines. Context:\n  41:   pub fn new_name() {",
    "context": {
      "file": "src/lib.rs",
      "searched_for": "fn old_name()",
      "similar_matches": ["fn old_name(i32) -> i32", "pub fn old_name()"]
    }
  }
}
```

## Testing Strategy

### Unit Tests

- `db_discovery.rs` — 11 tests covering all resolution paths
- `text_replace.rs` — Tests for exact match, ambiguous match, not found
- `edit.rs` — Preview mode, apply mode, error paths
- `suggest.rs` — Keyword extraction, scaffold generation

### Integration Tests

- `tests/cli_edit.rs` — Full edit workflow with validation
- `tests/cli_suggest.rs` — Suggest with real Magellan DB
- `tests/db_discovery.rs` — Auto-discovery in git repos, worktrees

### Verification Tests

- `tests/validate_edit.rs` — Syntax/compiler validation gates
- `tests/rollback.rs` — Auto-rollback on validation failure

## Migration from v2.x

### For CLI Users

**No changes required** — Existing commands unchanged:
```bash
splice rename --symbol foo --to bar --path src/lib.rs --db .magellan/splice.db
splice complete --file src/lib.rs --line 27 --column 8 --db .magellan/splice.db
```

**Optional: Drop `--db` flag** (auto-discovery):
```bash
# Before v3.0
splice status --db .magellan/splice.db

# v3.0+ (DB auto-discovered from git root)
splice status
```

### For Agent Developers

**Before v3.0** — Used `Write` tool or manual edits:
```python
# Agent workflow (old)
file_content = read_file("src/lib.rs")
modified = file_content.replace("old_func()", "new_func()")
write_file("src/lib.rs", modified)
# No validation, no rollback
```

**v3.0+** — Use `splice edit` for validated edits:
```python
# Agent workflow (new)
result = run_command([
    "splice", "edit",
    "--file", "src/lib.rs",
    "--replace-old", "old_func()",
    "--replace-new", "new_func()",
    "--preview"  # Preview first
])

# Check diff, then apply
result = run_command([
    "splice", "edit",
    "--file", "src/lib.rs",
    "--replace-old", "old_func()",
    "--replace-new", "new_func()"
])
# Auto-validated, auto-rollback on error
```

**Use `splice suggest` for scaffolding:**
```python
# Get scaffold with imports
scaffold = run_command([
    "splice", "suggest",
    "--fn-name", "process_data",
    "--desc", "Parse JSON and validate schema",
    "--output", "json"
])
# Returns: {signature, imports, inferred_from, keywords}
```

## Performance Characteristics

| Operation | Time Complexity | Bottleneck |
|-----------|----------------|------------|
| `db_discovery` | O(1) | File system checks |
| `find_replacement_span` | O(n) | File read + string search |
| `apply_patch_with_validation` | O(n) + compiler | Syntax check + compiler |
| `suggest` | O(q) | Magellan query (q = query complexity) |
| Auto-rollback | O(1) | Atomic file write |

**Typical Latencies:**
- DB discovery: ~5ms (4 path checks)
- Text replace (1MB file): ~10ms
- Validation (Rust): ~500ms (cargo check)
- Validation (Python): ~50ms (py_compile)

## Security Considerations

1. **Path traversal** — All file paths resolved to workspace root
2. **Command injection** — No shell execution, direct exec
3. **Arbitrary write** — Span validation prevents write outside target
4. **DoS protection** — File size limits in validation gates
5. **Rollback safety** — Atomic writes prevent corruption

## Future Work

### Potential Enhancements

1. **Regex-based edit** — `splice edit --regex` (currently exact-match only)
2. **Semantic edit** — Use Magellan refs for rename-aware replace
3. **Multi-file edit** — Batch edits across files with transaction
4. **Suggest improvements** — Better keyword extraction, LLM-augmented scaffolds
5. **Incremental validation** — Cache compiler artifacts for faster re-validation

### Known Limitations

1. **Suggest accuracy** — Simple keyword extraction, no semantic understanding
2. **Edit granularity** — No regex, no partial matches (by design for safety)
3. **Compiler check** — Requires build tools installed (cargo, python, etc.)
4. **DB discovery** — Requires git repo (falls back to legacy path otherwise)

## References

- **[CHANGELOG.md](../CHANGELOG.md)** — Version history and breaking changes
- **[docs/API.md](API.md)** — Library API reference
- **[CLAUDE.md](../CLAUDE.md)** — Project development rules
- **[src/graph/db_discovery.rs](../src/graph/db_discovery.rs)** — DB discovery implementation
- **[src/cmds/edit.rs](../src/cmds/edit.rs)** — Edit command implementation
- **[src/cmds/suggest.rs](../src/cmds/suggest.rs)** — Suggest command implementation
