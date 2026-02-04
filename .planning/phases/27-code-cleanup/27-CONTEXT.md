# Phase 27: Code Cleanup - Context

**Gathered:** 2026-02-04
**Status:** Ready for planning

## Phase Boundary

Remove dead code from when Splice did its own indexing. Since Magellan now handles indexing and Splice is a consumer, the old `Ingestor` struct and related vestigial code should be removed. This is technical cleanup, not a capability change.

## Implementation Decisions

### Removal approach
- Piece by piece removal — not a single big delete
- Remove incrementally, testing after each part
- This ensures we can identify what breaks if something goes wrong

### Validation method
- Both approaches:
  1. Reference check — search for references before removing anything
  2. Test after — run `cargo test` after each removal step
- Use `magellan refs` or `grep` to find references before deletion

### Documentation
- Add CHANGELOG entry documenting the removal
- Transparent about what was removed and why
- It was dead code, but users should know what changed

### Scope
- Primary: Remove dead `Ingestor` struct stub from `src/ingest/mod.rs:42-68`
- Secondary: Clean up any directly related vestigial code
- **Out of scope:** Unused imports sweeping, dependency pruning (those are separate efforts)

### Claude's Discretion
- What counts as "directly related" to the Ingestor stub
- Exact order of removal steps
- Whether to add deprecation notices (probably not for dead code)

## Specific Ideas

From CONTEXT.md:
> **Why it exists**: Original design vision where Splice would have its own `CodeGraph`.
> **Why it was abandoned**: Realized Magellan already has a perfectly good graph. Implemented `MagellanIngestor` instead.

The `Ingestor` struct at `src/ingest/mod.rs:42-68` contains:
```rust
pub struct Ingestor {
    _graph: CodeGraph,
}

impl Ingestor {
    pub fn ingest_file(&mut self, _path: &Path) -> Result<()> {
        // TODO: Implement in Task 1
        Err("Not implemented yet")  // ← DEAD CODE
    }
}
```

What to use instead: `MagellanIngestor` or `extract_symbols()` directly.

## Deferred Ideas

- Unused imports cleanup — separate pass, not this phase
- Dependency pruning — separate pass, not this phase
- Resume mode improvements — belongs in a future phase (workflow enhancement)

---

*Phase: 27-code-cleanup*
*Context gathered: 2026-02-04*
