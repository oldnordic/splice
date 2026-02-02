# Magellan Integration Pitfalls

**Project:** Splice v2.2.2 - Magellan Query Command Delegation
**Domain:** CLI tool integration and command delegation
**Researched:** 2026-01-24
**Overall confidence:** HIGH

---

## Executive Summary

Splice already integrates Magellan as a **library** (`magellan 0.5.3` in Cargo.toml), not as a subprocess. The integration is well-structured through `src/graph/magellan_integration.rs` and `src/ingest/magellan.rs`. However, the upcoming **query command delegation** introduces specific risks around command identity, data format alignment, and testing coverage.

**Key finding:** Since Magellan is already a library dependency, the main pitfalls are **NOT subprocess-related** (no shell escaping, no buffer blocking, no zombie processes). Instead, the risks are:
1. **Flag/namespace conflicts** between Splice and Magellan concepts
2. **Data format misalignment** causing LLM confusion
3. **Test coverage gaps** at the integration boundary
4. **Performance issues** from relationship graph queries
5. **Database path confusion** between tools

---

## Critical Pitfalls

Mistakes that cause rewrites or major issues.

### Pitfall 1: Flag Namespace Collision with Magellan Concepts

**What goes wrong:**
Splice adds flags that conflict with Magellan's internal concepts or future Magellan commands, causing confusion when users work with both tools.

**Why it happens:**
- Magellan has its own CLI with flags like `--label`, `--kind`, `--db`
- Splice's `Query` and `Get` commands (lines 240-329 in `src/cli/mod.rs`) already use similar flags
- No explicit namespace separation between "Splice-native" and "delegated-to-Magellan" flags

**Consequences:**
- Users can't remember which tool uses which flag syntax
- Documentation becomes confusing ("use `--label` with Splice, but `--labels` with Magellan")
- Future Magellan version may add conflicting flag names

**Warning signs:**
- Documentation needs "note: this is different from Magellan's flag" disclaimers
- Users asking "which tool do I run this command with?"
- Need for `--magellan-flag` prefix workarounds

**Prevention:**
1. **Explicit flag namespacing:** Use `--magellan-*` prefix for any flag that directly delegates to Magellan's API
   ```rust
   // Good: Clear delegation
   MagLabels(Vec<String>),     // Direct delegation to Magellan.get_symbols_by_labels()

   // Avoid: Ambiguous ownership
   Labels(Vec<String>),         // Is this Splice or Magellan?
   ```
2. **Flag ownership documentation:** In `src/cli/mod.rs`, document which flags are:
   - Splice-native (implemented in Splice)
   - Magellan-delegated (pass-through to Magellan API)
   - Hybrid (Splice processes, then queries Magellan)

3. **Subcommand isolation:** Consider `splice magellan <subcommand>` pattern for pure delegation
   - Keeps Splice's core commands clean
   - Clear boundary: "everything under `magellan` is delegation"
   - Allows future expansion without flag conflicts

**Phase to address:** Phase 18 (Error Code Integration) or earlier during CLI design

**Detection:**
- Audit all flags in `src/cli/mod.rs::Commands`
- Compare against Magellan's CLI (`magellan --help`)
- Flag any overlaps for explicit namespacing

**Sources:** HIGH - Reviewed `src/cli/mod.rs` lines 240-329, identified existing flag overlap potential

---

### Pitfall 2: Data Format Misalignment Breaking LLM Consumption

**What goes wrong:**
Splice's JSON output format differs from Magellan's internal format, causing LLMs to misinterpret spans or apply patches to wrong locations.

**Why it happens:**
- Magellan's `SymbolQueryResult` has fields: `entity_id`, `name`, `file_path`, `kind`, `byte_start`, `byte_end`
- Splice's `SpanResult` (from `src/output.rs`) has different fields and structure
- Converting between formats loses information or adds inconsistent fields

**Consequences:**
- LLM receives span from Magellan query, tries to patch with Splice, but byte offsets don't align
- "I patched the function but it says symbol not found" — the symbol name or path format differs
- LLM has to maintain two different JSON parsers in its head

**Warning signs:**
- Test failures where "query found it but patch says it doesn't exist"
- Format conversion logic with `unwrap()` or lossy transformations
- Documentation showing different JSON structures for Query vs Patch

