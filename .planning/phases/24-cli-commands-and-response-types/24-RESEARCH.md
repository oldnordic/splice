# Phase 24: CLI Commands & Response Types - Research

**Researched:** 2026-01-24
**Domain:** CLI output formatting, exit codes, command categorization, Magellan-compatible response types
**Confidence:** HIGH

## Summary

Phase 24 implements CLI polish for the five Magellan-delegated query commands delivered in Phase 23: `status`, `find`, `refs`, and `files`. The phase adds `--output` formatting (human/json/pretty), standardizes exit codes to match Magellan conventions, organizes commands into categories in `--help`, and creates Magellan-compatible response types with translated field names.

The existing CLI structure in `src/cli/mod.rs` uses clap 4.5 with derive API. The `--json` flag already exists as a global option; the phase extends this to a proper `--output` enum and adds response type wrappers for Magellan compatibility using the field translation utilities from Phase 22.

**Key findings:**
1. **clap 4.5 supports command categorization** via `#[command(group = "...")]` attribute and custom help templates
2. **Exit codes currently use 0=success, 1=error** - need to extend to support Magellan's 6-code system (0-5)
3. **Field translation already exists** in `src/format/magellan.rs` from Phase 22
4. **Response types follow JsonResponse wrapper pattern** already in `src/output.rs`
5. **Output mode enum** is a standard clap pattern using `ValueEnum` derive

**Primary recommendation:** Add `OutputFormat` enum with three variants (human/json/pretty), create `SpliceExitCode` enum mapping Magellan's 6-code system, group commands into 4 categories (Query, Edit, Export, Validation), and create 4 response types wrapping Phase 23 query results with translated field names.

## Standard Stack

### Core
| Library | Version | Purpose | Why Standard |
|---------|---------|---------|--------------|
| `clap` | 4.5 | CLI argument parsing | Already in dependencies, derive API supports categorization |
| `serde`/`serde_json` | 1.0 | JSON serialization | Already in dependencies, used by output module |
| `splice::format::magellan` | (Phase 22) | Field name translation | Provides `to_magellan()`, `from_magellan()`, `translate_field_name()` |
| `splice::graph::MagellanIntegration` | (Phase 23) | Query command methods | All query methods (`find_symbol_by_name`, `get_call_relationships`, etc.) |

### Supporting
| Library | Version | Purpose | When to Use |
|---------|---------|---------|-------------|
| `splice::output::JsonResponse` | (existing) | Response wrapper | For wrapping Magellan-delegated query results |
| `splice::symbol_id` | (Phase 22) | 16-char hex IDs | For including `symbol_id` in response types |

**Installation:** No new dependencies required. All functionality uses existing crates.

## Architecture Patterns

### Recommended Extension Structure
```
src/cli/
├── mod.rs           // EXTEND: Add OutputFormat enum, SpliceExitCode enum
├── output.rs        // NEW: Output formatting dispatcher
└── response.rs      // NEW: Magellan-compatible response types

src/format/
└── magellan.rs      // EXISTING: Field translation (Phase 22)

src/output.rs        // EXTEND: Add StatusResponse, FindResponse, RefsResponse, FilesResponse
```

### Pattern 1: Output Format Enum (CLI-01)

**What:** Three-mode output format selector (human/json/pretty).

**When to use:** All query commands delegated to Magellan (status, find, refs, files).

**Example:**
```rust
// Source: clap 4.5 ValueEnum derive pattern (verified in src/cli/mod.rs:427-453)
// Existing SymbolKind enum demonstrates the pattern

use clap::ValueEnum;

/// Output format for query results.
#[derive(Clone, Debug, Default, ValueEnum)]
pub enum OutputFormat {
    /// Human-readable text (default)
    #[default]
    Human,
    /// Compact JSON
    Json,
    /// Pretty-printed JSON
    Pretty,
}

// Add to Cli struct
#[derive(Parser, Debug)]
#[command(name = "splice")]
pub struct Cli {
    /// Output format (human, json, pretty)
    #[arg(short, long, global = true, value_enum, default_value_t = OutputFormat::Human)]
    output: OutputFormat,

    // Keep existing --json for backward compatibility (deprecated)
    /// Output structured JSON (deprecated: use --output json)
    #[arg(long, global = true, hide = true)]
    json: bool,
}
```

