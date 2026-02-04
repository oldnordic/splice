# Phase 27: Code Cleanup - Research

**Researched:** 2026-02-04
**Domain:** Dead code removal, technical debt cleanup
**Confidence:** HIGH

## Summary

Phase 27 removes vestigial code from Splice's early design phase. The `Ingestor` struct at `src/ingest/mod.rs:42-68` (27 lines) is dead code—an abandoned design where Splice would implement its own CodeGraph-based indexing. This approach was abandoned when the project integrated Magellan, which already provides a production-quality multi-language code graph. The dead struct has been replaced by `MagellanIngestor` and the `extract_symbols()` API.

Research confirms this is a straightforward cleanup with no functional changes. The dead code has no references in the active codebase (grep search returns zero matches for `Ingestor::` usage, excluding `MagellanIngestor`). The safe removal approach is incremental: verify no references exist with grep/magellan refs, remove the struct, run `cargo test` to confirm no breakage, and document the removal in CHANGELOG.md.

**Primary recommendation:** Remove the 27-line dead `Ingestor` struct and its impl block from `src/ingest/mod.rs:42-68` in a single atomic edit, verify with `cargo test`, and update documentation to clarify that `MagellanIngestor` and `extract_symbols()` are the correct ingestion APIs.

## Standard Stack

### Core
| Tool | Version | Purpose | Why Standard |
|------|---------|---------|--------------|
| `grep` / `rg` | system | Find references before removal | Standard code search, no dependencies |
| `cargo test` | 1.92 | Verify no breakage after removal | Rust's built-in test harness |
| `cargo check` | 1.92 | Fast compilation check | Catchs type errors before full test run |

### Supporting
| Tool | Version | Purpose | When to Use |
|------|---------|---------|-------------|
| `magellan refs` | 0.5+ | Cross-reference checking | If codebase has Magellan index (optional) |

### Alternatives Considered
| Instead of | Could Use | Tradeoff |
|------------|-----------|----------|
| Direct removal | Incremental removal | Direct removal is safe here; incremental only needed for complex interdependencies |
| grep | IDE search | grep is scriptable and produces audit trail |

**Installation:** No new dependencies required. All tools are already in the development environment.

## Architecture Patterns

### Dead Code Pattern
**What:** A struct or function that is defined but never called, typically from an abandoned design direction.

**When it's safe to remove:**
- No references exist in the codebase (verified with grep)
- No tests reference the code
- No documentation recommends using it
- An alternative implementation exists

**Example from this codebase:**
```rust
// Source: /home/feanor/Projects/splice/src/ingest/mod.rs:42-68
// DEAD CODE - To be removed in Phase 27

pub struct Ingestor {
    _graph: CodeGraph,
}

impl Ingestor {
    pub fn new(graph: CodeGraph) -> Self {
        Self { _graph: graph }
    }

    pub fn ingest_file(&mut self, _path: &Path) -> Result<()> {
        Err(crate::error::SpliceError::Other(
            "Not implemented yet".to_string(),
        ))
    }

    pub fn ingest_dir(&mut self, _path: &Path) -> Result<()> {
        Err(crate::error::SpliceError::Other(
            "Not implemented yet".to_string(),
        ))
    }
}
```

**What to use instead:**
```rust
// Option 1: Magellan-based ingestion
use splice::ingest::magellan::MagellanIngestor;
let mut ingestor = MagellanIngestor::new(&db_path)?;
ingestor.index_file(&file_path)?;

// Option 2: Direct symbol extraction
use splice::ingest::dispatch::extract_symbols;
let symbols = extract_symbols(&file_path, &source)?;
```

### Removal Verification Pattern
**What:** A systematic process to verify dead code can be safely removed.

**Steps:**
1. **Reference check** - Search for all usages:
   ```bash
   grep -r "Ingestor" src/ --include="*.rs" | grep -v "MagellanIngestor"
   # Expected: No results (confirmed dead)
   ```

2. **Test check** - Verify tests don't reference it:
   ```bash
   grep -r "Ingestor" tests/ --include="*.rs"
   # Expected: No results in tests
   ```

3. **Documentation check** - Verify docs don't recommend it:
   ```bash
   grep -r "Ingestor" docs/ --include="*.md"
   # Expected: Only mentions as "dead code" or "abandoned"
   ```

