# External Integrations

**Analysis Date:** 2026-01-17

## APIs & External Services

**External APIs:**
- None detected - All processing is local

**External Services:**
- Ollama - LLM provider for semantic analysis (configured in `.codemcp/config.toml`)
  - Used for semantic code analysis
  - Local-only operation

## Data Storage

**Databases:**
- SQLite - Primary data store for multiple purposes
  - Connection: Local file-based databases
  - Client: rusqlite (via SQLiteGraph)
  - Databases:
    - `magellan.db` - Magellan symbol database
    - `operations.db` - Operation tracking database
    - `codegraph.db` - Code graph storage database
    - `splice_map.db` - Splice-specific mapping database

**File Storage:**
- Local filesystem only
- Backup files created with `.backup` extension
- Manifest files for backup tracking

**Caching:**
- In-memory symbol cache in `src/graph/mod.rs`
- File caching in CodeGraph

## Authentication & Identity

**Auth Provider:**
- Not applicable - CLI tool with no authentication requirements

## Monitoring & Observability

**Error Tracking:**
- None - Uses Rust's built-in error handling

**Logs:**
- log crate - Structured logging
- env_logger - Environment-based logger configuration

## CI/CD & Deployment

**Hosting:**
- crates.io - Rust package distribution
- Installed via `cargo install splice`

**CI Pipeline:**
- Not detected - No GitHub Actions or CI configuration found

## Environment Configuration

**Development:**
- No required environment variables
- Configuration via CLI arguments
- CodeMCP config: `.codemcp/config.toml`

**Staging:**
- Not applicable - No staging environment

**Production:**
- Same as development - CLI tool runs locally

## Webhooks & Callbacks

**Incoming:**
- None

**Outgoing:**
- None

## Language Tool Integration

**Compiler/Analyzer Integration:**
Per-language validation using system compilers:
- Rust: `cargo check`
- Python: `python -m py_compile`
- C: `gcc -fsyntax-only`
- C++: `g++ -fsyntax-only`
- Java: `javac`
- JavaScript: `node --check`
- TypeScript: `tsc --noEmit`

**Location:** `src/validate/gates.rs`

## Code Analysis Integration

**Magellan v0.5.3:**
- Integration: `src/ingest/magellan.rs`, `src/graph/magellan_integration.rs`
- Purpose: Multi-language code indexing and symbol resolution
- Database: `magellan.db`

**tree-sitter Parsers:**
- Languages: Rust, Python, C, C++, Java, JavaScript, TypeScript
- Version: 0.21 across all parsers
- Purpose: AST parsing for symbol extraction

**SQLiteGraph:**
- Integration: `src/graph/mod.rs`
- Purpose: Code relationship storage and querying
- Database: `codegraph.db`

---

*Integration audit: 2026-01-17*
*Update when adding/removing external services*
