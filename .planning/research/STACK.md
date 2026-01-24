# Technology Stack

**Project:** Splice v2.2.2 - Magellan Integration
**Researched:** 2026-01-24
**Mode:** Ecosystem (Stack additions for query delegation)

---

## Executive Summary

Splice already has **all dependencies** needed for Magellan query delegation. The Magellan crate (v0.5.3) provides the complete query API. This milestone requires **no new dependencies** - only code structure additions:

1. New CLI subcommands (`status`, `find`, `refs`, `files`)
2. Response type wrappers matching Magellan's JSON format
3. Delegation functions in `src/graph/magellan_integration.rs`
4. CLI flags (`--output`, `--db`) already implemented globally

---

## Existing Stack (Splice v2.0.0)

### Core Dependencies (from Cargo.toml)

| Technology | Version | Purpose | Status |
|------------|---------|---------|--------|
| `magellan` | 0.5.3 | Code indexing, label queries, status, find, refs, files | **Installed** |
| `sqlitegraph` | 1.0 | Graph backend (re-exported from magellan) | **Installed** |
| `rusqlite` | 0.31 | Direct SQLite access | **Installed** |
| `clap` | 4.5 | CLI argument parsing | **Installed** |
| `serde` | 1.0 | JSON serialization | **Installed** |
| `serde_json` | 1.0 | JSON output | **Installed** |
| `chrono` | 0.4 | Timestamp generation | **Installed** |
| `sha2` | 0.10 | Stable ID generation (span_id, symbol_id) | **Installed** |
| `uuid` | 1.10 | Execution ID generation | **Installed** |
| `tree-sitter` | 0.22 | AST parsing (7 languages) | **Installed** |

### Language Support (via tree-sitter)

| Language | Parser Version | File Extensions |
|----------|---------------|-----------------|
| Rust | tree-sitter-rust 0.21 | .rs |
| Python | tree-sitter-python 0.21 | .py |
| C | tree-sitter-c 0.21 | .c, .h |
| C++ | tree-sitter-cpp 0.21 | .cpp, .hpp, .cc, .cxx |
| Java | tree-sitter-java 0.21 | .java |
| JavaScript | tree-sitter-javascript 0.21 | .js, .mjs, .cjs |
| TypeScript | tree-sitter-typescript 0.21 | .ts, .tsx |

---

## Stack Additions (Magellan Delegation)

### No New Dependencies Required

The existing `magellan = { version = "0.5.3", features = ["native-v2"] }` dependency provides:

```rust
// From magellan 0.5.3 API
use magellan::CodeGraph;

impl CodeGraph {
    // Database status
    pub fn get_stats(&self) -> Result<GraphStats>

    // Query by file
    pub fn query_file(&self, file_path: &str) -> Result<Vec<SymbolInfo>>

    // Find by name or symbol_id
    pub fn find_symbol(&self, name: &str) -> Result<Vec<SymbolInfo>>
    pub fn find_by_symbol_id(&self, symbol_id: &str) -> Result<SymbolInfo>

    // References/call graph
    pub fn get_references(&self, symbol_id: &str, direction: Direction) -> Result<Vec<Reference>>
    pub fn get_callers(&self, symbol_id: &str) -> Result<Vec<CallInfo>>
    pub fn get_callees(&self, symbol_id: &str) -> Result<Vec<CallInfo>>

    // File listing
    pub fn list_files(&self) -> Result<Vec<String>>
}
```

### New Code Structure (Not Dependencies)

| Addition | Location | Purpose |
|----------|----------|---------|
| CLI commands | `src/cli/mod.rs` | Add `Status`, `Find`, `Refs`, `Files` variants to `Commands` enum |
| Response types | `src/output.rs` | Add `StatusResponse`, `FindResponse`, `RefsResponse`, `FilesResponse` |
| Delegation functions | `src/graph/magellan_integration.rs` | Wrapper functions calling Magellan API |
| Execution handlers | `src/main.rs` | Match arms for new commands |

---

## Delegation Strategy

### Pattern: Splice as Thin Adapter

