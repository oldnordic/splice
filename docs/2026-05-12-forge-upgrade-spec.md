# Forge Upgrade Spec — Honest Assessment & Path Forward

**Date:** 2026-05-12
**Status:** Draft
**Decision:** Upgrade, not rewrite. Keep skeleton, replace organs.

---

## Why Not From Scratch

| Factor | Rewrite | Upgrade |
|--------|---------|---------|
| Type system (Symbol, Reference, Block, Path, DominatorTree) | Re-solve from zero | Already correct, keep |
| Module boundaries (graph/search/cfg/edit/analysis) | Re-solve from zero | Already correct, keep |
| Feature flag architecture (optional deps) | Re-solve from zero | Already works, keep |
| ForgeBuilder API | Re-solve from zero | Already ergonomic, keep |
| Agent 6-phase loop | Re-solve from zero | Structure sound, keep |
| Workflow DAG engine (1,300+ lines) | Re-solve from zero | Executor + checkpoint + rollback are real code |
| Stub implementations | N/A | Gut and rewrite (this is the work) |
| Stale dependencies | N/A | Bump and fix breaking changes |
| Event system (ci-events) | Write new | Add as new feature flag |

**Bottom line:** ~30% of forge is real, working code worth keeping. ~70% is stubs, placeholders, or wrong implementations. But that 30% is the architecture — the expensive design work. The 70% is mechanical fill-in.

---

## What To Keep (Real, Working Code)

### forge_core
- **storage/mod.rs** — UnifiedGraphStore, dual backend, connection management. Works.
- **graph/mod.rs** — Symbol/Reference types, find_symbols (magellan path works). Keep the type conversions.
- **types.rs** — SymbolId, FileId, SymbolKind, ReferenceKind. Solid.
- **lib.rs** — Forge struct, ForgeBuilder, module organization. Keep.
- **error.rs** — ForgeError enum. Well-typed.

### forge_agent
- **workflow/dag.rs** (1,014 lines) — DAG construction, validation, topological sort. Real.
- **workflow/checkpoint.rs** (1,479 lines) — State serialization, recovery. Real.
- **workflow/rollback.rs** (1,323 lines) — Compensation-based rollback. Mostly real.
- **workflow/executor.rs** (3,144 lines) — Fork-join parallelism, timeout handling. Real engine.
- **workflow/tools.rs** (1,630 lines) — Tool registry, external tool management. Real.
- **planner.rs** — Plan representation, step decomposition. Keep structure.
- **observe.rs** — File diff computation. Keep structure.

### forge-reasoning
- **All types** — Hypothesis, KnowledgeGap, Checkpoint. Keep as types.
- **The implementations** — Gut. Most return placeholders.

### forge_runtime
- **watcher.rs** — File watching with notify crate. Works but thin.
- **cache.rs** — LRU cache structure. Works but unused.

---

## What To Gut (Stubs, Placeholders, Wrong Implementations)

### forge_core — Gut these implementations:

**cfg/mod.rs** — `paths_execute()` returns `vec![BlockId(0)]`. `dominators()` builds an empty tree. These should delegate to mirage, not pretend to work.

**edit/mod.rs** — `patch_symbol()` walks directories grepping for `fn name(`. This is what splice exists to do. The entire string-matching implementation gets replaced with splice calls.

**analysis/mod.rs** (1,220 lines) — Dead code detection, complexity metrics. The struct/types are fine, but the actual analysis logic needs to come from mirage/magellan, not be reimplemented.

**indexing.rs** — "For v0.2, we'll store a placeholder record". The indexing should delegate to magellan.

### forge_agent — Gut these:

**mutate.rs** — Rename/Delete/Modify all say "placeholder for v0.3". Replace with splice library calls.

**observe.rs:76** — "Return a placeholder symbol". Replace with magellan queries.

### forge-reasoning — Gut implementations, keep types:

The cognitive scaffolding types are good (Hypothesis, KnowledgeGap, ImpactAnalysis). But the implementations don't actually reason — they return stubs. Replace with atheneum queries where applicable, keep the type structure for in-memory reasoning.

---

## What To Build New

### 1. ci-events integration

New feature flag in forge_core:

```toml
[features]
events = ["ci-events"]
```

Forge gains event emission after mutations, event subscription for auto-refresh. See the integration spec at `docs/2026-05-12-code-intelligence-integration-design.md`.

