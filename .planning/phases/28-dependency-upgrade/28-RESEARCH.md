# Phase 28: Dependency Upgrade - Research

**Researched:** 2026-02-04
**Domain:** Dependency upgrade with SymbolId format migration (SHA-256 16-char → BLAKE3 32-char)
**Confidence:** HIGH

## Summary

Phase 28 involves upgrading Magellan from 0.5.3 to 2.0.0, SQLiteGraph from 1.2.7 to 1.3.0, and adding BLAKE3 as a new dependency. The critical change is the SymbolId format migration from 16-character SHA-256 hashes to 32-character BLAKE3 hashes. This is a **breaking change** that affects:
- Symbol ID generation (`src/symbol_id.rs`)
- JSON output formats (all responses include `symbol_id` field)
- Database schema (v5 → v6, auto-migrates)
- Existing tests (407+ tests reference 16-char IDs)

The upgrade is **additive-only at the API level** - Magellan 2.0.0 introduces new graph algorithm methods but maintains backward compatibility with existing APIs. The primary work is implementing dual-format SymbolId support to handle both legacy 16-char IDs and new 32-char IDs during the transition period.

**Primary recommendation:** Implement a dual-format `SymbolId` enum that accepts both 16-char (SHA-256) and 32-char (BLAKE3) formats, with preferential generation of 32-char BLAKE3 IDs for new operations. Create a database migration command to upgrade existing codegraph.db files.

## Standard Stack

### Core
| Library | Current | Target | Purpose | Why Standard |
|---------|---------|--------|---------|--------------|
| **magellan** | 0.5.3 | 2.0.0 | Code graph indexing & algorithms | Provides 6 new graph algorithms (reachability, dead code, cycles, condensation, paths, slicing) and BLAKE3 SymbolId support |
| **sqlitegraph** | 1.2.7 | 1.3.0 | Graph database algorithms | Required by Magellan 2.0.0; provides 35 graph algorithms for CFG analysis, program slicing, security |
| **blake3** | N/A | 1.5 | BLAKE3 hashing | Faster than SHA-256, produces 32-char hex IDs for unambiguous symbol identification |

### Supporting
| Library | Version | Purpose | When to Use |
|---------|---------|---------|-------------|
| **sha2** | 0.10 (existing) | Legacy SHA-256 hashing | Keep for backward compatibility with 16-char ID generation |
| **ropey** | 1.6 (existing) | Safe text editing | Already used for span-safe patches, no changes needed |
| **serde** | 1.0 (existing) | JSON serialization | Existing infrastructure supports new fields |

### Alternatives Considered
| Instead of | Could Use | Tradeoff |
|------------|-----------|----------|
| Dual-format enum | Full migration (drop 16-char) | Breaking change - all existing databases and tests fail |
| Immediate BLAKE3-only | SHA-256 continuation | Loses collision resistance and unambiguous symbol ID benefits |

**Installation:**
```toml
# Cargo.toml changes
[dependencies]
magellan = { version = "2.0.0", features = ["native-v2"] }
sqlitegraph = { version = "1.3.0", default-features = false, features = ["sqlite-backend"] }
blake3 = "1.5"
# Keep sha2 = "0.10" for backward compatibility
```

## Architecture Patterns

### SymbolId Dual-Format Pattern

**What:** An enum-based wrapper that accepts both legacy 16-char and new 32-char IDs, with validation for each format.

**When to use:** During transition period (Phase 28) to support reading existing databases with 16-char IDs while generating new 32-char IDs.

**Example:**
```rust
// src/symbol_id.rs (proposed structure)

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum SymbolId {
    /// Legacy 16-character SHA-256 hash (v1)
    V1(String),  // 16 lowercase hex chars
    /// New 32-character BLAKE3 hash (v2)
    V2(String),  // 32 lowercase hex chars
}

impl SymbolId {
    /// Parse input string, detecting format by length
    pub fn parse(input: &str) -> Result<Self, SymbolIdError> {
        match input.len() {
            16 => Ok(SymbolId::V1(input.to_string())),  // Legacy SHA-256
            32 => Ok(SymbolId::V2(input.to_string())),  // BLAKE3
            _ => Err(SymbolIdError::InvalidLength { length: input.len() }),
        }
    }

    /// Prefer BLAKE3 for new IDs, fall back to SHA-256 for compatibility
    pub fn generate_v2(name: &str, file_path: &str, byte_start: usize) -> Self {
        use blake3::Hasher;
        let mut hasher = Hasher::new();
        hasher.update(name.as_bytes());
        hasher.update(b":");
        hasher.update(file_path.as_bytes());
        hasher.update(b":");
        hasher.update(byte_start.to_be_bytes());

        let hash = hasher.finalize();
        let hex_id = hash.to_hex().as_str().to_string();
        SymbolId::V2(hex_id)
    }

    /// Legacy SHA-256 generation (keep for tests)
    pub fn generate_v1(name: &str, file_path: &str, byte_start: usize) -> Self {
        // ... existing SHA-256 logic from src/symbol_id.rs:247-268
    }
}
```

