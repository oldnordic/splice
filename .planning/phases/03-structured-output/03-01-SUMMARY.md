# Plan 03-01 Summary: Design Unified Output Schema

**Date:** 2026-01-17
**Status:** ✅ COMPLETE
**Commit:** 1f722e5

---

## Overview

Designed and documented a comprehensive, unified output schema for Splice v2.0 that provides explicit fields for all operations with consistent naming patterns and versioning support.

---

## Deliverables

### SCHEMA.md Created

**Location:** `/home/feanor/Projects/splice/.planning/phases/03-structured-output/SCHEMA.md`

**Size:** 816 lines

**Content:**
- Complete type definitions for all output structures
- Field descriptions and types for every struct
- JSON examples for each operation type
- Migration strategy with timeline
- Mapping from old outputs to new schema
- References to existing code (file:line)

---

## Types Defined

### Core Types (5 total)

1. **OperationResult** - Top-level wrapper for all operations
   - Fields: version, operation_id, operation_type, status, message, timestamp, workspace, result, error

2. **OperationData** - Tagged enum with 5 variants
   - Patch(PatchResult)
   - Delete(DeleteResult)
   - Plan(PlanResult)
   - Query(QueryResult)
   - ApplyFiles(ApplyFilesResult)

3. **SpanResult** - Unified span representation
   - Fields: file_path, symbol, kind, byte_start, byte_end, line_start, line_end, col_start, col_end, before_hash, after_hash

4. **ErrorDetails** - Structured error reporting
   - Fields: kind, message, symbol, file, hint, diagnostics

5. **DiagnosticPayload** - Validation tool diagnostics
   - Fields: tool, level, message, file, line, column, code, note, tool_path, tool_version, remediation

### Supporting Types (4 total)

6. **PatchResult** - Single file patch result
7. **DeleteResult** - Symbol deletion result
8. **PlanResult** - Multi-step plan execution result
9. **QueryResult** - Magellan query result
10. **ApplyFilesResult** - Pattern replacement result
11. **StepResult** - Individual plan step result
12. **FilePatternResult** - Individual file pattern result

**Total:** 12 types defined

---

## Examples Created

### JSON Examples (7 total)

1. **Successful Patch** - Complete OperationResult with PatchResult
2. **PatchResult** - Detailed patch operation result
3. **DeleteResult** - Symbol deletion with multiple spans
4. **PlanResult** - 3-step plan execution
5. **QueryResult** - Label-based symbol search
6. **ApplyFilesResult** - Pattern replacement across files
7. **Validation Failure** - ErrorDetails with diagnostics

---

## Design Decisions

### 1. Explicit Fields Over Ad-Hoc JSON

**Decision:** Avoid `serde_json::Value` in favor of typed structs
**Rationale:** Type safety, IDE support, documentation generation
**Implementation:** All result types use explicit struct fields

### 2. Consistent snake_case Naming

**Decision:** Use snake_case for field names with serde rename where needed
**Rationale:** Rust convention, CLI compatibility via serde(rename)
**Implementation:** `symbol_kind` -> `kind` in JSON via serde(rename)

### 3. Version Field on Top-Level Structs

**Decision:** Add `version` field using semver format
**Rationale:** Schema evolution, consumer compatibility
**Implementation:** `"2.0.0"` format for current schema

### 4. Unified Span Representation

**Decision:** Single `SpanResult` type for all spans
**Rationale:** Consistency, reusability, simplified consumer code
**Implementation:** All operations use the same span structure

### 5. Line/Column Placeholder Strategy

**Decision:** Set line/col to 0/1 in Phase 3, populate in Phase 5
**Rationale:** Backward compatibility, phased implementation
**Implementation:** Documented in SpanResult section with phase notes

---

## Deviations from DISCOVERY.md

None. The implementation follows the DISCOVERY.md recommendations exactly:

✅ Explicit fields over `Value`
✅ Consistent snake_case naming
✅ Versioning on top-level structs
✅ Unified span representation
✅ Backward compatibility strategy
✅ References to existing code

---

## Migration Strategy

### Phase 3.1: Add Missing Serialize Derives
- Add `#[derive(Serialize)]` to `FilePatchSummary`
- Add `#[derive(Serialize)]` to `ResolvedSpan`
- Add `#[derive(Serialize)]` to `SpanReplacement`
- Add `version` field to top-level structs

### Phase 3.2: Create Unified Output Types
- Create `src/output/mod.rs` module
- Implement `OperationResult` wrapper
- Implement `SpanResult` unified representation
- Implement all `OperationData` variants

### Phase 3.3: Replace Ad-Hoc JSON
- Replace `CliSuccessPayload.data: Option<Value>` with `Option<OperationData>`
- Update CLI to use structured output
- Add `--json` flag for structured output mode

### Future Enhancements
- **Phase 4:** Add execution tracking (execution_id, match_id, span_id)
- **Phase 5:** Populate line/col fields from tree-sitter

---

## Code References

The schema is based on analysis of existing code:

- **src/plan/mod.rs:13-37** - Plan, PatchStep (well-structured)
- **src/patch/mod.rs:97-113** - PreviewReport (serializable)
- **src/patch/mod.rs:86-94** - FilePatchSummary (needs Serialize)
- **src/resolve/mod.rs:17-51** - ResolvedSpan (needs Serialize, has placeholders)
- **src/cli/mod.rs:310-318** - CliSuccessPayload (uses Value, needs replacement)
- **src/cli/mod.rs:342-348** - CliErrorPayload (well-structured)
- **src/cli/mod.rs:408-440** - DiagnosticPayload (complete)

---

## Success Criteria

✅ SCHEMA.md created with complete type definitions
✅ All top-level types have field descriptions and types
✅ JSON examples for each operation type
✅ Migration strategy documented with timeline
✅ References to existing code (file:line) included

---

## Next Steps

**Plan 03-02:** Implement structured output types
- Create `src/output/mod.rs` module
- Implement OperationResult and all variants
- Add Serialize derives to existing types
- Write unit tests for serialization/deserialization

---

## Files Modified

- **Created:** `.planning/phases/03-structured-output/SCHEMA.md` (816 lines)
- **Created:** `.planning/phases/03-structured-output/03-01-SUMMARY.md` (this file)

---

## Commit Details

**Hash:** 1f722e5
**Message:** feat(03-01): design unified output schema for Splice v2.0
**Files Changed:** 1 file, 816 insertions(+)
