# Project Research Summary

**Project:** Splice Native-V2 Backend Integration
**Domain:** CLI refactoring tool with dual backend support (SQLite + native-v2)
**Researched:** 2026-02-09
**Confidence:** HIGH

## Executive Summary

Splice is a CLI refactoring tool that currently hardcodes `features = ["native-v2"]` on its magellan dependency, making native-v2 always enabled but not exposed to users. The research reveals that **minimal architectural changes are required** because Splice's existing `CodeGraph` abstraction using `Box<dyn GraphBackend>` is already backend-agnostic. The sqlitegraph and magellan ecosystems provide the necessary feature flag infrastructure for dual backend support.

The recommended approach is to expose native-v2 as an optional feature flag while keeping SQLite as the default. This requires:
1. Removing hardcoded `features = ["native-v2"]` from Cargo.toml
2. Adding proper feature flag declarations that propagate to dependencies
3. Implementing compile-time guards to prevent mutual feature activation
4. Feature-gating new native-v2-specific commands (backup, restore, snapshot, verify, batch)

The key risks are **feature flag configuration errors** (hardcoded features causing unexpected backend activation) and **incomplete testing across feature variants** (code paths not tested). Mitigation involves using `resolver = "2"`, adding `compile_error!` guards, and implementing a CI test matrix for both backends.

## Key Findings

### Recommended Stack

**Core technologies:**
- **sqlitegraph 1.5.5+**: Graph storage with dual backend support — upgrade from 1.3.0 required for native-v2 stability
- **magellan 2.1+**: Code indexing with native-v2 feature propagation — already demonstrates the integration pattern
- **Feature flags**: Compile-time backend selection — `default-features = false` with explicit feature control

**Critical dependency changes:**
- Upgrade sqlitegraph from 1.3.0 to 1.5.5+ for native-v2 API
- Remove hardcoded `features = ["native-v2"]` from magellan dependency
- Add `default-features = false` to both sqlitegraph and magellan

### Expected Features

**Must have (table stakes):**
- `--snapshot-before` flag — auto-capture DB state before refactoring for safe rollback
- `verify` command — compare snapshots to detect unintended changes
- `--detect-backend` flag — show active backend (SQLite vs native-v2)
- `--migrate` utility — convert SQLite DBs to native-v2 format

**Should have (competitive):**
- `--impact-graph` flag — visualize refactoring impact with DOT output
- `batch` command — multi-file refactor with coordinated proof and rollback

**Defer (v2+):**
- Watch mode — real-time symbol tracking via pub/sub (API not yet documented)
- Hot-path analysis — most-traversed execution paths (needs benchmarking)

### Architecture Approach

**Major components:**
1. **CodeGraph** — unified graph operations interface using `Box<dyn GraphBackend>`; already backend-agnostic
2. **SqliteGraphBackend / NativeGraphBackend** — storage implementations from sqlitegraph crate; both implement same trait
3. **MagellanIntegration** — wrapper that abstracts backend choice; no changes needed
4. **proof::generation** — snapshot export/import using Magellan APIs; already backend-agnostic

**Key architectural insight:** The existing `CodeGraph` abstraction requires **zero API changes**. Only backend detection in `CodeGraph::open()` needs enhancement to distinguish SQLite vs Native V2 files via header checking.

### Critical Pitfalls

1. **Hardcoded feature flags in dependencies** — The current Cargo.toml has `features = ["native-v2"]` hardcoded on magellan, forcing native-v2 regardless of splice's feature flags. Remove this and control via splice's own features.

2. **API type changes across cfg guards** — Using `#[cfg(feature = "native-v2")]` on public functions changes the API based on features, creating SemVer violations. Maintain stable public API with trait objects internally.

3. **Mutually exclusive features without compile-time checks** — Both backends could theoretically be enabled simultaneously. Add `compile_error!` guards to prevent this and use `resolver = "2"`.

4. **Incomplete test coverage across feature variants** — Tests only run with default features, leaving native-v2 and SQLite-specific code paths untested. Implement CI matrix testing both backends.

5. **Path dependency version skew** — Using `path = "../magellan"` bypasses Cargo's version resolution. Use version constraints alongside path dependencies and test against published versions.

## Implications for Roadmap

Based on research, suggested phase structure:

### Phase 1: Feature Flag Infrastructure