**Prevention:**
1. **Unified JSON schema:** Splice's output should be a superset of Magellan's format
   ```json
   // Splice output should include all Magellan fields PLUS Splice additions
   {
     "entity_id": 123,        // From Magellan
     "name": "my_function",   // From Magellan
     "byte_start": 42,        // From Magellan
     "byte_end": 100,         // From Magellan
     "splice_kind": "function", // Splice extension
     "checksum_before": "abc123" // Splice extension
   }
   ```
2. **Zero-loss conversion:** `From<magellan::SymbolQueryResult> for SpanResult` must preserve all fields
3. **Format compatibility tests:** Add test that queries Magellan, gets output, and feeds to patch

**Phase to address:** Phase 14 (Cross-File Relationships) or Phase 15 (CLI Integration)

**Detection:**
- Test: Query a symbol via Magellan -> Get Splice JSON -> Try to patch same symbol
- Should work without any field transformation

**Sources:** HIGH - Reviewed `src/graph/magellan_integration.rs` lines 140-168, `SymbolInfo` struct definition

---

### Pitfall 3: Database Path Confusion Between Splice and Magellan

**What goes wrong:**
Users have multiple `magellan.db` files (one for Magellan CLI, one for Splice), causing queries to return stale or wrong results.

**Why it happens:**
- Splice uses `--db` flag to specify Magellan database path (line 242-244 in `src/cli/mod.rs`)
- Magellan CLI may use a default location (e.g., current directory or `~/.magellan/`)
- No validation that the database Splice queries is the same one Magellan would use

**Consequences:**
- User queries with Splice, gets no results (wrong database)
- User indexes with Magellan, queries with Splice, sees stale data
- "I just indexed this file why doesn't Splice find it?"

**Warning signs:**
- Documentation needs "make sure you're using the same database file" warnings
- Support issues about "query returns empty but I just ran magellan index"
- Test fixtures hardcoding `magellan.db` paths

**Prevention:**
1. **Database path detection:** Auto-detect Magellan's default database location
   ```rust
   fn find_magellan_db() -> PathBuf {
       // Check: 1. Current directory magellan.db
       //        2. .magellan/magellan.db
       //        3. ~/.magellan/magellan.db
       //        4. Require --db flag if none found
   }
   ```
2. **Database version validation:** Check Magellan database version on open
   ```rust
   // Prevent opening incompatible database versions
   MagellanIntegration::open_with_version_check(db_path, MAGELLAN_VERSION_0_5_3)
   ```
3. **Explicit database status command:** `splice query --db-status` showing:
   - Which database is being used
   - Last indexed time
   - Number of symbols
   - Magellan version compatibility

**Phase to address:** Phase 15 (CLI Integration)

**Detection:**
- Integration test: Create DB with Magellan CLI, query with Splice
- Should work with explicit `--db` flag, warn/fail without it

**Sources:** HIGH - Reviewed `src/cli/mod.rs` lines 240-305, `Query` and `Get` command definitions with `--db` flag

---

### Pitfall 4: Performance Collapse on Relationship Queries

**What goes wrong:**
Callers/callees queries (`--relationships` flag) cause exponential slowdown on large codebases, making CLI hang.

**Why it happens:**
- Magellan's CALLER/CALLS edge traversal is O(n) per query
- Splice's `--relationships` flag may trigger full graph traversal
- No depth limiting or result caching by default

**Consequences:**
- `splice query --label rust --relationships` hangs for 30+ seconds on 10K file codebase
- Users think CLI is broken, kill process
- Tool gets reputation for being slow

**Warning signs:**
- Query time increases non-linearly with codebase size
- No progress indication for long-running queries
- Tests only run on tiny fixtures (< 100 files)

**Prevention:**
1. **Lazy relationship loading:** Only traverse relationships when explicitly requested
   ```rust
   // src/graph/magellan_integration.rs
   pub fn get_relationships_lazy(&self, symbol_id: i64) -> impl Iterator<Item = Relationship> {
       // Don't traverse until caller asks for first result
   }
   ```
2. **Depth limiting:** Default to depth=1, require explicit `--max-depth` for deeper
   ```rust
   Relationships {
       max_depth: 1,  // Default: only direct relationships
       max_results: 100,  // Prevent result explosion
   }
   ```
3. **Progress indication:** Show progress bar for queries that may take >1 second
4. **Result caching:** Cache relationship queries per session

**Phase to address:** Phase 14 (Cross-File Relationships)

**Detection:**
- Benchmark queries on 1K, 10K, 100K file codebases
- Alert if query time > O(files) instead of O(1)

**Sources:** HIGH - `ARCHITECTURE.md` lines 427-461 document relationship graph performance risks

---

