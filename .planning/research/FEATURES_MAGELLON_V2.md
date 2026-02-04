# Feature Landscape: Magellan v2.0.0 Graph Algorithm Integration

**Project:** Splice v2.3.0 - Semantic Program Transformation
**Domain:** Cross-file rename + Graph algorithms for refactoring safety
**Researched:** 2026-02-04
**Overall Confidence:** HIGH

---

## Executive Summary

Splice v2.3.0 integrates Magellan v2.0.0's graph algorithms to enable **semantic program transformation** instead of just "find & replace with guardrails." The new capabilities leverage Magellan's byte-accurate `ReferenceFact` and `CallFact` structures plus `sqlitegraph`'s algorithm library to provide:

1. **Cross-file rename** with exact reference locations (byte spans)
2. **Impact analysis** before refactoring (what changes, what breaks)
3. **Dead code detection** (find unreachable symbols)
4. **Cycle detection** (mutual recursion safety)
5. **Program slicing** (forward/backward impact analysis)

**Key insight:** Magellan v2.0.0 provides **reference facts with byte-accurate spans** (`byte_start`, `byte_end`) and **call graph edges** (`CallFact` with caller/callee IDs). Splice can now perform semantic-aware refactoring using graph reachability, SCC decomposition, and path enumeration - all without custom parsers or external databases.

**Critical differentiator:** Splice's existing cross-file reference resolution (Rust-only, AST-based) + Magellan's cross-language reference extraction = **semantic refactoring across all 7 supported languages**, not just pattern replacement.

---

## Table Stakes

Features required for semantic refactoring. Missing = tool remains "text search + validation" only.

### 1. Cross-File Rename (Reference Resolution)

**Core requirement: Rename symbol across all files using byte-accurate reference locations**

**What Magellan v2.0.0 provides:**
```rust
// From /home/feanor/Projects/magellan/src/references.rs
pub struct ReferenceFact {
    pub file_path: PathBuf,
    pub referenced_symbol: String,
    pub byte_start: usize,
    pub byte_end: usize,
    pub start_line: usize,
    pub start_col: usize,
    pub end_line: usize,
    pub end_col: usize,
}
```

**Workflow:**
1. **Find symbol definition** via `CodeGraph::find_symbol(name)`
2. **Query all references** via `CodeGraph::references_to_symbol(symbol_id)`
3. **For each reference:**
   - Read file at `reference.file_path`
   - Replace bytes `[reference.byte_start, reference.byte_end)` with new name
   - Validate with tree-sitter + compiler
4. **Rollback atomically** if any validation fails

**Why essential:** Without cross-file reference resolution, rename only works in single file. Users must manually find/replace across files - error-prone and breaks semantic correctness.

**Current state (Splice v2.2.3):**
- ✅ Rust-only cross-file reference finding via `src/resolve/references/rust.rs`
- ❌ No rename operation (delete exists, but not rename)
- ❌ No cross-language support (Python, C, C++, Java, JS, TS lack import tracking)

**With Magellan v2.0.0:**
- ✅ Cross-language reference extraction (all 7 languages)
- ✅ Byte-accurate spans for precise replacement
- ✅ Call graph edges for impact analysis

**Complexity:** MEDIUM
- Use existing `src/resolve/references/rust.rs` for Rust (already works)
- Add Magellan-based reference lookup for other languages
- Reuse existing validation gates from `src/validate/gates.rs`

**Dependencies:**
- Magellan v2.0.0+ dependency
- Existing `src/graph/magellan_integration.rs` wrapper
- Existing `src/patch/mod.rs` for span-safe replacement

---

### 2. Impact Analysis (Before Refactoring)

**Core requirement: Show what will change before performing refactoring**

**What Magellan v2.0.0 provides:**
```rust
// From /home/feanor/Projects/magellan/src/graph/algorithms.rs
pub fn reachable_symbols(&self, symbol_id: &str, max_depth: Option<usize>) -> Result<Vec<SymbolInfo>>
pub fn reverse_reachable_symbols(&self, symbol_id: &str, max_depth: Option<usize>) -> Result<Vec<SymbolInfo>>
```

**Workflow:**
1. **User requests rename** `splice rename --symbol "process_data" --to "handle_data"`
2. **Splice runs impact analysis:**
   - `reverse_reachable_symbols("process_data")` → all functions that call this
   - `reachable_symbols("process_data")` → all functions this calls
