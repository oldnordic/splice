# Code Intelligence Integration Design

**Date:** 2026-05-12
**Status:** Draft
**Applies to:** magellan, llmgrep, mirage, splice, envoy, atheneum

---

## Problem

The code intelligence stack is six tools that share a data format (Magellan SQLite) but have no runtime coordination. Splice edits code, magellan indexes it, mirage analyzes it, llmgrep queries it — but nothing connects these steps. Agents using these tools must manually orchestrate: run splice, then run magellan refresh, then run mirage cfg. Results and discoveries are lost between sessions.

## Goal

A unified runtime where:

1. **Agent orchestration** — agents use the full stack through envoy; multi-agent refactoring is coordinated
2. **Unified CLI** — `splice` delegates to llmgrep/mirage for queries, making one tool the entry point
3. **Live feedback loop** — code mutations flow through the stack automatically
4. **Knowledge accumulation** — every operation feeds atheneum, building persistent project memory

## Architecture

```
                    ┌─────────────┐
                    │    Envoy     │  ← HTTP message broker, event fan-out
                    │   (hub)      │
                    └──────┬──────┘
                           │
              ┌────────────┼────────────┐
              │            │            │
        ┌─────┴─────┐ ┌───┴───┐ ┌─────┴─────┐
        │ Atheneum   │ │ ci-   │ │ Envoy      │
        │ (memory)   │ │ events│ │ subs/filter│
        └───────────┘ │ crate │ └───────────┘
                      └───┬───┘
                          │
     ┌────────────────────┼────────────────────┐
     │                    │                    │
┌────┴─────┐       ┌─────┴──────┐      ┌─────┴──────┐
│ Splice   │       │ Magellan   │      │ Mirage     │
│ (mutate) │──────▶│ (index)    │◀─────│ (analyze)  │
└──────────┘       └─────┬──────┘      └─────┬──────┘
                         │                    │
                   ┌─────┴──────┐      ┌─────┴──────┐
                   │ llmgrep    │      │ atheneum   │
                   │ (query)    │      │ (store)    │
                   └────────────┘      └────────────┘
```

