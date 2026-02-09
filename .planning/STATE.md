# State: Splice

**Current Milestone:** v2.5.0 - Native-V2 Backend Support

## Project Reference

See: `.planning/PROJECT.md` (updated 2026-02-09)

**Core value:** Safe code transformation with byte-level precision and rollback support
**Current focus:** Phase 33 - Feature Flag Infrastructure

---

## Current Position

**Phase:** 33 of 38 (Feature Flag Infrastructure)
**Plan:** 2 of 3 in current phase
**Status:** In progress
**Last activity:** 2026-02-09 — Plan 33-02 completed: Compile-time mutual exclusion guard added

---

## Performance Metrics

**Velocity:**
- Total plans completed: 2 (v2.5.0)
- Average duration: 6 min
- Total execution time: ~0.2 hours

**By Phase:**

| Phase | Plans | Total | Avg/Plan |
|-------|-------|-------|----------|
| 33 | 2 | 3 | 6 min |

---

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

**API Incompatibility with native-v2 Backend:**
- Splice code uses `get_all_labels()` and `count_entities_by_label()` methods
- These methods are only available in SQLite backend (`#[cfg(not(feature = "native-v2"))]` in magellan)
- Native-v2 backend feature flag exists in Cargo.toml but won't compile without code changes
- Needs resolution in Phase 34 (Backend Detection & Migration)

**Feature Flag Infrastructure:**
- Feature flags now in place (sqlite, native-v2)
- Compile-time guard prevents both features being enabled simultaneously
- Default: SQLite backend for backward compatibility
- Usage: `cargo build --features native-v2 --no-default-features` for native-v2 backend

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

---

*State updated: 2026-02-09*
