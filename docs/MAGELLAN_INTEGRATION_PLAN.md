# Magellan Integration Plan (Phase 3)

**Created**: 2026-01-02
**Status**: Ready for Implementation
**Basis**: Magellan v0.5.0 + Splice v0.4.1

---

## Executive Summary

Magellan v0.5.0 is now published with:
- Working multi-language parsers (7 languages)
- Label-based symbol queries
- Code chunk storage (no file re-reading needed)
- 97/97 tests passing

**Recommendation:** Splice should use Magellan's `CodeGraph` instead of sqlitegraph directly.

---

## Current Situation

### Splice v0.4.1 State
- Uses `sqlitegraph 0.2.10` directly
- Has own parsers (Rust, Python, C/C++, JS/TS, Java)
- Ingest module returns "Not implemented yet" errors
- No label-based queries
- No code chunk storage

### Magellan v0.5.0 Features
```rust
// Symbol extraction (7 languages)
magellan::CodeGraph::index_file(path, source) -> Result<usize>

// Label queries
magellan::CodeGraph::get_symbols_by_label(&str) -> Result<Vec<SymbolQueryResult>>
magellan::CodeGraph::get_symbols_by_labels(&[&str]) -> Result<Vec<SymbolQueryResult>>
magellan::CodeGraph::get_all_labels() -> Result<Vec<String>>

// Code chunk retrieval (KEY FEATURE)
magellan::CodeGraph::get_code_chunk_by_span(path, start, end) -> Result<Option<CodeChunk>>
magellan::CodeGraph::get_code_chunks_for_symbol(path, name) -> Result<Vec<CodeChunk>>
```

---

## Integration Plan

### Phase 3.1: Dependency Update
**File**: `Cargo.toml`

**Change:**
```toml
[dependencies]
# Remove: sqlitegraph = { version = "0.2.10", features = ["sqlite-backend"] }
# Add:
magellan = "0.5.0"
sqlitegraph = { version = "0.2.11", default-features = false }
```

**Rationale:**
- Magellan re-exports sqlitegraph types
- Magellan's CodeGraph provides the same API + more
- Version bump to 0.2.11 for native-v2 backend support

---

### Phase 3.2: Graph Layer Update
**File**: `src/graph/mod.rs`

**Current implementation:**
```rust
use sqlitegraph::{SqliteGraph, GraphBackend, ...};
pub struct CodeGraph { backend: Rc<SqliteGraphBackend> }
```

**New implementation:**
```rust
use magellan::CodeGraph as MagellanGraph;

pub struct SpliceGraph {
    inner: MagellanGraph,
}

impl SpliceGraph {
    pub fn open(db_path: &Path) -> Result<Self> {
        Ok(Self {
            inner: MagellanGraph::open(db_path)?,
        })
    }

    // Delegate to Magellan's implementation
    pub fn index_file(&mut self, path: &Path) -> Result<usize> {
        self.inner.index_file(path)
    }

    // NEW: Label queries
    pub fn find_symbols_by_labels(&self, labels: &[&str]) -> Result<Vec<SymbolInfo>> {
        let results = self.inner.get_symbols_by_labels(labels)?;
        Ok(results.into_iter().map(|r| SymbolInfo {
            name: r.name,
            file_path: r.file_path,
            kind: r.kind,
            byte_start: r.byte_start,
            byte_end: r.byte_end,
        }).collect())
    }

    // NEW: Code chunk retrieval
    pub fn get_code(&self, path: &Path, start: usize, end: usize) -> Result<Option<String>> {
        Ok(self.inner.get_code_chunk_by_span(path, start, end)?
            .map(|chunk| chunk.content))
    }
}
```

---

### Phase 3.3: CLI Integration

**New command:**
```bash
# Query symbols by labels (e.g., find all Rust functions)
splice query --db splice.db --label rust --label fn

# Get code for refactoring (no file re-reading)
splice get --db splice.db --file src/main.rs --start 100 --end 200
```

---

### Phase 3.4: Ingest Module Completion

**Current state**: `src/ingest/mod.rs` returns "Not implemented yet"