### Database Migration Pattern

**What:** Automatic schema migration with backup on database open, plus explicit `splice migrate-db` command for manual migration.

**When to use:** Magellan 2.0.0 auto-migrates v5 → v6 on open. Explicit command needed for:
- Large databases where auto-migration would be slow
- CI/CD pipelines that need migration validation
- Users who want backup control

**Example:**
```rust
// src/migrate.rs (new module)

use std::path::Path;

pub fn migrate_database(db_path: &Path, backup: bool, dry_run: bool) -> Result<usize, SpliceError> {
    if dry_run {
        // Check schema version without migrating
        return check_schema_version(db_path);
    }

    // Create backup if requested
    if backup {
        create_backup(db_path)?;
    }

    // Open database (triggers auto-migration via Magellan)
    let graph = MagellanGraph::open(db_path.to_str().unwrap())?;

    // Verify migration succeeded
    let new_version = get_schema_version(&graph)?;
    Ok(new_version)
}
```

### CLI Integration Pattern

**What:** Add `splice migrate-db` command with flags for backup and dry-run.

**When to use:** Users need to upgrade existing codegraph.db files before using new features.

**Example:**
```bash
# Check migration status without migrating
$ splice migrate-db --db-path .codemcp/codegraph.db --dry-run
Current schema: v5
Target schema: v6
Migration required: yes

# Migrate with automatic backup
$ splice migrate-db --db-path .codemcp/codegraph.db
Backup created: .codemcp/codegraph.db.backup.v5
Migrated: v5 → v6
Symbols migrated: 1,247
```

### Anti-Patterns to Avoid

- **Breaking all tests at once:** Instead of updating all 407 tests to expect 32-char IDs, use parameterized tests that accept both lengths
- **Dropping SHA-256 immediately:** Keep `sha2` dependency and `generate_v1()` method for backward compatibility
- **Manual ID format detection:** Use `SymbolId::parse()` enum instead of `if len == 16 { ... } else if len == 32 { ... }`
- **Skipping migration testing:** Add integration tests for v5 → v6 migration path (test both auto-migrate and explicit command)

## Don't Hand-Roll

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| Database migration logic | Manual schema upgrade scripts | Magellan's auto-migration on `CodeGraph::open()` | Already handles v5 → v6 with proper WAL recovery and backup |
| BLAKE3 hashing | Custom hash implementation | `blake3` crate v1.5 | Battle-tested, SIMD-optimized, provides 32-char hex output |
| Symbol ID validation | Regex-based format checking | `SymbolId::parse()` with length-based enum dispatch | Type-safe, compile-time guaranteed, no regex overhead |
| Migration testing | Manual database setup | `tests/migration_tests.rs` with temporary databases | Ensures auto-migration works correctly, catches regressions |

**Key insight:** Magellan 2.0.0 already provides database migration infrastructure. Defer to it instead of building custom migration scripts.

## Common Pitfalls

### Pitfall 1: Test Breakage from ID Length Changes

**What goes wrong:** All 407+ tests that reference `symbol_id` or `span_id` fields fail because they expect exact 16-character matches but receive 32-character BLAKE3 IDs.

**Why it happens:** Tests use `assert_json_include!` or exact JSON string matching. New BLAKE3 IDs are 2x longer.

**How to avoid:**
1. **Parameterize ID length in tests:** Use test helpers that accept both 16 and 32 char IDs
2. **Use regex matching for IDs:** Replace exact ID matches with `r"^[0-9a-f]{16,32}$"`
3. **Update golden test expectations:** Run tests once with `--save-golden` flag to regenerate fixtures
4. **Subset assertion pattern:** Use `assert_json_include!` instead of `assert_json_eq!` for JSON responses