```
User Command
    |
    v
[main.rs: execute_*()]  -- Parse args, call delegation
    |
    v
[magellan_integration.rs]  -- Call Magellan API, convert types
    |
    v
[magellan crate]  -- Execute query on graph
    |
    v
[magellan_integration.rs]  -- Convert to Splice response types
    |
    v
[output.rs]  -- Serialize to JsonResponse wrapper
    |
    v
User Output (JSON or human)
```

### Example: `status` Command

```rust
// src/cli/mod.rs
pub enum Commands {
    // ... existing commands ...

    /// Show database statistics
    Status {
        /// Path to the Magellan database
        #[arg(short, long)]
        db: std::path::PathBuf,

        /// Output format (human, json, pretty)
        #[arg(long, value_name = "FORMAT")]
        output: Option<OutputFormat>,
    },
}

// src/graph/magellan_integration.rs
impl MagellanIntegration {
    pub fn get_status(&self) -> Result<StatusData> {
        let stats = self.inner.get_stats()?;
        Ok(StatusData {
            files: stats.file_count,
            symbols: stats.symbol_count,
            references: stats.reference_count,
            calls: stats.call_count,
            code_chunks: stats.chunk_count,
        })
    }
}

// src/main.rs
fn execute_status(db: &Path, output_format: OutputFormat) -> Result<CliSuccessPayload> {
    let integration = MagellanIntegration::open(db)?;
    let status_data = integration.get_status()?;

    let response = JsonResponse::new(
        StatusResponse::from(status_data),
        &Uuid::new_v4().to_string(),
    );

    match output_format {
        OutputFormat::Json => emit_json_response(&response)?,
        OutputFormat::Pretty => emit_pretty_json_response(&response)?,
        OutputFormat::Human => print_human_status(&response.data)?,
    }

    Ok(CliSuccessPayload::message_only("Status retrieved".to_string()))
}
```

### Example: `find` Command

```rust
// src/cli/mod.rs
Find {
    /// Path to the Magellan database
    #[arg(short, long)]
    db: std::path::PathBuf,

    /// Symbol name to find
    #[arg(short, long)]
    name: Option<String>,

    /// Stable symbol ID (16-char hex)
    #[arg(long)]
    symbol_id: Option<String>,

    /// Limit search to file path
    #[arg(long)]
    path: Option<std::path::PathBuf>,

    /// Output format
    #[arg(long, value_name = "FORMAT")]
    output: Option<OutputFormat>,
},

// Delegation function
impl MagellanIntegration {
    pub fn find_symbol(
        &self,
        name: Option<&str>,
        symbol_id: Option<&str>,
        path_filter: Option<&Path>,
    ) -> Result<Vec<SymbolMatch>> {
        let results = if let Some(sid) = symbol_id {
            vec![self.inner.find_by_symbol_id(sid)?]
        } else if let Some(n) = name {
            let mut results = self.inner.find_symbol(n)?;
            if let Some(p) = path_filter {
                let p_str = p.to_str().unwrap();
                results.retain(|r| r.file_path == p_str);
            }
            results
        } else {
            return Err(SpliceError::Other("Either --name or --symbol-id required".into()));
        };

        Ok(results.into_iter().map(SymbolMatch::from).collect())
    }
}
```

---

## CLI Alignment

### Global Flags (Already Present)

| Flag | Current Implementation | Notes |
|------|------------------------|-------|
| `--json` | Global in `Cli` struct | Enables JSON output |
| `--db` | Per-command in `Query`, `Get` | Move to global or add to new commands |

### New Flag: `--output`

Magellan uses `--output human|json|pretty`. Splice currently uses global `--json` bool.

**Approach:** Add `--output` as an optional override. If `--output` is specified, it takes precedence. Otherwise, `--json` flag controls behavior.

```rust
// src/cli/mod.rs
#[derive(clap::ValueEnum, Debug, Clone, Copy, PartialEq, Eq)]
pub enum OutputFormat {
    /// Human-readable text output
    Human,
    /// Compact JSON for programmatic consumption
    Json,
    /// Formatted JSON with indentation
    Pretty,
}

// In each command struct that needs it:
#[arg(long, value_name = "FORMAT")]
output: Option<OutputFormat>,
```

---

## Data Format Alignment

### Execution ID Format

**Current Splice:** UUID v4 (e.g., "a1b2c3d4-e5f6-1234-5678-90abcdef1234")

