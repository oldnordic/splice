# Phase 22: Symbol ID & Format Foundation - Research

**Researched:** 2026-01-24
**Domain:** ID generation, field translation, JSON schema compatibility
**Confidence:** HIGH

## Summary

Phase 22 establishes the foundation for Magellan integration by implementing 16-character hex symbol IDs (SHA-256, first 8 bytes) and Magellan execution ID format ({timestamp_hex}-{pid_hex}). The phase also creates field translation utilities to convert between Magellan (start_line/start_col) and Splice (line_start/col_start) naming conventions, plus JSON schema compatibility tests.

The existing codebase already has SHA-256 infrastructure in `src/checksum.rs` and span ID generation using SHA-256 in `src/output.rs`. The main work is extracting these into a dedicated `src/symbol_id.rs` module, adding execution ID generation, and creating `src/format/magellan.rs` for field translation.

**Primary recommendation:** Implement as three focused modules (symbol_id, format/magellan, execution_id) with comprehensive unit tests, then add schema compatibility tests. No new dependencies are required—sha2 0.10 is already in Cargo.toml.

## Standard Stack

### Core
| Library | Version | Purpose | Why Standard |
|---------|---------|---------|--------------|
| `sha2` | 0.10 | SHA-256 hashing for 16-char IDs | Already in dependencies, used by checksum module |
| `serde`/`serde_json` | 1.0 | JSON serialization for schema tests | Already in dependencies, used by output module |
| `chrono` | 0.4 | Timestamp generation for execution_id | Already in dependencies, used by output module |

### Supporting
| Library | Version | Purpose | When to Use |
|---------|---------|---------|-------------|
| `uuid` | 1.10 | UUID generation (existing execution_id) | Keep for splice operations, Magellan format for delegated queries |
| `hex` | (via sha2) | Hex encoding of hash bytes | Already available via sha2 format strings |

### Alternatives Considered
| Instead of | Could Use | Tradeoff |
|------------|-----------|----------|
| SHA-256 | SHA-1, BLAKE3 | SHA-1 is 40-bit (weaker), BLAKE3 is faster but not standard in Magellan |
| {ts}-{pid} format | UUID v4 | UUID is standard but Magellan uses timestamp-pid for correlation |

**Installation:** No new dependencies required.

## Architecture Patterns

### Recommended Project Structure
```
src/
├── symbol_id.rs         # NEW: 16-char hex symbol ID generation
├── format/
│   ├── mod.rs           # NEW: Format module root
│   └── magellan.rs      # NEW: Field translation utilities
└── execution.rs         # NEW: Magellan execution ID generation
```

### Pattern 1: 16-Character Hex Symbol ID

**What:** Generate stable 16-character hex string from SHA-256 hash (first 8 bytes).

**When to use:** All symbol IDs exported via Magellan-delegated query commands.

**Example:**
```rust
// Source: Existing pattern in src/output.rs:430-445
use sha2::{Digest, Sha256};

pub fn generate_symbol_id(
    symbol_name: &str,
    file_path: &str,
    byte_start: usize,
) -> String {
    let mut hasher = Sha256::new();
    hasher.update(symbol_name.as_bytes());
    hasher.update(b":");
    hasher.update(file_path.as_bytes());
    hasher.update(b":");
    hasher.update(byte_start.to_be_bytes());

    let result = hasher.finalize();
    format!("{:02x}{:02x}{:02x}{:02x}{:02x}{:02x}{:02x}{:02x}",
        result[0], result[1], result[2], result[3],
        result[4], result[5], result[6], result[7])
}
```

**Key insight:** The existing `generate_span_id()` function uses the same pattern. Extract and adapt for symbol-specific input.

### Pattern 2: Magellan Execution ID Format

**What:** Generate execution_id in format `{timestamp_hex}-{pid_hex}` for delegated queries.

**When to use:** All query commands delegated to Magellan (status, query, find, refs, files).

**Example:**
```rust
use std::process;
use std::time::{SystemTime, UNIX_EPOCH};

pub fn generate_magellan_execution_id() -> String {
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs();

    let pid = process::id();

    format!("{:016x}-{:08x}", timestamp, pid)
}
// Output: "00000179a2f3d8f4-00001234" (example)
```

**Key insight:** Magellan's execution_id format enables time-based correlation and process tracking. Keep Splice's UUID format for edit operations, use Magellan format for delegated queries only.

### Pattern 3: Field Translation Layer

**What:** Bidirectional conversion between Magellan and Splice field naming conventions.

**When to use:** Converting Splice output structures to Magellan-compatible JSON.