### 2. Real tool delegation

Every module that currently reimplements tool logic gets replaced with actual library calls:

| Module | Current | Replace With |
|--------|---------|-------------|
| graph | sqlitegraph direct | magellan crate API |
| search | SQL filter builder | llmgrep crate API |
| cfg | placeholder paths | mirage crate API |
| edit | string grep | splice crate API |
| analysis | reimplemented | mirage + magellan queries |

### 3. Envoy subscription in forge_runtime

`ForgeRuntime` gains an envoy client that subscribes to `CodeMutated` events and triggers incremental re-index via magellan.

### 4. Atheneum persistence in forge-reasoning

The reasoning layer's hypotheses and knowledge gaps get persisted to atheneum instead of being session-only.

---

## Dependency Bump

### Current → Target

| Dep | Current | Target | Breaking Changes |
|-----|---------|--------|-----------------|
| magellan | 2.4.6 | 3.3.8 | Schema v14→v16, API restructured, CodeGraph::open signature changed |
| llmgrep | 3.0.9 | 3.4.2 | Query API changes, mode parameter added |
| mirage-analyzer | 1.0.3 | 1.4.2 | CFG extraction API changed |
| splice | 2.5.3 | 2.6.9 | `validate_utf8_span` and `parser_for_language` now require `file_path` param; `From<io::Error>` removed |
| sqlitegraph | 2.0.8 | 2.2.4 | Schema v3→v4 expected |

This is the highest-risk part. The magellan 2.4→3.3 jump spans ~50 releases with API restructuring. The safest approach: update one dep at a time, fix compile errors, run tests.

---

## Phased Plan

### Phase 1: Dependency bump (1-2 days)

Bump all deps to current versions. Fix compile errors. Get tests passing again with the new APIs. Don't change any logic — just adapt to breaking changes.

**Validation:** `cargo test` passes with updated deps.

### Phase 2: Gut stubs, delegate to real tools (3-5 days)

Replace placeholder implementations with actual library calls:

1. **cfg/mod.rs** — `PathEnumerator::execute()` calls mirage instead of returning `BlockId(0)`
2. **edit/mod.rs** — `EditModule::patch_symbol()` calls splice instead of grep
3. **graph/mod.rs** — Update magellan integration for v3.3.8 API
4. **agent/mutate.rs** — Rename/delete/modify call splice library
5. **analysis/mod.rs** — Dead code and complexity delegate to mirage

**Validation:** Every function that previously returned a placeholder now returns real data. Tests updated to test against real tool behavior.

### Phase 3: Add ci-events (2-3 days)

New `ci-events` dependency. Forge emits `CodeMutated` after edits, `AnalysisCompleted` after analysis. Controlled by feature flag.

**Validation:** Forge operation → event appears in envoy.

### Phase 4: Envoy subscription in runtime (2-3 days)

`ForgeRuntime` subscribes to events, triggers re-index and re-analysis automatically.

**Validation:** External splice edit → forge auto-refreshes.

### Phase 5: Atheneum persistence (2-3 days)

Reasoning discoveries persisted to atheneum. Query API for past knowledge.

**Validation:** `forge.query_knowledge("symbol")` returns past discoveries.

---

## What Gets Removed Entirely

- The `patch_symbol_in_dir` recursive string-grep implementation in edit/mod.rs
- The placeholder `BlockId(0)` return in cfg/mod.rs
- The `"placeholder for v0.3"` comments and their surrounding no-op code in mutate.rs
- The `Native V3` backend code (already decided to remove in other projects)
- Dead code in storage that returns empty results "for placeholder implementation"

## What Gets Added

- `ci-events` dependency and event emission
- `envoy` client in forge_runtime
- `atheneum` client in forge-reasoning
- Real mirage/splice library calls replacing stubs

## Estimated Effort

| Phase | Time | Risk |
|-------|------|------|
| Dep bump | 1-2 days | Medium (magellan API changes) |
| Gut stubs | 3-5 days | Medium (integration testing) |
| ci-events | 2-3 days | Low (additive) |
| Envoy subscription | 2-3 days | Low (additive) |
| Atheneum persistence | 2-3 days | Low (additive) |
| **Total** | **10-16 days** | |

Compare to writing from scratch: 30-40 days to reach the same point (re-solving architecture, types, DAG engine, workflow orchestration, agent loop). The upgrade saves ~60% of the effort by keeping the design work.