**Magellan convention:** `{timestamp_hex}-{pid_hex}` (e.g., "67abc123d4567-123a")

**Decision:** Splice should keep its UUID format for Splice operations (patch, delete, etc.) but **adopt Magellan's format for delegated query commands**. This provides:

1. Traceability across the integrated toolset
2. Correlation with Magellan's own execution logs
3. Compliance with `docs/JSON_EXPORT_FORMAT.md` specification

```rust
// New helper in src/output.rs
pub fn generate_magellan_execution_id() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    use std::process;
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs();
    let pid = process::id();
    format!("{:x}-{:x}", timestamp, pid)
}

// Use for delegated commands
let exec_id = generate_magellan_execution_id();
let response = JsonResponse::new(data, &exec_id);
```

### Symbol ID Format

**Specification:** 16 hex characters (64 bits) from SHA-256 hash

```
symbol_id = SHA256(language + ":" + fqn + ":" + span_id)[0:16]
```

**Current Splice:** Splice uses `span_id` (16 hex chars) generated by `generate_span_id()` in `src/output.rs:430-445`. This matches the spec.

**Action:** No change needed. Magellan provides `symbol_id` in query results.

### FQN (Fully Qualified Name)

**Specification:**
- `canonical_fqn`: `{crate}::{file_path}::{kind} {symbol_name}`
- `display_fqn`: `{crate}::{module_chain}::{symbol_name}`

**Current Splice:** Splice does not currently compute FQNs.

**Action:** Add FQN computation module or use Magellan's FQN from query results. For delegated commands, use Magellan's FQN fields directly.

---

## What NOT to Add

### Do NOT Add

| Category | What | Why |
|----------|------|-----|
| Dependencies | Any new crates | Magellan 0.5.3 already provides all needed APIs |
| Database access | Direct rusqlite for queries | Use Magellan's typed API instead |
| FQN computation | New FQN builder module | Magellan provides FQNs in query results |
| CLI flag library | New CLI parsing crate | clap 4.5 is already installed and sufficient |
| JSON library | Alternative JSON serializers | serde_json is already used throughout |
| SHA-256 library | Alternative hash library | sha2 0.10 already installed for span_id generation |

### Do NOT Re-implement

| Function | Use Instead |
|----------|-------------|
| `find_by_name` | `magellan::CodeGraph::find_symbol` |
| `get_references` | `magellan::CodeGraph::get_references` |
| `get_callers/callees` | `magellan::CodeGraph::get_callers/callees` |
| `list_files` | `magellan::CodeGraph::list_files` |
| `get_stats` | `magellan::CodeGraph::get_stats` |

---

## Integration Points

### 1. `src/graph/magellan_integration.rs` (Extend)

**Current state:** Provides `query_by_labels`, `get_code_chunk`

**Add:** Delegation wrappers for status, find, refs, files

```rust
impl MagellanIntegration {
    // NEW: Get database status
    pub fn get_status(&self) -> Result<StatusData>;

    // NEW: Find by name or symbol_id
    pub fn find(&self, name: Option<&str>, symbol_id: Option<&str>, path: Option<&Path>) -> Result<Vec<SymbolMatch>>;

    // NEW: Get references for a symbol
    pub fn get_refs(&self, symbol_id: &str, direction: Direction) -> Result<Vec<ReferenceMatch>>;

    // NEW: List indexed files
    pub fn list_files(&self) -> Result<Vec<FileInfo>>;
}
```

### 2. `src/cli/mod.rs` (Extend)

**Current state:** Has `Query`, `Get`, `Log`, `Explain`, `Search` commands

**Add:** `Status`, `Find`, `Refs`, `Files` commands

### 3. `src/output.rs` (Extend)

**Current state:** Has `JsonResponse`, `OperationResult`, `SpanResult`, `SymbolMatch`

**Add:** Response data types matching Magellan spec

```rust
// Already have JsonResponse wrapper - just add data types:
pub struct StatusResponse { pub files: usize, pub symbols: usize, ... }
pub struct FindResponse { pub matches: Vec<SymbolMatch>, ... }
pub struct RefsResponse { pub references: Vec<ReferenceMatch>, ... }
pub struct FilesResponse { pub files: Vec<String>, pub symbol_counts: HashMap<String, usize> }
```