**Key insight:** `#[default]` attribute on `Human` variant makes it the default. `hide = true` on `--json` flag deprecates it without breaking existing scripts.

### Pattern 2: Exit Code Enumeration (CLI-03)

**What:** Six-code exit system matching Magellan conventions.

**When to use:** All commands - map specific error types to exit codes.

**Example:**
```rust
// Source: Magellan exit code convention from requirements documentation
//   0 = success, 1 = error, 2 = usage, 3 = database, 4 = file not found, 5 = validation

use std::process::ExitCode;

/// Splice exit codes matching Magellan conventions.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum SpliceExitCode {
    /// Operation succeeded
    Success = 0,
    /// Generic error (catch-all)
    Error = 1,
    /// Usage error (invalid arguments, missing required args)
    Usage = 2,
    /// Database error (Magellan graph access failure)
    Database = 3,
    /// File not found (requested file doesn't exist)
    FileNotFound = 4,
    /// Validation error (pre-verification, compiler check failed)
    Validation = 5,
}

impl SpliceExitCode {
    /// Map SpliceError to appropriate exit code
    pub fn from_error(error: &SpliceError) -> Self {
        match error {
            // Database errors
            SpliceError::Graph(_) | SpliceError::ExecutionLogError { .. } => Self::Database,

            // File not found
            SpliceError::Io { .. } | SpliceError::FileExternallyModified { .. }
                if error.file_path().is_some() => Self::FileNotFound,

            // Validation errors
            SpliceError::ParseValidationFailed { .. }
            | SpliceError::CompilerValidationFailed { .. }
            | SpliceError::AnalyzerFailed { .. }
            | SpliceError::CargoCheckFailed { .. }
            | SpliceError::PreVerificationFailed { .. } => Self::Validation,

            // Usage errors (clap handles these, but catch explicit usage issues)
            SpliceError::InvalidPlanSchema { .. } | SpliceError::InvalidBatchSchema { .. }
            | SpliceError::InvalidDateFormat { .. } => Self::Usage,

            // Default to generic error
            _ => Self::Error,
        }
    }

    /// Convert to std::process::ExitCode
    pub fn as_exit_code(self) -> ExitCode {
        ExitCode::from(self as u8)
    }
}

// Usage in main.rs
fn main() -> ExitCode {
    let cli = splice::cli::parse_args();

    match execute_command(cli) {
        Ok(payload) => {
            emit_output(&payload, cli.output_format());
            SpliceExitCode::Success.as_exit_code()
        }
        Err(e) => {
            if matches!(e, SpliceError::BrokenPipe) {
                return SpliceExitCode::Success.as_exit_code();
            }
            let exit_code = SpliceExitCode::from_error(&e);
            emit_error(&e, cli.output_format());
            exit_code.as_exit_code()
        }
    }
}
```

**Key insight:** clap handles argument parsing errors internally and exits with code 2. Only application-level errors need explicit mapping.

### Pattern 3: Command Categorization (CLI-04)

**What:** Group related commands in `--help` output.

**When to use:** Organizing Commands enum into logical categories.

