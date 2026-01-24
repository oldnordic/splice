# Requirements: Splice v2.2.2 - Magellan Integration

**Defined:** 2026-01-24
**Core Value:** Span-safe refactoring with validation
**Milestone:** v2.2.2 - Magellan Integration (Unified CLI Interface)

## v2.2.2 Requirements

Requirements for Magellan query command delegation and CLI/data format alignment.

### Query Commands (QUERY-01 through QUERY-05)

- [x] **QUERY-01**: status command shows database statistics (files, symbols, references, calls, code_chunks counts)
- [x] **QUERY-02**: query command lists symbols in a file (--file, --kind, --with-context, --with-callers, --with-callees flags)
- [x] **QUERY-03**: find command finds symbol by name or symbol_id (--name, --symbol-id, --ambiguous flags)
- [x] **QUERY-04**: refs command shows callers/callees for a symbol (--name, --path, --direction flags)
- [x] **QUERY-05**: files command lists indexed files (--symbols flag for counts)

### CLI Alignment (CLI-01 through CLI-04)

- [ ] **CLI-01**: --output flag supports human (default), json, pretty formats
- [ ] **CLI-02**: --db flag specifies database path (delegates to Magellan)
- [ ] **CLI-03**: Exit codes match Magellan conventions (0=success, 1=error, 2=usage, 3=database, 4=file not found, 5=validation)
- [ ] **CLI-04**: --help shows command categories (Query, Edit, Export, Validation)

### Data Format Alignment (DATA-01 through DATA-04)

- [ ] **DATA-01**: Symbol ID uses 16-character hex format (SHA-256 hash, first 8 bytes)
- [ ] **DATA-02**: Execution ID uses {timestamp_hex}-{pid_hex} format for delegated queries
- [ ] **DATA-03**: Field name translation between Magellan (start_line) and Splice (line_start) conventions
- [ ] **DATA-04**: Response types defined (StatusResponse, FindResponse, RefsResponse, FilesResponse)

### Export (EXPORT-01 through EXPORT-02)

- [ ] **EXPORT-01**: export command exports graph data (--format json|jsonl|csv, --output flag)
- [ ] **EXPORT-02**: Export includes files, symbols, references, calls with proper schema version

### Error Handling (ERROR-01)

- [ ] **ERROR-01**: Magellan errors mapped to Splice SPL-E### codes with original error preserved in chain

## v2.3 Requirements

Deferred to future release.

### Performance Optimization

- **PERF-01**: Query performance optimization for large codebases (10K+ files)
- **PERF-02**: Relationship graph indexing for O(1) lookups
- **PERF-03**: Lazy context loading for files >32KB

### Advanced Features

- **ADV-01**: Real-time indexing delegation (watch command)
- **ADV-02**: Full LSP integration
- **ADV-03**: Custom Magellan commands

## Out of Scope

| Feature | Reason |
|---------|--------|
| Re-implementing Magellan parsers | Use Magellan as library, don't duplicate |
| Separate Splice database | Share Magellan's database via delegation |
| Subprocess delegation | Use in-process library delegation only |
| Magellan schema migration | Users run `magellan migrate` separately |

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
| CLI-01 | Phase 24 | Pending |
| CLI-02 | Phase 24 | Pending |
| CLI-03 | Phase 24 | Pending |
| CLI-04 | Phase 24 | Pending |
| DATA-03 | Phase 24 | Pending |
| DATA-04 | Phase 24 | Pending |
| EXPORT-01 | Phase 25 | Pending |
| EXPORT-02 | Phase 25 | Pending |
| ERROR-01 | Phase 25 | Pending |
| Integration validation | Phase 26 | Pending |

**Coverage:**
- v2.2.2 requirements: 15 total
- Mapped to phases: 15 (100%)

---
*Requirements defined: 2026-01-24*
