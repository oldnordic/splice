# Feature Landscape: Magellan Query Command Delegation

**Project:** Splice v2.2.2 - Magellan Integration
**Domain:** Unified CLI interface - Splice delegates query commands to Magellan
**Researched:** 2026-01-24
**Overall Confidence:** HIGH

---

## Executive Summary

Splice v2.2.2 adds Magellan query command delegation, creating a **unified CLI interface** where LLMs and developers can use a single tool (`splice`) for both code discovery (via Magellan) and code modification (via Splice's span-safe operations). The integration follows the specification in `docs/CLI_PATTERNS.md` and leverages existing Magellan v0.5.3 integration infrastructure.

**Key insight:** This is primarily a **CLI delegation layer** - Splice forwards query commands to Magellan's existing APIs, then normalizes output to match Splice's JSON schema conventions. No new parsing or indexing logic is required.

---

## Table Stakes

Features required for delegation to work correctly. Missing = incomplete integration.

### Command Delegation

**Core requirement: Forward query commands to Magellan**

| Command | Delegates To | Required Flags | Purpose | Complexity |
|---------|--------------|----------------|---------|------------|
| `status` | Magellan database stats | `--db` | Show database statistics | LOW |
| `query` | Magellan symbol query | `--db`, `--file` | List symbols in a file | LOW |
| `find` | Magellan symbol lookup | `--db`, `--name` | Find symbol by name | MEDIUM |
| `refs` | Magellan call graph | `--db`, `--name`, `--path` | Show callers/callees | MEDIUM |
| `files` | Magellan file listing | `--db` | List indexed files | LOW |
| `export` | Magellan data export | `--db`, `--format` | Export graph data | MEDIUM |

**Why essential:** LLMs need discovery capabilities before editing. Without these commands, LLMs must use separate `magellan` CLI, breaking the unified tool workflow.

**Dependencies:**
- Existing `src/ingest/magellan.rs` - MagellanIntegration wrapper
- Existing `src/graph/magellan_integration.rs` - CodeGraph access
- Magellan 0.5.3 dependency already in Cargo.toml

### Flag Alignment

**Core requirement: Match Magellan's CLI flag names**

| Flag | Values | Purpose | Complexity |
|------|--------|---------|------------|
| `--output` | `human`, `json`, `pretty` | Output format selection | LOW |
| `--db` | `<PATH>` | Database path | LOW |
| `--format` | `json`, `jsonl`, `csv`, `scip` | Export format | LOW |

**Why essential:** CLI consistency prevents user confusion. Users familiar with Magellan should find the same flags in Splice.

### JSON Schema Alignment

**Core requirement: Match Magellan's response structure**

Per `docs/CLI_PATTERNS.md`, Magellan responses include:

```json
{
  "schema_version": "1.0",
  "execution_id": "uuid-v4",
  "tool": "magellan",
  "timestamp": "ISO-8601",
  "data": { ... }
}
```

**Splice must:**
1. Use same top-level structure
2. Include `schema_version`, `execution_id`, `timestamp`
3. Set `tool` to `"splice"` (delegation is transparent to user)
4. Wrap Magellan data in the `data` field

**Why essential:** LLMs consuming Splice output expect consistent structure. Breaking schema breaks LLM parsers.

**Complexity:** LOW - add wrapper function to normalize Magellan responses

---

## Differentiators

Features that make Splice's unified interface valuable beyond simple command forwarding.

### Single-Tool Workflow

**Value Proposition:** LLMs can discover and modify using one tool

**Before (two tools):**
```bash
# Step 1: Find symbol
magellan find --db codegraph.db --name "process_data"

# Step 2: Edit symbol
splice patch --file src/main.rs --symbol "process_data" --with replacement.txt
```

**After (one tool):**
```bash
# Step 1: Find symbol
splice find --db codegraph.db --name "process_data"

# Step 2: Edit symbol (same tool)
splice patch --file src/main.rs --symbol "process_data" --with replacement.txt
```

**Complexity:** LOW - no changes needed, just unified CLI

**LLM benefit:** Simpler prompts, fewer tool switches, consistent JSON schema

### Consistent Error Codes

**Value Proposition:** Splice's SPL-E### error codes applied to Magellan errors

**Example:**
```json
{
  "error": {
    "code": "SPL-REF-001",
    "message": "Symbol not found: 'process_data'",
    "hint": "Check the symbol name and path, or use --ambiguous to see all candidates"
  }
}
```

**Why valuable:** LLMs get structured, actionable errors from both query and edit operations. Error handling logic is consistent.

**Complexity:** LOW - map Magellan error types to Splice error codes

### Optional Context Enhancement

**Value Proposition:** Splice can add rich span context to Magellan results

**Magellan provides:**
- Symbol name, kind, file path, byte offsets

**Splice can add:**
- Context lines (before/after)
- Semantic kind detection
- Relationships (callers/callees)
- Checksums

**Why valuable:** LLMs get richer data without extra tool calls. One `find` command returns everything needed for editing.

**Complexity:** MEDIUM - requires combining Magellan symbol query with Splice's rich span infrastructure

---

## Anti-Features

Features to explicitly NOT build. Common mistakes in this domain.

### DO NOT: Reimplement Magellan's Parsers

**Why avoid:**
- Magellan has 7 production-ready parsers
- Duplicates 2000+ lines of code
- Maintenance burden

**Instead:** Delegate all indexing to Magellan. Splice only uses Magellan's CodeGraph API.

### DO NOT: Create Separate Splice Database Schema

**Why avoid:**
- Dual databases cause sync issues
- Schema drift between Splice and Magellan
- Disk space waste

**Instead:** Use Magellan's database directly. Splice reads from the same `codegraph.db` file.

### DO NOT: Modify Magellan's Symbol ID Format

**Why avoid:**
- Breaks compatibility with existing Mag databases
- LLMs trained on Magellan output expect specific format
- Unnecessary complexity

**Instead:** Pass through Magellan's 16-character symbol IDs unchanged.

### DO NOT: Add Authentication/Authorization

**Why avoid:**
- CLI tools are local-only
- Adds unnecessary complexity
- Out of scope for code refactoring

**Instead:** Rely on filesystem permissions for database access control.

### DO NOT: Build Real-time Indexing

**Why avoid:**
- `magellan watch` already handles this
- Users can run indexer separately
- Splice is a refactoring tool, not an indexer

**Instead:** Document that users should run `magellan watch` for real-time updates.

---

## Per-Command Breakdown

### status

**Purpose:** Show database statistics

**Arguments:**
```bash
splice status --db <FILE> [--output <FORMAT>]
```

**Magellan API:**
```rust
// Database statistics
magellan::CodeGraph::get_stats() -> Result<DatabaseStats>
```

**Output (human):**
```
files: 42
symbols: 1337
references: 256
```

**Output (json):**
```json
{
  "schema_version": "2.2.2",
  "execution_id": "uuid-v4",
  "tool": "splice",
  "timestamp": "2026-01-24T12:00:00Z",
  "data": {
    "files": 42,
    "symbols": 1337,
    "references": 256
  }
}
```

**Complexity:** LOW

**Dependencies:** None (delegates to Magellan)

---

### query

**Purpose:** List symbols in a file

**Arguments:**
```bash
splice query --db <FILE> --file <PATH> [--kind <KIND>] [--with-context] [--with-callers] [--with-callees] [--with-semantics] [--with-checksums] [--context-lines <N>]
```

**Magellan API:**
```rust
// Get symbols for file
magellan::CodeGraph::get_symbols_in_file(&str) -> Result<Vec<SymbolQueryResult>>
```

**Splice additions (when flags present):**
- `--with-context`: Add `context` field via `src/context.rs`
- `--with-callers`: Query CALLER edges via `src/relationships/mod.rs`
- `--with-callees`: Query CALLS edges via `src/relationships/mod.rs`
- `--with-semantics`: Add semantic kind via `src/ingest/semantic_kind.rs`
- `--with-checksums`: Add SHA-256 via `src/checksum.rs`

**Output (json):**
```json
{
  "schema_version": "2.2.2",
  "execution_id": "uuid-v4",
  "tool": "splice",
  "timestamp": "2026-01-24T12:00:00Z",
  "data": {
    "file_path": "src/main.rs",
    "symbols": [
      {
        "symbol_id": "0123456789abcdef",
        "name": "process_data",
        "kind": "function",
        "byte_start": 100,
        "byte_end": 200,
        "line_start": 5,
        "line_end": 10
      }
    ]
  }
}
```

**Complexity:** LOW (base), MEDIUM (with Splice enhancements)

---

### find

**Purpose:** Find a symbol by name

**Arguments:**
```bash
splice find --db <FILE> (--name <NAME> | --symbol-id <ID> | --ambiguous <NAME>) [--path <PATH>] [--first] [--output <FORMAT>]
```

**Magellan API:**
```rust
// Find by name
magellan::CodeGraph::find_symbol(&str) -> Result<Vec<SymbolQueryResult>>

// Find by ID
magellan::CodeGraph::get_symbol_by_id(&str) -> Result<SymbolQueryResult>

// Find ambiguous
magellan::CodeGraph::find_all_symbols_named(&str) -> Result<Vec<SymbolQueryResult>>
```

**Key behavior:** `--name` returns first match by default. Use `--ambiguous` to show all candidates for disambiguation.

**Output (json):**
```json
{
  "schema_version": "2.2.2",
  "execution_id": "uuid-v4",
  "tool": "splice",
  "timestamp": "2026-01-24T12:00:00Z",
  "data": {
    "matches": [
      {
        "symbol_id": "0123456789abcdef",
        "name": "process_data",
        "kind": "function",
        "file_path": "src/main.rs",
        "byte_start": 100,
        "byte_end": 200,
        "line_start": 5,
        "line_end": 10,
        "display_fqn": "myapp::process_data",
        "canonical_fqn": "crate::myapp::process_data"
      }
    ]
  }
}
```

**Complexity:** MEDIUM (handling ambiguity and FQNs)

---

### refs

**Purpose:** Show callers or callees for a symbol

**Arguments:**
```bash
splice refs --db <FILE> --name <NAME> --path <PATH> [--symbol-id <ID>] [--direction <in|out>] [--output <FORMAT>]
```

**Magellan API:**
```rust
// Get callers (incoming calls)
magellan::CodeGraph::callers_of_symbol(i64) -> Result<Vec<CallFact>>

// Get callees (outgoing calls)
magellan::CodeGraph::callees_of_symbol(i64) -> Result<Vec<CallFact>>
```

**Key behavior:**
- `--direction in` (default): Show functions that call this symbol
- `--direction out`: Show functions called by this symbol

**Output (json):**
```json
{
  "schema_version": "2.2.2",
  "execution_id": "uuid-v4",
  "tool": "splice",
  "timestamp": "2026-01-24T12:00:00Z",
  "data": {
    "symbol": {
      "symbol_id": "0123456789abcdef",
      "name": "process_data",
      "file_path": "src/main.rs"
    },
    "direction": "in",
    "references": [
      {
        "file_path": "src/handler.rs",
        "symbol_id": "fedcba9876543210",
        "name": "handle_request",
        "line_start": 15,
        "line_end": 15
      }
    ]
  }
}
```

**Complexity:** MEDIUM

---

### files

**Purpose:** List indexed files

**Arguments:**
```bash
splice files --db <FILE> [--symbols] [--output <FORMAT>]
```

**Magellan API:**
```rust
// List all files
magellan::CodeGraph::get_all_files() -> Result<Vec<String>>

// Get symbol count per file
magellan::CodeGraph::count_symbols_in_file(&str) -> Result<usize>
```

**Key behavior:**
- Default: List file paths only
- `--symbols`: Include symbol count per file

**Output (human):**
```
src/main.rs (12 symbols)
src/handler.rs (8 symbols)
src/lib.rs (25 symbols)
```

**Output (json):**
```json
{
  "schema_version": "2.2.2",
  "execution_id": "uuid-v4",
  "tool": "splice",
  "timestamp": "2026-01-24T12:00:00Z",
  "data": {
    "files": [
      {
        "file_path": "src/main.rs",
        "symbol_count": 12
      }
    ]
  }
}
```

**Complexity:** LOW

---

### export

**Purpose:** Export graph data in various formats

**Arguments:**
```bash
splice export --db <FILE> [--format json|jsonl|csv|scip] [--output <PATH>] [--minify] [--no-symbols] [--no-references] [--no-calls] [--include-collisions] [--collisions-field <FIELD>]
```

**Magellan API:**
```rust
// Export to various formats
magellan::CodeGraph::export(ExportConfig) -> Result<String>
```

**Format options:**
- `json`: Compact JSON array (default)
- `jsonl`: JSON Lines (one record per line)
- `csv`: Comma-separated values
- `scip`: Source Code Intelligence Protocol format

**Output destination:**
- Default: stdout
- `--output <PATH>`: Write to file

**Complexity:** MEDIUM (format conversion)

---

## Feature Dependencies

```
[Magellan Integration] (existing)
    └─> [Command Delegation Layer] (NEW)
        ├─> [status] ──> [JSON Wrapper]
        ├─> [query] ──> [JSON Wrapper + Optional Enhancements]
        ├─> [find] ──> [Ambiguity Handler + JSON Wrapper]
        ├─> [refs] ──> [Call Graph Query + JSON Wrapper]
        ├─> [files] ──> [JSON Wrapper]
        └─> [export] ──> [Format Converter]

[Existing Splice Features] (optional integration)
    ├─> [Context Module] ──> Enhances query output
    ├─> [Relationships Module] ──> Enhances query/refs output
    ├─> [Semantic Kind] ──> Enhances query/find output
    └─> [Checksums] ──> Enhances query output
```

### Dependency Chain for MVP

1. **Phase 1: Core Delegation** (Foundation)
   - Add CLI commands for status, query, find, refs, files
   - Implement JSON wrapper for Magellan responses
   - Add `--output` flag support

2. **Phase 2: Optional Enhancements** (Differentiators)
   - Add `--with-context` flag (delegates to context.rs)
   - Add `--with-callers`/`--with-callees` flags (delegates to relationships.rs)
   - Add `--with-semantics` flag (delegates to semantic_kind.rs)

3. **Phase 3: Export Format** (Completeness)
   - Implement export command with format options
   - Add SCIP format support for LSP integration

---

## Implementation Complexity by Feature

| Feature | Complexity | Dependencies | Risk Level |
|---------|------------|--------------|------------|
| status | LOW | Magellan API | LOW |
| query (base) | LOW | Magellan API | LOW |
| query (with enhancements) | MEDIUM | Context, Relationships, SemanticKind modules | MEDIUM |
| find | MEDIUM | Ambiguity handling, FQN lookup | MEDIUM |
| refs | MEDIUM | Magellan call graph API | MEDIUM |
| files | LOW | Magellan API | LOW |
| export | MEDIUM | Format conversion | MEDIUM |
| JSON wrapper | LOW | serde | LOW |
| Error code mapping | LOW | error_codes.rs | LOW |

---

## Cross-Tool Alignment Impact

### Unified Schema Benefits

**Per `docs/LLM_TOOL_ECOSYSTEM_ALIGNMENT.md`:**

1. **Consistent field names**
   - Magellan uses `start_line`/`start_col`
   - Splice uses `line_start`/`col_start`
   - **Delegation layer must translate** to Splice conventions

2. **Execution ID format**
   - Both use UUID v4 format
   - Splice uses `operation_id`, Magellan uses `execution_id`
   - **Delegation layer must standardize on `execution_id`**

3. **Symbol ID format**
   - Magellan uses 16-character hex strings
   - **Pass through unchanged** (backward compatibility)

**Field name mapping required:**

| Magellan Field | Splice Field | Action |
|----------------|--------------|--------|
| `start_line` | `line_start` | Translate |
| `start_col` | `col_start` | Translate |
| `end_line` | `line_end` | Translate |
| `end_col` | `col_end` | Translate |
| `symbol_id` | `symbol_id` | Pass through |
| `display_fqn` | `display_fqn` | Pass through |
| `canonical_fqn` | `canonical_fqn` | Pass through |

---

## MVP Recommendation

For **Splice v2.2.2 MVP**, prioritize in this order:

### Must-Have (Blocker if missing):
1. **status** — Database verification
2. **query** — Basic symbol listing in files
3. **find** — Symbol lookup by name
4. **refs** — Caller/callee queries
5. **files** — File listing

### Nice-to-Have (Stretch goals):
6. **export** — Data export in multiple formats
7. **--with-context** flag — Context enhancement
8. **--with-callers**/`--with-callees` flags — Relationship enhancement

### Defer to v2.3+:
- **Real-time indexing delegation** — Users run `magellan watch` separately
- **Full LSP integration** — Belongs in llmfilewrite, not Splice
- **Custom Magellan commands** — Use Magellan CLI directly for advanced operations

**Rationale:** Core query commands enable the single-tool workflow. Enhancement flags add value but are not blockers for delegation.

---

## Open Questions & Research Flags

### Verified (No Research Needed):

- CLI command specifications from `docs/CLI_PATTERNS.md`
- Magellan API signatures from `src/graph/magellan_integration.rs`
- Existing MagellanIntegration wrapper infrastructure
- JSON schema alignment requirements from `docs/LLM_TOOL_ECOSYSTEM_ALIGNMENT.md`

### Research Flags (LOW confidence):

1. **Magellan export format specifications**
   - **Flag:** Exact JSON/JSONL/CSV/SCIP output formats
   - **Research needed:** "What fields are included in each export format?"
   - **Confidence:** LOW (WebSearch blocked, local docs incomplete)
   - **Action:** Check Magellan source code or test with actual magellan CLI

2. **Ambiguity resolution behavior**
   - **Flag:** How Magellan handles `--ambiguous` flag internally
   - **Research needed:** "Does Magellan return all candidates sorted by relevance?"
   - **Confidence:** MEDIUM (can infer from API)
   - **Action:** Write integration test to verify behavior

3. **Call graph query performance**
   - **Flag:** Latency for refs command on large codebases
   - **Research needed:** "What's the query time for callers in 10K+ file codebase?"
   - **Confidence:** LOW (no benchmarks yet)
   - **Action:** Performance testing during implementation

---

## Sources

### HIGH Confidence (Verified from source code and documentation)

- **Splice CLI module:** `/home/feanor/Projects/splice/src/cli/mod.rs` - Existing command definitions
- **Magellan integration:** `/home/feanor/Projects/splice/src/graph/magellan_integration.rs` - Magellan API wrapper
- **CLI Patterns:** `/home/feanor/Projects/splice/docs/CLI_PATTERNS.md` - Complete command specifications
- **Ecosystem alignment:** `/home/feanor/Projects/splice/docs/LLM_TOOL_ECOSYSTEM_ALIGNMENT.md` - Schema requirements
- **Project context:** `/home/feanor/Projects/splice/.planning/PROJECT.md` - v2.2.2 milestone definition

### MEDIUM Confidence (Inferred from API structure)

- Magellan CodeGraph API behavior (from magellan_integration.rs wrapper)
- Symbol ID format (16-character hex string)
- FQN fields (display_fqn, canonical_fqn)

### LOW Confidence (WebSearch blocked, needs verification)

- Export format exact specifications
- Call graph query performance characteristics
- Ambiguity resolution sorting behavior

---

*Document created: 2026-01-24*
*Status: Ready for roadmap creation*
*Confidence: HIGH (command specifications), MEDIUM (API behavior), LOW (export formats)*
