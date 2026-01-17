# Plan 03-03 Summary: Integrate Structured Output into CLI

**Status:** COMPLETED (2026-01-17)
**Tasks Completed:** 7/8 (plan/apply/query commands deferred)

## Changes Made

### 1. CLI Infrastructure

**File:** `src/cli/mod.rs`

Added global `--json` flag to enable structured JSON output mode:
- Added `json: bool` field to `Cli` struct (private)
- Added `json_output()` getter method
- Flag is global (available for all subcommands)
- Backward compatible: default output unchanged

### 2. Payload Structure

**File:** `src/cli/mod.rs`

Enhanced `CliSuccessPayload` to support dual output modes:
- Added `already_emitted: bool` field (marked with `#[serde(skip)]`)
- Added `already_emitted()` builder method
- When `already_emitted=true`, emit functions skip output
- Allows execute functions to directly emit OperationResult

### 3. Main Function Updates

**File:** `src/main.rs`

Updated command execution flow:
- Extract `json_output` flag from CLI
- Pass to all execute functions
- Updated `emit_success_payload()` and `emit_error_payload()` signatures
- All execute functions now accept `json_output: bool` parameter

### 4. Patch Command Structured Output

**File:** `src/main.rs` - `execute_patch()`

Implemented structured output for `patch --json`:
```json
{
  "version": "2.0.0",
  "operation_id": "uuid",
  "operation_type": "patch",
  "status": "ok",
  "message": "...",
  "timestamp": "2026-01-17T...",
  "workspace": "/path/to/workspace",
  "result": {
    "type": "patch",
    "file": "src/lib.rs",
    "symbol": "my_function",
    "kind": "function",
    "spans": [...],
    "before_hash": "...",
    "after_hash": "...",
    "lines_added": 0,
    "lines_removed": 0
  }
}
```

**Implementation Details:**
- Creates `SpanResult` from byte span with symbol metadata
- Calculates file hashes before/after
- Uses `OperationResult::new()` builder pattern
- Outputs directly via `println!` when `json_output=true`
- Marks payload as `already_emitted()` to prevent double output

**TODOs:**
- `lines_added` and `lines_removed` are hardcoded to 0
- Requires diff calculation (deferred to future phase)

### 5. Delete Command Structured Output

**File:** `src/main.rs` - `execute_delete()`

Implemented structured output for `delete --json`:
```json
{
  "version": "2.0.0",
  "operation_type": "delete",
  "result": {
    "type": "delete",
    "file": "src/lib.rs",
    "symbol": "old_function",
    "kind": "function",
    "spans": [...],
    "bytes_removed": 123,
    "lines_removed": 0,
    "references_removed": 3
  }
}
```

**Implementation Details:**
- Creates `SpanResult` for each reference + definition
- Calculates total `bytes_removed` from all spans
- Tracks `references_removed` count
- Supports glob ambiguity warning in message

**TODOs:**
- `lines_removed` is hardcoded to 0
- Requires diff calculation (deferred to future phase)

### 6. Chrono Dependency

**File:** `Cargo.toml`

Updated chrono dependency for ISO 8601 timestamps:
```toml
chrono = { version = "0.4", default-features = false, features = ["std", "clock"] }
```
- Used by `OperationResult::new()` for timestamp generation
- Minimal feature set for smaller binary size

### 7. Other Commands

**Status:** DEFERRED

Structured output for following commands not implemented:
- `plan` - Requires plan execution tracking
- `apply-files` - Requires pattern matching results
- `query` - Read-only, lower priority
- `get` - Read-only, lower priority
- `undo` - Low priority

These can be added in future phases following the same pattern.

## Verification

### Test Suite

All 111 tests pass:
```bash
cargo test --lib
# test result: ok. 111 passed; 0 failed
```

### CLI Verification

```bash
cargo build --release
./target/release/splice patch --help | grep json
# Output: --json - Output structured JSON (default: human-readable)
```

## Backward Compatibility

**Default behavior unchanged:**
- Without `--json` flag, outputs existing JSON format
- All existing scripts/automation continue to work
- `CliSuccessPayload` structure unchanged for default mode

