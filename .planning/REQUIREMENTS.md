# Requirements: Splice v2.3 - Magellan v2 Integration

**Defined:** 2026-02-04
**Core Value:** Span-safe refactoring with validation
**Milestone:** v2.3 - Magellan v2 Integration (Semantic Program Transformation)

---

## Milestone v2.3 Requirements

Requirements for integrating Magellan v2.0.0 capabilities - enabling cross-file rename with byte-accurate references and semantic program transformation features.

### Core Refactoring (REFACTOR-01 through REFACTOR-04)

- [ ] **REFACTOR-01**: Cross-file rename using byte-accurate ReferenceFact (byte_start/byte_end) for precise replacement
- [ ] **REFACTOR-02**: Impact analysis before refactoring (caller/callee chains, affected files count, reachability analysis)
- [ ] **REFACTOR-03**: Cross-language semantic rename across all 7 supported languages (Rust, Python, C, C++, Java, JavaScript, TypeScript)
- [ ] **REFACTOR-04**: Proof-based refactoring with machine-checkable behavioral equivalence proof (--proof flag)

### Graph Analysis (GRAPH-01 through GRAPH-03)

- [ ] **GRAPH-01**: Dead code detection using dead_symbols() query (find unreachable symbols from entry points)
- [ ] **GRAPH-02**: Cycle detection using detect_cycles() (find SCCs in call graph for refactoring safety)
- [ ] **GRAPH-03**: Condensation graph analysis using condense_call_graph() (collapse SCCs to DAG for safe refactoring order)

### Program Slicing (SLICE-01)

- [ ] **SLICE-01**: Forward/backward program slicing using slice() queries (find what affects symbol / what symbol affects)

### CLI Interface (CLI-05 through CLI-07)

- [ ] **CLI-05**: rename command with --symbol, --to, --db, --preview, --create-backup flags
- [ ] **CLI-06**: dead-code command with --entry flag for specifying entry point
- [ ] **CLI-07**: cycles, condense, slice commands for graph algorithm access

### Dependency Upgrade (DEPS-01 through DEPS-03)

- [ ] **DEPS-01**: Upgrade magellan dependency from 0.5.3 to 2.0.0
- [ ] **DEPS-02**: Upgrade sqlitegraph dependency from 1.2.7 to 1.3.0
- [ ] **DEPS-03**: Add blake3 dependency (v1.5) for BLAKE3 SymbolId support

### BLAKE3 SymbolId (SYMBOLID-01 through SYMBOLID-02)

- [ ] **SYMBOLID-01**: Migrate from 16-char SHA-256 IDs to 32-char BLAKE3 IDs
- [ ] **SYMBOLID-02**: Dual-format support for backward compatibility (accept both formats during transition)

### Data Format (DATA-05)

- [ ] **DATA-05**: Update symbol ID format in JSON responses (32-char BLAKE3 hex)

---

## v2.2.2 Requirements (Complete)

Requirements for Magellan query command delegation and CLI/data format alignment.

### Query Commands (QUERY-01 through QUERY-05)

- [x] **QUERY-01**: status command shows database statistics (files, symbols, references, calls, code_chunks counts)
- [x] **QUERY-02**: query command lists symbols in a file (--file, --kind, --with-context, --with-callers, --with-callees flags)
- [x] **QUERY-03**: find command finds symbol by name or symbol_id (--name, --symbol-id, --ambiguous flags)
- [x] **QUERY-04**: refs command shows callers/callees for a symbol (--name, --path, --direction flags)
- [x] **QUERY-05**: files command lists indexed files (--symbols flag for counts)

### CLI Alignment (CLI-01 through CLI-04)

- [x] **CLI-01**: --output flag supports human (default), json, pretty formats
- [x] **CLI-02**: --db flag specifies database path (delegates to Magellan)
- [x] **CLI-03**: Exit codes match Magellan conventions (0=success, 1=error, 2=usage, 3=database, 4=file not found, 5=validation)
- [x] **CLI-04**: --help shows command categories (Query, Edit, Export, Validation)

### Data Format Alignment (DATA-01 through DATA-04)

- [x] **DATA-01**: Symbol ID uses 16-character hex format (SHA-256 hash, first 8 bytes)
- [x] **DATA-02**: Execution ID uses {timestamp_hex}-{pid_hex} format for delegated queries
- [x] **DATA-03**: Field name translation between Magellan (start_line) and Splice (line_start) conventions
- [x] **DATA-04**: Response types defined (StatusResponse, FindResponse, RefsResponse, FilesResponse)

### Export (EXPORT-01 through EXPORT-02)

- [x] **EXPORT-01**: export command exports graph data (--format json|jsonl|csv, --output flag)
- [x] **EXPORT-02**: Export includes files, symbols, references, calls with proper schema version

### Error Handling (ERROR-01)

- [x] **ERROR-01**: Magellan errors mapped to Splice SPL-E### codes with original error preserved in chain

---

## Future Requirements

Deferred to future releases.

### Performance Optimization

- **PERF-01**: Query performance optimization for large codebases (10K+ files)
- **PERF-02**: Relationship graph indexing for O(1) lookups
- **PERF-03**: Lazy context loading for files >32KB

### Advanced Features

- **ADV-01**: Full CFG-based program slicing (current uses call-graph reachability fallback)
- **ADV-02**: Real-time indexing delegation (watch command)
- **ADV-03**: Full LSP integration
- **ADV-04**: Custom Magellan commands

---

## Out of Scope

| Feature | Reason |
|---------|--------|
| Re-implementing Magellan parsers | Use Magellan as library, don't duplicate |
| Separate Splice database | Share Magellan's database via delegation |
| Subprocess delegation | Use in-process library delegation only |
| Magellan schema migration | Users run `magellan migrate` separately |
| Type-based refactoring | Requires type inference (extremely complex), Magellan doesn't provide type info |
| Auto-fixing broken references | Compiler errors are important signals, fail fast instead |
| "Rename all instances" generic mode | Breaks with same name in different scopes, require explicit symbol selection |
| Full indexing for every operation | Large codebases take time to index, support lazy indexing |

---

## Traceability

Which phases cover which requirements. Updated during roadmap creation.

| Requirement | Phase | Status |
|-------------|-------|--------|
| DATA-01 | Phase 22 | Complete |
| DATA-02 | Phase 22 | Complete |
| QUERY-01 | Phase 23 | Complete |
| QUERY-02 | Phase 23 | Complete |
| QUERY-03 | Phase 23 | Complete |
| QUERY-04 | Phase 23 | Complete |
| QUERY-05 | Phase 23 | Complete |
| CLI-01 | Phase 24 | Complete |
| CLI-02 | Phase 24 | Complete |
| CLI-03 | Phase 24 | Complete |
| CLI-04 | Phase 24 | Complete |
| DATA-03 | Phase 24 | Complete |
| DATA-04 | Phase 24 | Complete |
| EXPORT-01 | Phase 25 | Complete |
| EXPORT-02 | Phase 25 | Complete |
| ERROR-01 | Phase 25 | Complete |
| Integration validation | Phase 26 | Complete |
| Code cleanup | Phase 27 | Complete |

**Coverage:**
- v2.2.2 requirements: 15 total
- Mapped to phases: 15 (100%)
- v2.3 requirements: 20 total
- To be mapped to phases: 20 (0%)

---
*Requirements defined: 2026-02-04*
*Updated: 2026-02-04 — v2.3 requirements added*
