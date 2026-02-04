# Roadmap: Splice Refactoring Tool

## Milestones

- ✅ **v2.0 Production Safety** — Phases 1-10 (shipped 2026-01-18)
- ✅ **v2.2 Unified JSON & LLM Optimization** — Phases 11-18 (shipped 2026-01-23)
- ✅ **v2.2.1 Code Quality & Bug Fixes** — Phases 19-21 (shipped 2026-01-24)
- ✅ **v2.2.2 Magellan Integration** — Phases 22-26 (shipped 2026-01-24)
- ✅ **v2.2.4 Code Cleanup** — Phase 27 (shipped 2026-02-04)
- 🚧 **v2.3 Magellan v2 Integration** — Phases 28-32 (in progress)

## Phases

<details>
<summary>✅ v2.0 Production Safety (Phases 1-10) — SHIPPED 2026-01-18</summary>

**Milestone Goal:** Comprehensive overhaul for production safety, SQLiteGraph v1.0 upgrade, and structured output

See `.planning/milestones/v2.0-ROADMAP.md` for complete details of phases 1-10.

**Key Features Delivered:**
- Safety Foundation: Eliminated all unwrap() calls in production paths
- SQLiteGraph v1.0: Migrated to Native V2 backend
- Structured Output: Explicit field schema with stable identifiers
- Span-Aware Metadata: Line/column coordinates in all output
- Deterministic Ordering: Sorted output across all operations
- Validation Hooks: Pre/post verification with checksums
- Execution Logging: Complete audit trail with query capabilities
- Integration Testing: 75+ new tests for 7 languages
- Documentation: Full v2.0 documentation (README, manual, API)

</details>

<details>
<summary>✅ v2.2 Unified JSON & LLM Optimization (Phases 11-18) — SHIPPED 2026-01-23</summary>

**Milestone Goal:** Unified JSON Schema across all LLM tools with rich span extensions optimized for AI agent consumption and human-friendly CLI improvements

See `.planning/milestones/v2.2-ROADMAP.md` for complete details of phases 11-18.

**Key Features Delivered:**
- Rich Span Extensions: Context, semantic kind, language, checksums, error codes with zero breaking changes
- Rich Span Advanced: Relationships (callers, callees, imports, exports), tool hints, suggested actions
- CLI Conventions: `-n` dry-run, `-A`/`-B`/`-C` context flags, unified diff output, git-style exit codes
- Enhanced Errors: SPL-E### codes, severity levels, fuzzy suggestions, `splice explain` command
- Symbol Expansion: AST-aware parent chain walking with 6 language expanders
- Search & Apply: `splice search --pattern` with glob filtering, atomic find-and-replace
- Integration Testing: 340 tests passing across 7 languages, Magellan alignment

</details>

<details>
<summary>✅ v2.2.1 Code Quality & Bug Fixes (Phases 19-21) — SHIPPED 2026-01-24</summary>

**Milestone Goal:** Fix all 67 issues identified in comprehensive bug analysis, improving code reliability and safety

**Bug Analysis:** docs/BUG_ANALYSIS.md

**Summary:**
- Fixed 67 issues across 11 bug categories
- Eliminated all unsafe unwrap() patterns in production code
- Improved UTF-8 handling across all language modules
- Consolidated duplicate APIs (parser creation, import extraction, resolve symbol)
- Introduced testable configuration for execution logging
- All 340+ tests passing

**Delivered Plans:** 20/20
- Phase 19: 7 plans (Critical Error Handling)
- Phase 20: 7 plans (Lifetime & Resource Safety)
- Phase 21: 6 plans (API Consolidation & Code Quality)

</details>

<details>
<summary>✅ v2.2.2 Magellan Integration (Phases 22-26) — SHIPPED 2026-01-24</summary>

**Milestone Goal:** Unified CLI interface - Splice provides both Magellan query commands and span-safe editing

See `.planning/milestones/v2.2.2-ROADMAP.md` for complete details of phases 22-26.

**Key Features Delivered:**
- Symbol ID Format: 16-character hex IDs (SHA-256, first 8 bytes) compatible with Magellan
- Field Translation: Automatic conversion between Magellan (start_line) and Splice (line_start) field conventions
- Query Commands: status, find, refs, files, query commands with Magellan database backend
- Export Formats: json, jsonl, csv export with proper schema versioning
- Error Mapping: Magellan errors mapped to SPL-E### codes with original error preserved
- Exit Codes: Magellan-compatible exit codes (0=success, 1=error, 2=usage, 3=database, 4=file not found, 5=validation)
- Response Types: Magellan-compatible JSON response types for all query commands
- Documentation: Comprehensive Magellan integration documentation (docs/magellan_integration.md)

