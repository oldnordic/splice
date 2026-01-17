# Phase 4 Discovery: Stable Identifiers

**Date:** 2026-01-17
**Status:** ✅ COMPLETE

---

## Overview

Phase 4 aims to add stable identifiers (execution_id, match_id, span_id) to all operations for traceability. This discovery analyzes current ID usage patterns and identifies where new identifiers need to be integrated.

---

## Current State Analysis

### 1. UUID Generation (Already Available)

**Location:** `src/output.rs:38-43`

```rust
pub fn new(operation_type: String) -> Self {
    use uuid::Uuid;

    Self {
        version: SCHEMA_VERSION.to_string(),
        operation_id: Uuid::new_v4().to_string(),
        ...
    }
}
```

**Status:** ✅ UUID generation is already working
- `uuid` crate is in dependencies (v1.10 with v4 feature)
- `OperationResult::new()` generates a new `operation_id` automatically
- No changes needed to dependencies

### 2. operation_id in Structured Output

**Location:** `src/output.rs:12-16`

```rust
pub struct OperationResult {
    pub version: String,
    /// Unique operation ID (UUID)
    pub operation_id: String,
    ...
}
```

**Status:** ✅ `operation_id` field exists and is populated

**Issue:** In `src/main.rs:655`, the CLI generates its own `_op_id` but doesn't propagate it to `OperationResult`:
```rust
let _op_id = operation_id.clone().unwrap_or_else(|| Uuid::new_v4().to_string());
let result = OperationResult::new("patch".to_string())  // Generates NEW UUID, ignores _op_id
```

**Fix Required:** Pass `operation_id` to `OperationResult::new()` or add a setter.

### 3. match_id - New Field

**Purpose:** Unique ID for each symbol resolution attempt

**Where Needed:**
- `src/resolve/mod.rs:resolve_symbol()` - When a symbol is resolved
- Should be included in `SpanResult` to track which resolution produced this span

**Current State:** Does not exist
- ResolvedSpan has `node_id` but it's from SQLiteGraph and not serialized
- No tracking of individual resolution attempts

### 4. span_id - New Field

**Purpose:** Unique ID for individual spans within an operation

**Where Needed:**
- Each `SpanResult` should have a unique ID
- Enables tracking spans through logs and audit trails

**Current State:** Does not exist
- Spans are identified by `(file_path, byte_start, byte_end)` tuple
- No stable identifier that survives across operations

---

## Symbol Resolution Flow

### Current Flow (Patch Command)

```
1. execute_patch() called
2. extract_symbols_with_language() - extracts symbols from source
3. CodeGraph::open() - opens/creates graph database
4. code_graph.store_symbol_with_file_and_language() - stores symbols
5. resolve_symbol() - resolves symbol name to ResolvedSpan
6. apply_patch_with_validation() - applies the patch
7. OperationResult created with generated operation_id
```

**Observation:** The `resolve_symbol()` step is where `match_id` should be generated.

### Current Flow (Delete Command)

```
1. execute_delete() called
2. extract_symbols_with_language() - extracts symbols
3. CodeGraph::open() - opens graph database
4. find_references() - finds all references to symbol
5. For each reference: apply_patch_with_validation()
6. Definition deleted last
7. OperationResult created with generated operation_id
```

**Observation:** Multiple spans need `span_id` in this case.

---

## ID Generation Points

| ID Type | Generation Point | Scope | Notes |
|---------|-----------------|-------|-------|
| `execution_id` | CLI entry point | Entire operation | Same as current `operation_id` |
| `match_id` | `resolve_symbol()` | Per symbol resolution | NEW - one per resolved symbol |
| `span_id` | `SpanResult::from_byte_span()` | Per span | NEW - one per SpanResult |

---

## Dependencies Analysis

### Existing Dependencies

✅ `uuid = { version = "1.10", features = ["v4"] }` - Already in Cargo.toml
✅ `chrono = { version = "0.4", default-features = false, features = ["std", "clock"] }` - Already in Cargo.toml

