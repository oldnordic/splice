# External Integrations

**Analysis Date:** 2026-01-23

## APIs & External Services

**Code Analysis Tools:**
- Magellan v0.5.3 - Primary code indexing service
  - SDK/Client: Direct Rust crate integration
  - Database: SQLiteGraph backend (`sqlitegraph` crate)
  - Purpose: Multi-language symbol extraction, label-based queries, code chunk retrieval
  - Integration: `/home/feanor/Projects/splice/src/graph/magellan_integration.rs`

## Data Storage

**Databases:**
- SQLiteGraph (via Magellan) - Primary code graph storage
  - Connection: Direct file-based SQLite database
  - Client: Re-exported from Magellan (`sqlitegraph` crate)

- SQLite (rusqlite 0.31) - Operation execution logging
  - Connection: `.splice/operations.db`
  - Client: Direct Rusqlite client in `/home/feanor/Projects/splice/src/execution/log.rs`

**File Storage:**
- Local filesystem only - All code and database files stored locally

**Caching:**
- None detected - Direct database operations throughout

## Authentication & Identity

**Auth Provider:**
- Custom/local authentication only
  - Implementation: No external authentication required
  - All operations are local filesystem-based

## Monitoring & Observability

**Error Tracking:**
- Custom error handling (`thiserror`)
- Environment-based logging (env_logger)
- Execution logs stored in SQLite database

**Logs:**
- Environment logger with configurable verbosity
- Persistent execution logs in `.splice/operations.db`
- JSON-formatted output for machine consumption

## CI/CD & Deployment

**Hosting:**
- Local binary execution only
- No cloud or remote deployment detected

**CI Pipeline:**
- None detected - Manual testing via shell scripts
- Test validation: `/home/feanor/Projects/splice/test_splice_magellan_integration.sh`
- Schema validation: `/home/feanor/Projects/splice/validate_integration.sh`

## Environment Configuration

**Required env vars:**
- `SPLICE_EXECUTION_LOG` - Enable/disable operation logging (default: true)
- Optional environment variables for external tool paths

**Secrets location:**
- No external secrets required
- All configuration is file-based

## Webhooks & Callbacks

**Incoming:**
- None detected - All operations are command-driven

**Outgoing:**
- None detected - No external API calls or webhooks

## External Tool Integration

**Magellan Integration:**
- Direct crate dependency: `magellan = { version = "0.5.3", features = ["native-v2"] }`
- Integration layer: `/home/feanor/Projects/splice/src/graph/magellan_integration.rs`
- External configuration: `.codemcp/config.toml` (optional)
- Compatibility validation: Schema alignment tests in integration scripts

**Cross-tool Alignment:**
- JSON output format consistency between Splice and Magellan
- Schema version alignment for interoperability
- Byte-level precision preservation across tools

---

*Integration audit: 2026-01-23*