**Rationale:** This is foundational — the current hardcoded `features = ["native-v2"]` must be removed before any other work. Without proper feature flag setup, all subsequent work will inherit the wrong backend.

**Delivers:** Working feature flag system with SQLite as default, native-v2 as opt-in

**Addresses:**
- Stack dependency upgrades (sqlitegraph 1.3.0 -> 1.5.5+)
- Feature flag declarations in Cargo.toml
- `compile_error!` guards for mutual exclusion
- Resolver "2" configuration

**Avoids:** Pitfall 1 (hardcoded features), Pitfall 3 (mutual exclusion), Pitfall 6 (feature unification)

**Verification:**
- `cargo tree -e features -i magellan` shows correct feature flow
- `cargo build --all-features` fails with compile_error!

### Phase 2: Backend Abstraction Layer

**Rationale:** The public API must remain stable across feature combinations. This phase ensures backend switching is internal, not exposed to users.

**Delivers:** Enhanced backend detection with clear error messages, stable public API

**Addresses:**
- Enhanced `detect_backend_format()` helper
- Clear error messages when opening wrong format with wrong build
- Runtime backend detection in CLI status

**Uses:** Stack's GraphBackend trait

**Implements:** Architecture's unified trait object pattern

**Avoids:** Pitfall 2 (API changes), Pitfall 8 (auto-detection confusion)

**Verification:**
- `cargo doc` output identical across feature combinations
- Opening native-v2 DB without feature gives clear rebuild instruction

### Phase 3: Core Native-V2 Commands (v1 MVP)

**Rationale:** These are the table stakes features users expect. Build the foundational snapshot workflow before advanced features.

**Delivers:** `--snapshot-before`, `verify`, `--detect-backend`, `--migrate`

**Addresses:**
- Feature-gated command modules (backup, restore, snapshot, verify, migrate)
- Snapshot directory layout (`.splice/snapshots/`)
- Migration utility for SQLite -> native-v2 conversion

**Implements:** Features from P1 priority list

**Avoids:** Pitfall 9 (breaking changes), Pitfall 12 (inconsistent errors)

**Scope:**
- Reuse existing `generate_snapshot()` from `src/proof/generation.rs`
- Reuse existing `validate_invariants()` from `src/proof/validation.rs`
- NO impact graph, NO batch command (defer to v1.1)

### Phase 4: Testing Infrastructure

**Rationale:** Conditional compilation creates hidden code paths. Both backends must be tested in CI before release.

**Delivers:** CI matrix testing both backends, conditional test coverage

**Addresses:**
- GitHub Actions workflow with backend matrix
- Backend-specific tests with `#[cfg(all(test, feature = "..."))]`
- Integration tests for migration

**Avoids:** Pitfall 4 (incomplete testing), Pitfall 10 (missing cfg_attr tests)

**Verification:**
- CI passes with both default and `--features native-v2`
- Coverage report shows both backend code paths executed

### Phase 5: Documentation

**Rationale:** Users need clear guidance on backend choice. Without documentation, users will pick the wrong backend or get stuck.

**Delivers:** README section on backend choice, migration guide, performance comparison

**Addresses:**
- "Which Backend Should I Use?" decision table
- Installation examples for each backend
- Migration command documentation
- Performance characteristics table

**Avoids:** Pitfall 7 (documentation confusion)

### Phase 6: Advanced Features (v1.1)

**Rationale:** These are differentiators but not required for MVP. Build after core workflow is validated.

**Delivers:** `--impact-graph`, `batch` command

**Addresses:**
- DOT output for graph visualization
- YAML spec for batch refactoring
- Coordinated multi-file operations with rollback

**Implements:** Features from P2 priority list

**Research flag:** Impact graph requires verification of DOT generation performance on large graphs

### Phase 7: Future Capabilities (v2+)

**Rationale:** These require deeper native-v2 capabilities (pub/sub) that are not yet documented or stable.

**Delivers:** Watch mode, hot-path analysis

**Addresses:**
- Real-time symbol tracking
- Performance profiling

**Research flag:** Pub/sub API design needs investigation; defer until sqlitegraph documents this capability

### Phase Ordering Rationale

- **Phase 1 first**: Feature flags must be correct before any implementation, or all work inherits the wrong backend
- **Phase 2 before Phase 3**: Stable API must be designed before adding commands that use it
- **Phase 3 before Phase 4**: Tests require working implementation; testing infrastructure validates the commands
- **Phase 5 after Phase 3**: Documentation should describe working features, not planned ones
- **Phase 6 after Phase 4**: Advanced features need solid test foundation
- **Phase 7 last**: Depends on external API maturity (pub/sub)