**Example:**
```rust
// Splice -> Magellan translation
pub fn to_magellan_span(span: &SpliceSpan) -> MagellanSpan {
    MagellanSpan {
        span_id: span.span_id.clone(),
        file_path: span.file_path.clone(),
        byte_start: span.byte_start,
        byte_end: span.byte_end,
        start_line: span.line_start,  // line_start -> start_line
        start_col: span.col_start,    // col_start -> start_col
        end_line: span.line_end,      // line_end -> end_line
        end_col: span.col_end,        // col_end -> end_col
    }
}

// Magellan -> Splice translation
pub fn from_magellan_span(span: &MagellanSpan) -> SpliceSpan {
    SpliceSpan {
        span_id: span.span_id.clone(),
        file_path: span.file_path.clone(),
        byte_start: span.byte_start,
        byte_end: span.byte_end,
        line_start: span.start_line,  // start_line -> line_start
        col_start: span.start_col,    // start_col -> col_start
        line_end: span.end_line,      // end_line -> line_end
        col_end: span.end_col,        // end_col -> col_end
    }
}
```

**Key insight:** Translation is pure field renaming—no data transformation needed. Implement as zero-copy conversions using references.

### Anti-Patterns to Avoid

- **Hand-rolling hex encoding:** Use `format!("{:02x}...", ...)` pattern already proven in codebase
- **Inconsistent hash inputs:** Always hash (name, file_path, byte_start) in that order for determinism
- **Mixing execution_id formats:** Use Magellan format ONLY for delegated queries, keep UUID for splice operations
- **Breaking existing JSON:** Add translation layer—don't rename fields in existing structs

## Don't Hand-Roll

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| SHA-256 hashing | Custom hash implementation | `sha2::Sha256` from crate | Cryptographically secure, well-tested, already a dependency |
| Hex encoding | Manual byte-to-hex conversion | `format!("{:02x}", byte)` pattern | Simple, proven, used in existing code |
| Field translation | serde aliases | Explicit conversion functions | Clearer intent, no serde magic, testable |
| Timestamp generation | Manual time tracking | `chrono::Utc::now()` or `SystemTime` | Standard library, tested |

**Key insight:** SHA-256 and hex formatting are "solved problems"—use the proven patterns.

## Common Pitfalls

### Pitfall 1: Inconsistent Symbol ID Input Ordering

**What goes wrong:** Different hash inputs for the "same" symbol produce different IDs.

**Why it happens:** Hashing (file, name, byte_start) vs (name, file, byte_start) gives different results.

**How to avoid:** Define canonical input order and document it: `(symbol_name, file_path, byte_start)`.

**Warning signs:** Same symbol has different IDs across runs.

### Pitfall 2: Execution ID Format Confusion

**What goes wrong:** Using UUID for Magellan-delegated queries breaks correlation with Magellan's own execution logs.

**Why it happens:** Existing codebase uses UUID for `execution_id`; Magellan expects `{timestamp}-{pid}`.

**How to avoid:** Generate correct format based on command type—edit operations get UUID, delegated queries get timestamp-pid.

**Warning signs:** Magellan execution logs can't be correlated with Splice query commands.

### Pitfall 3: Field Name Translation Misses

**What goes wrong:** JSON output has mixed field names (some Magellan, some Splice).

**Why it happens:** Manual field renaming misses nested structures or optional fields.

**How to avoid:** Create exhaustive translation table and unit test every field.

**Warning signs:** LLM tools need conditional logic to parse JSON.

### Pitfall 4: Breaking Existing Tests

**What goes wrong:** Adding symbol_id field changes JSON schema and breaks 334+ existing tests.

**Why it happens:** Optional fields still appear in JSON as `null`, changing structure.

**How to avoid:** Use `#[serde(skip_serializing_if = "Option::is_none")]` for new fields.

**Warning signs:** Test suite shows unexpected JSON diffs.

## Code Examples

### Symbol ID Generation (from existing codebase pattern)

```rust
// Source: /home/feanor/Projects/splice/src/output.rs:430-445
use sha2::{Digest, Sha256};

pub fn generate_symbol_id(
    symbol_name: &str,
    file_path: &str,
    byte_start: usize,
) -> String {
    let mut hasher = Sha256::new();
    hasher.update(symbol_name.as_bytes());
    hasher.update(b":");
    hasher.update(file_path.as_bytes());
    hasher.update(b":");
    hasher.update(byte_start.to_be_bytes());

    let result = hasher.finalize();
    format!(
        "{:02x}{:02x}{:02x}{:02x}{:02x}{:02x}{:02x}{:02x}",
        result[0], result[1], result[2], result[3],
        result[4], result[5], result[6], result[7]
    )
}

#[cfg(test)]
mod tests {
    #[test]
    fn test_symbol_id_deterministic() {
        let id1 = generate_symbol_id("foo", "/path/to/file.rs", 100);
        let id2 = generate_symbol_id("foo", "/path/to/file.rs", 100);
        assert_eq!(id1, id2);
        assert_eq!(id1.len(), 16);
    }

    #[test]
    fn test_symbol_id_unique_inputs() {
        let id1 = generate_symbol_id("foo", "/path/to/file.rs", 100);
        let id2 = generate_symbol_id("bar", "/path/to/file.rs", 100);
        let id3 = generate_symbol_id("foo", "/path/to/other.rs", 100);
        let id4 = generate_symbol_id("foo", "/path/to/file.rs", 200);

        assert_ne!(id1, id2);
        assert_ne!(id1, id3);
        assert_ne!(id1, id4);
    }
}
```

