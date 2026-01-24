# Phase 23: Magellan Integration Extensions - Research

**Researched:** 2026-01-24
**Domain:** Magellan CodeGraph API integration, query command delegation
**Confidence:** HIGH

## Summary

Phase 23 extends the existing `MagellanIntegration` wrapper with query methods needed for the five Magellan-delegated CLI commands (status, query, find, refs, files). Direct analysis of Magellan 0.5.3 source code reveals that Magellan's `CodeGraph` provides all necessary primitives: entity counts, label queries, symbol lookups by name, call graph traversal, and file listing.

**Key findings:**
1. **No pagination APIs exist** - All query methods return `Vec<T>`, requiring Splice to implement client-side limiting
2. **Statistics are available** - `count_files()`, `count_symbols()`, `count_references()`, `count_chunks()` provide exact counts
3. **ID-based lookup is NOT provided** - Magellan has `entity_id` internally but no reverse lookup method; must use `symbol_id_by_name(path, name)` pattern
4. **Call graph is bidirectional** - `calls_from_symbol()` and `callers_of_symbol()` enable both forward and reverse traversal
5. **File listing is complete** - `all_file_nodes()` returns all indexed files with metadata

**Primary recommendation:** Add 6 focused methods to `MagellanIntegration`: `get_statistics()`, `query_symbols_by_file()`, `find_symbol_by_name()`, `find_symbol_by_id()`, `get_call_relationships()`, and `list_indexed_files()`. Implement client-side result limiting for large result sets.

## Standard Stack

### Core
| Library | Version | Purpose | Why Standard |
|---------|---------|---------|--------------|
| `magellan` | 0.5.3 | Code indexing and graph queries | Already in dependencies, provides all required APIs |
| `sqlitegraph` | 1.0 | Graph backend (re-exported by Magellan) | Required for direct database access if needed |

### Supporting
| Library | Version | Purpose | When to Use |
|---------|---------|---------|-------------|
| `splice::symbol_id` | (Phase 22) | 16-char hex symbol ID generation | For generating stable symbol IDs in query results |

### Alternatives Considered
| Instead of | Could Use | Tradeoff |
|------------|-----------|----------|
| Magellan CodeGraph methods | Direct SQL queries | CodeGraph API is cleaner; direct SQL breaks abstraction, fragile to schema changes |

**Installation:** No new dependencies required. Magellan 0.5.3 already provides all needed functionality.

## Architecture Patterns

### Recommended Extension Structure
```
src/graph/magellan_integration.rs
├── // EXISTING: MagellanIntegration wrapper
├── // ADD: Query command methods
│   ├── get_statistics()         -> DatabaseStats
│   ├── query_symbols_by_file()  -> Vec<SymbolWithRelations>
│   ├── find_symbol_by_name()    -> Vec<SymbolInfo>
│   ├── find_symbol_by_id()      -> Option<SymbolInfo>
│   ├── get_call_relationships() -> CallRelationships
│   └── list_indexed_files()     -> Vec<FileMetadata>
├── // ADD: Response types
│   ├── DatabaseStats
│   ├── SymbolWithRelations
│   ├── CallRelationships
│   └── FileMetadata
```

### Pattern 1: Database Statistics (QUERY-01: status command)

**What:** Aggregate all entity counts into a single statistics struct.

**When to use:** The `splice status --magellan` command displays database summary.

