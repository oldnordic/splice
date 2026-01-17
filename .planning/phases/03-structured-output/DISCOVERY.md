# Phase 3 Discovery: Structured Output Schema

**Date:** 2026-01-17
**Discovery Level:** Level 2 (Standard Research)
**Status:** Complete

---

## Objective

Understand the current JSON output structures in Splice and design a unified, structured output schema that provides explicit fields for all operations.

---

## Current State Analysis

### Existing Structured Outputs

The codebase already has several structured output types using `serde::Serialize`:

#### 1. **Plan Module** (`src/plan/mod.rs`)
```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Plan {
    pub steps: Vec<PatchStep>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PatchStep {
    pub file: String,
    pub symbol: String,
    #[serde(rename = "kind")]
    pub symbol_kind: Option<String>,
    #[serde(rename = "with")]
    pub with_file: String,
}
```
**Status:** Well-structured, uses serde, explicit field names with serde rename.

#### 2. **Patch Module** (`src/patch/mod.rs`)
```rust
#[derive(Debug, Clone, Serialize)]
pub struct PreviewReport {
    pub file: String,
    pub line_start: usize,
    pub line_end: usize,
    pub lines_added: usize,
    pub lines_removed: usize,
    pub bytes_added: usize,
    pub bytes_removed: usize,
}

#[derive(Debug, Clone)]
pub struct FilePatchSummary {
    pub file: PathBuf,
    pub before_hash: String,
    pub after_hash: String,
}
```
**Status:** `PreviewReport` is serializable, `FilePatchSummary` is NOT serializable (missing `#[derive(Serialize)]`).

#### 3. **Resolve Module** (`src/resolve/mod.rs`)
```rust
#[derive(Debug, Clone, PartialEq)]
pub struct ResolvedSpan {
    pub node_id: NodeId,
    pub name: String,
    pub kind: String,
    pub language: Option<String>,
    pub file_path: String,
    pub byte_start: usize,
    pub byte_end: usize,
    pub line_start: usize,   // TODO: Always 0, not yet implemented
    pub line_end: usize,     // TODO: Always 0, not yet implemented
    pub col_start: usize,    // TODO: Always 0, not yet implemented
    pub col_end: usize,      // TODO: Always 0, not yet implemented
}
```
**Status:** NOT serializable, has placeholder fields for line/col.

#### 4. **CLI Module** (`src/cli/mod.rs`)
```rust
#[derive(Serialize)]
pub struct CliSuccessPayload {
    pub status: &'static str,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<Value>,
}

#[derive(Serialize)]
pub struct CliErrorPayload {
    pub status: &'static str,
    pub error: ErrorDetails,
}

#[derive(Serialize)]
pub struct DiagnosticPayload {
    pub tool: String,
    pub level: String,
    pub message: String,
    // ... multiple optional fields
}
```
**Status:** Well-structured CLI payloads with proper serde annotations.

### Ad-Hoc JSON Patterns

Several modules use `serde_json::Value` for flexible JSON:
- CLI `CliSuccessPayload.data` field is `Option<Value>` - allows arbitrary JSON
- Pattern replacement results use informal structures
- Some outputs use println! with manual JSON construction

---

## Design Principles for Unified Schema

Based on SQLiteGraph v1.0 patterns and existing codebase conventions:

### 1. **Explicit Fields Over Ad-Hoc JSON**
- Avoid `serde_json::Value` in favor of typed structs
- Use `#[serde(skip_serializing_if = "Option::is_none")]` for optional fields
- Explicit field names with `#[serde(rename = "...")]` where needed

### 2. **Consistent Field Naming**
- Use snake_case for field names (Rust convention)
- Use serde rename for CLI-facing names if needed (e.g., `with` -> `with_file`)
- Consistent boolean names: `is_*`, `has_*`, `can_*`

### 3. **Versioning Consideration**
- Add `version` field to top-level output structs
- Use semver for schema versioning (e.g., "2.0.0")

### 4. **Span Representation**
- Byte offsets: `byte_start`, `byte_end` (usize)
- Line/column: `line_start`, `line_end`, `col_start`, `col_end` (usize)
- File path: `file_path` (String) or `file` (PathBuf)

---

## Recommended Schema Structure

### Top-Level Operation Result

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OperationResult {
    /// Schema version
    pub version: String,

    /// Unique operation ID (UUID)
    pub operation_id: String,

    /// Operation type ("patch", "delete", "plan", etc.)
    pub operation_type: String,

    /// Status ("ok", "error", "partial")
    pub status: String,

    /// Human-readable message
    pub message: String,

    /// Timestamp (ISO 8601)
    pub timestamp: String,

    /// Workspace root
    pub workspace: String,

    /// Primary result data
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result: Option<OperationData>,

    /// Error details if status is "error"
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<ErrorDetails>,
}
```

### Operation Data Variants

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
}
```

### Span Result (unified)

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpanResult {
    /// File path
    pub file_path: String,

    /// Symbol name
    pub symbol: String,

    /// Symbol kind
    pub kind: String,

    /// Byte span
    pub byte_start: usize,
    pub byte_end: usize,

    /// Line/column (1-based line, 0-based col)
    pub line_start: usize,
    pub line_end: usize,
    pub col_start: usize,
    pub col_end: usize,

    /// Hashes for verification
    pub before_hash: String,
    pub after_hash: String,
}
```

---

## Migration Strategy

### Phase 3.1: Design and Add Missing Serialize Derives
- Add `#[derive(Serialize)]` to `FilePatchSummary`, `ResolvedSpan`, `SpanReplacement`
- Add version field to all top-level structs

### Phase 3.2: Create Unified Output Types
- Create new `output` module with structured types
- Implement `OperationResult` as top-level wrapper
- Create `SpanResult` as unified span representation

### Phase 3.3: Replace Ad-Hoc JSON
- Replace `serde_json::Value` uses with typed structs
- Update CLI to use structured output
- Add JSON output mode (currently output is mixed)

---

## Dependencies on Previous Phases

- **Phase 1 (Safety Foundation):** Error handling patterns established, use `SpliceError` for conversion failures
- **Phase 2 (SQLiteGraph v1.0):** Graph types stable, can use `NodeId` in output schemas

---

## Recommendations

1. **Keep backward compatibility** where possible - add new structured outputs alongside existing
2. **Use serde rename** to maintain existing JSON field names where already published
3. **Add tests** for serialization/deserialization of all new types
4. **Document schema** in a separate SCHEMA.md file for external consumers
5. **Consider JSON Schema** generation for tool integration

---

## Next Steps

1. Plan 03-01: Design final output schema structure
2. Plan 03-02: Implement structured output types
3. Plan 03-03: Replace ad-hoc JSON with structured schema

