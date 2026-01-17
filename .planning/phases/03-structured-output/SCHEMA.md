# Splice v2.0 Output Schema

**Version:** 2.0.0
**Date:** 2026-01-17
**Status:** Design Document

---

## Overview

This document defines the unified output schema for Splice v2.0. The schema provides explicit fields for all operations, versioning support, and consistent naming patterns across patch, delete, plan, query, and pattern replacement operations.

### Design Principles

1. **Explicit Fields Over Ad-Hoc JSON** - All data uses typed structs with `serde::Serialize`
2. **Consistent snake_case Naming** - Field names use snake_case with serde rename where needed
3. **Versioning** - Top-level structs include `version` field using semver
4. **Unified Span Representation** - All spans use the same `SpanResult` structure

---

## Top-Level OperationResult

The `OperationResult` is the top-level wrapper for all Splice operations.

### Type Definition

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OperationResult {
    /// Schema version (semver, e.g., "2.0.0")
    pub version: String,

    /// Unique operation ID (UUID v4)
    pub operation_id: String,

    /// Operation type
    pub operation_type: String,

    /// Operation status
    pub status: String,

    /// Human-readable message
    pub message: String,

    /// ISO 8601 timestamp
    pub timestamp: String,

    /// Workspace root (absolute path)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub workspace: Option<String>,

    /// Primary result data
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result: Option<OperationData>,

    /// Error details (when status is "error")
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<ErrorDetails>,
}
```

### Field Descriptions

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `version` | String | Yes | Schema version in semver format (e.g., "2.0.0") |
| `operation_id` | String | Yes | UUID v4 uniquely identifying this operation |
| `operation_type` | String | Yes | One of: "patch", "delete", "plan", "query", "apply_files" |
| `status` | String | Yes | One of: "ok", "error", "partial" |
| `message` | String | Yes | Human-readable description of the operation outcome |
| `timestamp` | String | Yes | ISO 8601 timestamp (e.g., "2026-01-17T10:30:00Z") |
| `workspace` | String | No | Absolute path to workspace root |
| `result` | OperationData | No | Present when status is "ok" or "partial" |
| `error` | ErrorDetails | No | Present when status is "error" |

### JSON Example (Successful Patch)

```json
{
  "version": "2.0.0",
  "operation_id": "550e8400-e29b-41d4-a716-446655440000",
  "operation_type": "patch",
  "status": "ok",
  "message": "Successfully patched 'my_function' at bytes 1234..1456",
  "timestamp": "2026-01-17T10:30:00Z",
  "workspace": "/home/user/project",
  "result": {
    "type": "patch",
    "file": "src/lib.rs",
    "symbol": "my_function",
    "kind": "function",
    "spans": [
      {
        "file_path": "src/lib.rs",
        "symbol": "my_function",
        "kind": "function",
        "byte_start": 1234,
        "byte_end": 1456,
        "line_start": 42,
        "line_end": 58,
        "col_start": 4,
        "col_end": 8,
        "before_hash": "a1b2c3d4e5f6...",
        "after_hash": "f6e5d4c3b2a1..."
      }
    ],
    "before_hash": "a1b2c3d4e5f6...",
    "after_hash": "f6e5d4c3b2a1...",
    "lines_added": 5,
    "lines_removed": 3
  }
}
```

---

## OperationData Variants

The `OperationData` enum is a tagged union containing operation-specific results.

### Type Definition

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum OperationData {
    #[serde(rename = "patch")]
    Patch(PatchResult),

    #[serde(rename = "delete")]
    Delete(DeleteResult),

    #[serde(rename = "plan")]
    Plan(PlanResult),

    #[serde(rename = "query")]
    Query(QueryResult),

    #[serde(rename = "apply_files")]
    ApplyFiles(ApplyFilesResult),
}
```

### PatchResult

