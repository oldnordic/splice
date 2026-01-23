# Phase 18: Error Code Integration - Context

**Gathered:** 2026-01-23
**Status:** Ready for planning

<domain>
## Phase Boundary

Attach SPL-E### error codes to all CLI error responses. Error code infrastructure exists (28 codes, `with_error_code()` builder, `splice explain` command) but is not being called from CLI error paths. This phase wires the infrastructure into all CLI error sites.

**Out of scope:** Adding new error types, changing error handling architecture, modifying error code definitions.
</domain>

<decisions>
## Implementation Decisions

### Error code granularity
- Only error-level failures get SPL-E### codes
- Warnings and notes do NOT get error codes
- All 22 error-level SPL-E### variants must be covered across CLI
- Prioritize common errors (SymbolNotFound, ParseFailed, FileNotFound) but complete all variants

### Implementation approach
- Use wrapper function pattern (not direct calls or constructor pattern)
- Single centralized wrapper function in src/error.rs (not per-command wrappers)
- Wrapper function takes SpliceError, determines appropriate code, returns enriched error
- Fail fast if error code attachment fails (no graceful degradation)

### Backward compatibility
- error_code field remains optional in JSON output (using `#[serde(skip_serializing_if = "Option::is_none")]`)
- Only present when error has a code attached
- Maintains compatibility with existing JSON consumers

### Error explain integration
- Error responses auto-reference the explain command
- Reference format: machine-readable field (explain_command: "splice explain SPL-E001")
- Auto-add hint to human-readable error messages pointing to explain command

### Claude's Discretion
- Exact name and signature of wrapper function
- Whether to use match statement or lookup table for code determination
- Exact format of human-readable hint message
- Test coverage approach for error code verification

</decisions>

<specifics>
## Specific Ideas

- Error code infrastructure already exists: src/error_codes.rs with 28 variants
- SpliceError::with_error_code() method exists but has 0 CLI calls
- splice explain command already works with get_error_explanation()
- Need to find all SpliceError::to_error() call sites in CLI and wrap with error code attachment
- Audit report: "Error codes infrastructure exists but 0 calls in CLI" - need to change this

</specifics>

<deferred>
## Deferred Ideas

None — discussion stayed within phase scope

</deferred>

---

*Phase: 18-error-code-integration*
*Context gathered: 2026-01-23*