**Example:**
```rust
// Source: clap 4.5 command group attribute
// NOTE: As of clap 4.5, use external help template or subcommand grouping

#[derive(clap::Subcommand, Debug)]
pub enum Commands {
    // === Query Commands (delegated to Magellan) ===
    /// Show database statistics (Magellan-delegated)
    #[command(display_order = 100)]
    Status {
        /// Path to the Magellan database
        #[arg(short, long)]
        db: std::path::PathBuf,
    },

    /// Find symbols by name or ID (Magellan-delegated)
    #[command(display_order = 101)]
    Find { /* ... */ },

    /// Show call relationships (Magellan-delegated)
    #[command(display_order = 102)]
    Refs { /* ... */ },

    /// List indexed files (Magellan-delegated)
    #[command(display_order = 103)]
    Files { /* ... */ },

    /// Query symbols by file (Magellan-delegated)
    #[command(display_order = 104)]
    Query { /* ... */ },

    // === Edit Commands ===
    /// Delete a symbol by removing its definition
    #[command(display_order = 200)]
    Delete { /* ... */ },

    /// Apply a patch to a symbol's span
    #[command(display_order = 201)]
    Patch { /* ... */ },

    /// Execute a multi-step refactoring plan
    #[command(display_order = 202)]
    Plan { /* ... */ },

    /// Apply a pattern replacement to multiple files
    #[command(display_order = 203)]
    ApplyFiles { /* ... */ },

    // === Export Commands ===
    /// Query execution log
    #[command(display_order = 300)]
    Log { /* ... */ },

    /// Undo a previous operation by restoring from a backup manifest
    #[command(display_order = 301)]
    Undo { /* ... */ },

    // === Validation Commands ===
    /// Explain an error code with detailed documentation
    #[command(display_order = 400)]
    Explain { /* ... */ },

    /// Search for code patterns in files
    #[command(display_order = 401)]
    Search { /* ... */ },

    /// Get code chunks from the database
    #[command(display_order = 402)]
    Get { /* ... */ },
}

// Add to Cli struct for better help organization
#[derive(Parser, Debug)]
#[command(name = "splice")]
#[command(long_about = "
Splice: Span-safe refactoring kernel for Rust.

Query Commands (Magellan-delegated):
  status, find, refs, files, query    Query code graph database

Edit Commands:
  delete, patch, plan, apply-files    Modify code with span safety

Export Commands:
  log, undo                            Export and restore operations

Validation Commands:
  explain, search, get                 Validate and explain code

Use 'splice help <command>' for more information on a specific command.
")]
pub struct Cli {
    // ...
}
```

**Key insight:** clap 4.5 doesn't have native "groups" in subcommand enum, but `display_order` and custom `long_about` achieve similar effect. Categories are documented in help text, not enforced by derive API.

### Pattern 4: Response Type Wrappers (DATA-03, DATA-04)

**What:** Magellan-compatible response types with translated field names.

**When to use:** Wrapping Phase 23 query results for JSON output.