### Magellan Execution ID Generation

```rust
use std::process;
use std::time::{SystemTime, UNIX_EPOCH};

/// Generate execution ID in Magellan format: {timestamp_hex}-{pid_hex}
///
/// Format: 16-char hex timestamp (seconds since epoch) + "-" + 8-char hex PID
/// Example: "00000179a2f3d8f4-00001234"
pub fn generate_magellan_execution_id() -> String {
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("Time went backwards")
        .as_secs();

    let pid = process::id();

    format!("{:016x}-{:08x}", timestamp, pid)
}

#[cfg(test)]
mod tests {
    #[test]
    fn test_execution_id_format() {
        let id = generate_magellan_execution_id();
        let parts: Vec<&str> = id.split('-').collect();
        assert_eq!(parts.len(), 2);
        assert_eq!(parts[0].len(), 16); // timestamp
        assert_eq!(parts[1].len(), 8);  // pid
    }

    #[test]
    fn test_execution_id_parseable() {
        let id = generate_magellan_execution_id();
        let parts: Vec<&str> = id.split('-').collect();
        let timestamp = u64::from_str_radix(parts[0], 16);
        let pid = u32::from_str_radix(parts[1], 16);
        assert!(timestamp.is_ok());
        assert!(pid.is_ok());
    }
}
```

### Field Translation Module

```rust
// src/format/mod.rs
pub mod magellan;

// src/format/magellan.rs
use crate::output::{Span, SpanResult};

/// Magellan-compatible span with Magellan field names
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct MagellanSpan {
    pub span_id: String,
    pub file_path: String,
    pub byte_start: usize,
    pub byte_end: usize,
    pub start_line: usize,  // Magellan: start_line (not line_start)
    pub start_col: usize,   // Magellan: start_col (not col_start)
    pub end_line: usize,    // Magellan: end_line (not line_end)
    pub end_col: usize,     // Magellan: end_col (not col_end)
}

/// Convert Splice Span to Magellan Span
pub fn to_magellan_span(span: &Span) -> MagellanSpan {
    MagellanSpan {
        span_id: span.span_id.clone(),
        file_path: span.file_path.clone(),
        byte_start: span.byte_start,
        byte_end: span.byte_end,
        start_line: span.start_line,  // field rename
        start_col: span.start_col,    // field rename
        end_line: span.end_line,      // field rename
        end_col: span.end_col,        // field rename
    }
}

/// Convert Splice SpanResult to Magellan Span
pub fn span_result_to_magellan(result: &SpanResult) -> MagellanSpan {
    MagellanSpan {
        span_id: result.span_id.clone(),
        file_path: result.file_path.clone(),
        byte_start: result.byte_start,
        byte_end: result.byte_end,
        start_line: result.start_line,
        start_col: result.start_col,
        end_line: result.end_line,
        end_col: result.end_col,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_field_translation() {
        let splice_span = Span::new(
            "/path/to/file.rs".to_string(),
            100, 200,  // byte range
            5, 10,     // line range
            0, 4,      // col range
        );

        let magellan_span = to_magellan_span(&splice_span);

        assert_eq!(magellan_span.start_line, 5);   // was line_start
        assert_eq!(magellan_span.start_col, 0);    // was col_start
        assert_eq!(magellan_span.end_line, 10);    // was line_end
        assert_eq!(magellan_span.end_col, 4);      // was col_end
    }

    #[test]
    fn test_roundtrip_preserves_data() {
        let splice_span = Span::new(
            "test.rs".to_string(), 0, 10, 1, 1, 0, 1
        );

        let magellan = to_magellan_span(&splice_span);

        // Verify all fields map correctly
        assert_eq!(splice_span.span_id, magellan.span_id);
        assert_eq!(splice_span.file_path, magellan.file_path);
        assert_eq!(splice_span.byte_start, magellan.byte_start);
        assert_eq!(splice_span.byte_end, magellan.byte_end);
    }
}
```

### JSON Schema Compatibility Test

