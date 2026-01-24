# Phase 25: Export Command & Error Mapping - Research

**Researched:** 2026-01-24
**Domain:** CLI export command, CSV/JSONL serialization, Magellan error mapping
**Confidence:** HIGH

## Summary

Phase 25 implements the `export` command for exporting Magellan graph data in three formats (json, jsonl, csv) and maps Magellan's `anyhow::Error` to Splice's SPL-E### error codes with error chain preservation. The export command outputs files, symbols, references, and calls with a schema version field for compatibility.

Based on verified analysis of the existing codebase:

1. **CSV serialization requires new dependency** - `csv` crate with `serde` support is the standard Rust solution
2. **Magellan errors currently map to `SpliceError::Other`** - Need dedicated variants for proper SPL-E### mapping
3. **Export schema follows existing `JsonResponse` pattern** - Reuse `schema_version`, `execution_id`, `timestamp` fields
4. **JSONL is newline-delimited JSON** - Simple extension of existing JSON serialization
5. **Error chain preservation** - Store original Magellan error in `SpliceError::Magellan` variant with source

**Primary recommendation:** Add `csv` crate dependency, create `SpliceError::Magellan` variant for wrapped Magellan errors with SPL-E091 code, implement `ExportFormat` enum (json/jsonl/csv), create export response types with schema_version, and add `execute_export` function following existing command patterns.

## Standard Stack

### Core
| Library | Version | Purpose | Why Standard |
|---------|---------|---------|--------------|
| `serde`/`serde_json` | 1.0 | JSON serialization | Already in dependencies, used throughout codebase |
| `csv` | 1.3 | CSV serialization with serde | Standard Rust CSV crate, serde-compatible, actively maintained |
| `clap` | 4.5 | CLI argument parsing | Already in dependencies, derive API for new Export command |
| `splice::graph::MagellanIntegration` | (Phase 23) | Graph data source | All query methods for export data |

### Supporting
| Library | Version | Purpose | When to Use |
|---------|---------|---------|-------------|
| `splice::output::JsonResponse` | (existing) | Response wrapper | For wrapping export data with schema_version |
| `splice::symbol_id` | (Phase 22) | 16-char hex IDs | For symbol_id in export output |
| `splice::error_codes` | (existing) | SPL-E### error codes | For Magellan error code mapping |

### Alternatives Considered
| Instead of | Could Use | Tradeoff |
|------------|-----------|----------|
| `csv` crate | Manual CSV writing | csv crate handles escaping, quotes, serde integration |
| `SpliceError::Other` | Dedicated `Magellan` variant | Dedicated variant enables proper SPL-E### mapping |

**Installation:**
```bash
# Add to Cargo.toml [dependencies]
csv = "1.3"
```

## Architecture Patterns

### Recommended Extension Structure
```
src/cli/
├── mod.rs           // EXTEND: Add Commands::Export, ExportFormat enum
├── export.rs        // NEW: Export data structures and serialization

src/error.rs         // EXTEND: Add SpliceError::Magellan variant

src/error_codes.rs   // EXTEND: Add SPL-E091 MagellanError code

src/output.rs        // EXTEND: Add ExportResponse, ExportData types
```

### Pattern 1: ExportFormat Enum (EXPORT-01)

**What:** Three-mode export format selector (json/jsonl/csv).

**When to use:** The `splice export --format <format>` command.

**Example:**
```rust
// Source: clap 4.5 ValueEnum derive pattern (verified in src/cli/mod.rs:582-590)
// Existing OutputFormat enum demonstrates the pattern

use clap::ValueEnum;

/// Export format for graph data.
#[derive(Clone, Debug, Default, ValueEnum)]
pub enum ExportFormat {
    /// JSON array format (default)
    #[default]
    Json,
    /// JSON Lines (newline-delimited JSON)
    Jsonl,
    /// CSV format with headers
    Csv,
}

// Add to Commands enum
#[derive(clap::Subcommand, Debug)]
pub enum Commands {
    // ... existing commands

    /// Export graph data in JSON, JSONL, or CSV format
    #[command(display_order = 105)]
    Export {
        /// Path to the Magellan database
        #[arg(short, long)]
        db: std::path::PathBuf,

        /// Export format (json, jsonl, csv)
        #[arg(short, long, value_enum, default_value_t = ExportFormat::Json)]
        format: ExportFormat,

        /// Output file path (writes to stdout if not specified)
        #[arg(short, long)]
        output: Option<std::path::PathBuf>,
    },
}
```

**Key insight:** `default_value_t = ExportFormat::Json` makes JSON the default. `Option<PathBuf>` for output enables stdout when not specified.