3. **Display preview:**
   ```json
   {
     "operation": "rename",
     "symbol": "process_data",
     "new_name": "handle_data",
     "impact": {
       "references": 12,
       "files_affected": 4,
       "callers": ["main", "handle_request", "process_batch"],
       "callees": ["parse_input", "validate_output"]
     }
   }
   ```
4. **User confirms** → Apply changes
5. **Auto-rollback** if validation fails

**Why essential:** Refactoring without impact analysis is unsafe. Users need to know:
- How many files will change?
- What functions depend on this symbol?
- Will this break downstream code?

**Current state (Splice v2.2.3):**
- ❌ No impact analysis (delete shows references, but not caller/callee chains)
- ✅ Preview mode exists (`--preview` flag) but doesn't show graph impact

**With Magellan v2.0.0:**
- ✅ Forward reachability (what this symbol affects)
- ✅ Backward reachability (what affects this symbol)
- ✅ Call graph traversal depth (optional `--max-depth` flag)

**Complexity:** LOW
- Delegate to `CodeGraph::reachable_symbols()`
- Format results into existing JSON schema
- Add `--impact` flag to existing commands

**Dependencies:**
- Magellan graph algorithms already implemented
- Existing JSON output schema (`src/output.rs`)

---

### 3. Dead Code Detection

**Core requirement: Find unreachable symbols from entry points**

**What Magellan v2.0.0 provides:**
```rust
// From /home/feanor/Projects/magellan/src/graph/algorithms.rs
pub fn dead_symbols(&self, entry_symbol_id: &str) -> Result<Vec<DeadSymbol>>
```

**Workflow:**
```bash
# Find dead code unreachable from main
splice dead-code --db codegraph.db --entry main

# Output
{
  "data": {
    "entry_point": "main",
    "dead_symbols": [
      {
        "symbol": "unused_helper",
        "file_path": "src/lib.rs",
        "reason": "unreachable from entry point"
      }
    ]
  }
}
```

**Why essential:**
- **Code cleanup:** Remove unused functions before refactoring
- **Coverage gaps:** Find code tests never reach
- **Dead code elimination:** Safe removal of unreachable code

**Use cases:**
- Before large refactor: Remove dead code to reduce scope
- CI/CD integration: Fail PR if dead code exceeds threshold
- Technical debt cleanup: Identify unused abstractions

**Complexity:** LOW (delegates to Magellan)

**Dependencies:**
- Entry point symbol ID (e.g., `main`, `test_main`)
- Magellan call graph already indexed

