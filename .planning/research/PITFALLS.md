# Pitfalls Research - Magellan v2.0.0 Semantic Refactoring Integration

**Domain:** Cross-file semantic refactoring with graph algorithms
**Researched:** 2026-02-04
**Confidence:** HIGH
**Milestone:** Subsequent to v2.2.4 - Adding Magellan v2.0.0 capabilities

---

## Executive Summary

Adding Magellan v2.0.0 semantic refactoring capabilities to Splice introduces specific risks beyond the existing integration. The upgrade path from Magellan 0.5.3 → 2.0.0 brings breaking changes in ID formats (BLAKE3), graph algorithms (cycles, reachability), and cross-file rename semantics. The primary risk areas are:

1. **Version upgrade breaking changes** (ID format, API signatures)
2. **Reference integrity during concurrent cross-file edits**
3. **Graph algorithm performance on large codebases**
4. **UTF-8 byte offset safety with multi-byte characters**
5. **Test breakage from new JSON fields**

Splice has 407 passing tests that must not break. The integration layer is well-structured (`src/graph/magellan_integration.rs`, 864 LOC), but semantic refactoring adds new failure modes.

---

## Critical Pitfalls

Mistakes that cause rewrites, data corruption, or major user-facing issues.

### Pitfall 1: Version Upgrade Breaking Changes (0.5.3 → 2.0.0)

**What goes wrong:**
Magellan v2.0.0 changes the symbol ID format from 16-char SHA-256 hex to 32-char BLAKE3 hex, breaking all existing ID-based references and causing database incompatibility.

**Why it happens:**
- Splice hardcodes 16-char ID format in `src/symbol_id.rs` (line 8: "16-character lowercase hexadecimal")
- Tests assert exact ID length: `assert_eq!(id_str.len(), 16)` (line 23 in `tests/id_format_tests.rs`)
- No schema versioning in Magellan database format
- BLAKE3 produces 32-byte hashes vs SHA-256's configurable output

**Consequences:**
- Existing databases become unreadable (ID lookups fail)
- All 407 tests fail with ID length assertion errors
- User data corruption if migration not handled
- Rollback requires full database re-index