### Pitfall 5: Test Coverage Gaps at Integration Boundary

**What goes wrong:**
Integration tests cover Magellan in isolation and Splice in isolation, but not the delegation path, causing regressions when either tool changes.

**Why it happens:**
- `tests/magellan_integration_tests.rs` tests Magellan wrapper directly
- `tests/` folder has Splice unit tests
- No tests that run: User command -> Splice CLI -> Magellan API -> Splice output

**Consequences:**
- Magellan 0.5.4 changes a field name -> Splice tests pass but production breaks
- Splice adds a field -> Magellan can't consume it
- Discover bugs only when users hit them

**Warning signs:**
- Test coverage report shows < 80% for `src/graph/magellan_integration.rs`
- No integration tests with actual `magellan.db` files
- Tests mocking Magellan instead of using real library

**Prevention:**
1. **End-to-end integration tests:** Test full delegation path
   ```rust
   #[test]
   fn test_query_delegation_e2e() {
       // 1. Create real Magellan database
       // 2. Index a file
       // 3. Run splice query CLI command
       // 4. Verify JSON output matches expected
       // 5. Try to patch the found symbol
       // 6. Verify patch succeeds
   }
   ```
2. **Contract tests:** Define expected Magellan API behavior
   ```rust
   // tests/magellan_contract.rs
   #[test]
   fn test_magellan_symbol_query_contract() {
       // Test that SymbolQueryResult has expected fields
       // Test that label queries work as documented
   }
   ```
3. **Version matrix testing:** Test against multiple Magellan versions

**Phase to address:** Phase 17 (Integration and Testing)

**Detection:**
- Measure test coverage for delegation path
- Add tests for any uncovered code paths

**Sources:** HIGH - Reviewed `tests/magellan_integration_tests.rs`, identified single-tool testing pattern

---

## Moderate Pitfalls

Mistakes that cause delays or technical debt.

### Pitfall 6: Error Code Inconsistency Across Tools

**What goes wrong:**
Splice returns `SPL-E001` errors but Magellan returns different error formats, causing LLMs to mishandle error recovery.

**Why it happens:**
- Magellan may have its own error codes (or none)
- Splice's error code system (Phase 11) doesn't account for delegated errors
- No mapping between Magellan errors and Splice errors

**Consequences:**
- LLM doesn't know how to retry Magellan-sourced errors
- Users see "Magellan error" without Splice error code context
- Error documentation is split between tools

**Warning signs:**
- `unwrap()` on Magellan errors without mapping to SpliceError
- Errors like "Magellan failed: ..." without structured code
- LLM prompts need "if error contains 'Magellan', do X"

**Prevention:**
1. **Error code mapping:** Map Magellan errors to Splice error codes
   ```rust
   impl From<magellan::Error> for SpliceError {
       fn from(err: magellan::Error) -> Self {
           match err {
               magellan::Error::SymbolNotFound => SpliceError::SymbolNotFound,
               magellan::Error::DatabaseLocked => SpliceError::DatabaseLocked,
               // Map all Magellan errors to SPL-E*** codes
           }
       }
   }
   ```
2. **Error source tracking:** Include which tool generated the error
   ```json
   {
     "error_code": "SPL-E001",
     "source": "magellan",  // Which tool actually failed
     "original_error": "MAG-REF-001"  // Original error if different
   }
   ```

**Phase to address:** Phase 11 (Foundation Extensions) or Phase 18

**Detection:**
- Audit all `From<magellan::Error>` implementations
- Verify all have corresponding SpliceError variant

**Sources:** HIGH - Reviewed `src/graph/magellan_integration.rs` error handling patterns (lines 25-26, 62-65)

---

### Pitfall 7: Context Flag Semantics Differing Between Tools

**What goes wrong:**
Splice's `-A`/`-B`/`-C` flags (lines 62-72 in `src/cli/mod.rs`) have different behavior than Magellan's context flags, causing confusion.

**Why it happens:**
- Splice follows grep conventions: `-C` sets default, `-A`/`-B` override
- Magellan may have different context semantics
- No documentation explicitly comparing the two

**Consequences:**
- Users expect `splice query -C 5` to work like `magellan query -C 5`
- Different default values cause surprise
- LLM prompts need tool-specific context logic

**Warning signs:**
- Documentation explaining "unlike Magellan, Splice does..."
- Users asking "why does -C work differently?"
- Need for `--magellan-context-compat` flag