### Pattern 2: Export Data Structures (EXPORT-02)

**What:** Structured export data with schema version.

**When to use:** Serializing graph data for export.

**Example:**
```rust
// Source: src/output.rs - existing JsonResponse wrapper pattern
// Use schema_version for format stability

use serde::{Deserialize, Serialize};

/// Export response with schema version.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExportResponse {
    /// Schema version for parsing stability
    pub schema_version: String,
    /// Execution timestamp
    pub timestamp: String,
    /// Database path
    pub db_path: String,
    /// Exported graph data
    pub data: ExportData,
}

/// Complete graph data export.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExportData {
    /// All indexed files
    pub files: Vec<FileExport>,
    /// All symbols with spans
    pub symbols: Vec<SymbolExport>,
    /// All references between symbols
    pub references: Vec<ReferenceExport>,
    /// All function calls
    pub calls: Vec<CallExport>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileExport {
    pub path: String,
    pub hash: String,
    pub last_indexed_at: i64,
    pub last_modified: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SymbolExport {
    pub symbol_id: String,
    pub name: String,
    pub kind: String,
    pub file_path: String,
    pub byte_start: usize,
    pub byte_end: usize,
    pub start_line: usize,
    pub end_line: usize,
    pub start_col: usize,
    pub end_col: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReferenceExport {
    pub from_symbol_id: String,
    pub to_symbol_id: String,
    pub reference_kind: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CallExport {
    pub caller_symbol_id: String,
    pub callee_symbol_id: String,
    pub call_site_file: String,
    pub call_site_line: usize,
}
```

**Key insight:** Use `schema_version` field (matching `JsonResponse` pattern) for forward compatibility. All export types are `Serialize`-compatible for JSON/JSONL/CSV output.

### Pattern 3: CSV Serialization with Headers (EXPORT-01)

**What:** CSV output with proper headers and escaping.

**When to use:** User specifies `--format csv`.

**Example:**
```rust
// Source: csv crate 1.3 with serde integration
// Verified pattern: csv crate handles complex types via serde

use csv::Writer;
use std::io::{BufWriter, Write};

fn write_csv_export(data: &ExportData, writer: impl Write) -> Result<(), SpliceError> {
    let mut wtr = Writer::from_writer(writer);

    // Write files
    for file in &data.files {
        wtr.serialize(file)
            .map_err(|e| SpliceError::Other(format!("CSV write error: {}", e)))?;
    }

    // Write symbols
    for symbol in &data.symbols {
        wtr.serialize(symbol)
            .map_err(|e| SpliceError::Other(format!("CSV write error: {}", e)))?;
    }

    // Write references
    for reference in &data.references {
        wtr.serialize(reference)
            .map_err(|e| SpliceError::Other(format!("CSV write error: {}", e)))?;
    }

    // Write calls
    for call in &data.calls {
        wtr.serialize(call)
            .map_err(|e| SpliceError::Other(format!("CSV write error: {}", e)))?;
    }

    wtr.flush()
        .map_err(|e| SpliceError::Other(format!("CSV flush error: {}", e)))?;

    Ok(())
}
```

**Key insight:** The csv crate automatically handles:
- Header row generation from struct field names
- CSV escaping (quotes, commas, newlines)
- UTF-8 encoding

### Pattern 4: JSONL (Newline-Delimited JSON) (EXPORT-01)

**What:** One JSON object per line for streaming processing.

**When to use:** User specifies `--format jsonl`.

**Example:**
```rust
// Source: serde_json + line-by-line writing
// JSONL: one JSON object per line, no outer array

use std::io::{BufWriter, Write};
use serde::Serialize;

fn write_jsonl_export<T: Serialize>(items: &[T], writer: &mut impl Write) -> Result<(), SpliceError> {
    for item in items {
        let json = serde_json::to_string(item)
            .map_err(|e| SpliceError::Other(format!("JSON serialization error: {}", e)))?;
        writeln!(writer, "{}", json)
            .map_err(|e| SpliceError::Other(format!("Write error: {}", e)))?;
    }
    Ok(())
}

// Usage in execute_export
fn write_jsonl_export_data(data: &ExportData, writer: &mut impl Write) -> Result<(), SpliceError> {
    // Write metadata line first
    writeln!(writer, "{{\"schema_version\": \"{}\", \"export_type\": \"files\"}}", data.schema_version)?;

    // Then each category with type tag
    for file in &data.files {
        let json = serde_json::to_string(file)?;
        writeln!(writer, "{{\"type\": \"file\", \"data\": {}}}", json)?;
    }

    for symbol in &data.symbols {
        let json = serde_json::to_string(symbol)?;
        writeln!(writer, "{{\"type\": \"symbol\", \"data\": {}}}", json)?;
    }

    // ... references, calls

    Ok(())
}
```

