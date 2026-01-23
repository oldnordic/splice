# External Integrations

**Analysis Date:** 2026-01-23

## APIs & External Services

**Documentation URLs (reference only):**
- Rust error documentation - https://doc.rust-lang.org/error-index.html
- TypeScript error documentation - https://www.typescriptlang.org/errors/
- Used for: Providing remediation links in diagnostic messages

## Data Storage

**Databases:**
- SQLite (via SQLiteGraph/Magellan)
  - Connection: Local filesystem (`.splice/codegraph.db` and `.splice/operations.db`)
  - Client: sqlitegraph 1.0 (native-v2 backend) + rusqlite 0.31
  - Purpose: Code graph storage, symbol indexing, execution logging

**File Storage:**
- Local filesystem only
  - Backup directory: `.splice/backups/`
  - No cloud storage or S3 integration
  - Source files edited in-place

**Caching:**
- Symbol name-to-NodeId caching in memory (HashMap)
- File path-to-NodeId caching in memory
- No external cache services

## Authentication & Identity

**Auth Provider:**
- None - Standalone CLI tool
  - No OAuth, JWT, or API keys required
  - All operations performed locally

## Monitoring & Observability

**Error Tracking:**
- None (external)
  - Local error handling via `thiserror` crate
  - Execution logging to local SQLite database

**Logs:**
- env_logger 0.11 - Structured logging to stderr
- Execution logging to `.splice/operations.db` (SQLite)
  - Timestamps, durations, command lines, error details
  - Disabled via `SPLICE_EXECUTION_LOG=false`

## CI/CD & Deployment

**Hosting:**
- Not applicable (CLI tool distributed via crates.io)
- Binary distribution via `cargo install splice`

**CI Pipeline:**
- None detected (no `.github/workflows/`, `GitLab CI`, or `Jenkinsfile`)
  - Local testing via `cargo test`
  - Manual release to crates.io

## Environment Configuration

**Required env vars:**
- None strictly required
  - Optional: `SPLICE_EXECUTION_LOG` (default: true)
  - Optional: `NO_COLOR` (disable colored output)
  - Optional: `RUST_LOG` (log level control)

**Secrets location:**
- None - No secrets management
  - No API keys, tokens, or credentials
  - All authentication-free operation

## Webhooks & Callbacks

**Incoming:**
- None - Splice does not expose any HTTP endpoints or webhook receivers

**Outgoing:**
- None - Splice does not make HTTP requests
  - Documentation URLs are static references only
  - No external API calls for validation or analysis

## Compiler Toolchain Integration

**External compilers (invoked via std::process::Command):**
- `cargo check` - Rust compilation validation
- `rust-analyzer` - Rust static analysis diagnostics (optional)
- `python -m py_compile` - Python syntax validation
- `gcc -fsyntax-only` - C compilation validation
- `g++ -fsyntax-only` - C++ compilation validation
- `javac` - Java compilation validation
- `node --check` - JavaScript syntax validation
- `tsc --noEmit` - TypeScript compilation validation

**Binary location:**
- Auto-discovery via `which` crate
- Fallback to PATH lookup
- Support for explicit analyzer path via `--analyzer` CLI flag

## CodeMCP Integration

**Local MCP Server:**
- `.codemcp/codemcp` binary - Optional local MCP server for enhanced indexing
- Configuration: `.codemcp/config.toml`
  - LLM provider: ollama at http://localhost:11434
  - LLM model: qwen2.5-coder:7b
  - Indexing: auto_watch enabled (500ms debounce)
- Databases in `.codemcp/`:
  - `codegraph.db` - Primary code graph
  - `operations.db` - Execution log
  - `staging.db` - Staged changes
  - `graph.db` - Additional graph storage

**Note:** This is an optional enhancement, not required for core Splice functionality.

---

*Integration audit: 2026-01-23*