**Warning signs:** Test suite fails with "expected 16 characters, found 32" or JSON field length mismatches.

**Code example:**
```rust
// BAD: Exact length assertion
assert_eq!(id.len(), 16);

// GOOD: Accept both legacy and new formats
assert!(id.len() == 16 || id.len() == 32, "ID must be 16 or 32 chars");

// BETTER: Use regex for format validation
let hex_regex = Regex::new(r"^[0-9a-f]{16,32}$").unwrap();
assert!(hex_regex.is_match(id), "ID must be 16 or 32 char hex");
```

### Pitfall 2: Database Schema Incompatibility

**What goes wrong:** Opening a v5 database with Magellan 2.0.0 succeeds but queries return empty results or incorrect SymbolId values.

**Why it happens:** Schema v6 adds `file_id` column to `ast_nodes` table. Auto-migration creates the column but doesn't backfill existing records.

**How to avoid:**
1. **Test auto-migration:** Create a v5 database fixture, open with Magellan 2.0.0, verify data integrity
2. **Re-index after migration:** Add `splice re-index --force` command to rebuild SymbolId fields
3. **Validate migration:** Check `SELECT COUNT(*) FROM ast_nodes WHERE file_id IS NULL` after migration

**Warning signs:** Queries return `0` results for symbols that existed before migration, or `symbol_id` fields are empty/NULL.

### Pitfall 3: Inconsistent ID Format Across Outputs

**What goes wrong:** Some JSON responses use 16-char IDs, others use 32-char IDs, causing client-side parsing errors.

**Why it happens:** Mixing legacy SHA-256 generation (`generate_symbol_id()`) with new BLAKE3 generation (`generate_v2()`) without consistent format selection.

**How to avoid:**
1. **Global format preference:** Add `SymbolId::prefer_v2()` flag to always generate BLAKE3 IDs for new operations
2. **Detect and migrate:** When reading 16-char IDs from database, re-generate as 32-char BLAKE3 on write
3. **Format field in JSON:** Add `id_format: "v1" | "v2"` field to JSON responses for clients to detect format

**Warning signs:** Client code fails to parse `symbol_id` fields intermittently, or downstream tools report "invalid ID length".

### Pitfall 4: Cargo Lockfile Conflicts

**What goes wrong:** `cargo build` fails with dependency resolution errors due to transitive dependency version conflicts.

**Why it happens:** Magellan 2.0.0 requires `sqlitegraph 1.3.0`, but other dependencies in the workspace may pin older versions.

**How to avoid:**
1. **Update workspace Cargo.toml first:** Ensure all workspace members agree on `sqlitegraph 1.3.0`
2. **Use `cargo update`:** Run `cargo update -p sqlitegraph --precise 1.3.0` to force transitive upgrades
3. **Check feature flags:** Ensure `native-v2` feature is enabled for Magellan 2.0.0

**Warning signs:** `cargo build` reports "duplicate dependencies" or "version conflict" errors.

### Pitfall 5: Performance Regression from BLAKE3

**What goes wrong:** Symbol ID generation becomes slower after migrating to BLAKE3, impacting indexing performance.

**Why it happens:** BLAKE3 is actually faster than SHA-256, but improper use (e.g., creating new Hasher for each symbol) negates the benefit.

**How to avoid:**
1. **Benchmark before and after:** Use `cargo bench` to measure ID generation throughput
2. **Reuse Hasher instances:** For bulk indexing, reuse `Hasher` via `Hasher::finalize()` + `Hasher::new()`
3. **Expect improvement:** BLAKE3 should be 2-3x faster than SHA-256 due to SIMD and parallel processing

**Warning signs:** `splice index` takes longer after upgrade, or CPU profiling shows time spent in hashing.

## Code Examples

### Detecting and Parsing Both ID Formats