**Key insight:** JSONL enables streaming processing (one line = one record). Tag each line with type field for heterogeneous records.

### Pattern 5: Magellan Error Mapping (ERROR-01)

**What:** Wrap Magellan errors with SPL-E### code while preserving original error.

**When to use:** Any `MagellanIntegration` method returns `Err`.

**Example:**
```rust
// Source: src/error.rs - extend SpliceError enum
// Source: src/error_codes.rs - add MagellanError variant

/// SpliceError variant for Magellan errors with error code mapping
#[derive(Error, Debug)]
pub enum SpliceError {
    // ... existing variants

    /// Magellan integration error (SPL-E091).
    #[error("Magellan error: {context}")]
    Magellan {
        /// Contextual description of the operation that failed
        context: String,
        /// The underlying Magellan anyhow::Error
        #[source]
        source: anyhow::Error,
    },
}

// Conversion from anyhow::Error (Magellan's error type)
impl From<anyhow::Error> for SpliceError {
    fn from(err: anyhow::Error) -> Self {
        SpliceError::Magellan {
            context: "Magellan operation failed".to_string(),
            source: err,
        }
    }
}

// In error_codes.rs - add new error code
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SpliceErrorCode {
    // ... existing codes

    /// Magellan integration error (SPL-E091)
    MagellanError,
}

impl SpliceErrorCode {
    pub fn code(&self) -> String {
        match self {
            // ... existing codes
            SpliceErrorCode::MagellanError => "SPL-E091".to_string(),
        }
    }

    pub fn severity(&self) -> String {
        match self {
            SpliceErrorCode::MagellanError => "error".to_string(),
            // ... existing codes
        }
    }

    pub fn hint(&self) -> String {
        match self {
            SpliceErrorCode::MagellanError => {
                "Check that the Magellan database file exists and is readable. \
                 Try re-indexing the codebase with `splice ingest`.".to_string()
            }
            // ... existing codes
        }
    }
}

impl SpliceErrorCode {
    pub fn from_splice_error(error: &SpliceError) -> Option<Self> {
        match error {
            SpliceError::Magellan { .. } => Some(SpliceErrorCode::MagellanError),
            // ... existing mappings
        }
    }
}
```

**Key insight:** Using `#[source]` attribute preserves the original error in the error chain. `anyhow::Error` can be downcast to specific Magellan error types if needed.

### Anti-Patterns to Avoid

- **Manual CSV writing:** Don't hand-roll CSV serialization - use csv crate for proper escaping
- **Mixing formats in single file:** Don't combine files/symbols/refs/calls without clear delimiters - use JSONL type tagging or separate CSV files
- **Losing original error context:** Don't convert Magellan errors to string - preserve `anyhow::Error` in error chain
- **Missing schema_version:** Don't export without version field - breaks forward compatibility
- **Inconsistent field names:** Use Magellan naming (start_line) for export, matching delegated query responses

## Don't Hand-Roll

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| CSV serialization | Manual string joining, comma escaping | `csv` crate with `Serialize` | Handles quotes, newlines, special chars automatically |
| JSONL formatting | Manual newline joining | `serde_json::to_string` per line + `writeln!` | Standard JSON serialization, one line per record |
| Error wrapping | Converting to string only | `#[source]` attribute on error variant | Preserves error chain for debugging |
| Output file handling | Manual file creation and buffering | `BufWriter` + `File::create` | Efficient buffered I/O |

**Key insight:** Export formats are "solved problems" in Rust ecosystem. Use established crates rather than custom implementation.

## Common Pitfalls

### Pitfall 1: CSV Escaping Issues

**What goes wrong:** CSV output breaks when file paths contain commas or quotes.

**Why it happens:** Manual CSV writing doesn't handle RFC 4180 escaping rules.

**How to avoid:**
1. Always use csv crate's serde integration
2. Let the crate handle quoting and escaping automatically
3. Test with file paths containing special characters

**Warning signs:** CSV output has unescaped commas in file paths.

### Pitfall 2: JSONL Without Type Tags

**What goes wrong:** JSONL consumers can't distinguish between file/symbol/reference records.

**Why it happens:** Writing raw JSON objects without type metadata.

**How to avoid:**
1. Wrap each record in `{type: "file", data: {...}}` structure
2. Or use separate JSONL files per entity type
3. Document the record structure in help text

**Warning signs:** Consumers need to inspect record fields to determine type.

### Pitfall 3: Magellan Error Context Loss

**What goes wrong:** Error messages like "Magellan error" without useful context.