### 4. `src/main.rs` (Extend)

**Current state:** Command matching in main() function

**Add:** Match arms for new commands calling delegation functions

---

## Version Compatibility

| Component | Version | Compatibility |
|-----------|---------|---------------|
| Magellan crate | 0.5.3 | Full support for all query APIs |
| SQLiteGraph | 1.0 | Compatible (same backend) |
| rusqlite | 0.31 | Compatible (same version as Magellan) |
| tree-sitter | 0.22 | Compatible (same versions as Magellan) |

---

## Testing Strategy

### Unit Tests (No new dependencies)

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_status() {
        let temp = TempDir::new().unwrap();
        let db = temp.path().join("test.db");
        let integration = MagellanIntegration::open(&db).unwrap();

        let status = integration.get_status().unwrap();
        assert_eq!(status.files, 0);
        assert_eq!(status.symbols, 0);
    }

    #[test]
    fn test_find_by_name() {
        // Index a test file
        // Query by name
        // Verify results
    }

    #[test]
    fn test_get_refs() {
        // Index test file with calls
        // Get references
        // Verify caller/callee relationships
    }
}
```

### Integration Tests

```bash
# Test splice status mirrors magellan status
splice status --db test.db --output json > /tmp/splice-out.json
magellan status --db test.db --output json > /tmp/magellan-out.json
# Compare structure

# Test splice find mirrors magellan find
splice find --db test.db --name main --output json
magellan find --db test.db --name main --output json
# Compare results
```

---

## Migration Path

### Phase 1: CLI Structure (1-2 days)
- Add `Status`, `Find`, `Refs`, `Files` to `Commands` enum
- Add `OutputFormat` enum
- Add `--output` flag to new commands

### Phase 2: Response Types (1 day)
- Add `StatusResponse`, `FindResponse`, `RefsResponse`, `FilesResponse` to `src/output.rs`
- Verify JSON serialization matches Magellan spec

### Phase 3: Delegation Functions (2-3 days)
- Extend `MagellanIntegration` with status, find, refs, files methods
- Add type conversions from Magellan types to Splice response types

### Phase 4: Execution Handlers (1-2 days)
- Add `execute_status`, `execute_find`, `execute_refs`, `execute_files` to `src/main.rs`
- Wire up command matching

### Phase 5: Testing & Validation (1-2 days)
- Unit tests for delegation functions
- Integration tests comparing Splice output to Magellan CLI
- Validate against `docs/JSON_EXPORT_FORMAT.md` and `docs/SCHEMA_REFERENCE.md`

**Total Estimate:** 6-10 days of focused work

---

## Sources

| Source | Confidence | Notes |
|--------|------------|-------|
| `Cargo.toml` | HIGH | Actual dependency versions in codebase |
| `src/lib.rs` | HIGH | Module structure |
| `src/cli/mod.rs` | HIGH | Existing CLI commands and flags |
| `src/graph/mod.rs` | HIGH | Existing CodeGraph implementation |
| `src/graph/magellan_integration.rs` | HIGH | Existing Magellan integration patterns |
| `src/output.rs` | HIGH | Existing JSON response types |
| `docs/CLI_PATTERNS.md` | HIGH | Magellan CLI conventions |
| `docs/JSON_EXPORT_FORMAT.md` | HIGH | Magellan JSON format specification |
| `docs/SCHEMA_REFERENCE.md` | HIGH | Magellan data schema conventions |
| Magellan crate documentation | MEDIUM | Assumed 0.5.3 API - verify with actual crate docs |

---

## Gaps Requiring Validation

1. **Magellan 0.5.3 API surface**: Verify exact method signatures for `get_stats()`, `find_symbol()`, `get_references()`, `list_files()`. Current research assumes based on typical code graph patterns.

2. **Type compatibility**: Confirm Magellan's `SymbolInfo`, `Reference` types can convert to Splice's `SymbolMatch`, `ReferenceMatch` without data loss.

3. **Error handling**: Verify Magellan's error types convert cleanly to `SpliceError`.

4. **Execution ID generation**: Test that `generate_magellan_execution_id()` produces IDs matching Magellan's format exactly.