**Example:**
```rust
// Source: Magellan CodeGraph API (verified in src/graph/mod.rs:265-294)
//   - count_files() -> Result<usize>
//   - count_symbols() -> Result<usize>
//   - count_references() -> Result<usize>
//   - count_chunks() -> Result<usize>

use crate::graph::magellan_integration::MagellanIntegration;

/// Database statistics for Magellan graph
#[derive(Debug, Clone)]
pub struct DatabaseStats {
    pub files: usize,
    pub symbols: usize,
    pub references: usize,
    pub calls: usize,       // Derived: count entities with kind "Call"
    pub code_chunks: usize,
}

impl MagellanIntegration {
    /// Get comprehensive database statistics
    pub fn get_statistics(&self) -> Result<DatabaseStats> {
        let files = self.inner.count_files()?;
        let symbols = self.inner.count_symbols()?;
        let references = self.inner.count_references()?;
        let code_chunks = self.inner.count_chunks()?;

        // Count Call nodes explicitly (no count_calls() method exists)
        let calls = self.count_call_nodes()?;

        Ok(DatabaseStats {
            files,
            symbols,
            references,
            calls,
            code_chunks,
        })
    }

    // Helper: count Call entities via label query
    fn count_call_nodes(&self) -> Result<usize> {
        // Use count_entities_by_label if available, or iterate
        // Magellan's count module only has files/symbols/references
        // Workaround: use label counting with "Call" kind
        Ok(0) // Placeholder - implement via entity_id iteration or label
    }
}
```

**Key insight:** Magellan has `count_files()`, `count_symbols()`, `count_references()` but NOT `count_calls()`. Must implement Call counting via label queries or entity iteration.

### Pattern 2: File-Scoped Symbol Query (QUERY-02: query command)

**What:** Query symbols in a specific file, optionally by kind, with optional context.

**When to use:** The `splice query --file <path> [--kind <kind>] [--with-context]` command.

**Example:**
```rust
// Source: Magellan CodeGraph API (verified in src/graph/mod.rs:141-159)
//   - symbols_in_file(path) -> Result<Vec<SymbolFact>>
//   - symbols_in_file_with_kind(path, kind) -> Result<Vec<SymbolFact>>

use crate::graph::magellan_integration::MagellanIntegration;
use magellan::ingest::SymbolKind;

/// Symbol with optional relationship context
#[derive(Debug, Clone)]
pub struct SymbolWithRelations {
    pub symbol: SymbolInfo,
    pub callers: Vec<SymbolInfo>,    // If --with-callers
    pub callees: Vec<SymbolInfo>,    // If --with-callees
}

impl MagellanIntegration {
    /// Query symbols in a file, with optional filters and context
    pub fn query_symbols_by_file(
        &mut self,
        file_path: &Path,
        kind_filter: Option<SymbolKind>,
        with_callers: bool,
        with_callees: bool,
    ) -> Result<Vec<SymbolWithRelations>> {
        // Use Magellan's symbols_in_file_with_kind()
        let symbols = if let Some(kind) = kind_filter {
            self.inner.symbols_in_file_with_kind(file_path, Some(kind))?
        } else {
            self.inner.symbols_in_file(file_path)?
        };

        // Convert to SymbolWithRelations, optionally fetching relationships
        symbols.into_iter().map(|fact| {
            let symbol = SymbolInfo::from_fact(fact.clone(), file_path);
            let (callers, callees) = if with_callers || with_callees {
                self.fetch_call_relationships(file_path, &symbol.name, with_callers, with_callees)?
            } else {
                (Vec::new(), Vec::new())
            };

            Ok(SymbolWithRelations { symbol, callers, callees })
        }).collect()
    }
}
```

**Key insight:** `symbols_in_file_with_kind()` exists and handles kind filtering. Relationship fetching is done via separate call graph queries.

### Pattern 3: Symbol Lookup by Name (QUERY-03: find command)

**What:** Find symbols by name across all files, or by symbol_id.

**When to use:** The `splice find --name <name> [--ambiguous]` or `--symbol-id <id>` command.