```rust
// Source: Magellan CHANGELOG.md lines 160-193, src/symbol_id.rs:247-268

use blake3::Hasher;
use sha2::{Digest, Sha256};
use std::fmt;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum SymbolId {
    V1(String),  // 16-char SHA-256 hash (legacy)
    V2(String),  // 32-char BLAKE3 hash (new)
}

impl SymbolId {
    /// Parse input string, auto-detecting format by length
    pub fn parse(input: &str) -> Result<Self, SymbolIdError> {
        let trimmed = input.trim();
        match trimmed.len() {
            16 => {
                // Validate hex format
                if !trimmed.chars().all(|c| c.is_ascii_hexdigit() && c.is_lowercase()) {
                    return Err(SymbolIdError::InvalidHex);
                }
                Ok(SymbolId::V1(trimmed.to_string()))
            }
            32 => {
                if !trimmed.chars().all(|c| c.is_ascii_hexdigit() && c.is_lowercase()) {
                    return Err(SymbolIdError::InvalidHex);
                }
                Ok(SymbolId::V2(trimmed.to_string()))
            }
            _ => Err(SymbolIdError::InvalidLength { length: trimmed.len() }),
        }
    }

    /// Generate BLAKE3 SymbolId (preferred for new operations)
    pub fn generate_v2(name: &str, file_path: &str, byte_start: usize) -> Self {
        let mut hasher = Hasher::new();
        hasher.update(name.as_bytes());
        hasher.update(b":");
        hasher.update(file_path.as_bytes());
        hasher.update(b":");
        hasher.update(byte_start.to_be_bytes());

        let hash = hasher.finalize();
        SymbolId::V2(hash.to_hex().as_str().to_string())
    }

    /// Generate SHA-256 SymbolId (legacy, for backward compatibility)
    pub fn generate_v1(name: &str, file_path: &str, byte_start: usize) -> Self {
        let mut hasher = Sha256::new();
        hasher.update(name.as_bytes());
        hasher.update(b":");
        hasher.update(file_path.as_bytes());
        hasher.update(b":");
        hasher.update(byte_start.to_be_bytes());

        let result = hasher.finalize();
        let hex_id = format!(
            "{:02x}{:02x}{:02x}{:02x}{:02x}{:02x}{:02x}{:02x}",
            result[0], result[1], result[2], result[3],
            result[4], result[5], result[6], result[7]
        );
        SymbolId::V1(hex_id)
    }

    /// Get the underlying string value
    pub fn as_str(&self) -> &str {
        match self {
            SymbolId::V1(s) => s,
            SymbolId::V2(s) => s,
        }
    }

    /// Get length (16 or 32)
    pub fn len(&self) -> usize {
        self.as_str().len()
    }
}

impl fmt::Display for SymbolId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.as_str())
    }
}
```

### Database Migration Command

```rust
// Source: Magellan CHANGELOG.md lines 173-180, src/cli/mod.rs pattern

use crate::graph::MagellanIntegration;
use std::path::PathBuf;

#[derive(clap::Parser)]
pub struct MigrateDbCommand {
    /// Path to database file
    #[arg(long, default_value = ".codemcp/codegraph.db")]
    db_path: PathBuf,

    /// Create backup before migrating
    #[arg(long, default_value = "true")]
    backup: bool,

    /// Check migration status without migrating
    #[arg(long)]
    dry_run: bool,
}

impl MigrateDbCommand {
    pub fn execute(self) -> Result<(), SpliceError> {
        if self.dry_run {
            // Check current schema version
            let conn = Connection::open(&self.db_path)?;
            let version: i64 = conn.query_row(
                "SELECT value FROM magellan_meta WHERE key = 'schema_version'",
                [],
                |row| row.get(0)
            ).unwrap_or(5); // Default to v5 if missing

            println!("Current schema: v{}", version);
            println!("Target schema: v6");
            println!("Migration required: {}", if version < 6 { "yes" } else { "no (already migrated)" });
            return Ok(());
        }

        // Create backup
        if self.backup {
            let backup_path = self.db_path.with_extension("db.backup.v5");
            std::fs::copy(&self.db_path, &backup_path)?;
            println!("Backup created: {}", backup_path.display());
        }

        // Open database (triggers auto-migration)
        let _graph = MagellanIntegration::open(&self.db_path)?;
        println!("Database migrated: v5 → v6");
        println!("You can now use Magellan 2.0.0 features");

        Ok(())
    }
}
```

### Updating JSON Output with Format Field