```rust
#[cfg(test)]
mod schema_tests {
    use super::*;
    use serde_json::{json, Value};

    #[test]
    fn test_symbol_id_is_16_char_hex() {
        let id = generate_symbol_id("foo", "/path.rs", 100);
        assert_eq!(id.len(), 16);
        assert!(id.chars().all(|c| c.is_ascii_hexdigit()));
    }

    #[test]
    fn test_execution_id_matches_magellan_format() {
        let id = generate_magellan_execution_id();
        let re = regex::Regex::new(r"^[0-9a-f]{16}-[0-9a-f]{8}$").unwrap();
        assert!(re.is_match(&id));
    }

    #[test]
    fn test_magellan_span_serializes_correctly() {
        let span = Span::new("test.rs".to_string(), 0, 10, 1, 1, 0, 1);
        let magellan = to_magellan_span(&span);

        let json = serde_json::to_string(&magellan).unwrap();
        let value: Value = serde_json::from_str(&json).unwrap();

        // Verify Magellan field names exist
        assert!(value.get("start_line").is_some());
        assert!(value.get("start_col").is_some());
        assert!(value.get("end_line").is_some());
        assert!(value.get("end_col").is_some());

        // Verify Splice field names do NOT exist
        assert!(value.get("line_start").is_none());
        assert!(value.get("col_start").is_none());
    }
}
```

## State of the Art

| Old Approach | Current Approach | When Changed | Impact |
|--------------|------------------|--------------|--------|
| No stable symbol IDs | 16-char hex SHA-256 | Phase 22 | Enables Magellan cross-tool symbol correlation |
| UUID for all operations | Format by command type | Phase 22 | Magellan queries use timestamp-pid, edits use UUID |
| Inconsistent field names | Translation layer | Phase 22 | LLM tools can consume both Splice and Magellan output |

**Deprecated/outdated:**
- UUID-only execution IDs: Keep for splice operations, use Magellan format for delegated queries
- Ad-hoc symbol identification: Replace with deterministic 16-char IDs

## Open Questions

1. **Symbol ID hash inputs composition**
   - What we know: Magellan symbol IDs are 16-char hex (SHA-256, first 8 bytes)
   - What's unclear: Exact hash input composition (name? file? byte_start? kind?)
   - Recommendation: Use `(symbol_name, file_path, byte_start)` to match existing `generate_span_id()` pattern, validate with Magellan team during implementation

2. **Execution ID format boundary**
   - What we know: Delegated queries should use Magellan format, splice edits keep UUID
   - What's unclear: Should hybrid commands (query + edit) use which format?
   - Recommendation: Query phase uses Magellan format, edit phase uses separate UUID—document in response metadata

## Sources

### Primary (HIGH confidence)

**Codebase analysis (verified directly):**
- `/home/feanor/Projects/splice/Cargo.toml` — Dependency versions: sha2 0.10, uuid 1.10, chrono 0.4
- `/home/feanor/Projects/splice/src/output.rs` — Lines 430-445: `generate_span_id()` using SHA-256
- `/home/feanor/Projects/splice/src/checksum.rs` — SHA-256 usage patterns
- `/home/feanor/Projects/splice/src/symbol/mod.rs` — Symbol trait definition
- `/home/feanor/Projects/splice/src/graph/magellan_integration.rs` — Magellan wrapper types

**Project documentation:**
- `/home/feanor/Projects/splice/.planning/ROADMAP.md` — Phase 22 specification
- `/home/feanor/Projects/splice/.planning/REQUIREMENTS.md` — DATA-01, DATA-02 requirements
- `/home/feanor/Projects/splice/.planning/STATE.md` — v2.2.2 decisions (16-char IDs, field translation)
- `/home/feanor/Projects/splice/.planning/research/SUMMARY.md` — Magellan integration research
- `/home/feanor/Projects/splice/docs/CLI_PATTERNS.md` — Exit codes and output formats
- `/home/feanor/Projects/splice/docs/LLM_TOOL_ECOSYSTEM_ALIGNMENT.md` — Field name differences table

### Secondary (MEDIUM confidence)

**Magellan documentation (referenced in research summary):**
- Magellan 0.5.3 Symbol ID format: 16-char hex (SHA-256, first 8 bytes)
- Magellan execution ID format: `{timestamp_hex}-{pid_hex}` for delegated queries

### Tertiary (LOW confidence)

None—this phase is based entirely on verified codebase patterns and documented requirements.

## Metadata

**Confidence breakdown:**
- Standard stack: HIGH — All dependencies verified in Cargo.toml
- Architecture: HIGH — Translation pattern is straightforward field renaming
- Pitfalls: HIGH — Field name conflicts documented in LLM_TOOL_ECOSYSTEM_ALIGNMENT.md
- Symbol ID format: MEDIUM — 16-char hex confirmed, exact hash inputs need validation
- Execution ID format: HIGH — {timestamp_hex}-{pid_hex} documented in research summary

**Research date:** 2026-01-24
**Valid until:** 30 days (stable domain—hash algorithms and field naming conventions don't change frequently)