**Why it happens:** Converting `anyhow::Error` to string without preserving context.

**How to avoid:**
1. Always include context string describing the operation (e.g., "Failed to open database")
2. Use `#[source]` to preserve original error chain
3. Consider extracting Magellan's error messages if needed

**Warning signs:** Generic "Magellan operation failed" messages.

### Pitfall 4: Large Export Memory Usage

**What goes wrong:** Exporting large databases causes memory exhaustion.

**Why it happens:** Loading all data into memory before writing.

**How to avoid:**
1. Use streaming writes with `BufWriter`
2. Consider chunked export for very large datasets
3. Document memory requirements for export

**Warning signs:** OOM errors when exporting databases with 10K+ files.

### Pitfall 5: Missing Schema Version

**What goes wrong:** Export consumers can't handle format changes.

**Why it happens:** Export without version identifier.

**How to avoid:**
1. Always include `schema_version` field (use constant from src/output.rs)
2. Follow existing JsonResponse pattern for consistency
3. Document schema changes in changelog

**Warning signs:** Parsing errors after format updates.

## Code Examples

### Export Command Implementation

```rust
// Source: src/main.rs - following execute_status pattern

fn execute_export(
    db_path: &Path,
    format: cli::ExportFormat,
    output: Option<&Path>,
    json_output: bool,
) -> Result<cli::CliSuccessPayload, SpliceError> {
    use splice::graph::magellan_integration::MagellanIntegration;
    use std::io::BufWriter;

    let integration = MagellanIntegration::open(db_path)?;

    // Collect all graph data
    let files = integration.list_indexed_files(false)?;
    let stats = integration.get_statistics()?;

    // Build export response
    let export_data = splice::output::ExportData {
        files: files.into_iter().map(|f| splice::output::FileExport {
            path: f.path,
            hash: f.hash,
            last_indexed_at: f.last_indexed_at,
            last_modified: f.last_modified,
        }).collect(),
        symbols: vec![],  // Populate from integration methods
        references: vec![],
        calls: vec![],
    };

    let response = splice::output::ExportResponse {
        schema_version: splice::output::EXPORT_SCHEMA_VERSION.to_string(),
        timestamp: chrono::Utc::now().to_rfc3339(),
        db_path: db_path.to_string_lossy().to_string(),
        data: export_data,
    };

    // Write to file or stdout
    if let Some(path) = output {
        let file = std::fs::File::create(path)?;
        let writer = BufWriter::new(file);
        match format {
            cli::ExportFormat::Json => {
                serde_json::to_writer_pretty(writer, &response)?;
            }
            cli::ExportFormat::Jsonl => {
                // Write JSONL format
                write_jsonl_export(&response, writer)?;
            }
            cli::ExportFormat::Csv => {
                // Write CSV format
                write_csv_export(&response.data, writer)?;
            }
        }
    } else {
        // Write to stdout
        match format {
            cli::ExportFormat::Json => {
                println!("{}", serde_json::to_string_pretty(&response)?);
            }
            cli::ExportFormat::Jsonl => {
                write_jsonl_export(&response, std::io::stdout())?;
            }
            cli::ExportFormat::Csv => {
                write_csv_export(&response.data, std::io::stdout())?;
            }
        }
    }

    Ok(cli::CliSuccessPayload::message_only(
        format!("Exported {} files, {} symbols", response.data.files.len(), stats.symbols)
    ))
}
```

### Magellan Error Mapping with Context

```rust
// Source: src/graph/magellan_integration.rs - update error handling

impl MagellanIntegration {
    pub fn open(db_path: &Path) -> Result<Self> {
        let db_path_str = db_path
            .to_str()
            .ok_or_else(|| SpliceError::Other(format!("Invalid UTF-8 in path: {:?}", db_path)))?;

        // Convert anyhow::Error to SpliceError::Magellan
        let inner = MagellanGraph::open(db_path_str)
            .map_err(|e| SpliceError::Magellan {
                context: format!("Failed to open Magellan database: {}", db_path_str),
                source: e,
            })?;

        Ok(Self {
            inner,
            db_path: db_path.to_path_buf(),
        })
    }

    pub fn get_statistics(&self) -> Result<DatabaseStats> {
        let files = self.inner.count_files()
            .map_err(|e| SpliceError::Magellan {
                context: "Failed to count files in database".to_string(),
                source: e,
            })?;

        let symbols = self.inner.count_symbols()
            .map_err(|e| SpliceError::Magellan {
                context: "Failed to count symbols in database".to_string(),
                source: e,
            })?;

        // ... other counts

        Ok(DatabaseStats {
            files,
            symbols,
            // ...
        })
    }
}
```