**Example:**
```rust
// Source: Magellan CodeGraph API (verified in src/graph/query.rs:96-109)
//   - symbol_extents(path, name) -> Result<Vec<(i64, SymbolFact)>>
//
// NOTE: No global name search exists. Must iterate all files or build index.

use crate::graph::magellan_integration::MagellanIntegration;

impl MagellanIntegration {
    /// Find symbol by name across ALL indexed files
    pub fn find_symbol_by_name(
        &mut self,
        name: &str,
        ambiguous: bool,  // If true, return all matches; if false, return first
    ) -> Result<Vec<SymbolInfo>> {
        let mut results = Vec::new();

        // Get all indexed files
        let all_files = self.inner.all_file_nodes()?;

        for file_path in all_files.keys() {
            // Use symbol_extents for each file
            if let Ok(matches) = self.inner.symbol_extents(file_path, name) {
                for (_entity_id, fact) in matches {
                    let symbol_info = SymbolInfo::from_fact(fact, Path::new(file_path));
                    results.push(symbol_info);
                    if !ambiguous && !results.is_empty() {
                        return Ok(results);  // Early exit on first match
                    }
                }
            }
        }

        Ok(results)
    }

    /// Find symbol by 16-character hex symbol_id
    ///
    /// NOTE: Magellan does NOT provide entity_id -> symbol lookup.
    /// This requires scanning all entities or maintaining a reverse index.
    pub fn find_symbol_by_id(&mut self, symbol_id: &str) -> Result<Option<SymbolInfo>> {
        // Parse symbol_id to extract lookup criteria
        // Since symbol_id = SHA-256(name:path:byte_start), we can't reverse.
        // Must scan all entities and match on generated symbol_id.

        let entity_ids = self.inner().entity_ids()?;

        for entity_id in entity_ids {
            if let Ok(node) = self.inner().get_node(entity_id) {
                if node.kind == "Symbol" {
                    // Generate symbol_id from node data and compare
                    if let Some(symbol_info) = self.try_match_symbol_id(&node, symbol_id)? {
                        return Ok(Some(symbol_info));
                    }
                }
            }
        }

        Ok(None)
    }
}
```

**Key insight:** Magellan has NO global symbol search. Name-based search requires iterating all files. ID-based search requires iterating all entities and matching generated IDs.

### Pattern 4: Call Graph Traversal (QUERY-04: refs command)

**What:** Get callers or callees for a specific symbol.

**When to use:** The `splice refs --name <name> --path <path> --direction <in|out>` command.

**Example:**
```rust
// Source: Magellan CodeGraph API (verified in src/graph/calls.rs:62-94)
//   - calls_from_symbol(path, name) -> Result<Vec<CallFact>>
//   - callers_of_symbol(path, name) -> Result<Vec<CallFact>>

use crate::graph::magellan_integration::MagellanIntegration;

/// Call relationships for a symbol
#[derive(Debug, Clone)]
pub struct CallRelationships {
    pub symbol: SymbolInfo,
    pub callers: Vec<CallReference>,  // Symbols that call this symbol
    pub callees: Vec<CallReference>,  // Symbols this symbol calls
}

/// Reference to a call relationship
#[derive(Debug, Clone)]
pub struct CallReference {
    pub symbol: SymbolInfo,
    pub call_site: CallSite,
}

#[derive(Debug, Clone)]
pub struct CallSite {
    pub file_path: String,
    pub byte_start: usize,
    pub byte_end: usize,
    pub start_line: usize,
    pub start_col: usize,
    pub end_line: usize,
    pub end_col: usize,
}

impl MagellanIntegration {
    /// Get call relationships for a symbol
    pub fn get_call_relationships(
        &mut self,
        path: &Path,
        name: &str,
        direction: CallDirection,
    ) -> Result<CallRelationships> {
        // Get the target symbol info first
        let target = self.get_symbol_info(path, name)?;

        let (callers, callees) = match direction {
            CallDirection::In => {
                let calls = self.inner.callers_of_symbol(path, name)?;
                (self.resolve_calls_to_symbols(calls)?, Vec::new())
            }
            CallDirection::Out => {
                let calls = self.inner.calls_from_symbol(path, name)?;
                (Vec::new(), self.resolve_calls_to_symbols(calls)?)
            }
            CallDirection::Both => {
                let callers_facts = self.inner.callers_of_symbol(path, name)?;
                let callees_facts = self.inner.calls_from_symbol(path, name)?;
                (
                    self.resolve_calls_to_symbols(callers_facts)?,
                    self.resolve_calls_to_symbols(callees_facts)?
                )
            }
        };

        Ok(CallRelationships {
            symbol: target,
            callers,
            callees,
        })
    }
}

pub enum CallDirection {
    In,     // Callers only
    Out,    // Callees only
    Both,   // Both directions
}
```