**Prevention:**
1. **Align with grep conventions:** Document that Splice follows grep, not Magellan
2. **Explicit defaults:** State default context counts in help text
3. **Compatibility mode:** Add `--magellan-compat` flag if needed

**Phase to address:** Phase 15 (CLI Integration)

**Detection:**
- Compare Splice and Magellan flag behavior side-by-side
- Document any differences

**Sources:** MEDIUM - Need to verify Magellan's actual flag semantics with `magellan --help`

---

### Pitfall 8: Symbol Kind Mismatch Between Tools

**What goes wrong:**
Magellan uses "fn" but Splice uses "function", causing label queries to fail.

**Why it happens:**
- `src/cli/mod.rs` defines `SymbolKind` enum (lines 428-453)
- Magellan may use different strings for symbol kinds
- No translation layer between the two

**Consequences:**
- `splice query --label fn` returns nothing (Magellan uses "function")
- Users have to remember which tool uses which terminology
- LLM can't reuse labels across tools

**Warning signs:**
- Multiple label names for the same concept (fn vs function)
- Tests using hardcoded label strings
- Documentation listing "Magellan uses: fn, Splice uses: function"

**Prevention:**
1. **Label normalization:** Normalize labels at query time
   ```rust
   fn normalize_label(label: &str) -> &str {
       match label {
           "function" => "fn",     // Map Splice -> Magellan
           "struct" => "struct",
           // Handle all aliases
           _ => label
       }
   }
   ```
2. **Accept both forms:** Allow queries to use either terminology
3. **Document mapping:** Explicit table showing label equivalences

**Phase to address:** Phase 12 (Context Extraction & Semantic Kind Detection)

**Detection:**
- Test queries with both "fn" and "function" labels
- Verify both work

**Sources:** MEDIUM - Need to verify Magellan's actual label strings

---

## Minor Pitfalls

Mistakes that cause annoyance but are fixable.

### Pitfall 9: Version Skew Detection Missing

**What goes wrong:**
User has Magellan 0.5.3 installed but Splice expects 0.6.0, causing cryptic runtime errors.

**Why it happens:**
- Cargo.toml specifies `magellan = "0.5.3"` but doesn't validate at runtime
- No check that the library version matches expected database format
- User may have multiple Magellan versions installed

**Consequences:**
- "Database format error" with no explanation
- Users don't know to update Magellan
- Support time wasted on version mismatches

**Prevention:**
1. **Runtime version check:** Verify Magellan version on init
2. **Clear error message:** "Magellan version mismatch: expected 0.5.3, found 0.6.0"
3. **Version in help text:** `--version` shows both Splice and Magellan versions

**Phase to address:** Phase 15 (CLI Integration)

**Sources:** HIGH - `Cargo.toml` line 22 confirms `magellan = "0.5.3"` dependency

---

### Pitfall 10: JSON Output Format Drift

**What goes wrong:**
Splice's JSON output changes format between versions, breaking LLM parsers.

**Why it happens:**
- Fields added without considering backward compatibility
- Optional fields not properly marked
- No schema version in output

**Consequences:**
- Old LLM prompts break
- Users need to maintain multiple prompt versions
- "It worked yesterday" bugs

**Prevention:**
1. **Schema version field:** Include `"schema_version": "2.2"` in all JSON output
2. **Additive-only changes:** Never remove or rename fields
3. **Field aliases:** Support old and new field names for one version

**Phase to address:** Phase 11 (Foundation Extensions)

**Sources:** HIGH - Covered in existing `PITFALLS.md` pitfall #8 (Breaking LLM Compatibility)

---

## Anti-Patterns to Avoid

### Anti-Pattern 1: Subprocess Delegation

**What it is:** Spawning `magellan` subprocess and parsing stdout
**Why avoid:**
- Already using Magellan as library (correct approach)
- Subprocess adds overhead, complexity, error surface
- No benefit over library calls

**Instead:** Continue using `magellan` crate directly

**Sources:** HIGH - `Cargo.toml` confirms library usage; `src/ingest/magellan.rs` shows direct API calls

---

### Anti-Pattern 2: Dual Database

**What it is:** Splice maintains its own code graph separate from Magellan's
**Why avoid:**
- Duplicate data, sync issues
- Users must index twice
- Wasted disk space

**Instead:** Use Magellan's database directly via `MagellanIntegration`

**Sources:** HIGH - `src/graph/magellan_integration.rs` shows shared database usage

---

### Anti-Pattern 3: Silent Error Translation