**Example:**
```rust
// Source: src/format/magellan.rs (Phase 22 field translation)
// Source: src/output.rs (existing JsonResponse wrapper)

use crate::format::magellan::{to_magellan, translate_field_name};
use crate::output::JsonResponse;
use serde::{Deserialize, Serialize};

/// Status response (Magellan-compatible field names)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StatusResponse {
    /// Number of indexed files
    pub files: usize,
    /// Number of indexed symbols
    pub symbols: usize,
    /// Number of indexed references
    pub references: usize,
    /// Number of indexed function calls
    pub calls: usize,
    /// Number of stored code chunks
    pub code_chunks: usize,
    /// Database file path
    pub db_path: String,
}

/// Find response with Magellan-compatible span fields
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FindResponse {
    /// Matching symbols with Magellan field names
    pub symbols: Vec<MagellanSymbol>,
    /// Number of results
    pub count: usize,
    /// Whether results were truncated
    #[serde(skip_serializing_if = "Option::is_none")]
    pub truncated: Option<bool>,
}

/// Symbol with Magellan field naming (start_line vs line_start)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MagellanSymbol {
    /// 16-character hex symbol ID
    pub symbol_id: String,
    /// Symbol name
    pub name: String,
    /// Symbol kind (fn, struct, class, etc.)
    pub kind: String,
    /// File path
    pub file_path: String,
    /// Byte offset start (inclusive)
    pub byte_start: usize,
    /// Byte offset end (exclusive)
    pub byte_end: usize,
    /// Start line (1-indexed) - Magellan convention
    pub start_line: usize,
    /// End line (1-indexed) - Magellan convention
    pub end_line: usize,
    /// Start column (0-indexed) - Magellan convention
    pub start_col: usize,
    /// End column (0-indexed) - Magellan convention
    pub end_col: usize,
}

/// Refs response with call relationships
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RefsResponse {
    /// The symbol being queried
    pub symbol: MagellanSymbol,
    /// Symbols that call this symbol (if direction=in/both)
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub callers: Vec<MagellanCallReference>,
    /// Symbols that this symbol calls (if direction=out/both)
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub callees: Vec<MagellanCallReference>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MagellanCallReference {
    /// The referenced symbol
    pub symbol: MagellanSymbol,
    /// Call site location (Magellan field names)
    pub call_site: MagellanSpan,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MagellanSpan {
    pub file_path: String,
    pub byte_start: usize,
    pub byte_end: usize,
    pub start_line: usize,
    pub start_col: usize,
    pub end_line: usize,
    pub end_col: usize,
}

/// Files response with indexed file list
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FilesResponse {
    /// Indexed files with metadata
    pub files: Vec<MagellanFileMetadata>,
    /// Total count
    pub count: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MagellanFileMetadata {
    /// File path
    pub path: String,
    /// Content hash
    pub hash: String,
    /// Last indexed timestamp (Unix)
    pub last_indexed_at: i64,
    /// Last modified timestamp (Unix)
    pub last_modified: i64,
    /// Symbol count (if --symbols flag provided)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub symbol_count: Option<usize>,
}

// Conversion impl from Phase 23 types to response types
impl From<crate::graph::magellan_integration::DatabaseStats> for StatusResponse {
    fn from(stats: crate::graph::magellan_integration::DatabaseStats) -> Self {
        Self {
            files: stats.files,
            symbols: stats.symbols,
            references: stats.references,
            calls: stats.calls,
            code_chunks: stats.code_chunks,
            db_path: String::new(), // Set by caller
        }
    }
}

impl From<crate::graph::magellan_integration::SymbolInfo> for MagellanSymbol {
    fn from(info: crate::graph::magellan_integration::SymbolInfo) -> Self {
        Self {
            symbol_id: String::new(), // Generate via generate_symbol_id()
            name: info.name,
            kind: info.kind,
            file_path: info.file_path,
            byte_start: info.byte_start,
            byte_end: info.byte_end,
            // Note: SymbolInfo doesn't have line/col - must be populated separately
            start_line: 0,
            end_line: 0,
            start_col: 0,
            end_col: 0,
        }
    }
}
```

**Key insight:** Response types use Magellan field names (`start_line` vs `line_start`). Conversion from Splice types to response types applies field translation.

### Pattern 5: Output Formatting Dispatcher

**What:** Centralized output formatting based on OutputFormat enum.

**When to use:** All commands that support multiple output formats.

**Example:**
```rust
// Source: New module src/cli/output.rs

use crate::cli::OutputFormat;
use serde::Serialize;

/// Format and emit output based on format setting
pub fn emit_output<T>(data: &T, format: OutputFormat)
where
    T: std::fmt::Display + Serialize,
{
    match format {
        OutputFormat::Human => {
            // Use Display impl for human-readable output
            println!("{}", data);
        }
        OutputFormat::Json => {
            // Compact JSON
            println!("{}", serde_json::to_string(data).unwrap());
        }
        OutputFormat::Pretty => {
            // Pretty-printed JSON
            println!("{}", serde_json::to_string_pretty(data).unwrap());
        }
    }
}

/// Format and emit error based on format setting
pub fn emit_error(error: &SpliceError, format: OutputFormat) {
    match format {
        OutputFormat::Human => {
            eprintln!("Error: {}", error);
            if let Some(hint) = error.hint() {
                eprintln!("Hint: {}", hint);
            }
        }
        OutputFormat::Json | OutputFormat::Pretty => {
            let payload = crate::cli::CliErrorPayload::from_error(error);
            let json = if matches!(format, OutputFormat::Pretty) {
                serde_json::to_string_pretty(&payload).unwrap()
            } else {
                serde_json::to_string(&payload).unwrap()
            };
            eprintln!("{}", json);
        }
    }
}
```