**Key insight:** `callers_of_symbol()` and `calls_from_symbol()` return `CallFact` with file paths and locations, but NOT target symbol info. Must resolve `callee`/`caller` names to full `SymbolInfo` via additional lookups.

### Pattern 5: File Listing (QUERY-05: files command)

**What:** List all indexed files, optionally with symbol counts.

**When to use:** The `splice files [--symbols]` command.

**Example:**
```rust
// Source: Magellan CodeGraph API (verified in src/graph/mod.rs:326-332)
//   - all_file_nodes() -> Result<HashMap<String, FileNode>>
//
// Source: FileNode struct (verified in src/graph/schema.rs:8-16)
//   pub struct FileNode {
//       pub path: String,
//       pub hash: String,
//       pub last_indexed_at: i64,
//       pub last_modified: i64,
//   }

use crate::graph::magellan_integration::MagellanIntegration;

/// File metadata with optional symbol count
#[derive(Debug, Clone)]
pub struct FileMetadata {
    pub path: String,
    pub hash: String,
    pub last_indexed_at: i64,
    pub last_modified: i64,
    pub symbol_count: Option<usize>,  // If --symbols flag
}

impl MagellanIntegration {
    /// List all indexed files, with optional symbol counts
    pub fn list_indexed_files(&mut self, with_symbol_counts: bool) -> Result<Vec<FileMetadata>> {
        let file_nodes = self.inner.all_file_nodes()?;

        file_nodes.into_iter().map(|(path, node)| {
            let symbol_count = if with_symbol_counts {
                Some(self.count_symbols_in_file(&path)?)
            } else {
                None
            };

            Ok(FileMetadata {
                path,
                hash: node.hash,
                last_indexed_at: node.last_indexed_at,
                last_modified: node.last_modified,
                symbol_count,
            })
        }).collect()
    }

    // Helper: count symbols for a specific file
    fn count_symbols_in_file(&mut self, path: &str) -> Result<usize> {
        Ok(self.inner.symbols_in_file(path)?.len())
    }
}
```

**Key insight:** `all_file_nodes()` returns complete `FileNode` with timestamps. Symbol counting requires additional query per file.

### Anti-Patterns to Avoid

- **Direct SQL queries:** Don't bypass `CodeGraph` API to query database directly—schema changes will break integration
- **Unbounded iteration:** Don't iterate all entities without limits—implement client-side pagination for large codebases
- **Caching in wrapper:** Don't add caching to `MagellanIntegration`—let caller handle caching if needed
- **Blocking on large results:** Don't fetch all symbols for all files—implement lazy loading or streaming

## Don't Hand-Roll

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| Entity counting | Manual SQL COUNT queries | `CodeGraph::count_files/symbols/references/chunks()` | Tested, handles edge cases |
| File listing | Custom file tracking | `CodeGraph::all_file_nodes()` | Returns HashMap with metadata |
| Symbol queries by file | Manual graph traversal | `CodeGraph::symbols_in_file[_with_kind]()` | Handles DEFINES edge traversal |
| Call graph | Manual CALLS/CALLER edge queries | `CodeGraph::calls_from_symbol/callers_of_symbol()` | Handles bidirectional edges |
| Label queries | Custom label filtering | `CodeGraph::get_symbols_by_labels()` | Handles AND semantics |

**Key insight:** Magellan's `CodeGraph` already implements all graph traversal logic. Don't re-implement graph queries.

## Common Pitfalls

### Pitfall 1: O(N) Entity Scanning for Global Name Search

**What goes wrong:** Searching for a symbol name across all files requires O(N) file queries.

**Why it happens:** Magellan has NO global symbol name index. Only per-file `symbol_extents()` exists.

**How to avoid:**
1. Document performance characteristics in CLI help text
2. Implement client-side result limiting (e.g., `--limit 50`)
3. Consider caching symbol index for repeated queries

**Warning signs:** `splice find --name common` takes seconds on large codebases.

### Pitfall 2: Symbol ID Reverse Lookup is Impractical

**What goes wrong:** Looking up symbol by 16-char ID requires scanning ALL entities.