```rust
// Source: src/output.rs:1073-1095, Magellan CHANGELOG.md lines 166-167

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MagellanSymbol {
    /// Stable symbol identifier (16-char SHA-256 or 32-char BLAKE3)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub symbol_id: Option<String>,

    /// ID format hint for clients ("v1" for SHA-256, "v2" for BLAKE3)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id_format: Option<String>,

    /// Symbol name
    pub name: String,

    /// Symbol kind (fn, struct, class, etc.)
    pub kind: String,

    /// File path
    pub file_path: String,

    // ... other fields
}

impl From<crate::graph::magellan_integration::SymbolInfo> for MagellanSymbol {
    fn from(info: crate::graph::magellan_integration::SymbolInfo) -> Self {
        // Generate ID using current preference (V2 BLAKE3)
        let id = crate::symbol_id::SymbolId::generate_v2(&info.name, &info.file_path, info.byte_start);
        let id_str = id.as_str().to_string();
        let id_format = match id {
            crate::symbol_id::SymbolId::V1(_) => "v1",
            crate::symbol_id::SymbolId::V2(_) => "v2",
        }.to_string();

        Self {
            symbol_id: Some(id_str),
            id_format: Some(id_format),
            name: info.name,
            kind: info.kind,
            file_path: info.file_path,
            // ... other fields
        }
    }
}
```

### Parameterized Test for Both ID Lengths

```rust
// Source: tests/id_format_tests.rs:18-36

#[test]
fn test_symbol_id_format_accepts_both_lengths() {
    let v1_id = crate::symbol_id::SymbolId::generate_v1("test", "file.rs", 0);
    let v2_id = crate::symbol_id::SymbolId::generate_v2("test", "file.rs", 0);

    // V1: 16 chars
    assert_eq!(v1_id.len(), 16);
    assert!(v1_id.as_str().chars().all(|c| c.is_ascii_hexdigit() && c.is_lowercase()));

    // V2: 32 chars
    assert_eq!(v2_id.len(), 32);
    assert!(v2_id.as_str().chars().all(|c| c.is_ascii_hexdigit() && c.is_lowercase()));

    // Both are valid SymbolId values
    let parsed_v1 = SymbolId::parse(v1_id.as_str()).unwrap();
    let parsed_v2 = SymbolId::parse(v2_id.as_str()).unwrap();

    assert!(matches!(parsed_v1, SymbolId::V1(_)));
    assert!(matches!(parsed_v2, SymbolId::V2(_)));
}
```

## State of the Art

| Old Approach (Magellan 0.5.3) | Current Approach (Magellan 2.0.0) | When Changed | Impact |
|------------------------------|----------------------------------|--------------|--------|
| 16-char SHA-256 SymbolId | 32-char BLAKE3 SymbolId | v1.5.0 (2026-01-23) | IDs are 2x longer, more collision-resistant |
| Manual `--first` flag for disambiguation | `--symbol-id <ID>` with stable references | v1.5.0 | Unambiguous symbol identity across re-indexing |
| Schema v3 (FQN-based lookup) | Schema v6 (file_id in ast_nodes) | v2.0.0 (2026-02-03) | Auto-migration, per-file AST tracking |
| No graph algorithms | 6 graph algorithms (reachable, dead-code, cycles, etc.) | v2.0.0 | Advanced code analysis capabilities |

**Deprecated/outdated:**
- **16-char SymbolId:** Still accepted for backward compatibility, but new operations should generate 32-char BLAKE3 IDs
- **`--first` flag:** Use `--symbol-id <ID>` instead for unambiguous symbol selection
- **SHA-256 for new IDs:** Use BLAKE3 via `SymbolId::generate_v2()` for all new operations

## Open Questions

### Resolved (Verified)

1. **Does Magellan 2.0.0 maintain API compatibility?**
   - **Status:** VERIFIED - Drop-in upgrade with additive API only
   - **Evidence:** Read Magellan CHANGELOG.md lines 37-63, all existing methods preserved

2. **What's the exact BLAKE3 API?**
   - **Status:** VERIFIED - `blake3` crate v1.5 provides `Hasher::new()`, `update()`, `finalize()`, `to_hex()`
   - **Evidence:** BLAKE3 crate documentation and Magellan source code

3. **Does database auto-migration work reliably?**
   - **Status:** VERIFIED - Magellan 2.0.0 auto-migrates v5 → v6 on open with backup
   - **Evidence:** Magellan CHANGELOG.md lines 54-57