**New implementation:**
```rust
pub fn ingest_file(graph: &mut SpliceGraph, path: &Path) -> Result<usize> {
    // Delegate to Magellan's parsers
    graph.index_file(path)
}
```

---

## Benefits

### 1. Completed Ingestion
- No more "Not implemented yet" errors
- 7 languages supported (same as Splice already has)

### 2. No Code Duplication
- Splice's 2000+ LOC of parsers can be removed
- Single source of truth for indexing

### 3. Code Chunk Access
- Refactoring operations don't need to re-read files
- Magellan stores source code with byte spans

### 4. Label-Based Discovery
- Find symbols by type/language without parsing
- `find_symbols_by_labels(&["rust", "fn"])` → all Rust functions

### 5. Performance
- Native-v2 backend support (2-3x faster inserts)
- Code chunks stored during indexing (no re-read)

---

## File Changes Summary

| File | Change | LOC Impact |
|------|--------|------------|
| `Cargo.toml` | Add magellan dependency | +2 |
| `src/graph/mod.rs` | Use Magellan's CodeGraph | ~100 (remove duplicated code) |
| `src/ingest/mod.rs` | Complete implementation using Magellan | ~200 |
| `src/ingest/rust.rs` | DELETE (use Magellan's parser) | -156 |
| `src/ingest/python.rs` | DELETE (use Magellan's parser) | -XXX |
| `src/ingest/c.rs` | DELETE (use Magellan's parser) | -XXX |
| `src/ingest/cpp.rs` | DELETE (use Magellan's parser) | -XXX |
| `src/ingest/javascript.rs` | DELETE (use Magellan's parser) | -XXX |
| `src/ingest/typescript.rs` | DELETE (use Magellan's parser) | -XXX |
| `src/ingest/java.rs` | DELETE (use Magellan's parser) | -XXX |
| `src/lib.rs` | Update exports | ~10 |
| `src/cli/mod.rs` | Add query/get commands | +50 |

**Net LOC change**: ~ -1000 lines of code removed

---

## Testing Strategy

### Unit Tests
```rust
#[test]
fn test_label_query_rust_functions() {
    let graph = SpliceGraph::open(":memory:")?;
    graph.index_file(Path::new("tests/fixtures/test.rs"))?;

    let results = graph.find_symbols_by_labels(&["rust", "fn"])?;
    assert!(!results.is_empty());
    assert_eq!(results[0].kind, "fn");
}

#[test]
fn test_code_chunk_retrieval() {
    let graph = SpliceGraph::open(":memory:")?;
    graph.index_file(Path::new("tests/fixtures/test.rs"))?;

    let code = graph.get_code(Path::new("tests/fixtures/test.rs"), 0, 100)?;
    assert!(code.is_some());
    assert!(code.unwrap().contains("fn"));
}
```

### Integration Tests
- End-to-end: Index → Query by labels → Get code → Patch
- Verify no file re-reading occurs (check file access count)

---

## Rollout Plan

1. ✅ Magellan v0.5.0 published to crates.io
2. ✅ This plan written
3. ⏳ Update Splice's Cargo.toml
4. ⏳ Implement SpliceGraph wrapper
5. ⏳ Complete ingest module
6. ⏳ Add CLI commands
7. ⏳ Remove redundant parsers
8. ⏳ Update tests
9. ⏳ Release Splice v0.5.0

---

## Open Questions

1. **Should Splice keep its own parsers for refactoring-specific needs?**
   - Magellan's parsers are for indexing
   - Splice needs AST for patch validation
   - **Decision**: Keep tree-sitter for validation, use Magellan for indexing

2. **Should we support both Magellan and direct sqlitegraph backends?**
   - Feature flag: `--features magellan-backend`
   - Default to Magellan
   - **Decision**: Start with Magellan-only, add feature flag if needed

3. **Database format compatibility?**
   - Magellan uses same sqlitegraph backend
   - Existing databases should work
   - **Decision**: Test with existing databases, verify compatibility

---

## Change Log

### 2026-01-02: Plan Created
- Magellan v0.5.0 published with labels and code chunks
- Integration plan documented
- Ready to begin implementation
