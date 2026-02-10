# Roadmap: Splice

## Milestones

- ✅ **v2.0 - v2.4** - Phases 1-32 (shipped previously)
- 🚧 **v2.5.0** - Phases 33-38 (native-v2 backend support)

## Phases

<details>
<summary>✅ v1.0 - v2.4 (Phases 1-32) - SHIPPED PREVIOUSLY</summary>

Previous milestones completed. See git history for details.

</details>

### 🚧 v2.5.0 Native-V2 Backend Support (In Progress)

**Milestone Goal:** Deliver dual backend support with SQLite as default and native-v2 as opt-in, including snapshot-based rollback and advanced refactoring features.

#### Phase 33: Feature Flag Infrastructure

**Goal**: Users can build splice with either SQLite or native-v2 backend via compile-time feature flags.

**Depends on**: Nothing (v2.5.0 kickoff)

**Requirements**: BACKEND-01, BACKEND-02, BACKEND-05, BACKEND-06

**Success Criteria** (what must be TRUE):
  1. User can build splice with SQLite backend using `cargo build` (default behavior)
  2. User can build splice with native-v2 backend using `cargo build --features native-v2`
  3. Build fails with clear error message if both backends are enabled simultaneously
  4. Public library API (`cargo doc`) is identical regardless of which feature is enabled

**Plans**: 4 plans

Plans:
- [x] 33-01-PLAN.md — Configure Cargo.toml feature flags and dependency versions
- [x] 33-02-PLAN.md — Implement compile-time guards for mutual exclusion
- [x] 33-03-PLAN.md — Verify public API stability across feature variants
- [x] 33-04-PLAN.md — Gap closure for native-v2 compilation

#### Phase 34: Backend Detection & Migration

**Goal**: Users can detect which backend a database uses and migrate from SQLite to native-v2.

**Depends on**: Phase 33

**Requirements**: BACKEND-03, BACKEND-04

**Success Criteria** (what must be TRUE):
  1. User can run `splice --detect-backend` to see which backend a database file uses
  2. User receives clear error message when opening native-v2 database without native-v2 feature
  3. User can migrate SQLite database to native-v2 format via `splice migrate` command
  4. Migration preserves all symbols, edges, and invariants from source database

**Plans**: 4 plans

Plans:
- [ ] 34-01-PLAN.md — Implement backend format detection in CodeGraph
- [ ] 34-02-PLAN.md — Add --detect-backend flag to CLI status command
- [ ] 34-03-PLAN.md — Implement migrate command with progress reporting
- [ ] 34-04-PLAN.md — Add migration verification and rollback on failure

#### Phase 35: Snapshots & Verification

**Goal**: Users can capture snapshots before refactoring and verify changes against them.

**Depends on**: Phase 33

**Requirements**: SNAP-01, SNAP-02, SNAP-03, SNAP-04, SNAP-05

**Success Criteria** (what must be TRUE):
  1. User can capture snapshot before refactor with `--snapshot-before` flag
  2. Snapshots are stored in `.splice/snapshots/` directory with timestamp names
  3. User can compare two snapshots with `splice verify` command
  4. Verify command reports differences in symbols, edges, and invariants
  5. User can restore database from snapshot (native-v2 only)

**Plans**: 5 plans

Plans:
- [x] 35-01-PLAN.md — Implement snapshot capture with --snapshot-before flag
- [x] 35-02-PLAN.md — Design snapshot directory layout and storage format
- [x] 35-03-PLAN.md — Implement verify command for snapshot comparison
- [x] 35-04-PLAN.md — Add database restore from snapshot capability
- [x] 35-05-PLAN.md — Add snapshot cleanup and management utilities

#### Phase 36: Advanced Features

**Goal**: Users can visualize refactoring impact and execute batch multi-file operations.

**Depends on**: Phase 35

**Requirements**: ADV-01, ADV-02, ADV-03, ADV-04

**Success Criteria** (what must be TRUE):
  1. User can visualize refactoring impact with `--impact-graph` flag producing DOT output
  2. User can execute multi-file refactors with `splice batch` command
  3. Batch command accepts YAML spec file listing operations to perform
  4. Batch command provides automatic rollback if any operation fails

**Plans**: 4 plans

Plans:
- [x] 36-01-PLAN.md — Implement --impact-graph flag with DOT output generation
- [x] 36-02-PLAN.md — Design batch command YAML schema
- [x] 36-03-PLAN.md — Implement batch operation executor
- [x] 36-04-PLAN.md — Add transaction-based rollback for batch failures

#### Phase 37: Testing Infrastructure

**Goal**: Local test scripts for dual backend validation without CI.

**Depends on**: Phase 34

**Requirements**: TEST-01, TEST-02

**Success Criteria** (what must be TRUE):
  1. `./scripts/test-all.sh` runs full test suite against both SQLite and native-v2 backends
  2. Feature-gated tests verify backend-specific code paths (snapshot restore, native-v2 operations)
  3. All tests pass in both backend configurations

**Plans**: 3 plans

Plans:
- [ ] 37-01-PLAN.md — Create test scripts directory and dual-backend test script
- [ ] 37-02-PLAN.md — Add feature-gated tests for backend-specific functionality
- [ ] 37-03-PLAN.md — Add integration tests for migration workflow

#### Phase 38: Documentation

**Goal**: Users can make informed decisions about backend choice and migration path.

**Depends on**: Phase 34, Phase 35

**Requirements**: DOC-01, DOC-02

**Success Criteria** (what must be TRUE):
  1. README includes "Which Backend Should I Use?" section comparing SQLite vs native-v2
  2. README documents installation examples for each backend variant
  3. Manual explains migration command usage and workflow
  4. Manual documents native-v2 specific features (snapshots, verify, batch)

**Plans**: 4 plans

Plans:
- [ ] 38-01: Write "Which Backend Should I Use?" decision guide
- [ ] 38-02: Document installation and build options
- [ ] 38-03: Document migration workflow and command reference
- [ ] 38-04: Document native-v2 exclusive features

## Progress

**Execution Order:**
Phases execute in numeric order: 33 → 34 → 35 → 36 → 37 → 38

| Phase | Milestone | Plans Complete | Status | Completed |
|-------|-----------|----------------|--------|-----------|
| 33. Feature Flag Infrastructure | v2.5.0 | 4/4 | Complete | 2026-02-09 |
| 34. Backend Detection & Migration | v2.5.0 | 4/4 | Complete | 2026-02-09 |
| 35. Snapshots & Verification | v2.5.0 | 5/5 | Complete | 2026-02-10 |
| 36. Advanced Features | v2.5.0 | 4/4 | Complete | 2026-02-10 |
| 37. Testing Infrastructure | v2.5.0 | 0/3 | Planned | - |
| 38. Documentation | v2.5.0 | 0/4 | Not started | - |

**v2.5.0 Progress:** [████████░░░] 71% (17/24 plans)