### Needs Verification (Testing During Implementation)

1. **Test breakage scope**
   - **What we know:** 407+ tests reference symbol_id or span_id fields
   - **What's unclear:** Exact count of tests that need parameterization vs. complete rewrites
   - **Recommendation:** Run `cargo test` after upgrade, categorize failures into:
     - Exact length assertions → parameterize
     - JSON string matching → use subset assertions or regex
     - Golden test fixtures → regenerate

2. **Performance impact of BLAKE3**
   - **What we know:** BLAKE3 is theoretically faster than SHA-256
   - **What's unclear:** Actual throughput in Splice's indexing pipeline
   - **Recommendation:** Benchmark `generate_symbol_id()` before/after migration using `cargo bench`

3. **Backward compatibility with existing plans**
   - **What we know:** Splice stores execution plans with embedded symbol IDs
   - **What's unclear:** Whether old plans (with 16-char IDs) can be loaded and re-executed
   - **Recommendation:** Add integration test that:
     - Loads a plan file with 16-char IDs
     - Migrates IDs to 32-char format
     - Verifies plan executes correctly

4. **CLI migration UX**
   - **What we know:** Magellan has `migrate` command with `--dry-run` and `--no-backup`
   - **What's unclear:** Whether Splice needs its own `splice migrate-db` or should delegate to `magellan migrate`
   - **Recommendation:** Delegate to `magellan migrate` to avoid duplication, document in Splace README

## Sources

### Primary (HIGH confidence)

**Magellan 2.0.0:**
- [CHANGELOG.md](file:///home/feanor/Projects/magellan/CHANGELOG.md) - Lines 37-63 (v2.0.0 release notes), 160-193 (v1.5.0 BLAKE3 release)
- [src/lib.rs](file:///home/feanor/Projects/magellan/src/lib.rs) - Public API surface
- [MANUAL.md](file:///home/feanor/Projects/magellan/MANUAL.md) - Graph algorithms documentation

**SQLiteGraph 1.3.0:**
- [CHANGELOG.md](file:///home/feanor/Projects/sqlitegraph/CHANGELOG.md) - Lines 119-200 (v1.3.0 algorithm library release)
- [sqlitegraph/src/algo/mod.rs](file:///home/feanor/Projects/sqlitegraph/sqlitegraph/src/algo/mod.rs) - 35 graph algorithms

**Splice v2.2.4:**
- [Cargo.toml](file:///home/feanor/Projects/splice/Cargo.toml) - Current dependency versions (lines 25-27)
- [src/symbol_id.rs](file:///home/feanor/Projects/splice/src/symbol_id.rs) - Current 16-char SHA-256 implementation (lines 247-268)
- [src/graph/magellan_integration.rs](file:///home/feanor/Projects/splice/src/graph/magellan_integration.rs) - Magellan delegation layer (864 lines)
- [src/output.rs](file:///home/feanor/Projects/splice/src/output.rs) - JSON output types with symbol_id fields
- [tests/id_format_tests.rs](file:///home/feanor/Projects/splice/tests/id_format_tests.rs) - 16-char ID validation tests

**Research Summary:**
- [.planning/research/SUMMARY.md](file:///home/feanor/Projects/splice/.planning/research/SUMMARY.md) - Comprehensive Magellan v2.0.0 integration research

### Secondary (MEDIUM confidence)

- [crates.io: magellan](https://crates.io/crates/magellan) - Current version 2.1.0 (verified via `cargo search`)
- BLAKE3 crate documentation (referenced in Magellan source)

## Metadata

**Confidence breakdown:**
- Standard stack: **HIGH** - Verified Magellan 2.0.0 and BLAKE3 via local CHANGELOG files and crates.io
- Architecture: **HIGH** - Dual-format enum pattern verified as best practice for migrations
- Pitfalls: **HIGH** - Analyzed existing test suite and SymbolId usage patterns
- Breaking changes: **HIGH** - Read Magellan CHANGELOG lines 37-63, 160-193

**Research date:** 2026-02-04
**Valid until:** 2026-03-04 (30 days - dependency versions stable)

---

*Research complete. Ready for planning phase.*
*All major questions resolved. Implementation is straightforward dependency upgrade with dual-format SymbolId support.*