**Limitations:**
- Only considers **call graph** (symbols called via reflection, function pointers, or dynamic dispatch may be incorrectly flagged)
- Test functions, benchmarks, platform-specific code may appear as dead code
- Requires explicit entry point (doesn't auto-discover `main`)

---

### 4. Cycle Detection (Mutual Recursion Safety)

**Core requirement: Detect strongly connected components (SCCs) in call graph**

**What Magellan v2.0.0 provides:**
```rust
// From /home/feanor/Projects/magellan/src/graph/algorithms.rs
pub fn detect_cycles(&self) -> Result<CycleReport>
pub fn find_cycles_containing(&self, symbol_id: &str) -> Result<Vec<Cycle>>
```

**Workflow:**
```bash
# Find all cycles in call graph
splice cycles --db codegraph.db

# Find cycles containing specific symbol
splice cycles --db codegraph.db --symbol "problematic_function"

# Output
{
  "data": {
    "cycles": [
      {
        "kind": "MutualRecursion",
        "members": [
          {"name": "process_a", "file_path": "src/lib.rs"},
          {"name": "process_b", "file_path": "src/lib.rs"}
        ]
      }
    ]
  }
}
```

**Why essential:**
- **Refactoring safety:** Identify tightly coupled code before changes
- **Mutual recursion detection:** Find indirect recursion causing stack overflow
- **Code smell detection:** Cycles often indicate architectural issues

**Use cases:**
- Before breaking apart a large function: Check if it's part of a cycle
- Before extracting module: Verify no cycles with new boundaries
- Performance tuning: Mutual recursion can prevent tail call optimization

**Complexity:** LOW (delegates to Magellan)

**Algorithm:** Uses Tarjan's SCC algorithm via `sqlitegraph::algo::strongly_connected_components()`

---

### 5. Program Slicing (Forward/Backward Impact)

**Core requirement: Find all code that affects (backward) or is affected by (forward) a symbol**

**What Magellan v2.0.0 provides:**
```rust
// From /home/feanor/Projects/magellan/src/graph/algorithms.rs
pub fn backward_slice(&self, symbol_id: &str) -> Result<SliceResult>
pub fn forward_slice(&self, symbol_id: &str) -> Result<SliceResult>
```

**Workflow:**
```bash
# Find what affects this symbol (backward slice)
splice slice --db codegraph.db --target "bug_location" --direction backward

# Find what this symbol affects (forward slice)
splice slice --db codegraph.db --target "config_loader" --direction forward

# Output
{
  "data": {
    "target": {"name": "config_loader", ...},
    "direction": "forward",
    "included_symbols": [
      {"name": "parse_config", ...},
      {"name": "validate_settings", ...}
    ],
    "statistics": {
      "total_symbols": 2,
      "data_dependencies": 0,
      "control_dependencies": 2
    }
  }
}
```

**Why essential:**
- **Bug isolation:** Find root cause by tracing dependencies
- **Refactoring safety:** Verify what changes when modifying a function
- **Impact analysis:** Understand blast radius of code changes

**Use cases:**
- Debugging: Trace backwards from error location to find source
- Refactoring: Check downstream effects before changing function
- Code review: Understand implications of proposed changes

**Current limitation (Magellan v2.0.0):**
- Uses **call-graph reachability as fallback** (not full CFG-based slicing)
- Does not include data flow dependencies within functions
- Does not include control flow from conditionals/loops
- Full slicing will require AST CFG integration (future)

**Complexity:** LOW (current fallback), HIGH (full CFG-based)

---

## Differentiators

Features that make Splice's semantic refactoring unique vs generic refactoring tools.

### 1. Cross-Language Semantic Rename

**Value Proposition:** Rename works the same way across Rust, Python, C, C++, Java, JavaScript, TypeScript

**Competitors:**
- **rust-analyzer:** Rust-only
- **PyCharm/IntelliJ:** Language-specific, IDE-bound
- **codemod:** Python-only, pattern-based (not semantic)

**Splice advantage:**
- ✅ 7 languages with consistent CLI interface
- ✅ Byte-accurate reference locations (no regex false positives)
- ✅ Graph-based impact analysis (what breaks)
- ✅ CLI tool (LLM-friendly, CI/CD automation)

**Example workflow:**
```bash
# Rename function across entire polyglot codebase
splice rename --symbol "process_data" --to "transform_data" --db codegraph.db

# Preview impact first
splice rename --symbol "process_data" --to "transform_data" --preview --show-callers

# Apply with backup
splice rename --symbol "process_data" --to "transform_data" --create-backup
```

**Complexity:** MEDIUM (reuses existing infrastructure)

---

### 2. Proof-Based Refactoring (--proof Flag)

**Value Proposition:** Generate machine-checkable proof that refactoring preserves behavior

**What this means:**
- **Before refactoring:** Capture call graph snapshot, reachable set, cycle structure
- **After refactoring:** Verify graph properties unchanged (except intended changes)
- **Output:** Signed proof artifact attesting to behavioral equivalence

**Proof content:**
```json
{
  "proof": {
    "operation": "rename",
    "symbol_before": "process_data",
    "symbol_after": "transform_data",
    "graph_properties": {
      "reachable_count_before": 42,
      "reachable_count_after": 42,
      "caller_count_before": 5,
      "caller_count_after": 5,
      "callee_count_before": 3,
      "callee_count_after": 3,
      "cycle_membership_before": ["scc_12"],
      "cycle_membership_after": ["scc_12"]
    },
    "checksum": "sha256:abc123...",
    "timestamp": "2026-02-04T12:00:00Z"
  }
}
```

**Why valuable:**
- **Safety audits:** Prove refactoring didn't change behavior
- **Compliance:** Regulated industries require change verification
- **CI/CD gates:** Block PRs that change graph structure unexpectedly

**Complexity:** MEDIUM (requires snapshot comparison logic)

**Dependencies:**
- Existing `src/checksum.rs` for SHA-256
- Existing `src/execution/log.rs` for audit trail

---

### 3. Condensation Graph Analysis

**Value Proposition:** Collapse SCCs to create DAG for safe refactoring order

**What Magellan v2.0.0 provides:**
```rust
// From /home/feanor/Projects/magellan/src/graph/algorithms.rs
pub fn condense_call_graph(&self) -> Result<CondensationResult>
```

**Workflow:**
```bash
# Show condensation graph (SCCs collapsed to supernodes)
splice condense --db codegraph.db --members

# Output
{
  "data": {
    "supernodes": [
      {
        "id": 1,
        "members": [
          {"name": "process_a", "file_path": "src/lib.rs"},
          {"name": "process_b", "file_path": "src/lib.rs"}
        ]
      }
    ],
    "edges": [[1, 2], [2, 3]]
  }
}
```

**Use cases:**
- **Topological sorting:** Determine safe refactoring order (break cycles first)
- **Layered architecture:** Identify natural layers in codebase
- **Impact analysis:** Changing one symbol affects its entire SCC

**Example: Breaking a Cycle**

1. **Detect cycle:**
   ```bash
   splice cycles --db codegraph.db
   # Found cycle: process_a → process_b → process_a
   ```

2. **Analyze SCC:**
   ```bash
   splice condense --db codegraph.db --members
   # SCC #1 contains: process_a, process_b
   ```

3. **Refactor in safe order:**
   - Extract shared logic from `process_a` and `process_b`
   - Rename to break dependency
   - Verify cycle removed with `splice cycles`

**Complexity:** LOW (delegates to Magellan)

---

## Anti-Features

Features to explicitly NOT build. Common mistakes in this domain.

### 1. DO NOT: Build Custom Rename Logic

**Why avoid:**
- Magellan already provides byte-accurate `ReferenceFact`
- Splice's existing `src/patch/mod.rs` handles span replacement
- Custom logic duplicates effort, introduces bugs

**Instead:** Use Magellan's reference extraction + Splice's span-safe patching

---

### 2. DO NOT: Require Full Indexing for Every Operation

**Why avoid:**
- Large codebases take time to index
- Users want quick single-file edits
- Forces workflow change

**Instead:**
- Lazy indexing: Index files on-demand during refactoring
- Cached database: Reuse existing `codegraph.db` from `magellan watch`
- Explicit opt-in: Users run `splice index` when needed

---

### 3. DO NOT: Implement Type-Based Refactoring

**Why avoid:**
- Requires type inference (extremely complex)
- Magellan doesn't provide type information
- Out of scope for "semantic refactoring" (call graph is sufficient)

**Instead:**
- Focus on **name-based refactoring** with graph validation
- Use compiler errors to catch type mismatches
- Document limitation: "Rename doesn't update type signatures"

---

### 4. DO NOT: Auto-Fix Broken References

**Why avoid:**
- Compiler errors are important signals
- Auto-fixing can hide semantic changes
- Users lose control over what changes

**Instead:**
- **Fail fast:** Rollback + show compiler errors
- **Suggest fixes:** Provide hints but don't auto-apply
- **Require explicit confirmation:** User approves each change

---

### 5. DO NOT: Support Generic/Rename All Instances

**Why avoid:**
- "Rename all instances" = regex find/replace (not semantic)
- Breaks when same name appears in different scopes
- Magellan already handles disambiguation via `symbol_id`

**Instead:**
- Require explicit symbol selection (`--symbol-id` or `--path` to disambiguate)
- Use `--ambiguous` flag to show all candidates
- Never guess which symbol to rename

---

## Per-Feature Complexity Analysis

| Feature | Complexity | Dependencies | Risk Level | MVP Priority |
|---------|------------|--------------|------------|--------------|
| Cross-file rename | MEDIUM | Magellan refs, Splice patch | MEDIUM | P0 (Must-have) |
| Impact analysis | LOW | Magellan reachability | LOW | P0 (Must-have) |
| Dead code detection | LOW | Magellan dead_symbols | LOW | P1 (Should-have) |
| Cycle detection | LOW | Magellan SCC | LOW | P1 (Should-have) |
| Program slicing | LOW (fallback) / HIGH (full CFG) | Magellan slice | MEDIUM | P2 (Nice-to-have) |
| Proof-based refactoring | MEDIUM | Checksums, execution log | LOW | P2 (Nice-to-have) |
| Condensation analysis | LOW | Magellan condense | LOW | P2 (Nice-to-have) |

---

## Implementation Phases

### Phase 1: Cross-File Rename (MVP Foundation)

**Goal:** Enable semantic rename across all 7 languages

**Tasks:**
1. Add `splice rename` command
   - `--symbol <NAME>`: Symbol to rename
   - `--to <NAME>`: New name
   - `--db <FILE>`: Magellan database
   - `--preview`: Show changes without applying
   - `--create-backup`: Backup before rename

2. Reference resolution via Magellan
   - Query `ReferenceFact` by symbol name
   - Get byte spans for all references
   - Filter by file path if `--path` provided

3. Apply changes
   - Reuse `src/patch/mod.rs` span replacement
   - Validate with tree-sitter + compiler
   - Rollback atomically on failure

4. Output format
   ```json
   {
     "operation": "rename",
     "symbol": "process_data",
     "new_name": "handle_data",
     "references_updated": 12,
     "files_affected": 4,
     "validation": "passed"
   }
   ```

**Dependencies:**
- Existing `src/patch/mod.rs`
- Existing `src/validate/gates.rs`
- Existing `src/graph/magellan_integration.rs`

**Complexity:** MEDIUM
**Risk:** MEDIUM (rename is high-impact operation)

---

### Phase 2: Impact Analysis (Safety Layer)

**Goal:** Show what will change before applying rename

**Tasks:**
1. Add `--impact` flag to `splice rename`
   - Show caller/callee counts
   - List affected files
   - Display reference count

2. Integrate Magellan graph algorithms
   - `reverse_reachable_symbols()` → callers
   - `reachable_symbols()` → callees

3. Preview enhancement
   ```bash
   splice rename --symbol "process_data" --to "handle_data" --preview --impact
   ```

4. Output enhancement
   ```json
   {
     "operation": "rename",
     "impact": {
       "callers": ["main", "handle_request"],
       "callees": ["parse_input", "validate_output"],
       "references": 12,
       "files_affected": 4
     }
   }
   ```

**Dependencies:**
- Magellan graph algorithms (already implemented)
- Phase 1: Cross-file rename

**Complexity:** LOW
**Risk:** LOW (read-only queries)

---

### Phase 3: Graph Analysis Commands (Standalone Tools)

**Goal:** Add graph algorithm commands for code exploration

**Tasks:**
1. `splice dead-code`
   - `--entry <SYMBOL_ID>`: Entry point (e.g., `main`)
   - List unreachable symbols

2. `splice cycles`
   - `--symbol <SYMBOL_ID>`: Find cycles containing this symbol
   - List all SCCs with >1 member

3. `slice condense`
   - `--members`: Show SCC members
   - Display condensation DAG

4. `splice slice`
   - `--target <SYMBOL_ID>`: Target symbol
   - `--direction <backward|forward>`: Slice direction

**Dependencies:**
- Magellan graph algorithms (already implemented)
- JSON output formatting

**Complexity:** LOW (delegation only)
**Risk:** LOW (read-only queries)

---

### Phase 4: Proof-Based Refactoring (Differentiator)

**Goal:** Generate verifiable proofs of behavioral equivalence

**Tasks:**
1. Capture pre-refactoring graph snapshot
   - Reachable set from symbol
   - Caller/callee counts
   - Cycle membership
   - SHA-256 checksum

2. Apply refactoring (Phase 1-2)

3. Capture post-refactoring graph snapshot
   - Compare with pre-refactoring state
   - Verify invariants (counts, memberships)

4. Generate proof artifact
   ```json
   {
     "proof": {
       "operation": "rename",
       "invariants_preserved": ["reachable_count", "caller_count", "cycle_membership"],
       "checksum_before": "sha256:abc...",
       "checksum_after": "sha256:def...",
       "signature": "..."
     }
   }
   ```

5. Add `--proof` flag to `splice rename`

**Dependencies:**
- Phase 1-3: All previous features
- Existing `src/checksum.rs`
- Existing `src/execution/log.rs`

**Complexity:** MEDIUM (snapshot comparison logic)
**Risk:** LOW (verification only, doesn't change refactoring logic)

---

## Open Questions & Research Flags

### Verified (No Research Needed):

- Magellan `ReferenceFact` structure (byte spans)
- Magellan `CallFact` structure (caller/callee edges)
- Magellan graph algorithms API (`reachable_symbols`, `dead_symbols`, etc.)
- Splice's existing span-safe patching infrastructure
- Splice's existing validation gates

### Research Flags (LOW confidence):

1. **Rename operation error recovery**
   - **Flag:** How to handle validation failures mid-rename
   - **Research needed:** "If 5th file validation fails, do we rollback all 5 files or only the failed one?"
   - **Confidence:** MEDIUM (existing rollback infrastructure handles this)
   - **Action:** Test with multi-file rename scenarios

2. **Disambiguation UX**
   - **Flag:** How to present ambiguous symbol choices to users
   - **Research needed:** "Show interactive prompt or error with --ambiguous flag?"
   - **Confidence:** MEDIUM (Magellan provides `--ambiguous` flag)
   - **Action:** Follow Magellan's CLI conventions

3. **Performance on large codebases**
   - **Flag:** Latency for rename in 10K+ file codebase
   - **Research needed:** "What's the worst-case rename time?"
   - **Confidence:** LOW (no benchmarks yet)
   - **Action:** Performance testing during implementation

4. **Cross-language import tracking**
   - **Flag:** Does Magellan extract cross-language references (e.g., Python → Rust via PyO3?)
   - **Research needed:** "Are imports tracked across language boundaries?"
   - **Confidence:** LOW (need to test)
   - **Action:** Test with polyglot codebase

---

## Cross-Feature Dependencies

```
[Magellan v2.0.0 Integration]
    ├─> [Cross-File Rename] (P0)
    │   ├─> [Reference Resolution] ──> Magellan ReferenceFact
    │   ├─> [Span Replacement] ──> Existing src/patch/mod.rs
    │   └─> [Validation] ──> Existing src/validate/gates.rs
    │
    ├─> [Impact Analysis] (P0)
    │   ├─> [Reachability Queries] ──> Magellan reachable_symbols()
    │   └─> [Preview Enhancement] ──> Phase 1 rename command
    │
    ├─> [Dead Code Detection] (P1)
    │   └─> [Dead Symbols Query] ──> Magellan dead_symbols()
    │
    ├─> [Cycle Detection] (P1)
    │   └─> [SCC Decomposition] ──> Magellan detect_cycles()
    │
    ├─> [Program Slicing] (P2)
    │   └─> [Slice Queries] ──> Magellan backward_slice() / forward_slice()
    │
    ├─> [Condensation Analysis] (P2)
    │   └─> [DAG Construction] ──> Magellan condense_call_graph()
    │
    └─> [Proof-Based Refactoring] (P2)
        ├─> [Graph Snapshots] ──> All graph algorithms
        ├─> [Checksum Verification] ──> Existing src/checksum.rs
        └─> [Proof Generation] ──> New logic
```

### Dependency Chain for MVP (Phases 1-2)

1. **Foundation: Magellan Integration** (Already exists)
   - `src/graph/magellan_integration.rs`
   - `src/ingest/magellan.rs`

2. **Phase 1: Cross-File Rename**
   - Add `rename` command to `src/cli/mod.rs`
   - Implement reference resolution via Magellan `ReferenceFact`
   - Reuse `src/patch/mod.rs` for span replacement
   - Add to `src/execution/log.rs` for audit trail

3. **Phase 2: Impact Analysis**
   - Add `--impact` flag to rename command
   - Query Magellan graph algorithms (`reachable_symbols`, `reverse_reachable_symbols`)
   - Format impact data into JSON output

4. **Phase 3: Standalone Commands**
   - Add `dead-code`, `cycles`, `condense`, `slice` commands
   - Delegate to Magellan graph algorithms
   - Format output per Splice JSON schema

5. **Phase 4: Proof-Based Refactoring**
   - Add snapshot capture logic
   - Compare pre/post graph states
   - Generate proof artifact
   - Add `--proof` flag to rename command

---

## Sources

### HIGH Confidence (Verified from source code and documentation)

- **Magellan MANUAL.md:** `/home/feanor/Projects/magellan/MANUAL.md` - Complete API specification
- **Magellan graph algorithms:** `/home/feanor/Projects/magellan/src/graph/algorithms.rs` - Algorithm implementations
- **Magellan reference extraction:** `/home/feanor/Projects/magellan/src/references.rs` - `ReferenceFact` and `CallFact` structures
- **Splice cross-file resolution:** `/home/feanor/Projects/splice/src/resolve/references/rust.rs` - Existing Rust-only reference finding
- **Splice cross-file module:** `/home/feanor/Projects/splice/src/resolve/cross_file.rs` - Cross-language import tracking
- **Splice README:** `/home/feanor/Projects/splice/README.md` - v2.2.3 feature set

### MEDIUM Confidence (Inferred from API structure)

- Rename operation workflow (based on existing delete command)
- Impact analysis output format (based on existing JSON schema)
- Graph algorithm performance characteristics (from Magellan docs)

### LOW Confidence (Needs verification)

- Cross-language import tracking completeness
- Performance on large codebases (10K+ files)
- Polyglot codebase support (e.g., Python → Rust via PyO3)

---

*Document created: 2026-02-04*
*Status: Ready for roadmap creation*
*Confidence: HIGH (graph algorithms), MEDIUM (rename workflow), LOW (cross-language edge cases)*