**New opt-in behavior:**
- `--json` flag enables structured schema output
- Conforms to SCHEMA.md v2.0.0
- Includes all fields from specification

## Architecture Decisions

### Decision 1: Dual Output Paths

**Approach:** Execute functions output `OperationResult` directly when `json_output=true`

**Alternatives Considered:**
- Convert `CliSuccessPayload` to `OperationResult` in emit layer
- Return `Result<OperationResult>` instead of `CliSuccessPayload`
- Create separate "structured output" functions

**Rationale:**
- Minimal changes to existing code paths
- Backward compatible with existing behavior
- Clear separation: structured path vs legacy path

**Trade-offs:**
- Uses `println!` directly inside execute functions
- Requires `already_emitted` flag to prevent double output
- Slightly more complex control flow

### Decision 2: already_emitted Flag

**Approach:** Add boolean field to `CliSuccessPayload` to track emission

**Alternatives Considered:**
- Return `Option<CliSuccessPayload>` (None = already emitted)
- Create new enum `OutputPayload`
- Use different Result type

**Rationale:**
- Simple to implement
- Zero changes to success/error handling in main()
- Easy to understand and maintain

## Deviations from SCHEMA.md

1. **Line/Column Placeholders**: `line_start`, `line_end`, `col_start`, `col_end` are 0
   - Phase 5 will populate these from AST
   - Documented in SCHEMA.md as migration strategy

2. **TODO Fields**: `lines_added`, `lines_removed` are hardcoded to 0
   - Requires diff calculation implementation
   - Noted in code comments

3. **Missing Commands**: plan, apply-files, query, get, undo
   - Not implemented in this plan
   - Can be added incrementally

## Phase 3 Status

**Phase 3: Structured Output** - 3/4 COMPLETE
- ✅ 03-01: Design unified output schema (SCHEMA.md)
- ✅ 03-02: Implement structured output types (src/output.rs)
- ✅ 03-03: Integrate structured output into CLI
- ⏭️ 03-04: Documentation and examples (deferred)

**Next Steps:**
- Phase 4: Multi-language support
- Phase 5: Line/column information from AST
- Add structured output to remaining commands (as needed)

## Example Usage

### Patch Command with JSON Output

```bash
splice patch --file src/lib.rs --symbol old_func \
  --with new_func.rs --json
```

**Output:**
```json
{
  "version": "2.0.0",
  "operation_id": "550e8400-e29b-41d4-a716-446655440000",
  "operation_type": "patch",
  "status": "ok",
  "message": "Patched 'old_func' at bytes 1234..1456 (hash: abc -> def)",
  "timestamp": "2026-01-17T18:30:00Z",
  "workspace": "/home/user/project",
  "result": {
    "type": "patch",
    "file": "src/lib.rs",
    "symbol": "old_func",
    "kind": "function",
    "spans": [
      {
        "file_path": "src/lib.rs",
        "symbol": "old_func",
        "kind": "function",
        "byte_start": 1234,
        "byte_end": 1456,
        "line_start": 0,
        "line_end": 0,
        "col_start": 0,
        "col_end": 0,
        "before_hash": "abc123",
        "after_hash": "def456"
      }
    ],
    "before_hash": "abc123",
    "after_hash": "def456",
    "lines_added": 0,
    "lines_removed": 0
  }
}
```

## Files Modified

1. `src/cli/mod.rs` - Added --json flag, already_emitted field
2. `src/main.rs` - Pass json_output through, emit OperationResult
3. `Cargo.toml` - Updated chrono dependency

## Commit History

- `4d1edfa` feat(cli): add --json global flag for structured output
- `2ce9ea3` feat(deps): update chrono dependency with explicit features
- `92cdd46` feat(main): add json_output parameter to all execute functions
- `c878d6a` feat(cli): implement structured JSON output for patch command
- `138af46` feat(cli): implement structured JSON output for delete command

## Lessons Learned

1. **Backward Compatibility First**: Keeping existing behavior untouched prevents breaking changes
2. **Minimal Mutation**: Adding flags and parameters is safer than changing return types
3. **Test-Driven**: All 111 tests passing confirms no regressions
4. **Incremental Implementation**: Implementing high-value commands (patch/delete) first