**Why it happens:** Magellan stores `entity_id` but doesn't provide reverse lookup. Symbol ID is derived from (name, path, byte_start), not stored.

**How to avoid:**
1. Prefer name-based lookup for interactive use
2. Document that `--symbol-id` is O(N) where N = total symbols
3. Consider building symbol_id -> entity_id index if performance critical

**Warning signs:** `splice find --symbol-id <id>` slower than name search.

### Pitfall 3: Call Fact Resolution Requires Additional Queries

**What goes wrong:** `CallFact` contains only caller/callee NAME strings, not full symbol info.

**Why it happens:** Call graph stores edges, not full symbol metadata. Must resolve names to `SymbolInfo`.

**How to avoid:**
1. Batch resolution: collect unique names, query once per file
2. Cache resolved symbols within a single command invocation
3. Return names only if caller doesn't need full metadata

**Warning signs:** N+1 query pattern when fetching call relationships.

### Pitfall 4: No Built-in Pagination

**What goes wrong:** Large result sets (e.g., all symbols in a large file) consume memory and slow CLI output.

**Why it happens:** All Magellan query methods return `Vec<T>` with no LIMIT/OFFSET support.

**How to avoid:**
1. Implement client-side limiting: truncate results after N items
2. Add `--limit` flag to query commands
3. Document that results are NOT ordered—pagination is approximate

**Warning signs:** `splice query --file large_file.rs` prints thousands of symbols.

### Pitfall 5: File Path String Handling

**What goes wrong:** Inconsistent path types (`&str`, `String`, `&Path`) cause conversion errors.

**Why it happens:** Magellan APIs use `&str` for paths, Splice uses `&Path`.

**How to avoid:**
1. Convert paths early: `path.to_str().ok_or(SpliceError::InvalidPath)?`
2. Create wrapper function for path conversion
3. Document which APIs require valid UTF-8 paths

**Warning signs:** `Invalid UTF-8 in path` errors.

## Code Examples

### Database Statistics

```rust
// Source: Verified Magellan 0.5.3 CodeGraph API

use magellan::CodeGraph;

impl MagellanIntegration {
    pub fn get_statistics(&self) -> Result<DatabaseStats> {
        let files = self.inner.count_files()?;
        let symbols = self.inner.count_symbols()?;
        let references = self.inner.count_references()?;
        let code_chunks = self.inner.count_chunks()?;

        // Count calls: iterate entities and filter by kind="Call"
        let entity_ids = self.inner().entity_ids()?;
        let calls = entity_ids.iter()
            .filter_map(|id| self.inner().get_node(*id).ok())
            .filter(|node| node.kind == "Call")
            .count();

        Ok(DatabaseStats { files, symbols, references, calls, code_chunks })
    }
}
```

### Symbol Query with Kind Filter

```rust
// Source: Verified Magellan 0.5.3 CodeGraph::symbols_in_file_with_kind()

use magellan::ingest::SymbolKind;

impl MagellanIntegration {
    pub fn query_symbols_by_file(
        &mut self,
        file_path: &Path,
        kind_filter: Option<SymbolKind>,
    ) -> Result<Vec<SymbolInfo>> {
        let path_str = file_path.to_str()
            .ok_or_else(|| SpliceError::Other("Invalid UTF-8 path".into()))?;

        let symbols = self.inner.symbols_in_file_with_kind(
            path_str,
            kind_filter
        )?;

        symbols.into_iter()
            .map(|fact| Ok(SymbolInfo::from_fact(fact, file_path)))
            .collect()
    }
}
```

### Call Relationships

```rust
// Source: Verified Magellan 0.5.3 CodeGraph::calls_from_symbol/callers_of_symbol()

impl MagellanIntegration {
    pub fn get_callers(&mut self, path: &Path, name: &str) -> Result<Vec<SymbolInfo>> {
        let path_str = file_path_to_str(path)?;

        let call_facts = self.inner.callers_of_symbol(path_str, name)?;

        // Resolve caller names to SymbolInfo
        call_facts.into_iter()
            .map(|fact| {
                // Find symbol by fact.caller name in fact.file_path
                self.find_symbol_in_file(&PathBuf::from(&fact.file_path), &fact.caller)
            })
            .collect()
    }
}
```