4. **Remove and test** - Remove code, run test suite:
   ```bash
   cargo test
   # Expected: All tests pass
   ```

### Anti-Patterns to Avoid
- **Remove without verification**: Always search for references first
- **Batch removal unrelated things**: Keep scope tight, one dead code item at a time
- **Forget CHANGELOG**: Users deserve transparency about removed APIs

## Don't Hand-Roll

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| Dead code detection | Manual grep audit | `cargo dead_code` linter | Automated tools catch what you miss |
| Reference tracking | Manual text search | IDE "Find References" or `rg -r` | More comprehensive, less error-prone |

**Key insight:** For this phase, manual grep is sufficient because the dead code is isolated and well-documented. For larger codebases, use automated dead code detection tools.

## Common Pitfalls

### Pitfall 1: False Positive Dead Code
**What goes wrong:** You remove code that looks unused but is actually called via reflection, macros, or dynamic dispatch.

**Why it happens:** Static analysis can't see runtime call patterns.

**How to avoid:**
- Check for macro-generated references
- Check for trait implementations that might be called dynamically
- Run full test suite after removal

**Warning signs:** Code is part of a trait impl, exported in public API, or has `#[cfg(test)]` attributes nearby.

### Pitfall 2: Breaking Public API
**What goes wrong:** You remove a struct that's part of the public API, even if nothing in your codebase uses it.

**Why it happens:** External users depend on the API.

**How to avoid:**
- Check `src/lib.rs` for re-exports
- Check if item has `pub` visibility
- Review docs/API.md for public documentation
- Treat public API removal as a breaking change (requires semver bump)

**Warning signs:** Item is in `src/lib.rs` re-exports, documented in API.md, or has `pub` keyword.

### Pitfall 3: Orphaned Documentation
**What goes wrong:** Code is removed but documentation still references it.

**Why it happens:** Documentation updates are forgotten.

**How to avoid:**
- Run same grep search in `docs/` directory
- Update ARCHITECTURE.md if it mentions the removed code
- Add CHANGELOG entry explaining what was removed and why

**Warning signs:** Documentation mentions the struct name, examples use the removed API.

## Code Examples

### Dead Code Removal Pattern
```rust
// BEFORE: Dead code in src/ingest/mod.rs

/// Main ingest orchestrator.
///
/// Reads Rust files from the filesystem, parses them with tree-sitter,
/// and stores symbols/spans in the SQLiteGraph database.
pub struct Ingestor {
    /// Graph database handle (not yet used, pending implementation).
    _graph: CodeGraph,
}

impl Ingestor {
    /// Create a new ingestor with the given graph database.
    pub fn new(graph: CodeGraph) -> Self {
        Self { _graph: graph }
    }

    /// Ingest a single Rust source file.
    pub fn ingest_file(&mut self, _path: &Path) -> Result<()> {
        // TODO: Implement in Task 1
        Err(crate::error::SpliceError::Other(
            "Not implemented yet".to_string(),
        ))
    }

    /// Ingest a directory of Rust source files recursively.
    pub fn ingest_dir(&mut self, _path: &Path) -> Result<()> {
        // TODO: Implement in Task 1
        Err(crate::error::SpliceError::Other(
            "Not implemented yet".to_string(),
        ))
    }
}
```

```rust
// AFTER: Dead code removed
// File: src/ingest/mod.rs (lines 1-36 remain unchanged)
// (Lines 42-68 deleted entirely)
// File continues with remaining code...

/// Re-export common types for convenience.
pub use cpp::{extract_cpp_symbols, CppSymbol, CppSymbolKind};
// ... rest of file unchanged ...
```

**Impact:** 27 lines removed, no functional changes.

### CHANGELOG Entry Pattern
```markdown
## [2.2.4] - 2026-02-XX

### Removed

**Dead Code Cleanup**
- Removed unused `Ingestor` struct from `src/ingest/mod.rs`
  This was an abandoned design from early development.
  Use `MagellanIngestor` or `extract_symbols()` instead.

**Migration:** If you were using the `Ingestor` struct (unlikely, as it was never implemented),
switch to:
- `MagellanIngestor` for database-backed indexing
- `extract_symbols()` for direct symbol extraction
```

## State of the Art