**Principle:** Envoy is the only runtime dependency for integration. Tools work standalone without it (they just don't publish/consume events). The `ci-events` crate defines event types only — no transport, no I/O.

---

## ci-events Crate

A thin, stable crate defining typed events. All tools depend on this crate for event serialization. No transport code — just types and serde.

**Crate name:** `ci-events`
**Versioning:** Semver. Breaking changes to event types require a major version bump.

### Event Envelope

Every event has a standard envelope:

```rust
pub struct CiEvent {
    pub id: String,              // UUID v7 (time-ordered)
    pub timestamp: String,       // ISO 8601
    pub source: CiSource,        // which tool emitted this
    pub event_type: CiEventType, // the typed payload
    pub correlation_id: Option<String>, // links request → response events
    pub metadata: HashMap<String, String>,
}
```

### Sources

```rust
pub enum CiSource {
    Splice,
    Magellan,
    Mirage,
    Llmgrep,
    Envoy,
    Agent { name: String },
}
```

### Event Types

```rust
pub enum CiEventType {
    // --- Code mutation (splice emits) ---
    CodeMutated(CodeMutatedPayload),      // emitted for all mutation types
    RenameApplied(RenameAppliedPayload),  // additional detail when operation is rename
    BatchCompleted(BatchCompletedPayload),// summary after batch operations

    // --- Indexing (magellan emits) ---
    IndexStarted(IndexStartedPayload),
    IndexCompleted(IndexCompletedPayload),
    SymbolsDiscovered(SymbolsDiscoveredPayload),

    // --- Analysis (mirage/llmgrep emit) ---
    AnalysisCompleted(AnalysisCompletedPayload),
    BlastZoneComputed(BlastZonePayload),
    HotspotDetected(HotspotPayload),

    // --- Knowledge (atheneum emits) ---
    DiscoveryStored(DiscoveryStoredPayload),
    PatternRecognized(PatternPayload),

    // --- Agent coordination (envoy emits) ---
    HandoffRequested(HandoffPayload),
    HandoffClaimed(HandoffClaimedPayload),
}
```

### Key Payloads

```rust
pub struct CodeMutatedPayload {
    pub operation: MutationOperation,
    pub files: Vec<String>,
    pub symbols_affected: Vec<String>,
    pub workspace_root: String,
    pub dry_run: bool,
}

pub enum MutationOperation {
    Patch,
    Delete,
    Rename,
    ApplyFiles,
    Batch,
    Create,
}

pub struct IndexCompletedPayload {
    pub db_path: String,
    pub files_indexed: usize,
    pub symbols_added: usize,
    pub symbols_updated: usize,
    pub symbols_removed: usize,
    pub duration_ms: u64,
}

pub struct AnalysisCompletedPayload {
    pub analyzer: AnalyzerKind,
    pub db_path: String,
    pub findings_count: usize,
    pub duration_ms: u64,
}

pub enum AnalyzerKind {
    Cfg,
    Dominance,
    DeadCode,
    Cycles,
    PatternSearch,
}

pub struct BlastZonePayload {
    pub symbol: String,
    pub file: String,
    pub affected_files: Vec<String>,
    pub affected_symbols: Vec<String>,
    pub max_depth: usize,
}

pub struct SymbolsDiscoveredPayload {
    pub db_path: String,
    pub symbols: Vec<SymbolSummary>,
}

pub struct SymbolSummary {
    pub name: String,
    pub kind: String,
    pub file: String,
    pub line: usize,
}
```

### Serialization

All events serialize to JSON with a mandatory `event_type` discriminator:

```json
{
  "id": "019164a2-...",
  "timestamp": "2026-05-12T14:30:00Z",
  "source": "splice",
  "event_type": "code_mutated",
  "correlation_id": null,
  "metadata": {},
  "payload": {
    "operation": "rename",
    "files": ["src/lib.rs", "src/main.rs"],
    "symbols_affected": ["old_name"],
    "workspace_root": "/home/user/project",
    "dry_run": false
  }
}
```

---

## Envoy Extensions

Envoy becomes the event backbone. Three additions to its existing HTTP API:

### 1. Event Publishing

```
POST /api/v1/events
Content-Type: application/json
Authorization: Bearer <token>

<CiEvent JSON>
```

Returns `202 Accepted` with `{"event_id": "..."}`. Envoy persists the event and fans out to subscribers.

### 2. Typed Subscriptions

```
POST /api/v1/subscriptions
{
  "subscriber": "magellan-watcher",
  "filters": {
    "event_types": ["code_mutated", "rename_applied"],
    "sources": ["splice"],
    "file_patterns": ["src/**/*.rs"]
  },
  "delivery": {
    "webhook": "http://localhost:9877/events",
    "channel": "magellan-events"
  }
}
```

Subscribers can filter by event type, source, file glob, or symbol name. Delivery is via webhook push or channel poll.

### 3. Event Replay

```
GET /api/v1/events?since=2026-05-12T14:00:00Z&event_type=code_mutated
```

New subscribers can replay missed events. Retention is configurable (default: 7 days).

### 4. Atheneum Auto-Ingest

When the atheneum feature is enabled on envoy, every incoming event is automatically ingested as a knowledge node:

```
Symbol node ← CodeMutated.events.0.symbols_affected[0]
File node   ← CodeMutated.events.0.files[0]
Pattern     ← AnalysisCompleted → stored as discovery
```

This is the knowledge accumulation layer — no tool needs to explicitly write to atheneum.

---

## Tool Integration Points

### Splice (mutator)

**Emits:**
- `CodeMutated` after every successful apply (patch, delete, rename, apply-files, batch, create)
- `RenameApplied` specifically for rename operations (includes old/new name)
- `BatchCompleted` for batch operations (includes step results)

**Subscribes:**
- None initially. Future: subscribe to `AnalysisCompleted` to surface blast zone warnings before applying.

**Implementation:**
1. Add `ci-events` dependency
2. After successful mutation, serialize `CiEvent` and POST to `http://localhost:9876/api/v1/events`
3. If envoy is unreachable, silently continue (graceful degradation)
4. Controlled by `--no-events` flag (default: events enabled when envoy is running)

### Magellan (indexer)

**Emits:**
- `IndexStarted` when `magellan watch` begins a re-index cycle
- `IndexCompleted` when indexing finishes (includes symbol delta)
- `SymbolsDiscovered` when new symbols are found

**Subscribes:**
- `CodeMutated` — triggers incremental re-index of affected files (replaces current filesystem watcher for splice-originated changes)

**Implementation:**
1. Add `ci-events` dependency
2. `magellan watch` gains `--subscribe-envoy` flag
3. On receiving `CodeMutated`, run `magellan refresh --include <affected_files>` for just the changed files
4. After refresh completes, emit `IndexCompleted`

### Mirage (analyzer)

**Emits:**
- `AnalysisCompleted` after cfg/paths/dominance/cycles/dead-code runs
- `BlastZoneComputed` after blast zone analysis
- `HotspotDetected` when complexity thresholds are exceeded

**Subscribes:**
- `IndexCompleted` — triggers analysis of affected symbols (optional, opt-in)
- `CodeMutated` — triggers blast zone analysis of mutated symbols

**Implementation:**
1. Add `ci-events` dependency
2. After analysis, emit results to envoy
3. `mirage watch` mode: subscribe to events, run analysis on changes

### llmgrep (query)

**Emits:**
- None (read-only tool)

**Subscribes:**
- `IndexCompleted` — invalidates internal cache if caching is added

**Implementation:**
1. Minimal integration — llmgrep is query-only
2. Future: agent queries via envoy instead of direct CLI

### Envoy (hub)

**Changes:**
1. Add `/api/v1/events` endpoint (publish + query)
2. Add `/api/v1/subscriptions` endpoint (typed filters)
3. Auto-ingest events to atheneum when feature is enabled
4. Event retention and replay

### Atheneum (memory)

**Changes:**
1. Define node types: `Symbol`, `File`, `Mutation`, `Analysis`, `Pattern`, `Discovery`
2. Auto-populate from envoy event stream
3. Expose query API: `atheneum query --symbol foo --depth 2`
4. Support time-range queries: "what changed in the last hour?"

---

## Data Flow Examples

### Example 1: Splice Rename with Blast Radius

```
1. Agent:  splice rename --symbol old_fn --to new_fn --file src/lib.rs
2. Splice: emits CodeMutated { rename, files: [...], symbols: ["old_fn"] }
3. Envoy:  fans out to subscribers
4. Magellan: receives CodeMutated → magellan refresh --include <files>
5. Magellan: emits IndexCompleted { symbols_updated: 5, ... }
6. Mirage:   receives IndexCompleted → mirage blast-zone --symbol new_fn
7. Mirage:   emits BlastZoneComputed { affected_files: 3, affected_symbols: 7 }
8. Atheneum: stores mutation + blast zone as linked knowledge nodes
9. Agent:  queries atheneum for blast zone before confirming
```

### Example 2: Multi-Agent Refactoring

```
1. Agent A: splice rename --symbol foo --to bar --dry-run
2. Splice:  emits CodeMutated { dry_run: true, symbols: ["foo"] }
3. Envoy:   fans out
4. Mirage:  computes blast zone, emits BlastZoneComputed
5. Envoy:   creates Handoff for Agent B to handle affected files
6. Agent B: claims handoff, performs secondary renames
7. Agent B: splice rename --symbol baz --to qux --file src/utils.rs
8. Splice:  emits CodeMutated { rename, ... }
9. Envoy:   fans out, magellan re-indexes
10.Atheneum: stores full refactoring chain with causality
```

### Example 3: Knowledge Query

```
1. Agent:  llmgrep --db .magellan/project.db search --query "error handling"
2. Agent:  atheneum query --pattern "error_handling" --depth 2
3. Agent:  atheneum returns: symbol nodes, past mutations, blast zones, analysis results
4. Agent:  uses combined context to plan a safe refactoring
```

---

## Phased Implementation

### Phase 1: Event Plumbing (ci-events + envoy endpoints)

- Create `ci-events` crate with all event types
- Add `/api/v1/events` and `/api/v1/subscriptions` to envoy
- Add event publishing to splice (CodeMutated only)
- Validate: splice edit → event appears in envoy log

### Phase 2: Magellan Integration

- Add event subscription to magellan watch
- Emit IndexCompleted after re-index
- Validate: splice edit → magellan auto-refreshes → IndexCompleted emitted

### Phase 3: Mirage Integration

- Add event publishing for AnalysisCompleted, BlastZoneComputed
- Subscribe to IndexCompleted for automatic analysis
- Validate: splice edit → magellan re-indexes → mirage auto-analyzes → results in envoy

### Phase 4: Atheneum Knowledge Layer

- Define knowledge node types
- Auto-ingest events in envoy
- Expose query API
- Validate: query atheneum for past refactoring history

### Phase 5: Unified Splice CLI

- Splice gains `splice analyze --symbol X` (delegates to mirage)
- Splice gains `splice search --pattern Y` (delegates to llmgrep)
- Splice gains `splice context --symbol Z` (queries atheneum)
- All results in consistent JSON format

---

## Constraints and Decisions

- **Graceful degradation**: If envoy is down, all tools work normally. Events are best-effort. No blocking on envoy.
- **Event ordering**: UUID v7 provides time-ordering. No distributed consensus needed.
- **Event size**: Events are small (KB, not MB). Large payloads (full file contents, large analysis results) go in the DB, events contain only references.
- **Schema evolution**: Events are versioned by `ci-events` crate version. New fields are optional. Breaking changes bump major version.
- **Auth**: Envoy's existing token-based auth. Each tool authenticates with a named agent token.
- **Offline queuing**: Not in scope for Phase 1-3. Tools silently skip events when envoy is unreachable.

---

## Not In Scope

- Real-time collaborative editing (OT/CRDT)
- IDE plugin integration (use existing LSP layer)
- Cross-project analysis (each project has its own magellan DB)
- Distributed deployment (single-machine, local-first)