**Key insight:** Human format uses existing Display impls. JSON formats use existing payload types from `src/cli/mod.rs`.

### Anti-Patterns to Avoid

- **Mixing field names in JSON:** Don't output Splice field names (`line_start`) in Magellan-delegated query responses - use translated types
- **Inconsistent exit codes:** Don't return exit code 1 for all errors - map specific errors to codes 2-5
- **Breaking --json flag:** Don't remove existing `--json` global flag - deprecate it but keep functionality
- **Manual help categorization:** Don't write custom help renderers - use `display_order` and `long_about` for organization
- **Duplication in response types:** Don't redefine all fields - use conversion From impls from Phase 23 types

## Don't Hand-Roll

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| Output format enum | Custom match logic | `clap::ValueEnum` derive | Standard pattern, integrates with --help |
| Field name translation | Manual field renaming | `crate::format::magellan::translate_field_name()` | Phase 22 already has this |
| Exit code mapping | Per-command exit code handling | `SpliceExitCode::from_error()` | Centralized mapping, consistent |
| JSON serialization | Custom JSON writers | `serde_json::to_string[to_string_pretty]()` | Standard, handles all types |
| Response wrapping | Custom wrapper types | `crate::output::JsonResponse` | Existing pattern, schema_version included |

**Key insight:** Output formatting is a "solved problem" in Rust CLI ecosystem. Use derive macros and existing patterns.

## Common Pitfalls

### Pitfall 1: OutputFormat Enum vs Existing --json Flag

**What goes wrong:** Adding `--output` flag conflicts with existing `--json` flag.

**Why it happens:** Current CLI has `--json` as global bool flag. Both can't be specified.

**How to avoid:**
1. Hide `--json` flag with `#[arg(hide = true)]` attribute
2. Make `--json` an alias for `--output json` in logic
3. Document `--json` as deprecated in help text

**Warning signs:** Users see both flags, get confused about which to use.

### Pitfall 2: Field Name Translation Inconsistency

**What goes wrong:** Some fields use Splice names, some use Magellan names in same JSON output.

**Why it happens:** Direct serialization of Splice types (`SpanResult`) instead of using translated response types.

**How to avoid:**
1. Always use response types (`MagellanSymbol`, `MagellanSpan`) for delegated queries
2. Add unit test verifying JSON schema matches Magellan conventions
3. Use `#[serde(rename = "...")]` if translation layer can't be used

**Warning signs:** LLM consumers need conditional logic for field names.

### Pitfall 3: Exit Code 2 (Usage) Not Triggered

**What goes wrong:** Usage errors (invalid args) return exit code 1 instead of 2.

**Why it happens:** clap handles argument parsing and exits with code 2 internally before application code runs. Only explicit usage errors in application code need mapping.

**How to avoid:**
1. Understand clap exits with code 2 for parse errors automatically
2. Map only application-level usage errors (`InvalidPlanSchema`, etc.) to code 2
3. Document which errors map to which exit codes

**Warning signs:** Shell scripts checking `exit code == 2` for usage never see it.

### Pitfall 4: Command Categories Not Visible in Help

**What goes wrong:** Categories don't appear in `--help` output despite `display_order`.

**Why it happens:** clap 4.5 doesn't render section headers in default help template. Categories are visual grouping only.

**How to avoid:**
1. Use `long_about` at Cli struct level to document categories
2. Use consistent `display_order` ranges (100s, 200s, 300s, 400s)
3. Consider custom help template if sections are critical

**Warning signs:** Users ask "which commands are for query vs edit?"

### Pitfall 5: Human Format Not Implemented

**What goes wrong:** Commands default to human format but output raw JSON or unstructured text.