**No new dependencies required.**

---

## Code Locations to Modify

### 1. src/output.rs (383 lines)

**Changes Needed:**
- Add `match_id: Option<String>` to `SpanResult`
- Add `span_id: String` to `SpanResult`
- Add `with_match_id()` helper to `SpanResult`
- Modify `SpanResult::from_byte_span()` to generate `span_id`
- Add `set_operation_id()` to `OperationResult` to override auto-generated ID

### 2. src/resolve/mod.rs (265 lines)

**Changes Needed:**
- Modify `resolve_symbol()` to accept/generate `match_id`
- Return `match_id` along with `ResolvedSpan`
- Update signature to return `(ResolvedSpan, String)` or add field to struct

### 3. src/main.rs (~1448 lines)

**Changes Needed:**
- **Patch command (line 629-673):** Propagate `operation_id` to `OperationResult`, add `match_id` from resolution
- **Delete command (line 392-437):** Propagate `operation_id` to `OperationResult`, generate `match_id` for each span
- **Batch command (line 722-854):** Generate unique `execution_id` for batch operations
- **Plan command (line 863-881):** Generate `execution_id` for plan execution
- **ApplyFiles command (line 923-1007):** Generate `execution_id` for pattern operations

---

## Recommended Implementation Strategy

### Plan 04-01: ID Generation Utilities

**Goal:** Implement core ID generation and storage

**Tasks:**
1. Add `match_id` and `span_id` fields to `SpanResult`
2. Add `set_operation_id()` method to `OperationResult`
3. Add `with_match_id()` method to `SpanResult`
4. Modify `SpanResult::from_byte_span()` to auto-generate `span_id`
5. Write unit tests for ID generation

**Files:**
- `src/output.rs` - Modify SpanResult, add helper methods

### Plan 04-02: execution_id Integration

**Goal:** Ensure operation_id is consistently used across all operations

**Tasks:**
1. Modify `OperationResult::new()` to accept optional `operation_id` parameter
2. Update all CLI commands to pass `operation_id` through to `OperationResult`
3. Ensure `operation_id` from CLI flag takes precedence over auto-generated
4. Update execute_patch, execute_delete, execute_apply_files

**Files:**
- `src/output.rs` - Modify OperationResult::new()
- `src/main.rs` - Update all execute functions

### Plan 04-03: match_id and span_id Population

**Goal:** Add symbol resolution and span identifiers

**Tasks:**
1. Modify `resolve_symbol()` to generate and return `match_id`
2. Update resolution callers to capture and propagate `match_id`
3. Ensure all `SpanResult` instances have unique `span_id`
4. Add identifiers to structured JSON output

**Files:**
- `src/resolve/mod.rs` - Return match_id from resolution
- `src/main.rs` - Capture and use match_id

---

## Open Questions

1. **match_id Scope:** Should `match_id` be unique per symbol resolution attempt, or should it be deterministic for the same symbol query?
   - **Recommendation:** Unique per resolution attempt (UUID) for audit trail

2. **span_id Format:** Should `span_id` be a UUID or a composite ID like `{execution_id}-{span_index}`?
   - **Recommendation:** UUID for consistency, easier to generate

3. **Backward Compatibility:** How to handle existing code that doesn't use these IDs?
   - **Recommendation:** All fields are `Option<String>`, defaults to `None` or generated UUID

---

## Summary

**Current State:**
- ✅ UUID generation infrastructure exists
- ✅ `operation_id` in `OperationResult` (but not propagated from CLI)
- ❌ `match_id` does not exist
- ❌ `span_id` does not exist

**Required Changes:**
1. Add `match_id` and `span_id` to `SpanResult` (src/output.rs)
2. Add `set_operation_id()` to `OperationResult` (src/output.rs)
3. Modify `resolve_symbol()` to generate `match_id` (src/resolve/mod.rs)
4. Update all CLI commands to propagate IDs (src/main.rs)

**No new dependencies needed.**

**Estimated Complexity:** Low - straightforward field additions and propagation