**What it is:** Magellan errors caught and re-thrown as generic SpliceError
**Why avoid:**
- Loses debugging information
- Users can't look up Magellan-specific errors
- LLM can't provide specific remediation

**Instead:** Preserve original error in chain, add structured error code

**Sources:** HIGH - Error wrapping patterns observed in lines 25-26 of `src/graph/magellan_integration.rs`

---

## Phase-Specific Warnings

| Phase Topic | Likely Pitfall | Mitigation |
|-------------|----------------|------------|
| **Phase 11** (Error Codes) | Error code mapping gaps | Map all Magellan errors to SPL-E*** codes |
| **Phase 12** (Semantic Kind) | Label/terminology mismatch | Normalize label strings at query boundary |
| **Phase 14** (Relationships) | O(n) performance collapse | Default depth=1, require explicit --max-depth |
| **Phase 15** (CLI Integration) | Flag namespace conflicts | Use --magellan-* prefix for delegated flags |
| **Phase 17** (Testing) | Integration boundary gaps | Add end-to-end tests for full delegation path |
| **Phase 18** (Error Integration) | Cross-tool error handling | Unify error format across Splice/Magellan |

---

## Integration Pitfalls Summary Table

| Category | Pitfall | Severity | Phase | Detection Method |
|----------|---------|----------|-------|------------------|
| **CLI** | Flag namespace collision | HIGH | 15/18 | Audit flags vs Magellan CLI |
| **Data** | Format misalignment | HIGH | 14/15 | E2E query-to-patch test |
| **Config** | Database path confusion | HIGH | 15 | Multi-tool database test |
| **Performance** | Relationship query collapse | HIGH | 14 | Benchmark on 10K files |
| **Testing** | Integration boundary gaps | HIGH | 17 | Coverage report analysis |
| **Errors** | Error code inconsistency | MEDIUM | 11/18 | Audit error conversions |
| **UX** | Context flag differences | MEDIUM | 15 | Compare flag semantics |
| **Data** | Symbol kind mismatch | MEDIUM | 12 | Test both label forms |
| **Compatibility** | Version skew missing | LOW | 15 | Version check test |
| **Stability** | JSON format drift | LOW | 11 | Schema version field |

---

## Confidence Assessment

| Area | Confidence | Reason |
|------|------------|--------|
| **Integration pitfalls** | **HIGH** | Read all integration source code, identified current architecture |
| **CLI pitfalls** | **HIGH** | Reviewed complete CLI structure in `src/cli/mod.rs` |
| **Performance pitfalls** | **HIGH** | Relationship graph scaling documented in ARCHITECTURE.md |
| **Testing pitfalls** | **HIGH** | Reviewed existing tests, identified coverage gaps |
| **Data format pitfalls** | **HIGH** | Analyzed SpanResult vs SymbolQueryResult structures |

**Overall confidence:** HIGH

**Gaps to Address:**
- Magellan's actual CLI flag semantics (verify with `magellan --help`)
- Magellan's label string format for symbol kinds
- Magellan's error code format (if any)

---

## Sources

### Primary (HIGH confidence — verified source code)

- **Splice source code:**
  - `/home/feanor/Projects/splice/src/graph/magellan_integration.rs` — Magellan integration layer
  - `/home/feanor/Projects/splice/src/ingest/magellan.rs` — Magellan-based ingestion
  - `/home/feanor/Projects/splice/src/cli/mod.rs` — CLI command definitions (lines 240-329 for Query/Get)
  - `/home/feanor/Projects/splice/tests/magellan_integration_tests.rs` — Integration tests
  - `/home/feanor/Projects/splice/Cargo.toml` — Dependency specification (`magellan = "0.5.3"`)

- **Existing research:**
  - `.planning/research/ARCHITECTURE.md` — Relationship graph performance analysis
  - `.planning/research/STACK.md` — No new dependencies needed
  - `.planning/research/FEATURES.md` — Table stakes and differentiators
  - `.planning/research/PITFALLS.md` — Existing rich span pitfalls

### Secondary (MEDIUM confidence — standard CLI patterns)

- CLI flag namespacing conventions (clig.dev patterns)
- Library vs subprocess integration tradeoffs
- Error handling across crate boundaries

### Tertiary (LOW confidence — WebSearch unavailable, needs verification)

- Specific Magellan CLI flags (verify with `magellan --help`)
- Magellan's future roadmap (may affect compatibility)

---

*Research completed: 2026-01-24*
*Focus: Magellan integration pitfalls for query command delegation*
*Status: Ready for roadmap phase planning*