**Why it happens:** `OutputFormat::Human` just delegates to JSON or uses Debug output.

**How to avoid:**
1. Implement Display trait for all response types
2. Format human output with clear sections and labels
3. Use table formatting for lists (files command, etc.)

**Warning signs:** `splice status` outputs unformatted JSON by default.

## Code Examples

### Output Format Enum with Deprecation

```rust
// Source: clap 4.5 ValueEnum derive pattern

use clap::ValueEnum;

/// Output format for query results.
#[derive(Clone, Debug, Default, ValueEnum)]
pub enum OutputFormat {
    /// Human-readable text with formatting (default)
    #[default]
    Human,
    /// Compact JSON (machine-readable)
    Json,
    /// Pretty-printed JSON (human-readable JSON)
    Pretty,
}

// Resolve format from flags, handling deprecated --json
impl Cli {
    pub fn output_format(&self) -> OutputFormat {
        // --json flag takes precedence for backward compatibility
        if self.json {
            return OutputFormat::Json;
        }
        self.output.clone()
    }
}
```

### Exit Code Mapping

```rust
// Source: SpliceError variants in src/error.rs

impl SpliceExitCode {
    pub fn from_error(error: &SpliceError) -> Self {
        match error {
            // Database-specific errors
            SpliceError::Graph(_) => Self::Database,
            SpliceError::ExecutionLogError { .. } => Self::Database,

            // File access errors
            SpliceError::Io { path, .. }
            | SpliceError::IoContext { .. }
                if error.file_path().is_some() => Self::FileNotFound,

            // Validation errors
            SpliceError::ParseValidationFailed { .. } => Self::Validation,
            SpliceError::CompilerValidationFailed { .. } => Self::Validation,
            SpliceError::AnalyzerFailed { .. } => Self::Validation,
            SpliceError::CargoCheckFailed { .. } => Self::Validation,
            SpliceError::PreVerificationFailed { .. } => Self::Validation,

            // Usage/schema errors
            SpliceError::InvalidPlanSchema { .. } => Self::Usage,
            SpliceError::InvalidBatchSchema { .. } => Self::Usage,
            SpliceError::InvalidDateFormat { .. } => Self::Usage,

            // Default to generic error
            _ => Self::Error,
        }
    }
}
```

### Response Type Conversion

```rust
// Source: Using Phase 22 translation utilities

use crate::format::magellan::{to_magellan, from_magellan};

impl From<crate::output::SpanResult> for MagellanSymbol {
    fn from(span: crate::output::SpanResult) -> Self {
        Self {
            symbol_id: span.span_id,
            name: span.symbol.unwrap_or_default(),
            kind: span.kind.unwrap_or_else(|| "unknown".to_string()),
            file_path: span.file_path,
            byte_start: span.byte_start,
            byte_end: span.byte_end,
            // Direct mapping - SpanResult already uses Magellan field names
            start_line: span.start_line,
            end_line: span.end_line,
            start_col: span.start_col,
            end_col: span.end_col,
        }
    }
}
```

### Command with Display Order

```rust
#[derive(clap::Subcommand, Debug)]
pub enum Commands {
    // === Query Commands (display_order 100-199) ===
    /// Show database statistics (files, symbols, refs, calls, chunks)
    #[command(display_order = 100)]
    Status {
        /// Path to the Magellan database
        #[arg(short, long)]
        db: std::path::PathBuf,
    },

    /// Find symbols by name or 16-character symbol ID
    #[command(display_order = 101)]
    Find {
        /// Path to the Magellan database
        #[arg(short, long)]
        db: std::path::PathBuf,

        /// Symbol name to search
        #[arg(short, long, conflicts_with = "symbol_id")]
        name: Option<String>,

        /// 16-character hex symbol ID
        #[arg(long, conflicts_with = "name")]
        symbol_id: Option<String>,

        /// Return all matches (default: first match only)
        #[arg(short, long)]
        ambiguous: bool,
    },

    // ... other commands
}
```

