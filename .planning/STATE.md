# State: Splice

**Current Milestone:** v2.5.0 - Native-V2 Backend Support

## Project Reference

See: `.planning/PROJECT.md` (updated 2026-02-09)

**Core value:** Safe code transformation with byte-level precision and rollback support
**Current focus:** Phase 34 - Backend Detection & Migration

---

## Current Position

**Phase:** 35 of 38 (Snapshots & Verification)
**Plan:** 1 of 5 in current phase
**Status:** Completed
**Last activity:** 2026-02-09 — Plan 35-01 completed: Snapshot capture with --snapshot-before flag

---

## Performance Metrics

**Velocity:**
- Total plans completed: 6 (v2.5.0)
- Average duration: 5 min
- Total execution time: ~0.5 hours

**By Phase:**

| Phase | Plans | Total | Avg/Plan |
|-------|-------|-------|----------|
| 33 | 4 | 4 | 6 min |
| 34 | 4 | 4 | 5 min |
| 35 | 1 | 5 | 7 min |

## Accumulated Context

### Shipped Features (v2.0 - v2.4.1)

**Multi-Language Support:**
- 7 languages: Rust, Python, C, C++, Java, JavaScript, TypeScript
- Tree-sitter AST parsing for accurate symbol extraction
- Language-specific import resolution

**Refactoring Operations:**
- Span-safe patch application using ropey
- Cross-file rename with full reference tracking
- Delete symbol with validation
- Plan mode for dry-run previews

**Code Graph Integration:**
- Magellan integration for code indexing
- Query commands: status, find, refs, files, query
- Export formats: json, jsonl, csv

**Advanced Features:**
- Impact analysis (reachability, dead code, cycles, slicing)
- Proof-based refactoring with GraphSnapshot
- Compiler validation and rollback

### Current Architecture

```
splice (CLI + library)
├── src/graph/
│   ├── mod.rs              → CodeGraph (backend-agnostic wrapper)
│   ├── magellan_integration.rs  → Magellan wrapper
│   ├── schema.rs           → Graph schema labels
│   └── rename/             → Rename operations
├── src/proof/              → Proof-based refactoring
├── src/ingest/             → Symbol ingestion (Magellan-based)
└── src/cli/                → CLI commands
```

**Backend Detection (current):**
```rust
// src/graph/mod.rs:64-81
fn is_sqlite_db(path: &Path) -> Result<bool> {
    // Checks for "SQLite format 3" in file header
    ...
}
let cfg = if Self::is_sqlite_db(path)? {
    sqlitegraph::GraphConfig::sqlite()
} else {
    sqlitegraph::GraphConfig::native()
};
```

### Known Issues

**Label Query Limitations with native-v2 Backend:**
- Label queries (--list, --count, --label) require SQLite backend
- Native-v2 backend does not support label queries (not implemented in magellan)
- Clear error messages guide users when attempting to use these features with native-v2
- For full feature parity, label queries would need to be implemented for native-v2 backend (future work)

**Feature Flag Infrastructure:**
- Feature flags now in place (sqlite, native-v2)
- Compile-time guard prevents both features being enabled simultaneously
- Default: SQLite backend for backward compatibility
- Usage: `cargo build --features native-v2 --no-default-features` for native-v2 backend
- Status: Both backends now compile successfully

---

## Dependencies

| Crate | Version | Feature | Notes |
|-------|---------|---------|-------|
| magellan | 2.1 | optional, default-features=false | Feature-gated: sqlite vs native-v2 |
| sqlitegraph | 1.5.5 | sqlite-backend (default) | Upgraded from 1.3.0 for native-v2 compat |

---

## Milestones History

| Version | Name | Shipped | Key Features |
|---------|------|---------|--------------|
| v2.2.2 | Magellan Integration | 2026-01-24 | Unified CLI, query commands |
| v2.2.4 | Code Cleanup | 2026-02-04 | Removed dead Ingestor |
| v2.4.0 | Proof-Based Refactoring | 2026-02-04 | GraphSnapshot, verification |
| v2.4.1 | Bugfix Release | 2026-02-04 | Minor fixes |
| v2.5.0 | Native-V2 Backend | TBD | Current milestone |

**v2.5.0 Roadmap:** 6 phases (33-38), 24 total plans
- Phase 33: Feature Flag Infrastructure (3 plans)
- Phase 34: Backend Detection & Migration (4 plans)
- Phase 35: Snapshots & Verification (5 plans)
- Phase 36: Advanced Features (4 plans)
- Phase 37: Testing Infrastructure (4 plans)
- Phase 38: Documentation (4 plans)