### Research Flags

**Phases likely needing deeper research during planning:**
- **Phase 6:** Impact graph DOT generation — need to verify performance characteristics on graphs with 100K+ symbols
- **Phase 7:** Pub/sub event system — sqlitegraph API not yet documented; may need source code investigation

**Phases with standard patterns (skip research-phase):**
- **Phase 1:** Feature flag setup is standard Rust pattern, well-documented in Cargo Book
- **Phase 2:** Backend abstraction via trait objects is standard; sqlitegraph provides examples
- **Phase 3:** Core commands reuse existing proof infrastructure; low risk
- **Phase 4:** CI testing matrix is standard practice
- **Phase 5:** Documentation is content creation, not technical research

## Confidence Assessment

| Area | Confidence | Notes |
|------|------------|-------|
| Stack | HIGH | Verified sqlitegraph v1.5.5 and magellan v2.1.0 source code directly |
| Features | HIGH | Existing proof infrastructure verified in src/proof/*.rs files |
| Architecture | HIGH | CodeGraph abstraction reviewed, GraphBackend trait confirmed stable |
| Pitfalls | MEDIUM | Based on official docs and community consensus, but native-v2 is new |
| Migration | MEDIUM | sqlitegraph migration API exists but needs hands-on verification |
| Performance | LOW | No benchmarks yet; performance claims unverified |

**Overall confidence:** HIGH

The research is based on direct source code verification for stack and architecture (sqlitegraph, magellan, splice codebases). Feature requirements are grounded in existing splice infrastructure (proof system already implemented). The primary uncertainty is performance characteristics of native-v2, which will require benchmarking during implementation.

### Gaps to Address

- **Native-v2 snapshot API exact signature:** The export/import API exists in sqlitegraph but signature should be verified against current docs
- **Pub/sub event system:** Not documented in sqlitegraph; may need source code investigation for Phase 7
- **Windows compatibility:** Native-v2 Windows support unverified; splice has Windows users
- **Performance benchmarks:** No actual measurements yet; claims based on sqlitegraph documentation

## Sources

### Primary (HIGH confidence)
- `/home/feanor/Projects/sqlitegraph/sqlitegraph/src/lib.rs` — Public API re-exports
- `/home/feanor/Projects/sqlitegraph/sqlitegraph/src/backend.rs` — GraphBackend trait
- `/home/feanor/Projects/sqlitegraph/sqlitegraph/Cargo.toml` — Feature flag definitions
- `/home/feanor/Projects/magellan/src/graph/mod.rs` — Conditional compilation pattern (lines 189-350)
- `/home/feanor/Projects/splice/Cargo.toml` — Current dependency configuration
- `/home/feanor/Projects/splice/src/graph/mod.rs` — Existing CodeGraph abstraction
- `/home/feanor/Projects/splice/src/proof/generation.rs` — generate_snapshot() implementation
- `/home/feanor/Projects/splice/src/proof/validation.rs` — validate_invariants() implementation
- [The Cargo Book - Features](https://doc.rust-lang.org/cargo/reference/features.html) — Official feature flag documentation

### Secondary (MEDIUM confidence)
- `/home/feanor/Projects/sqlitegraph/sqlitegraph/src/backend/native/v2/wal/mod.rs` — WAL API
- `/home/feanor/Projects/sqlitegraph/sqlitegraph/src/backend/native/v2/backup/mod.rs` — Backup API
- `/home/feanor/Projects/sqlitegraph/sqlitegraph/src/backend/native/v2/export/mod.rs` — Snapshot export API
- `/home/feanor/Projects/sqlitegraph/sqlitegraph/src/backend/native/v2/kv_store/mod.rs` — KV store API
- [RFC 3013 - Conditional Compilation Checking](https://rust-lang.github.io/rfcs/3013-conditional-compilation-checking.html) — cfg best practices
- [Cargo Issue #5210](https://github.com/rust-lang/cargo/issues/5210) — Feature unification in workspaces

### Tertiary (LOW confidence)
- Web search results for sqlitegraph native-v2 performance — claims need benchmark validation
- Community discussions on feature flag patterns — general guidance, not splice-specific

---
*Research completed: 2026-02-09*
*Ready for roadmap: yes*