Single file patch operation result.

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PatchResult {
    /// File that was patched
    pub file: String,

    /// Symbol name that was patched
    pub symbol: String,

    /// Symbol kind (function, struct, etc.)
    pub kind: String,

    /// Spans that were modified
    pub spans: Vec<SpanResult>,

    /// File hash before patching
    pub before_hash: String,

    /// File hash after patching
    pub after_hash: String,

    /// Number of lines added
    pub lines_added: usize,

    /// Number of lines removed
    pub lines_removed: usize,
}
```

### JSON Example (PatchResult)

```json
{
  "type": "patch",
  "file": "src/lib.rs",
  "symbol": "my_function",
  "kind": "function",
  "spans": [
    {
      "file_path": "src/lib.rs",
      "symbol": "my_function",
      "kind": "function",
      "byte_start": 1234,
      "byte_end": 1456,
      "line_start": 42,
      "line_end": 58,
      "col_start": 4,
      "col_end": 8,
      "before_hash": "a1b2c3d4e5f6...",
      "after_hash": "f6e5d4c3b2a1..."
    }
  ],
  "before_hash": "a1b2c3d4e5f6...",
  "after_hash": "f6e5d4c3b2a1...",
  "lines_added": 5,
  "lines_removed": 3
}
```

### DeleteResult

Symbol deletion operation result.

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeleteResult {
    /// File containing the deleted symbol
    pub file: String,

    /// Symbol name that was deleted
    pub symbol: String,

    /// Symbol kind
    pub kind: String,

    /// All spans that were removed (definition + references)
    pub spans: Vec<SpanResult>,

    /// Total bytes removed
    pub bytes_removed: usize,

    /// Total lines removed
    pub lines_removed: usize,

    /// Number of references removed
    pub references_removed: usize,
}
```

### JSON Example (DeleteResult)

```json
{
  "type": "delete",
  "file": "src/lib.rs",
  "symbol": "old_function",
  "kind": "function",
  "spans": [
    {
      "file_path": "src/lib.rs",
      "symbol": "old_function",
      "kind": "function",
      "byte_start": 1000,
      "byte_end": 1200,
      "line_start": 40,
      "line_end": 50,
      "col_start": 0,
      "col_end": 1,
      "before_hash": "abc123...",
      "after_hash": "def456..."
    },
    {
      "file_path": "src/main.rs",
      "symbol": "old_function",
      "kind": "function",
      "byte_start": 500,
      "byte_end": 520,
      "line_start": 20,
      "line_end": 20,
      "col_start": 10,
      "col_end": 30,
      "before_hash": "123abc...",
      "after_hash": "456def..."
    }
  ],
  "bytes_removed": 720,
  "lines_removed": 31,
  "references_removed": 1
}
```

### PlanResult