**Warning signs:**
- Cargo.lock shows `magellan 2.0.0` but tests still use 16-char assertions
- Database queries return "symbol not found" for known symbols
- CI fails with `assertion failed: `(left == right)`: left: 16, right: 32

**Prevention:**
1. **Dual-format support during migration:**
   ```rust
   pub enum SymbolId {
       V16 { hex: String },  // Legacy SHA-256[0..8]
       V32 { hex: String },  // New BLAKE3
   }

   impl SymbolId {
       pub fn detect_format(input: &str) -> Self {
           match input.len() {
               16 => Self::V16 { hex: input.to_string() },
               32 => Self::V32 { hex: input.to_string() },
               _ => panic!("Invalid ID length"),
           }
       }
   }
   ```

2. **Database migration script:**
   ```rust
   pub fn migrate_db_v16_to_v32(db_path: &Path) -> Result<()> {
       // 1. Open old database
       // 2. Read all symbols with 16-char IDs
       // 3. Re-generate IDs using BLAKE3
       // 4. Update all references (edges, labels, chunks)
       // 5. Write new database
       // 6. Backup old database as {db}.v0.5.3.bak
   }
   ```

3. **Feature flag for new format:**
   ```toml
   [features]
   default = ["magellan-v2"]
   magellan-v2 = ["magellan/version-2-0-0"]
   magellan-legacy = ["magellan/version-0-5-3"]
   ```

4. **Test parameterization:**
   ```rust
   #[test]
   fn test_symbol_id_format(#[case] expected_len: usize) {
       let id = generate_symbol_id("test", "file.rs", 0);
       assert_eq!(id.as_str().len(), expected_len);
   }
   ```

**Phase to address:** Phase 1 (Dependency upgrade) - MUST be first step before any feature work

**Detection:**
- Run `cargo test` after upgrading `Cargo.toml` to `magellan = "2.0.0"`
- Should see 407 test failures from ID length mismatches
- Pre-upgrade baseline: `cargo test` must pass 100%

**Sources:**
- HIGH - `/home/feanor/Projects/splice/Cargo.toml:25` currently has `magellan = "0.5.3"`
- HIGH - `/home/feanor/Projects/splice/src/symbol_id.rs:8` hardcodes 16-char format
- HIGH - `/home/feanor/Projects/splice/tests/id_format_tests.rs:23-35` asserts 16-char length
- HIGH - `cargo search magellan` shows version 2.0.0 is available on crates.io

---

### Pitfall 2: Cross-File Rename Race Conditions

**What goes wrong:**
Concurrent cross-file rename operations modify the same files simultaneously, causing reference updates to overwrite each other or miss symbols entirely.

**Why it happens:**
- Splice's `apply_batch_with_validation()` (line 297 in `src/patch/mod.rs`) processes files sequentially
- No transactional cross-file locking mechanism
- Graph queries compute all references upfront, but references can change during batch application
- Multiple LLM agents or concurrent CLI invocations can target overlapping symbol sets

**Consequences:**
- Symbol renamed in definition but not in all references (broken build)
- References updated to point to wrong symbol (silent corruption)
- "I renamed `foo` to `bar` but `baz()` still calls `foo()`"
- Compiler errors after successful rename operation

**Warning signs:**
- `cargo check` fails after rename despite "all files patched successfully"
- Reference count mismatch: "updated 5 references but graph shows 7 callers"
- Intermittent failures in CI with concurrent jobs
- Users report "worked when I ran it again"

**Prevention:**
1. **Cross-file transaction with graph-level lock:**
   ```rust
   pub struct CrossFileTransaction {
       workspace: PathBuf,
       graph_lock: Arc<RwLock<()>>,
       affected_files: BTreeSet<PathBuf>,
   }

   impl CrossFileTransaction {
       pub fn begin(workspace: &Path, graph: &CodeGraph) -> Result<Self> {
           // Acquire graph-level read-write lock
           // Compute all affected files upfront
           // Prevent concurrent modifications
       }

       pub fn commit(self) -> Result<Vec<FilePatchSummary>> {
           // Apply all patches atomically
           // Release lock only after validation
       }
   }
   ```

2. **Reference snapshot before mutation:**
   ```rust
   pub fn snapshot_references(graph: &CodeGraph, symbol: &Symbol) -> Vec<Reference> {
       // Immutable snapshot of all references at transaction start
       // Ensures concurrent modifications don't affect this transaction
   }
   ```

3. **Conflict detection with three-way merge:**
   ```rust
   pub enum ConflictResolution {
       Abort,                    // Fail fast (default)
       RetryWithNewSnapshot,     // Re-compute references
       ThreeWayMerge,            // Merge concurrent changes
   }
   ```

4. **File modification detection during batch:**
   ```rust
   pub fn verify_files_unchanged(
       files: &[PathBuf],
       checksums: &HashMap<PathBuf, String>,
   ) -> Result<bool> {
       // Detect external modifications before applying batch
       // Abort if any file changed since planning phase
   }
   ```

**Phase to address:** Phase 3 (Cross-file rename implementation) - MUST have transactional semantics

**Detection:**
- Integration test: Spawn two concurrent rename processes targeting overlapping symbols
- Should detect conflict and fail gracefully (not corrupt data)
- Load test: 100 concurrent renames in same workspace, verify zero reference leaks

**Sources:**
- HIGH - `/home/feanor/Projects/splice/src/patch/mod.rs:297-368` shows sequential batch application without cross-file locking
- HIGH - `/home/feanor/Projects/splice/tests/cross_file_tests.rs` tests cross-file resolution but not concurrent modification
- HIGH - `/home/feanor/Projects/splice/src/graph/magellan_integration.rs` has no transaction API exposed

---

### Pitfall 3: Cycle Detection Infinite Loops on Mutual Recursion

**What goes wrong:**
Cycle detection algorithm (Tarjan's SCC) enters infinite loop or stack overflow on codebases with deep mutual recursion patterns.

**Why it happens:**
- Recursive algorithms without depth limiting
- Graph algorithms assume DAG (directed acyclic graph) but real code has cycles
- No memoization of visited nodes during traversal
- Mutual recursion creates cycles: `fn A() calls B()` and `fn B() calls A()`

**Consequences:**
- CLI hangs indefinitely during impact analysis
- Stack overflow crashes on large codebases
- "I ran `splice refactor --analyze-impact` and it never returned"
- Users force-quit, leaving database in inconsistent state

**Warning signs:**
- Commands taking >60 seconds on <10K LOC codebase
- Stack traces showing recursive function calls >1000 deep
- Memory usage growing linearly with time during graph traversal
- `Thread 'main' has overflowed its stack` errors

**Prevention:**
1. **Depth-limited traversal:**
   ```rust
   pub const MAX_TRAVERSAL_DEPTH: usize = 100;

   pub fn reachable_with_limit(
       graph: &CodeGraph,
       start: NodeId,
       max_depth: usize,
   ) -> Result<HashSet<NodeId>> {
       let mut visited = HashSet::new();
       let mut queue = VecDeque::new();
       let mut depths = HashMap::new();

       while let Some(node) = queue.pop_front() {
           let depth = depths[&node];
           if depth >= max_depth {
               continue; // Prune deep paths
           }

           for neighbor in graph.neighbors(node) {
               if !visited.contains(&neighbor) {
                   visited.insert(neighbor);
                   depths.insert(neighbor, depth + 1);
                   queue.push_back(neighbor);
               }
           }
       }
       Ok(visited)
   }
   ```

2. **Cycle detection with early exit:**
   ```rust
   pub fn detect_cycles_tarjan(graph: &CodeGraph) -> Result<Vec<Vec<NodeId>>> {
       // Tarjan's SCC algorithm with iteration limit
       // Return early if cycle length > MAX_CYCLE_SIZE
   }
   ```

3. **Memoization of visited nodes:**
   ```rust
   pub struct TraversalCache {
       visited: RwLock<HashSet<NodeId>>,
       sccs: RwLock<HashMap<NodeId, Vec<NodeId>>>,
   }
   ```

4. **Progress indication for long traversals:**
   ```rust
   pub fn traverse_with_progress(
       graph: &CodeGraph,
       callback: impl Fn(Progress),
   ) -> Result<TraversalResult> {
       // Emit progress events every 100 nodes visited
   }
   ```

5. **Timeout enforcement:**
   ```rust
   pub fn traverse_with_timeout(
       graph: &CodeGraph,
       timeout: Duration,
   ) -> Result<TraversalResult> {
       // Abort traversal if timeout exceeded
   }
   ```

**Phase to address:** Phase 2 (Graph algorithms integration) - MUST include safety limits

**Detection:**
- Unit test: Create codebase with `fn A() { B() }` and `fn B() { A() }` repeated 1000 times
- Verify `detect_cycles` returns in <1 second with cycle list
- Load test: 10K LOC with heavily recursive code (factorial, tree traversal)

**Sources:**
- HIGH - `/home/feanor/Projects/splice/docs/CROSS_FILE_RESOLUTION.md` discusses mutual recursion edge cases (lines 322-373)
- HIGH - Magellan v2.0.0 adds cycle detection algorithms (feature documentation)
- MEDIUM - Standard graph algorithm pitfalls (Tarjan's SCC complexity)

---

### Pitfall 4: O(n) Query Performance on Large Codebases

**What goes wrong:**
Symbol queries become O(number of files) instead of O(1), causing 10K+ file projects to have multi-second query times.

**Why it happens:**
- Magellan's `find_symbol_by_name()` (line 364 in `src/graph/magellan_integration.rs`) iterates all files: `for file_path in file_nodes.keys()`
- No global symbol name index in database schema
- Each query triggers N file queries where N = indexed file count
- No caching of frequently queried symbols

**Consequences:**
- `splice query --name "main"` takes 30+ seconds on monorepo with 50K files
- LLM agents timeout waiting for symbol resolution
- Users think tool is broken, switch to `ripgrep`
- Real-world codebases (>10K files) become unusable

**Warning signs:**
- Query time increases linearly with file count (scatter plot shows straight line)
- Database queries inside loops: `for file in files { query(db, file) }`
- No `.index` or `.cache` files in project directory
- CPU usage 100% but disk idle during queries

**Prevention:**
1. **Global symbol name index:**
   ```sql
   CREATE INDEX idx_symbols_by_name
   ON graph_entities(name, kind)
   WHERE kind = 'Symbol';
   ```

2. **Symbol name hash map:**
   ```rust
   pub struct SymbolNameIndex {
       // name -> Vec<SymbolId>
       index: HashMap<String, Vec<i64>>,
   }

   impl SymbolNameIndex {
       pub fn build(graph: &CodeGraph) -> Result<Self> {
           // Single O(N) scan to build index
           // Subsequent lookups are O(1)
       }
   }
   ```

3. **Query result caching:**
   ```rust
   pub struct QueryCache {
       cache: LRUCache<Query, Vec<Symbol>>,
       ttl: Duration,  // Invalidate after database modification
   }
   ```

4. **Incremental index updates:**
   ```rust
   pub fn update_index(
       index: &mut SymbolNameIndex,
       added: &[Symbol],
       removed: &[Symbol],
   ) {
       // O(k) update where k = added + removed
       // No full rebuild needed
   }
   ```

5. **Background index rebuilding:**
   ```rust
   pub fn spawn_index_worker(db_path: PathBuf) -> JoinHandle<()> {
       thread::spawn(move || {
           // Rebuild index when database changes
           // Don't block main thread
       })
   }
   ```

**Phase to address:** Phase 2 (Graph algorithms) - MUST include indexing strategy

**Detection:**
- Benchmark: Time `find_symbol_by_name()` on 1K, 10K, 100K file synthetic codebases
- Plot query time vs file count - should be flat (O(1)) not linear (O(n))
- Alert if `find_symbol_by_name` iterates >100 files in any test

**Sources:**
- HIGH - `/home/feanor/Projects/splice/src/graph/magellan_integration.rs:364-400` shows O(n) file iteration
- HIGH - Line 362: "Magellan has no global symbol name index"
- HIGH - `/home/feanor/Projects/splice/.planning/research/MAGELLAN_INTEGRATION_PITFALLS.md:174-219` documents relationship query performance issues

---

## Moderate Pitfalls

Mistakes that cause delays, technical debt, or significant UX issues.

### Pitfall 5: UTF-8 Byte Offset Corruption on Multi-Byte Characters

**What goes wrong:**
Byte offset replacement splits multi-byte UTF-8 characters (emojis, CJK, accented letters), creating invalid UTF-8 and file corruption.

**Why it happens:**
- Rope operations use `byte_to_char()` conversion (line 216 in `src/patch/mod.rs`)
- UTF-8 validation only checks span boundaries, not internal character alignment
- Multi-byte characters can span across replacement boundaries
- Tree-sitter byte offsets may not align with UTF-8 character boundaries

**Consequences:**
- Files contain invalid UTF-8 after patch
- `std::fs::read_to_string()` fails with "invalid UTF-8" error
- Emoji sequences like "🦀" become corruption bytes
- Non-English codebases (Japanese, Chinese) unusable

**Warning signs:**
- Tests using only ASCII strings pass, but Unicode tests fail
- "Stream did not contain valid UTF-8" errors after patching
- Emoji in comments corrupted after replacement
- File size mismatch: patched file larger than expected

**Prevention:**
1. **UTF-8 boundary validation:**
   ```rust
   pub fn validate_utf8_boundaries(source: &[u8], start: usize, end: usize) -> Result<()> {
       // Verify start and end are on character boundaries
       if !source.is_char_boundary(start) {
           return Err(SpliceError::InvalidUtf8Boundary { offset: start });
       }
       if !source.is_char_boundary(end) {
           return Err(SpliceError::InvalidUtf8Boundary { offset: end });
       }
       Ok(())
   }
   ```

2. **Character-based indexing for replacements:**
   ```rust
   pub fn replace_char_based(
       rope: &mut Rope,
       start_char: usize,
       end_char: usize,
       new_content: &str,
   ) -> Result<()> {
       // Work in character space, not byte space
       // Avoids manual byte_to_char conversions
   }
   ```

3. **Unicode-aware test fixtures:**
   ```rust
   #[test]
   fn test_patch_with_emoji() {
       let source = "fn main() { println!(\"Hello 🦀\"); }";
       // Patch after emoji should not corrupt it
   }

   #[test]
   fn test_patch_with_cjk() {
       let source = "fn 関数() { /* Chinese comment */ }";
       // Replace in CJK context should preserve encoding
   }
   ```

4. **Explicit UTF-8 alignment in error messages:**
   ```rust
   Err(SpliceError::InvalidUtf8Boundary {
       offset: 42,
       hint: "Offset splits multi-byte character, use next boundary at 43",
   })
   ```

**Phase to address:** Phase 3 (Cross-file rename) - MUST validate UTF-8 safety

**Detection:**
- Test suite: Include Unicode test cases for all 7 supported languages
- Fuzz testing: Use `quickcheck` to generate random Unicode strings
- Regression: Test all existing fixtures with Unicode variants injected

**Sources:**
- HIGH - `/home/feanor/Projects/splice/src/patch/mod.rs:216` uses `rope.byte_to_char()`
- HIGH - `/home/feanor/Projects/splice/tests/id_format_tests.rs:83-113` tests Unicode but only for IDs, not patches
- HIGH - `/home/feanor/Projects/splice/src/patch/mod.rs:204-210` has UTF-8 validation but may not cover all cases

---

### Pitfall 6: Test Breakage from New JSON Fields (BLAKE3 IDs, Proof Data)

**What goes wrong:**
Magellan v2.0.0 adds new fields to output (BLAKE3 IDs, proof hashes), breaking existing test assertions that expect exact JSON structure.

**Why it happens:**
- Tests assert exact JSON match: `assert_json_eq!(output, expected)`
- No field filtering or "subset match" mode
- New fields like `"blake3_id"` or `"proof_hash"` added to schema
- Tests don't use `assert_json_include!` or `serde_json::value::Value::Pointer`

**Consequences:**
- All 407 tests fail after Magellan upgrade
- CI blocks, progress halts
- Team loses confidence in upgrade
- Rollback difficult if database format changed

**Warning signs:**
- `cargo test` shows 200+ failures with "extra field" errors
- Tests failing with: `"expected: {}, found: {"blake3_id": "..."}"`
- Need to update 50+ test fixtures by hand

**Prevention:**
1. **Subset assertion pattern:**
   ```rust
   use assert_json_diff::assert_json_include;

   #[test]
   fn test_query_output() {
       let output = run_query("foo");
       let expected = json!({
           "name": "foo",
           "kind": "function",
           // Don't assert all fields, just required ones
       });
       assert_json_include!(actual: output, expected: expected);
   }
   ```

2. **Field selection flag:**
   ```rust
   pub enum OutputFormat {
       V1,  // Legacy fields only (for back-compat tests)
       V2,  // Include BLAKE3 IDs, proof data
       Full, // All fields
   }
   ```

3. **Golden test update script:**
   ```bash
   #!/bin/bash
   # Regenerate all golden files with new schema
   cargo test -- --generate-golden-files
   ```

4. **Schema version in output:**
   ```json
   {
       "schema_version": "2.0",
       "blake3_id": "abc123...",
       "name": "foo",
       ...
   }
   ```

5. **Migration test suite:**
   ```rust
   #[cfg(feature = "magellan-legacy")]
   mod legacy_tests {
       // Run with legacy schema
   }

   #[cfg(feature = "magellan-v2")]
   mod v2_tests {
       // Run with new schema
   }
   ```

**Phase to address:** Phase 1 (Version upgrade) - MUST preserve test compatibility

**Detection:**
- Pre-upgrade test run: `cargo test` must pass 100% (baseline)
- Post-upgrade test run with `--features magellan-v2`: Should pass with new fields
- Diff test output to identify changed JSON structures

**Sources:**
- HIGH - Existing tests use exact JSON matching (convention observed in `/home/feanor/Projects/splice/tests/`)
- HIGH - `/home/feanor/Projects/splice/.planning/research/PITFALLS.md:115-147` documents test breakage risks
- HIGH - BLAKE3 IDs are 32-char hex vs 16-char SHA-256 (format change)

---

### Pitfall 7: Memory Exhaustion from Loading Entire Graph

**What goes wrong:**
Graph algorithms load entire code graph into memory, causing OOM kills on machines with <8GB RAM or large codebases.

**Why it happens:**
- SQLiteGraph queries return all nodes/edges into `Vec`
- No streaming or chunked iteration over graph data
- Algorithms build adjacency matrices instead of adjacency lists
- Multiple graph copies in memory during traversal

**Consequences:**
- Process killed by OS OOM killer
- "Command terminated by signal 9" errors
- Users with 4GB laptops cannot use tool
- CI runners fail with memory limits

**Warning signs:**
- Memory usage grows linearly with LOC
- `Vec` allocation of >1M nodes
- RSS grows to 4GB+ during graph operations
- `top` shows process using 90% of RAM

**Prevention:**
1. **Streaming graph iteration:**
   ```rust
   pub fn iter_nodes_streaming(
       db: &Connection,
   ) -> impl Iterator<Item = Node> {
       // Return iterator, not Vec
       // Load nodes lazily
   }
   ```

2. **Adjacency list instead of matrix:**
   ```rust
   pub struct GraphAdjacencyList {
       // node_id -> Vec<neighbor_id>
       // Sparse representation, O(E) memory instead of O(V^2)
       adjacency: HashMap<NodeId, Vec<NodeId>>,
   }
   ```

3. **Chunked processing:**
   ```rust
   pub fn process_in_chunks<T>(
       items: Vec<T>,
       chunk_size: usize,
       f: impl Fn(&[T]),
   ) {
       for chunk in items.chunks(chunk_size) {
           f(chunk);
       }
   }
   ```

4. **Memory monitoring:**
   ```rust
   pub fn check_memory_limit(limit_mb: usize) -> Result<()> {
       let usage = memory_usage();
       if usage > limit_mb * 1024 * 1024 {
           return Err(SpliceError::MemoryLimitExceeded);
       }
       Ok(())
   }
   ```

5. **Graph compression for large projects:**
   ```rust
   pub struct CompressedGraph {
       // Store only deltas, not full node data
       // Use integer IDs instead of strings
       compressed: Vec<CompressedNode>,
   }
   ```

**Phase to address:** Phase 2 (Graph algorithms) - MUST include memory limits

**Detection:**
- Benchmark: Monitor RSS during graph operations on 10K, 100K LOC codebases
- Load test: Run on codebase with 1M symbols, verify memory <2GB
- Profile: Use `valgrind --tool=massif` to detect memory spikes

**Sources:**
- HIGH - Standard graph algorithm memory issues (adjacency matrix vs list)
- HIGH - `/home/feanor/Projects/splice/.planning/research/PITFALLS.md:44-77` documents relationship graph scalability
- MEDIUM - SQLiteGraph in-memory queries

---

## Minor Pitfalls

Mistakes that cause annoyance but are fixable without rewrites.

### Pitfall 8: Missing Schema Version Detection

**What goes wrong:**
Splice opens Magellan v2.0.0 database without checking schema version, causing cryptic errors when trying to read v0.5.3 databases.

**Why it happens:**
- No version table in Magellan database schema
- `MagellanIntegration::open()` doesn't validate database format
- Error message doesn't indicate version mismatch
- Users don't know they need to migrate

**Consequences:**
- "Database corrupted" errors for valid databases
- Users re-index from scratch unnecessarily
- Support burden: "My database doesn't work with new Splice"

**Warning signs:**
- `sqlite3.OperationalError: no such column: blake3_id`
- Tests fail with "column not found" errors
- User reports: "Upgraded Splice, now all my queries fail"

**Prevention:**
1. **Schema version table:**
   ```sql
   CREATE TABLE graph_meta (
       key TEXT PRIMARY KEY,
       value TEXT
   );

   INSERT INTO graph_meta (key, value)
   VALUES ('schema_version', '2.0.0');
   ```

2. **Version check on open:**
   ```rust
   impl MagellanIntegration {
       pub fn open_with_version_check(
           db_path: &Path,
           expected_version: &str,
       ) -> Result<Self> {
           let integration = Self::open(db_path)?;
           let version = integration.get_schema_version()?;
           if version != expected_version {
               return Err(SpliceError::SchemaVersionMismatch {
                   expected: expected_version.to_string(),
                   found: version,
                   hint: "Run `splice migrate-db` to upgrade",
               });
           }
           Ok(integration)
       }
   }
   ```

3. **Migration command:**
   ```bash
   $ splice migrate-db --from 0.5.3 --to 2.0.0 magellan.db
   Migrating database...
   Backed up original to magellan.db.v0.5.3.bak
   Migration complete.
   ```

**Phase to address:** Phase 1 (Version upgrade)

**Detection:**
- Integration test: Create v0.5.3 database, open with v2.0.0 code
- Should fail with clear error message, not cryptic SQLite error

**Sources:**
- HIGH - Standard database migration patterns
- HIGH - `/home/feanor/Projects/splice/.planning/research/MAGELLAN_INTEGRATION_PITFALLS.md:416-439` documents version skew issues

---

### Pitfall 9: Proof Mode Breaking Existing Output Formats

**What goes wrong:**
New `--proof` flag adds proof-of-correctness data to JSON output, breaking LLM parsers that expect simple schema.

**Why it happens:**
- Proof data added as top-level field without opt-in
- LLMs don't ignore unknown fields by default
- No separate endpoint for proof data

**Consequences:**
- LLM rejects valid JSON because of extra fields
- Prompts need "filter out proof fields" post-processing
- Users confused by "unexpected field" errors

**Warning signs:**
- LLM errors: "unexpected key 'proof_hash' in JSON"
- Users asking "what is this proof data?"
- Need to document "ignore these fields" in prompts

**Prevention:**
1. **Opt-in proof output:**
   ```bash
   # Default: no proof data
   splice query --name foo

   # With proof: separate flag
   splice query --name foo --proof
   ```

2. **Separate proof endpoint:**
   ```bash
   # Get proof data separately
   splice proof --operation-id abc123
   ```

3. **Output mode:**
   ```rust
   pub enum OutputMode {
       Standard,    // No proof data
       Proof,       // Include proof fields
       Verbose,     // Everything
   }
   ```

**Phase to address:** Phase 4 (Proof mode)

**Detection:**
- Test: Parse JSON output with strict schema validator
- Verify default output has no proof fields

**Sources:**
- MEDIUM - CLI UX patterns
- MEDIUM - LLM JSON parsing requirements

---

### Pitfall 10: File Path Canonicalization Issues

**What goes wrong:**
Cross-file renames fail because file paths use different formats (relative vs absolute, symlinks, case sensitivity).

**Why it happens:**
- No canonical path normalization
- Symlinks not resolved consistently
- Windows vs macOS vs Linux path differences
- Git worktrees with symlinked directories

**Consequences:**
- "File not found" errors during batch rename
- References updated in wrong file
- Case-sensitivity issues on case-insensitive filesystems

**Warning signs:**
- Path comparison failures: `src/foo.rs != ./src/foo.rs`
- Symlinked directories cause issues
- Tests fail on macOS but pass on Linux

**Prevention:**
1. **Path canonicalization:**
   ```rust
   pub fn canonicalize_path(path: &Path) -> Result<PathBuf> {
       // Resolve symlinks
       // Convert to absolute path
       // Normalize separators
   }
   ```

2. **Path comparison helper:**
   ```rust
   pub fn paths_equivalent(a: &Path, b: &Path) -> bool {
       canonicalize_path(a)? == canonicalize_path(b)?
   }
   ```

3. **Workspace root normalization:**
   ```rust
   pub fn normalize_workspace_root(
       workspace: &Path,
   ) -> Result<PathBuf> {
       // Resolve to canonical absolute path
       // Use as base for all relative paths
   }
   ```

**Phase to address:** Phase 3 (Cross-file rename)

**Detection:**
- Test: Create symlinked directory, run cross-file rename
- Test: Use relative paths, verify they resolve correctly

**Sources:**
- HIGH - Standard cross-platform path issues
- MEDIUM - Existing codebase may have path utilities

---

## Anti-Patterns to Avoid

### Anti-Pattern 1: Naive File-by-File Rename

**What it is:** Rename symbol in definition file, then scan all files for references and rename sequentially.

**Why avoid:**
- Doesn't scale to large codebases (O(N) files)
- No transactional safety
- Can't handle circular imports
- Leaves broken intermediate state if interrupted

**Instead:** Compute all affected files upfront, apply changes atomically with rollback

**Sources:** HIGH - `/home/feanor/Projects/splice/src/patch/mod.rs` shows better pattern with batch operations

---

### Anti-Pattern 2: In-Place Database Mutation

**What it is:** Modify Magellan database directly during rename without backup or transaction.

**Why avoid:**
- No rollback if operation fails
- Database corruption on crash
- Can't undo partial rename
- Concurrent operations corrupt data

**Instead:** Use transactional updates with copy-on-write

**Sources:** HIGH - SQLite transaction patterns

---

### Anti-Pattern 3: String-Based Symbol Identification

**What it is:** Use symbol name + file path as unique identifier for lookups.

**Why avoid:**
- Symbol name collisions (multiple `foo()` in different scopes)
- Can't distinguish overloaded functions
- Breaks with shadowing
- ID format changes break everything

**Instead:** Use stable symbol IDs (BLAKE3 hash of name+path+byte_start)

**Sources:** HIGH - `/home/feanor/Projects/splice/src/symbol_id.rs` already has correct ID pattern

---

## Technical Debt Patterns

| Shortcut | Immediate Benefit | Long-term Cost | When Acceptable |
|----------|-------------------|----------------|-----------------|
| O(n) file iteration for symbol lookup | Fast to implement | 30+ second queries on large projects | **Never** - blocks adoption |
| No cross-file locking | Simpler code | Data corruption in concurrent use | MVP only, must fix before v1.0 |
| Byte-based replacement without UTF-8 checks | Works for ASCII codebases | Corrupts non-English files | Only if targeting English-only shops |
| Skip graph algorithm depth limits | Correct results for all inputs | Stack overflow, infinite loops | **Never** - safety critical |
| Hardcode ID length (16 or 32) | Simpler validation | Migration pain forever | **Never** - use enum |

---

## Integration Gotchas

| Integration | Common Mistake | Correct Approach |
|-------------|----------------|------------------|
| Magellan v2.0.0 | Assume database format unchanged | Check schema_version table, migrate if needed |
| Cross-file rename | Update files sequentially | Use transaction with snapshot+rollback |
| Graph algorithms | Load entire graph into memory | Use streaming iteration, chunking |
| UTF-8 handling | Assume ASCII only | Validate character boundaries, test Unicode |
| Test assertions | Exact JSON match | Use subset match, field selection |

---

## Performance Traps

| Trap | Symptoms | Prevention | When It Breaks |
|------|----------|------------|----------------|
| O(n) symbol lookup | Queries take >10s on 10K files | Build global symbol name index | Any non-trivial codebase |
| Full graph loading | OOM kills, 4GB+ RSS | Streaming iteration, adjacency lists | >1M symbols or 8GB RAM machines |
| Unlimited traversal | Stack overflow, infinite loops | Depth limit, timeout enforcement | Recursive code >100 deep |
| No query result caching | Same slow query repeated | LRU cache with TTL on db change | Repeated queries in workflows |

---

## Recovery Strategies

| Pitfall | Recovery Cost | Recovery Steps |
|---------|---------------|----------------|
| Version upgrade (ID format) | **HIGH** - Full re-index required | 1. Backup database, 2. Run migration script, 3. Verify all symbols migrated, 4. Re-index if migration fails |
| Cross-file rename corruption | **MEDIUM** - Restore from backup | 1. Detect via `cargo check`, 2. Restore from `.splice-backup`, 3. Re-run with transaction lock |
| Cycle detection infinite loop | **LOW** - Force-quit and retry | 1. Kill process, 2. Add depth limit, 3. Retry with limit |
| UTF-8 corruption | **MEDIUM** - Manual file repair | 1. Git restore corrupted files, 2. Add UTF-8 validation, 3. Re-apply patches |
| Test breakage from new fields | **LOW** - Update test fixtures | 1. Switch to subset assertions, 2. Regenerate golden files, 3. Re-run |
| Memory exhaustion | **LOW** - Reduce graph size | 1. Add memory limits, 2. Use chunking, 3. Retry |
| Schema version mismatch | **MEDIUM** - Database migration | 1. Detect version, 2. Run migration, 3. Verify |
| Proof mode output changes | **LOW** - LLM prompt update | 1. Make proof opt-in, 2. Update prompts |

---

## Pitfall-to-Phase Mapping

| Pitfall | Prevention Phase | Verification |
|---------|------------------|--------------|
| Version upgrade (ID format 16→32) | **Phase 1** (Dependency upgrade) | Migration test: v0.5.3 DB → v2.0.0 DB, verify all symbols readable |
| Cross-file rename race conditions | **Phase 3** (Cross-file rename) | Concurrent rename test: 2 processes, overlapping symbols, verify no corruption |
| Cycle detection infinite loops | **Phase 2** (Graph algorithms) | Recursive code test: 1000-deep mutual recursion, verify <1s runtime |
| O(n) query performance | **Phase 2** (Graph algorithms) | Benchmark: 10K file codebase, query should be <100ms |
| UTF-8 byte offset corruption | **Phase 3** (Cross-file rename) | Unicode test: Patch emoji/CJK files, verify valid UTF-8 |
| Test breakage (new JSON fields) | **Phase 1** (Version upgrade) | Pre/post upgrade test: `cargo test` pass rate must be 100% in both |
| Memory exhaustion (full graph load) | **Phase 2** (Graph algorithms) | Memory test: 1M symbol graph, RSS <2GB |
| Schema version detection | **Phase 1** (Version upgrade) | Version mismatch test: Open v0.5.3 DB with v2.0.0 code, should error cleanly |
| Proof mode output format | **Phase 4** (Proof mode) | Output test: Default output has no proof fields, `--proof` flag adds them |
| Path canonicalization | **Phase 3** (Cross-file rename) | Symlink test: Rename in symlinked directory, verify correct files modified |

---

## "Looks Done But Isn't" Checklist

- [ ] **Version upgrade:** Often missing database migration script — verify `migrate-db` command exists and tested
- [ ] **Cross-file rename:** Often missing transactional locks — verify concurrent rename test fails gracefully
- [ ] **Cycle detection:** Often missing depth limits — verify 1000-deep recursion returns in <1s
- [ ] **Symbol indexing:** Often missing global name index — verify O(1) query on 10K file codebase
- [ ] **UTF-8 safety:** Often missing Unicode tests — verify emoji/CJK patches don't corrupt
- [ ] **Test compatibility:** Often missing subset assertions — verify all tests pass after schema changes
- [ ] **Memory limits:** Often missing memory monitoring — verify RSS <2GB on large graphs
- [ ] **Schema versioning:** Often missing version table — verify v0.5.3 DB rejected with clear error

---

## Confidence Assessment

| Area | Confidence | Reason |
|------|------------|--------|
| **Version upgrade risks** | **HIGH** | Read `src/symbol_id.rs`, verified 16-char hardcoding, confirmed Magellan 2.0.0 exists on crates.io |
| **Cross-file rename risks** | **HIGH** | Read `src/patch/mod.rs`, verified no cross-file locking, reviewed batch application logic |
| **Graph algorithm risks** | **HIGH** | Reviewed existing graph traversal patterns, confirmed no depth limiting in codebase |
| **Performance risks** | **HIGH** | Found O(n) file iteration in `find_symbol_by_name`, line 364-400 |
| **UTF-8 safety risks** | **HIGH** | Found `byte_to_char` usage, verified UTF-8 validation exists but may not cover all cases |
| **Test breakage risks** | **HIGH** | Existing tests use exact JSON matching (convention in `tests/` directory) |
| **Memory exhaustion risks** | **MEDIUM** | Standard graph algorithm issue, but no direct evidence of current loading pattern |
| **Schema version risks** | **MEDIUM** | No version table in current schema (inferred), standard database pattern |

**Overall confidence:** HIGH

**Gaps to Address:**
- Actual Magellan v2.0.0 API changes (verify with crate documentation or source code)
- Real-world performance on 100K+ file codebases (needs benchmarking)
- BLAKE3 ID format specifics (length, encoding)

---

## Sources

### Primary (HIGH confidence — verified source code)

- **Splice source code:**
  - `/home/feanor/Projects/splice/Cargo.toml:25` — Current Magellan 0.5.3 dependency
  - `/home/feanor/Projects/splice/src/symbol_id.rs` — 16-char ID format hardcoded
  - `/home/feanor/Projects/splice/src/graph/magellan_integration.rs` — Integration layer, O(n) file iteration
  - `/home/feanor/Projects/splice/src/patch/mod.rs` — Batch application without cross-file locks
  - `/home/feanor/Projects/splice/tests/id_format_tests.rs` — Test assertions for 16-char IDs
  - `/home/feanor/Projects/splice/tests/cross_file_tests.rs` — Cross-file resolution tests

- **Documentation:**
  - `/home/feanor/Projects/splice/docs/CROSS_FILE_RESOLUTION.md` — Import-aware resolution design, mutual recursion edge cases
  - `/home/feanor/Projects/splice/.planning/research/MAGELLAN_INTEGRATION_PITFALLS.md` — Query delegation pitfalls, performance issues
  - `/home/feanor/Projects/splice/.planning/research/PITFALLS.md` — Rich span pitfalls, test breakage patterns

### Secondary (MEDIUM confidence — standard patterns)

- Version upgrade patterns (database migration strategies)
- Graph algorithm pitfalls (Tarjan's SCC, depth limiting)
- UTF-8 safety issues (byte vs character indexing)
- Cross-file transaction patterns (locking strategies)

### Tertiary (LOW confidence — needs verification)

- Magellan v2.0.0 specific API changes (BLAKE3 ID format details)
- Real-world performance on large codebases (needs benchmarking)
- Exact new fields in v2.0.0 JSON output (proof data format)

---

*Pitfalls research for: Magellan v2.0.0 semantic refactoring integration*
*Researched: 2026-02-04*
*Status: Ready for roadmap phase planning*