</details>

<details>
<summary>✅ v2.2.4 Code Cleanup (Phase 27) — SHIPPED 2026-02-04</summary>

**Milestone Goal:** Remove vestigial code from early Splice design phase

See `.planning/milestones/v2.2.4-ROADMAP.md` for complete details of phase 27.

**Key Features Delivered:**
- Dead Code Removal: Removed unused `Ingestor` struct stub from `src/ingest/mod.rs`
- Documentation Update: Updated all documentation to reflect Magellan-based architecture
- Migration Guidance: CHANGELOG.md entry with clear migration path to `MagellanIngestor`

</details>

---

### 🚧 v2.3 Magellan v2 Integration (In Progress)

**Milestone Goal:** Integrate Magellan v2.0.0 capabilities - enable cross-file rename with byte-accurate references and add semantic program transformation features

#### Phase 28: Dependency Upgrade
**Goal**: Upgrade to Magellan 2.0.0, SQLiteGraph 1.3.0, and add BLAKE3 dependency with backward compatibility
**Depends on**: Phase 27
**Requirements**: DEPS-01, DEPS-02, DEPS-03, SYMBOLID-01, SYMBOLID-02, DATA-05
**Success Criteria** (what must be TRUE):
  1. User can run splice after upgrade with all existing tests passing
  2. User can read databases with old 16-char SHA-256 Symbol IDs (backward compatible)
  3. User sees new 32-char BLAKE3 Symbol IDs in all JSON responses
  4. User can migrate old databases to new format using migration command
**Plans**: 4 plans

Plans:
- [ ] 28-01: Upgrade magellan to 2.0.0 and sqlitegraph to 1.3.0 ([28-01-PLAN.md](.planning/phases/28-dependency-upgrade/28-01-PLAN.md))
- [ ] 28-02: Add blake3 dependency and implement dual-format SymbolId ([28-02-PLAN.md](.planning/phases/28-dependency-upgrade/28-02-PLAN.md))
- [ ] 28-03: Update JSON output to use 32-char BLAKE3 IDs ([28-03-PLAN.md](.planning/phases/28-dependency-upgrade/28-03-PLAN.md))
- [ ] 28-04: Create database migration tool ([28-04-PLAN.md](.planning/phases/28-dependency-upgrade/28-04-PLAN.md))

#### Phase 29: Cross-File Rename Foundation
**Goal**: Implement byte-accurate cross-file rename using ReferenceFact spans
**Depends on**: Phase 28
**Requirements**: REFACTOR-01, REFACTOR-03, CLI-05
**Success Criteria** (what must be TRUE):
  1. User can rename a symbol across all files using `splice rename --symbol <id> --to <new_name>`
  2. User sees all references updated at exact byte spans (no regex false positives)
  3. User can preview rename changes before applying with --preview flag
  4. User can perform rename across all 7 supported languages (Rust, Python, C, C++, Java, JavaScript, TypeScript)
  5. User has automatic backup created before rename is applied
**Plans**: 5 plans

Plans:
- [ ] 29-01: Rename command with symbol lookup ([29-01-PLAN.md](.planning/phases/29-cross-file-rename/29-01-PLAN.md))
- [ ] 29-02: ReferenceFact-based span extraction ([29-02-PLAN.md](.planning/phases/29-cross-file-rename/29-02-PLAN.md))
- [ ] 29-03: Byte-accurate replacement with UTF-8 safety ([29-03-PLAN.md](.planning/phases/29-cross-file-rename/29-03-PLAN.md))
- [ ] 29-04: Preview mode and automatic backup ([29-04-PLAN.md](.planning/phases/29-cross-file-rename/29-04-PLAN.md))
- [ ] 29-05: Cross-language rename testing ([29-05-PLAN.md](.planning/phases/29-cross-file-rename/29-05-PLAN.md))

#### Phase 30: Impact Analysis & Graph Algorithms
**Goal**: Add graph algorithm integration and impact analysis for safer refactoring
**Depends on**: Phase 29
**Requirements**: REFACTOR-02, GRAPH-01, GRAPH-02, GRAPH-03, SLICE-01, CLI-06, CLI-07
**Success Criteria** (what must be TRUE):
  1. User can see impact analysis before refactoring (caller/callee chains, affected files)
  2. User can detect dead code from entry points using `splice dead-code --entry <symbol>`
  3. User can find cycles in call graph using `splice cycles` command
  4. User can analyze condensation graph (SCC collapse to DAG) using `splice condense`
  5. User can perform forward/backward slicing using `splice slice --target <id> --direction <forward|backward>`