Multi-step plan execution result.

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlanResult {
    /// Number of steps in the plan
    pub total_steps: usize,

    /// Number of steps successfully executed
    pub steps_completed: usize,

    /// Individual step results
    pub steps: Vec<StepResult>,

    /// All files affected across all steps
    pub files_affected: Vec<String>,

    /// Total bytes changed across all steps
    pub total_bytes_changed: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StepResult {
    /// Step index (1-based)
    pub step: usize,

    /// Step status
    pub status: String,

    /// Step message
    pub message: String,

    /// File patched in this step
    pub file: String,

    /// Symbol patched in this step
    pub symbol: String,
}
```

### JSON Example (PlanResult)

```json
{
  "type": "plan",
  "total_steps": 3,
  "steps_completed": 3,
  "steps": [
    {
      "step": 1,
      "status": "ok",
      "message": "Patched 'foo' at bytes 100..200",
      "file": "src/lib.rs",
      "symbol": "foo"
    },
    {
      "step": 2,
      "status": "ok",
      "message": "Patched 'bar' at bytes 300..400",
      "file": "src/lib.rs",
      "symbol": "bar"
    },
    {
      "step": 3,
      "status": "ok",
      "message": "Patched 'baz' at bytes 500..600",
      "file": "src/main.rs",
      "symbol": "baz"
    }
  ],
  "files_affected": ["src/lib.rs", "src/main.rs"],
  "total_bytes_changed": 300
}
```

### QueryResult

Magellan query result (label-based symbol search).

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueryResult {
    /// Query labels that were used
    pub labels: Vec<String>,

    /// Number of results found
    pub count: usize,

    /// Matching symbols
    pub symbols: Vec<SpanResult>,
}
```

### JSON Example (QueryResult)

```json
{
  "type": "query",
  "labels": ["function", "rust"],
  "count": 2,
  "symbols": [
    {
      "file_path": "src/lib.rs",
      "symbol": "my_function",
      "kind": "function",
      "byte_start": 1234,
      "byte_end": 1456,
      "line_start": 42,
      "line_end": 58,
      "col_start": 4,
      "col_end": 8,
      "before_hash": "",
      "after_hash": ""
    },
    {
      "file_path": "src/main.rs",
      "symbol": "another_function",
      "kind": "function",
      "byte_start": 500,
      "byte_end": 700,
      "line_start": 20,
      "line_end": 30,
      "col_start": 0,
      "col_end": 1,
      "before_hash": "",
      "after_hash": ""
    }
  ]
}
```

### ApplyFilesResult

Pattern replacement across multiple files.

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApplyFilesResult {
    /// Glob pattern used for matching
    pub glob_pattern: String,

    /// Find pattern
    pub find_pattern: String,

    /// Replace pattern
    pub replace_pattern: String,

    /// Number of files matched
    pub files_matched: usize,

    /// Number of files modified
    pub files_modified: usize,

    /// Individual file results
    pub files: Vec<FilePatternResult>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FilePatternResult {
    /// File path
    pub file: String,

    /// Number of matches in this file
    pub matches: usize,

    /// Number of replacements made
    pub replacements: usize,

    /// Spans that were replaced
    pub spans: Vec<SpanResult>,

    /// File hash before
    pub before_hash: String,

    /// File hash after
    pub after_hash: String,
}
```

### JSON Example (ApplyFilesResult)

```json
{
  "type": "apply_files",
  "glob_pattern": "tests/**/*.rs",
  "find_pattern": "FIXME",
  "replace_pattern": "TODO",
  "files_matched": 2,
  "files_modified": 2,
  "files": [
    {
      "file": "tests/test_1.rs",
      "matches": 3,
      "replacements": 3,
      "spans": [
        {
          "file_path": "tests/test_1.rs",
          "symbol": "",
          "kind": "text",
          "byte_start": 100,
          "byte_end": 105,
          "line_start": 10,
          "line_end": 10,
          "col_start": 0,
          "col_end": 5,
          "before_hash": "abc123...",
          "after_hash": "def456..."
        }
      ],
      "before_hash": "abc123...",
      "after_hash": "def456..."
    }
  ]
}
```

---

## Unified SpanResult

The `SpanResult` type provides a consistent representation for all code spans.

### Type Definition

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpanResult {
    /// File path (relative to workspace or absolute)
    pub file_path: String,

    /// Symbol name (empty for pattern replacements)
    pub symbol: String,

    /// Symbol kind (function, struct, class, etc. or "text" for patterns)
    pub kind: String,

    /// Byte offset start (inclusive)
    pub byte_start: usize,

    /// Byte offset end (exclusive)
    pub byte_end: usize,

    /// Line start (1-based)
    pub line_start: usize,

    /// Line end (1-based)
    pub line_end: usize,

    /// Column start (0-based, in bytes)
    pub col_start: usize,

    /// Column end (0-based, in bytes)
    pub col_end: usize,

    /// SHA-256 hash before modification
    pub before_hash: String,

    /// SHA-256 hash after modification
    pub after_hash: String,
}
```

### Field Descriptions

| Field | Type | Description |
|-------|------|-------------|
| `file_path` | String | Path to file containing the span |
| `symbol` | String | Symbol name (empty for non-symbol spans) |
| `kind` | String | Symbol kind or "text" for pattern replacements |
| `byte_start` | usize | Start byte offset (inclusive) |
| `byte_end` | usize | End byte offset (exclusive) |
| `line_start` | usize | Start line number (1-based) |
| `line_end` | usize | End line number (1-based) |
| `col_start` | usize | Start column (0-based) |
| `col_end` | usize | End column (0-based) |
| `before_hash` | String | SHA-256 hash before modification |
| `after_hash` | String | SHA-256 hash after modification |

### Line/Column Fields Implementation Note

**Phase 3 (Current):** Line and column fields will be set to placeholder values:
- `line_start`, `line_end`: Set to 0 or 1 for byte-only results
- `col_start`, `col_end`: Set to 0

**Phase 5 (Future):** These fields will be populated from tree-sitter parse data:
- Line numbers will be 1-based (first line is 1)
- Column numbers will be 0-based (first column is 0)
- Both will be in bytes (not characters)

This ensures backward compatibility while allowing future enhancement.

### JSON Example

```json
{
  "file_path": "src/lib.rs",
  "symbol": "my_function",
  "kind": "function",
  "byte_start": 1234,
  "byte_end": 1456,
  "line_start": 42,
  "line_end": 58,
  "col_start": 4,
  "col_end": 8,
  "before_hash": "a1b2c3d4e5f6789...",
  "after_hash": "f6e5d4c3b2a1098..."
}
```

---

## ErrorDetails Schema

Error reporting uses structured types for consistent error information.

### Type Definition

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ErrorDetails {
    /// Error kind identifier
    pub kind: &'static str,

    /// Human-readable error message
    pub message: String,

    /// Optional symbol context
    #[serde(skip_serializing_if = "Option::is_none")]
    pub symbol: Option<String>,

    /// Optional file context
    #[serde(skip_serializing_if = "Option::is_none")]
    pub file: Option<String>,

    /// Optional hint for remediation
    #[serde(skip_serializing_if = "Option::is_none")]
    pub hint: Option<String>,

    /// Optional diagnostics from validation tools
    #[serde(skip_serializing_if = "Option::is_none")]
    pub diagnostics: Option<Vec<DiagnosticPayload>>,
}
```

### JSON Example (Validation Failure)

```json
{
  "version": "2.0.0",
  "operation_id": "550e8400-e29b-41d4-a716-446655440000",
  "operation_type": "patch",
  "status": "error",
  "message": "Compiler validation failed",
  "timestamp": "2026-01-17T10:30:00Z",
  "workspace": "/home/user/project",
  "error": {
    "kind": "CompilerValidationFailed",
    "message": "cargo-check found 2 errors in src/lib.rs",
    "symbol": Some("my_function"),
    "file": Some("src/lib.rs"),
    "hint": Some("Fix compilation errors before retrying"),
    "diagnostics": [
      {
        "tool": "cargo-check",
        "level": "error",
        "message": "expected type, found `&str`",
        "file": Some("src/lib.rs"),
        "line": Some(42),
        "column": Some(10),
        "code": Some("E0308"),
        "note": None,
        "tool_path": Some("/usr/bin/cargo"),
        "tool_version": Some("cargo 1.75.0"),
        "remediation": Some("https://doc.rust-lang.org/error-index.html#E0308")
      }
    ]
  }
}
```

## DiagnosticPayload

Individual diagnostic messages from validation tools.

### Type Definition

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiagnosticPayload {
    /// Tool emitting the diagnostic (e.g., "cargo-check", "rust-analyzer")
    pub tool: String,

    /// Severity level ("error", "warning", "info")
    pub level: String,

    /// Diagnostic message
    pub message: String,

    /// Optional file path
    #[serde(skip_serializing_if = "Option::is_none")]
    pub file: Option<String>,

    /// Optional line number (1-based)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub line: Option<usize>,

    /// Optional column number (0-based)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub column: Option<usize>,

    /// Optional error code
    #[serde(skip_serializing_if = "Option::is_none")]
    pub code: Option<String>,

    /// Optional hint/help text
    #[serde(skip_serializing_if = "Option::is_none")]
    pub note: Option<String>,

    /// Optional absolute path to tool binary
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_path: Option<String>,

    /// Optional tool version string
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_version: Option<String>,

    /// Optional remediation link or text
    #[serde(skip_serializing_if = "Option::is_none")]
    pub remediation: Option<String>,
}
```

### JSON Example

```json
{
  "tool": "rust-analyzer",
  "level": "error",
  "message": "cannot find value `x` in this scope",
  "file": "src/lib.rs",
  "line": 42,
  "column": 10,
  "code": Some("E0425"),
  "note": Some("help: consider adding `let x = 42;` before this line"),
  "tool_path": Some("/usr/bin/rust-analyzer"),
  "tool_version": Some("rust-analyzer 1.75.0"),
  "remediation": Some("https://doc.rust-lang.org/error-index.html#E0425")
}
```

---

## Migration Strategy

### Phase 3.1: Design and Add Missing Serialize Derives

**Objective:** Prepare existing types for serialization

**Actions:**
- Add `#[derive(Serialize)]` to `FilePatchSummary` (src/patch/mod.rs:86)
- Add `#[derive(Serialize)]` to `ResolvedSpan` (src/resolve/mod.rs:17)
- Add `#[derive(Serialize)]` to `SpanReplacement` (src/patch/mod.rs:33)
- Add `version` field to all top-level structs

**Backward Compatibility:** Existing JSON output format remains unchanged

### Phase 3.2: Create Unified Output Types

**Objective:** Implement new structured output module

**Actions:**
- Create `src/output/mod.rs` module
- Implement `OperationResult` as top-level wrapper
- Implement `SpanResult` as unified span representation
- Implement all `OperationData` variants

**Backward Compatibility:** New module is additive, doesn't replace existing output

### Phase 3.3: Replace Ad-Hoc JSON

**Objective:** Eliminate `serde_json::Value` usage

**Actions:**
- Replace `CliSuccessPayload.data: Option<Value>` with `Option<OperationData>`
- Update CLI to use structured output types
- Add `--json` flag for structured output (default remains human-readable)

**Backward Compatibility:** Human-readable output remains default

### Future Phases

**Phase 4 (Execution Tracking):**
- Add `execution_id` field for operation chaining
- Add `match_id` for symbol disambiguation
- Add `span_id` for span tracking

**Phase 5 (Line/Column Population):**
- Populate `line_start`, `line_end`, `col_start`, `col_end` from tree-sitter
- Remove placeholder values (0/1)
- Update documentation to reflect accurate line/col data

---

## Mapping Old Outputs to New Schema

| Old Output | New Schema Equivalent | Notes |
|------------|----------------------|-------|
| `CliSuccessPayload` | `OperationResult` | Add version, operation_id, operation_type |
| `CliErrorPayload` | `OperationResult` with `error` field | Unify success/error under same wrapper |
| `PreviewReport` | `PatchResult` | Add symbol, kind, spans fields |
| `FilePatchSummary` | Fields in `SpanResult` | Merge into unified span representation |
| `ResolvedSpan` | `SpanResult` | Add hash fields for verification |
| `serde_json::Value` | Specific `OperationData` variants | Replace ad-hoc JSON with typed structs |

---

## References to Existing Code

This schema is based on existing structures in the codebase:

- **src/plan/mod.rs:13-37** - `Plan` and `PatchStep` structs (well-structured, uses serde)
- **src/patch/mod.rs:97-113** - `PreviewReport` (serializable, good reference)
- **src/patch/mod.rs:86-94** - `FilePatchSummary` (missing Serialize derive)
- **src/resolve/mod.rs:17-51** - `ResolvedSpan` (not serializable, placeholder line/col fields)
- **src/cli/mod.rs:310-318** - `CliSuccessPayload` (uses `Value`, needs replacement)
- **src/cli/mod.rs:342-348** - `CliErrorPayload` (well-structured error handling)
- **src/cli/mod.rs:408-440** - `DiagnosticPayload` (complete diagnostic representation)

---

## Version History

| Version | Date | Changes |
|---------|------|---------|
| 2.0.0 | 2026-01-17 | Initial unified schema design |