## State of the Art

| Old Approach | Current Approach | When Changed | Impact |
|--------------|------------------|--------------|--------|
| Binary --json flag | --output enum (human/json/pretty) | Phase 24 | More flexible output control |
| Exit code 0/1 only | 6-code system (0-5) | Phase 24 | Fine-grained error reporting |
| No command organization | display_order + help categories | Phase 24 | Better discoverability |
| Splice-only JSON | Magellan-compatible response types | Phase 24 | Cross-tool compatibility |

**Deprecated/outdated:**
- `--json` global flag: Use `--output json` instead (but keep for backward compat)
- Exit code 1 for all errors: Map to specific codes 2-5

## Open Questions

1. **Custom help template vs long_about**
   - What we know: clap 4.5 supports custom templates via `help_template` attribute
   - What's unclear: Whether custom template is worth complexity vs using `long_about`
   - Recommendation: Start with `long_about` and `display_order`, add custom template only if needed

2. **Table formatting for human output**
   - What we know: No table crate in dependencies
   - What's unclear: Whether to add `comfy-table` or use simple formatting
   - Recommendation: Use simple formatting for Phase 24, consider table crate in later phase

3. **Exit code for dry-run with errors**
   - What we know: Dry-run returns code 1 if changes pending, code 0 if no changes
   - What's unclear: What if dry-run encounters validation error - code 5 or code 1?
   - Recommendation: Validation errors (code 5) take precedence over pending changes (code 1)

## Sources

### Primary (HIGH confidence)

**Existing Splice codebase (verified directly):**
- `/home/feanor/Projects/splice/src/cli/mod.rs` — CLI structure, clap 4.5 usage, existing Commands enum
- `/home/feanor/Projects/splice/src/main.rs` — Exit code handling (lines 180-210)
- `/home/feanor/Projects/splice/src/error.rs` — SpliceError variants, Diagnostic struct
- `/home/feanor/Projects/splice/src/output.rs` — JsonResponse wrapper, existing response types
- `/home/feanor/Projects/splice/src/format/magellan.rs` — Field translation (Phase 22)
- `/home/feanor/Projects/splice/src/graph/magellan_integration.rs` — Query methods (Phase 23)
- `/home/feanor/Projects/splice/src/symbol_id.rs` — 16-char hex ID generation (Phase 22)
- `/home/feanor/Projects/splice/Cargo.toml` — Dependency versions (clap 4.5, serde 1.0)

**Project documentation:**
- `/home/feanor/Projects/splice/.planning/ROADMAP.md` — Phase 24 success criteria
- `/home/feanor/Projects/splice/.planning/REQUIREMENTS.md` — CLI-01 through CLI-04, DATA-03 through DATA-04
- `/home/feanor/Projects/splice/.planning/phases/22-symbol-id-and-format-foundation/22-RESEARCH.md` — Phase 22 field translation
- `/home/feanor/Projects/splice/.planning/phases/23-magellan-integration-extensions/23-RESEARCH.md` — Phase 23 query methods

### Secondary (MEDIUM confidence)

**clap 4.5 documentation:**
- `clap` crate on docs.rs — ValueEnum derive, display_order attribute, custom help templates
- Common pattern: `#[default]` attribute on enum variants

**CLI conventions:**
- Magellan exit code convention from requirements (0-5 codes)
- Git diff exit code convention (existing pattern in codebase)

### Tertiary (LOW confidence)

None - all findings are based on verified source code analysis.

## Metadata

**Confidence breakdown:**
- Standard stack: HIGH — All dependencies verified in Cargo.toml
- Architecture: HIGH — clap patterns verified in existing code
- Pitfalls: HIGH — Based on analysis of existing CLI patterns
- Exit codes: MEDIUM — Magellan convention documented in requirements but not verified against Magellan source

**Research date:** 2026-01-24
**Valid until:** 60 days (stable APIs - clap 4.5 is stable, Magellan conventions are documented)