| Old Approach | Current Approach | When Changed | Impact |
|--------------|------------------|--------------|--------|
| `Ingestor` struct (placeholder) | `MagellanIngestor` + `extract_symbols()` | v0.5.0 (2026-01-02) | Dead code remained in codebase |

**Deprecated/outdated:**
- `Ingestor` struct: Abandoned design for custom CodeGraph implementation. Replaced by Magellan integration.

## Open Questions

None. This is a straightforward dead code removal with high confidence.

1. **Could there be external users of the `Ingestor` struct?**
   - What we know: The struct was never implemented (methods return "Not implemented yet")
   - What's unclear: Whether any external users imported it despite non-functionality
   - Recommendation: Check if `Ingestor` is re-exported in `src/lib.rs` (it is not), check if documented in API.md (it is mentioned as abandoned in docs/CONTEXT.md)

## Sources

### Primary (HIGH confidence)
- **Source code analysis**: `/home/feanor/Projects/splice/src/ingest/mod.rs` (lines 42-68) — confirmed dead code structure
- **Grep search results**: Zero references to `Ingestor::` in `src/` and `tests/` directories
- **Documentation review**: `/home/feanor/Projects/splice/docs/CONTEXT.md` — documents the struct as "abandoned design"
- **CHANGELOG.md**: `/home/feanor/Projects/splice/CHANGELOG.md` — confirms Magellan integration in v0.5.0 replaced the need for custom Ingestor

### Secondary (MEDIUM confidence)
- **Architecture documentation**: `/home/feanor/Projects/splice/.planning/codebase/ARCHITECTURE.md` — mentions Ingestor in layer description (will need updating)
- **Planning context**: `/home/feanor/Projects/splice/.planning/phases/27-code-cleanup/27-CONTEXT.md` — user decisions confirming removal approach

### Tertiary (LOW confidence)
- None. All findings are from source code or project documentation.

## Metadata

**Confidence breakdown:**
- Standard stack: HIGH - grep, cargo test, cargo check are universal Rust tools
- Architecture: HIGH - dead code removal pattern is well-understood
- Pitfalls: HIGH - common Rust dead code removal practices are documented

**Research date:** 2026-02-04
**Valid until:** 90 days (codebase structure stable, dead code won't become alive)

## Appendix: Removal Scope

### Files to Modify
1. **`/home/feanor/Projects/splice/src/ingest/mod.rs`**
   - Remove lines 42-68 (27 lines)
   - Impact: File reduced from 68 to 41 lines

2. **`/home/feanor/Projects/splice/.planning/codebase/ARCHITECTURE.md`**
   - Line 21: Remove `- **Ingestor:** Main orchestrator for parsing source files`
   - Line 78: Update "Ingestion Flow" section to remove `Ingestor` reference
   - Add note about `MagellanIngestor` being the correct API

3. **`/home/feanor/Projects/splice/CHANGELOG.md`**
   - Add entry for v2.2.4 documenting the removal

### Files That Reference Dead Code (Documentation Only)
- `/home/feanor/Projects/splice/docs/CONTEXT.md` — Already marks it as "abandoned design"
- `/home/feanor/Projects/splice/docs/TODO_MULTI_LANG.md` — Item "Remove unused `Ingestor.graph` field" can be closed
- `/home/feanor/Projects/splice/docs/EXECUTIVE_SUMMARY.md` — Mentions removal as TODO
- `/home/feanor/Projects/splice/docs/MULTI_LANGUAGE_V2.md` — Mentions removal as TODO

### Verification Commands
```bash
# 1. Check for references
grep -r "Ingestor" src/ --include="*.rs" | grep -v "MagellanIngestor"

# 2. Check tests
grep -r "Ingestor" tests/ --include="*.rs"

# 3. Run tests after removal
cargo test

# 4. Verify compilation
cargo check
```

### Estimated Removal Order
1. Remove dead struct from `src/ingest/mod.rs` (lines 42-68)
2. Update `ARCHITECTURE.md` to remove Ingestor references
3. Update CHANGELOG.md with removal entry
4. Verify with `cargo test`
5. Close related TODO items in documentation

### Risk Assessment
- **Breaking changes**: None (code was never functional)
- **Public API impact**: None (Ingestor was not re-exported in lib.rs)
- **Test impact**: None (no tests reference Ingestor)
- **Documentation impact**: Low (only internal planning docs mention it)

**Overall risk**: VERY LOW. This is textbook dead code removal.