---

---

## Key Decisions (Phase 33)

**33-01: Feature Flag Pattern**
- Decision: Use Cargo feature flags for backend selection (sqlite vs native-v2)
- Rationale: Compile-time selection avoids runtime overhead, maintains backward compatibility
- Trade-off: API incompatibility exists between backends, requires code changes for native-v2 support

**33-02: Compile-Time Mutual Exclusion Guard**
- Decision: Add compile_error! guard using #[cfg(all(feature = "sqlite", feature = "native-v2"))]
- Rationale: Prevents undefined behavior from conflicting backends, provides clear error guidance
- Implementation: Guard placed after inner attributes (required by Rust syntax), error message includes remediation steps

**33-03: Public API Stability**
- Decision: Verified that public API is identical across SQLite and native-v2 backends
- Rationale: Ensures backend choice is implementation detail only, no breaking changes for library users
- Verification: cargo-public-api confirmed 6,690 identical public items; no cfg-gated public items in lib.rs; CodeGraph uses Box<dyn GraphBackend> abstraction

**33-04: Gap Closure for Native-V2 Compilation**
- Decision: cfg-gate SQLite-only label query methods (query_by_labels, get_all_labels, count_by_label)
- Rationale: Native-v2 backend does not support label queries; cfg guards enable compilation while preserving SQLite functionality
- Trade-off: --list, --count, and --label CLI flags unavailable with native-v2 backend; clear error messages guide users to SQLite backend
- Implementation: Added #[cfg(feature = "sqlite")] to MagellanIntegration methods, CLI query paths, and 50+ tests

**34-01: Backend Enum and Detection API**
- Decision: Add public Backend enum with SQLite, NativeV2, and Unknown variants
- Rationale: Type-safe backend detection prevents confusion between sqlite and native-v2 formats
- Implementation: Backend enum with Display trait (outputs lowercase strings), detect_backend() function using file header magic string detection
- Trade-off: detect_backend() returns Ok(Unknown) for non-existent files rather than error for graceful handling

**34-02: CLI Backend Detection Flag**
- Decision: Use standard JSON response format with human-readable message field for --detect-backend flag
- Rationale: Maintains consistency with other CLI commands while keeping backend info readable
- Implementation: Early-exit pattern when flag is set, returns immediately without opening database
- Trade-off: Backend info in message field requires parsing for programmatic use, but enables human-readable output

**34-03: Database Migration Command**
- Decision: Use cfg-gated implementation for migrate_to_native_v2() method
- Rationale: Prevents users from calling migrate without proper build configuration, provides clear error message
- Implementation: #[cfg(feature = "native-v2")] with fallback error message, uses snapshot_export/import from sqlitegraph
- Trade-off: Feature flag required at compile time, but enables safe migration with proper error handling

**34-04: Migration Verification with Rollback**
- Decision: Use entity_ids() and node_degree() for verification instead of dedicated count methods
- Rationale: sqlitegraph GraphBackend trait doesn't have node_count()/edge_count() methods
- Trade-off: More expensive (need to iterate all nodes for edge counts) but works with available API
- Implementation: verify_migration() compares entity_ids().len() and sums node_degree() for all nodes
- Decision: Default verification to enabled (verify=true) for safety by default
- Rationale: Migrations are risky operations - users need assurance that migration completed successfully
- Trade-off: Slower migration by default, but prevents data corruption; --skip-verify flag available for performance

**35-01: Snapshot Capture with --snapshot-before Flag**
- Decision: Snapshot capture requires --db flag for patch operations
- Rationale: generate_snapshot() requires Magellan database path; patch has optional --db flag
- Trade-off: Snapshot only captured when both --snapshot-before AND --db are provided
- Implementation: capture_snapshot() checks for db path, logs warning if missing
- Decision: Delete operations log warning for --snapshot-before (not supported)
- Rationale: Delete uses local CodeGraph instance, not Magellan database
- Trade-off: Users informed of limitation, operation continues without snapshot
- Decision: Non-blocking error handling for snapshot capture failures
- Rationale: Snapshot issues shouldn't block legitimate refactoring work
- Trade-off: Operations continue even if snapshot fails; warning logged for awareness

---

*State updated: 2026-02-09*
*Last session: 2026-02-09T22:36:00Z - Completed Phase 35-01*