**Plans**: TBD

Plans:
- [ ] 30-01: Implement impact analysis (caller/callee chains, reachability)
- [ ] 30-02: Add dead-code detection command
- [ ] 30-03: Add cycle detection command
- [ ] 30-04: Add condensation graph analysis command
- [ ] 30-05: Add program slicing command

#### Phase 31: Proof-Based Refactoring
**Goal**: Add machine-checkable behavioral equivalence proof generation
**Depends on**: Phase 30
**Requirements**: REFACTOR-04
**Success Criteria** (what must be TRUE):
  1. User can generate proof for rename operation using `splice rename --proof` flag
  2. User receives proof.json file with before/after graph snapshots
  3. User can verify graph invariants are preserved (reference counts, call structure)
  4. User sees SHA-256 checksums for audit trail in proof output
**Plans**: TBD

Plans:
- [x] 31-01: Design proof data structure (before/after snapshots, invariants)
- [x] 31-02: Implement proof generation for rename operations
- [x] 31-03: Add graph invariant validation (reference counts, reachability)
- [x] 31-04: Add checksums and audit trail to proof output

#### Phase 32: Integration Validation & Documentation
**Goal**: Comprehensive testing, performance validation, and documentation for v2.3 release
**Depends on**: Phase 31
**Requirements**: None (validation phase)
**Success Criteria** (what must be TRUE):
  1. All existing tests pass with new dependencies (407+ tests)
  2. New cross-file rename tests cover all 7 languages
  3. Graph algorithm commands return in <1 second for 10K symbols
  4. Documentation covers all new features (rename, impact analysis, graph algorithms, proof)
  5. README includes v2.3 feature highlights and usage examples
**Plans**: TBD

Plans:
- [ ] 32-01: Integration tests for cross-file rename
- [ ] 32-02: Performance tests for graph algorithms
- [ ] 32-03: Update documentation (README, manual, examples)
- [ ] 32-04: Release preparation (v2.3.0)

---

## Progress

**Execution Order:**
Phases execute in numeric order: 28 → 29 → 30 → 31 → 32

| Phase | Milestone | Plans Complete | Status | Completed |
|-------|-----------|----------------|--------|-----------|
| 1-10 | v2.0 | 31/31 | Complete | 2026-01-18 |
| 11-18 | v2.2 | 55/55 | Complete | 2026-01-23 |
| 19 | v2.2.1 | 7/7 | Complete | 2026-01-23 |
| 20 | v2.2.1 | 7/7 | Complete | 2026-01-24 |
| 21 | v2.2.1 | 6/6 | Complete | 2026-01-24 |
| 22. Symbol ID & Format Foundation | v2.2.2 | 4/4 | Complete | 2026-01-24 |
| 23. Magellan Integration Extensions | v2.2.2 | 5/5 | Complete | 2026-01-24 |
| 24. CLI Commands & Response Types | v2.2.2 | 5/5 | Complete | 2026-01-24 |
| 25. Export Command & Error Mapping | v2.2.2 | 4/4 | Complete | 2026-01-24 |
| 26. Integration Testing | v2.2.2 | 6/6 | Complete | 2026-01-24 |
| 27. Code Cleanup | v2.2.4 | 3/3 | Complete | 2026-02-04 |
| 28. Dependency Upgrade | v2.3 | 4/4 | Complete | 2026-02-04 |
| 29. Cross-File Rename Foundation | v2.3 | 5/5 | Complete | 2026-02-04 |
| 30. Impact Analysis & Graph Algorithms | v2.3 | 6/6 | Complete | 2026-02-04 |
| 31. Proof-Based Refactoring | v2.3 | 4/4 | Complete | 2026-02-04 |
| 32. Integration Validation & Documentation | v2.3 | 0/4 | Not started | - |

**Milestone Progress:**
- v2.0: 31/31 plans complete (100%)
- v2.2: 55/55 plans complete (100%)
- v2.2.1: 20/20 plans complete (100%)
- v2.2.2: 24/24 plans complete (100%)
- v2.2.4: 3/3 plans complete (100%)
- v2.3: 15/22 plans complete (68%)

---
*Last updated: 2026-02-04 - Phase 31 complete*