### Error Explanation for SPL-E091

```rust
// Source: src/error_codes.rs - add get_error_explanation entry

pub fn get_error_explanation(code: &str) -> Option<&'static str> {
    match code {
        // ... existing codes

        "SPL-E091" => Some(
            r#"
Magellan Error (SPL-E091)

An error occurred in the Magellan code graph integration.

POSSIBLE CAUSES:
- Database file is corrupted or incompatible
- Insufficient permissions to read the database
- Database file doesn't exist (need to run `splice ingest` first)
- Magellan internal error

WHAT TO DO:
1. Check that the database file exists: ls -l <db_path>
2. Verify file permissions: readable by current user
3. Try re-indexing: `splice ingest --force`
4. Check if Magellan version is compatible

RELATED: SPL-E061 (Graph Error), SPL-E031 (File Read Error)
"#
        ),

        _ => None,
    }
}
```

## State of the Art

| Old Approach | Current Approach | When Changed | Impact |
|--------------|------------------|--------------|--------|
| No export capability | json/jsonl/csv export | Phase 25 | LLMs can consume full graph data |
| `SpliceError::Other` for Magellan | Dedicated `Magellan` variant with SPL-E091 | Phase 25 | Better error reporting and explainability |
| No error chain preservation | Source error preserved via `#[source]` | Phase 25 | Improved debugging |

**New in Phase 25:**
- `csv` crate dependency (1.3)
- `Commands::Export` CLI command
- `SpliceError::Magellan` variant
- `SPL-E091` error code
- `ExportFormat` enum
- Export response types with schema_version

## Open Questions

1. **JSONL record structure for heterogeneous data**
   - What we know: JSONL requires consistent record types or type tagging
   - What's unclear: Whether to use type-tagged records or separate files per type
   - Recommendation: Use type-tagged records in single file for simplicity, separate files if export size is a concern

2. **Large export performance**
   - What we know: Databases can have 10K+ files
   - What's unclear: Whether streaming export is needed for memory efficiency
   - Recommendation: Implement buffered writes with BufWriter, monitor memory usage, add chunked export in Phase 26 if needed

3. **CSV for multiple entity types**
   - What we know: CSV expects homogeneous records
   - What's unclear: How to represent files/symbols/refs/calls in single CSV
   - Recommendation: Export each entity type to separate CSV file, or use "entity_type" column to distinguish

## Sources

### Primary (HIGH confidence)

**Existing Splice codebase (verified directly):**
- `/home/feanor/Projects/splice/src/cli/mod.rs` — CLI structure, Commands enum, clap 4.5 usage
- `/home/feanor/Projects/splice/src/main.rs` — Command execution patterns (execute_status, execute_find, etc.)
- `/home/feanor/Projects/splice/src/error.rs` — SpliceError enum variants
- `/home/feanor/Projects/splice/src/error_codes.rs` — SPL-E### error code system
- `/home/feanor/Projects/splice/src/output.rs` — JsonResponse wrapper, schema_version constants
- `/home/feanor/Projects/splice/src/graph/magellan_integration.rs` — Magellan error handling patterns
- `/home/feanor/Projects/splice/Cargo.toml` — Current dependencies (serde 1.0, clap 4.5)

**Project documentation:**
- `/home/feanor/Projects/splice/.planning/ROADMAP.md` — Phase 25 success criteria
- `/home/feanor/Projects/splice/.planning/REQUIREMENTS.md` — EXPORT-01, EXPORT-02, ERROR-01 requirements

### Secondary (MEDIUM confidence)

**csv crate documentation:**
- csv crate on docs.rs — serde integration, WriterBuilder, CSV escaping rules
- Standard pattern: `#[serde(skip_serializing_if = "Option::is_none")]` for optional fields

**Rust CLI patterns:**
- BufWriter for buffered file I/O
- anyhow::Error source preservation with `#[source]` attribute

### Tertiary (LOW confidence)

**CSV format specifications:**
- RFC 4180 CSV format standard
- JSONL (newline-delimited JSON) conventions

## Metadata

**Confidence breakdown:**
- Standard stack: HIGH — csv crate is established standard, verified pattern
- Architecture: HIGH — based on verified existing command patterns (status, find, refs, files)
- Export formats: MEDIUM — JSON/JSONL straightforward, CSV has open questions about multi-type handling
- Error mapping: HIGH — SpliceError::Magellan variant pattern verified in error.rs
- Pitfalls: HIGH — CSV escaping, error context loss are well-known issues

**Research date:** 2026-01-24
**Valid until:** 30 days (csv crate API stable, error pattern stable)