## State of the Art

| Old Approach | Current Approach | When Changed | Impact |
|--------------|------------------|--------------|--------|
| Manual SQL queries | CodeGraph API methods | Magellan 0.5.0+ | Cleaner integration, schema-agnostic |
| Per-file wrappers | Unified MagellanIntegration | Phase 17+ | Single entry point for all Magellan operations |
| No symbol IDs | 16-char hex IDs (Phase 22) | Phase 22 | Enables cross-tool symbol correlation |

**Deprecated/outdated:**
- Direct database access: Use `CodeGraph` methods instead
- Manual graph traversal: Use `symbols_in_file()`, `calls_from_symbol()`, etc.

## Open Questions

1. **Call counting efficiency**
   - What we know: No `count_calls()` method exists in Magellan
   - What's unclear: Whether iterating all entities to count "Call" nodes is acceptable performance
   - Recommendation: Implement as entity iteration, profile on large codebases, optimize if needed (e.g., maintain cached count)

2. **Symbol ID reverse lookup feasibility**
   - What we know: Magellan doesn't store symbol_id, requires scanning all entities
   - What's unclear: Whether O(N) scan is acceptable for `--symbol-id` lookup
   - Recommendation: Implement scan for Phase 23, add symbol_id index in later phase if performance inadequate

3. **Global symbol name indexing**
   - What we know: No global name search API exists
   - What's unclear: Whether to build in-memory name->entity_id index
   - Recommendation: Skip for Phase 23 (use file iteration), consider for optimization phase

## Sources

### Primary (HIGH confidence)

**Magellan 0.5.3 source code (verified directly at ~/.cargo/registry/src/.../magellan-0.5.3/):**
- `src/graph/mod.rs` — CodeGraph struct definition (lines 38-95), public API methods (lines 98-395)
- `src/graph/count.rs` — Count operations: count_files, count_symbols, count_references
- `src/graph/query.rs` — Symbol queries: symbols_in_file, symbols_in_file_with_kind, symbol_extents, symbol_id_by_name
- `src/graph/calls.rs` — Call graph: calls_from_symbol, callers_of_symbol
- `src/graph/files.rs` — File operations: all_file_nodes, get_file_node
- `src/graph/call_ops.rs` — Call node operations: calls_from_symbol, callers_of_symbol implementation
- `src/references.rs` — ReferenceFact, CallFact struct definitions
- `src/graph/schema.rs` — FileNode, SymbolNode, ReferenceNode, CallNode schemas
- `src/generation/mod.rs` — ChunkStore API: count_chunks, get_chunks_for_file

**Existing Splice codebase:**
- `/home/feanor/Projects/splice/src/graph/magellan_integration.rs` — Current MagellanIntegration wrapper
- `/home/feanor/Projects/splice/src/symbol_id.rs` — Phase 22: 16-char hex ID generation
- `/home/feanor/Projects/splice/tests/magellan_integration_tests.rs` — Integration test patterns

### Secondary (MEDIUM confidence)

**Project documentation:**
- `/home/feanor/Projects/splice/.planning/phases/22-symbol-id-and-format-foundation/22-RESEARCH.md` — Phase 22 research context
- `/home/feanor/Projects/splice/.planning/REQUIREMENTS.md` — QUERY-01 through QUERY-05 requirements

### Tertiary (LOW confidence)

None—this research is based entirely on verified source code analysis.

## Metadata

**Confidence breakdown:**
- Standard stack: HIGH — All APIs verified in Magellan 0.5.3 source code
- Architecture: HIGH — Method signatures verified directly, patterns are straightforward wrappers
- Pitfalls: HIGH — All pitfalls identified through code analysis (missing APIs, O(N) operations)
- Call counting: MEDIUM — Implementation requires entity iteration, performance unknown

**Research date:** 2026-01-24
**Valid until:** 60 days (stable API—Magellan 0.5.3 is released, breaking changes unlikely)
