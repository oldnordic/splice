# State: Splice

**Current Milestone:** v2.5.0 - Native-V2 Backend Support

## Project Reference

See: `.planning/PROJECT.md` (updated 2026-02-09)

**Core value:** Safe code transformation with byte-level precision and rollback support
**Current focus:** Phase 36 - Advanced Features

---

## Current Position

**Phase:** 36 of 38 (Advanced Features)
**Plan:** 1 of 4 in current phase
**Status:** Completed
**Last activity:** 2026-02-10 — Plan 36-01 completed: Impact graph visualization with DOT output

---

## Performance Metrics

**Velocity:**
- Total plans completed: 11 (v2.5.0)
- Average duration: 5.2 min
- Total execution time: ~1.0 hours

**By Phase:**

| Phase | Plans | Total | Avg/Plan |
|-------|-------|-------|----------|
| 33 | 4 | 4 | 6 min |
| 34 | 4 | 4 | 5 min |
| 35 | 5 | 20 min | 4 min |

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

**35-02: Snapshot Storage with RFC 3339 Timestamp Format**
- Decision: Use RFC 3339 timestamp format for snapshot filenames
- Rationale: Provides readable, ISO-compliant timestamps that sort chronologically and prevent filename collisions
- Implementation: chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Secs, true)
- Trade-off: Longer filenames than Unix timestamps, but human-readable and sortable
- Decision: Chronological ordering (newest first) for list_snapshots() output
- Rationale: Users typically want most recent snapshots first for rollback operations
- Implementation: Sort by timestamp descending in list_snapshots()
- Trade-off: Requires sorting on each call, but list is typically small
- Decision: Auto-creation of .splice/snapshots/ directory on SnapshotStorage::new()
- Rationale: No manual setup required, directory exists when needed
- Implementation: fs::create_dir_all() in constructor, returns Io error on failure
- Trade-off: May fail if permissions insufficient, but provides clear error message

**35-03: Snapshot Comparison with Diff Algorithm**
- Decision: Use owned String types in HashMap for diff operations
- Rationale: Avoids complex lifetime issues with references; HashSet<&String> caused borrow checker errors
- Trade-off: Slightly more memory usage but significantly simpler code
- Implementation: Use .cloned().collect() on HashMap keys to create owned sets
- Decision: Track all SymbolInfo fields for modification detection
- Rationale: Any change to symbol (name, file_path, kind, byte_span, fan_in, fan_out) represents a meaningful diff
- Trade-off: More comprehensive than tracking just name changes; catches all refactoring side effects
- Decision: Conditional exit code handling with mutable payload
- Rationale: with_pending_changes() takes no arguments; must conditionally call based on diff detection
- Trade-off: Slightly more verbose than hypothetical with_pending_changes_if(bool) method
- Decision: Sort diff results by ID for consistency
- Rationale: Unsorted HashMap iteration produces non-deterministic output order
- Trade-off: O(n log n) sort cost but provides reproducible, testable output
- Implementation: Sort symbol_diffs by ID, edge_details by (from, to) tuple

**35-04: Database Restore from Snapshot**
- Decision: SQLite restore not supported - only native-v2 databases can be restored
- Rationale: sqlitegraph's snapshot_export/import only works with native-v2 backend format
- Trade-off: Users with SQLite databases must migrate to native-v2 first, but prevents data corruption from incompatible formats
- Implementation: Backend detection using CodeGraph::detect_backend() returns error for SQLite
- Decision: Automatic backup creation (.db.backup) before any restore operation
- Rationale: Restore is destructive operation; backup ensures users can recover if restore fails
- Trade-off: Uses additional disk space for backup, but provides critical safety net
- Implementation: fs::copy() creates backup before any modifications
- Decision: Feature-gated restore with clear error message for non-native-v2 builds
- Rationale: Prevents runtime errors by catching unsupported configuration at compile time
- Trade-off: Feature flag required at compile time, but enables safe migration with proper error handling
- Implementation: #[cfg(feature = "native-v2")] with fallback stub returning helpful error

**35-05: Snapshot Management CLI Commands**
- Decision: Use generic SpliceError::Other for snapshot deletion errors instead of adding new variant
- Rationale: Simpler error handling, no need for specialized error type for this use case
- Trade-off: Less specific error type but reduces enum bloat
- Implementation: Use SpliceError::Other with descriptive error messages
- Decision: ID-based snapshot lookup uses substring matching for flexibility
- Rationale: Users can specify full filename or just timestamp portion
- Trade-off: More flexible input handling
- Implementation: find_snapshot_path() checks both exact match and suffix match
- Decision: cleanup_old_snapshots() returns Vec<PathBuf> of deleted paths
- Rationale: Enables verification and reporting of what was deleted
- Trade-off: Slightly more complex return type than ()
- Implementation: Collect deleted paths and return to caller
- Decision: Confirmation prompts require explicit 'y' or 'yes' input
- Rationale: Prevents accidental deletions, requires intentional confirmation
- Trade-off: Extra step for user but improves safety
- Implementation: confirm_action() checks for lowercase 'y' or 'yes'

**36-01: Impact Graph Visualization with DOT Output**
- Decision: Use DOT format for graph visualization instead of custom binary format
- Rationale: DOT is the de facto standard for graph visualization, supported by Graphviz and many other tools
- Trade-off: Requires external tools (dot) for rendering but provides maximum flexibility
- Implementation: ImpactDotConfig struct with show_symbol_kinds, max_depth, highlight_symbol options
- Decision: --impact-graph flag is mutually exclusive with --output json
- Rationale: DOT is text-only format, JSON serialization would add unnecessary complexity
- Trade-off: Users must choose between structured JSON and visual DOT output
- Implementation: Early exit pattern when --impact-graph is set, bypasses normal JSON formatting
- Decision: Rename command --impact-graph requires --preview flag
- Rationale: Impact graph is most useful for previewing changes before applying them
- Trade-off: Extra flag required but prevents unintended modifications
- Implementation: Added #[arg(long, requires = "preview")] to Rename impact_graph field
- Decision: Reuse existing graph traversal from Phase 30 (reachable_symbols, reverse_reachable_symbols)
- Rationale: DOT generation is a view layer, should not duplicate traversal logic
- Trade-off: None - pure code reuse with no behavioral changes
- Implementation: generate_impact_dot() calls existing reachable_symbols()/reverse_reachable_symbols()

**36-02: YAML Batch Specification Schema**
- Decision: Use serde tag-based enum deserialization for operation discrimination
- Rationale: Using #[serde(tag = "type")] ensures type-safe discrimination while maintaining readable YAML format
- Trade-off: Slightly more verbose YAML (requires "type: patch" field) but prevents invalid operation specifications at parse time
- Decision: Path validation in parse_batch_spec() happens at parse time, not execution time
- Rationale: Checking file existence during parsing provides immediate feedback to users
- Trade-off: Valid specs at parse time may still fail at execution (e.g., file deleted between parse and execute)
- Decision: BatchError integration via From trait
- Rationale: Converting BatchError to SpliceError enables seamless error handling without changing existing call sites
- Trade-off: Some BatchError variants map to generic SpliceError::Other instead of dedicated variants

---

*State updated: 2026-02-10*
*Last session: 2026-02-10T00:08:00Z - Completed Phase 36-01*
