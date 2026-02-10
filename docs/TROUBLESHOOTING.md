# Troubleshooting

**Last Updated:** 2026-02-10
**Version:** v2.5.0

Common issues and their solutions when using splice.

---

## Table of Contents

1. [Installation Issues](#installation-issues)
2. [Database Issues](#database-issues)
3. [Operation Issues](#operation-issues)
4. [Validation Issues](#validation-issues)
5. [Performance Issues](#performance-issues)
6. [Native-V2 Issues](#native-v2-issues)
7. [Recovery Procedures](#recovery-procedures)

---

## Installation Issues

### Error: `command not found: splice`

**Symptom:** Command not found after installation.

**Solutions:**

1. Check if cargo bin is in PATH:
```bash
echo $PATH | grep -o "[^:]*cargo[^:]*"
```

2. Add cargo bin to PATH (add to `~/.bashrc` or `~/.zshrc`):
```bash
export PATH="$HOME/.cargo/bin:$PATH"
```

3. Verify installation:
```bash
cargo install --list | grep splice
```

4. Reinstall if needed:
```bash
cargo install splice --force
```

### Error: `linking with cc failed`

**Symptom:** Compilation fails during installation.

**Solution:** Install build dependencies:

**Debian/Ubuntu:**
```bash
sudo apt install build-essential pkg-config libsqlite3-dev
```

**Fedora:**
```bash
sudo dnf install gcc sqlite-devel
```

**macOS:**
```bash
xcode-select --install
```

**Arch:**
```bash
sudo pacman -S base-devel sqlite
```

### Error: Feature Flags Not Working

**Symptom:** Native-V2 features not available despite flag.

**Solution:**
```bash
# Correct: Use --no-default-features with native-v2
cargo install splice --features native-v2 --no-default-features

# Wrong: Missing --no-default-features
cargo install splice --features native-v2  # SQLite still default

# Verify installation
splice --help | grep -E "(snapshot|batch|verify)"
```

---

## Database Issues

### Error: `database is locked`

**Symptom:** Query fails with "database is locked" error.

**Cause:** Magellan watch process is writing to the database.

**Solution:** Wait a moment and retry. The lock is released quickly after file changes.

**If persistent:**
```bash
# Check if multiple watchers are running
ps aux | grep "magellan watch"

# Kill extra watchers
pkill -f "magellan watch"
```

### Error: `no such table: symbols`

**Symptom:** Query fails with missing table error.

**Cause:** Database was created with an old version of Magellan, or is corrupted.

**Solution:** Re-index the database:
```bash
# Stop the watcher
pkill -f "magellan watch"

# Remove old database
rm .codemcp/codegraph.db*

# Re-index
magellan watch --root ./src --db .codemcp/codegraph.db --scan-initial
```

### Error: `database disk image is malformed`

**Symptom:** SQLite reports database corruption.

**Cause:** Database file was corrupted (crash, disk error, etc.).

**Solution:** Recover or rebuild:
```bash
# Attempt recovery (may save some data)
sqlite3 .codemcp/codegraph.db "PRAGMA integrity_check;"
sqlite3 .codemcp/codegraph.db ".dump" > dump.sql
sqlite3 recovered.db < dump.sql

# Or rebuild entirely (recommended)
rm .codemcp/codegraph.db*
magellan watch --root ./src --db .codemcp/codegraph.db --scan-initial
```

### Error: SPL-E091: Magellan error

**Symptom:** Magelliand operations fail.

**Solution:**
```bash
# Check Magellan installation
magellan --version

# Verify database
magellan status --db .codemcp/codegraph.db

# Re-index if needed
magellan watch --root ./src --db .codemcp/codegraph.db --scan-initial
```

---

## Operation Issues

### Error: SPL-E001: Symbol not found

**Symptom:** Symbol lookup fails.

**Possible Causes:**

1. **Wrong file path** — Verify file path:
```bash
splice find --db .codemcp/codegraph.db --name symbol_name --path correct/path.rs
```

2. **Symbol not indexed** — Check database status:
```bash
magellan status --db .codemcp/codegraph.db
```

3. **Wrong symbol kind** — Try without kind filter:
```bash
# Don't specify kind if uncertain
splice find --db .codemcp/codegraph.db --name symbol_name
```

### Error: SPL-E040: Ambiguous symbol name

**Symptom:** Multiple symbols with the same name exist.

**Solution:**
```bash
# Use full path to disambiguate
splice find --db .codemcp/codegraph.db --name symbol_name --path src/module/

# Or use symbol_id directly
splice find --db .codemcp/codegraph.db --symbol-id <32-char-id>
```

### Error: SPL-E020: File not found

**Symptom:** File path doesn't exist.

**Solution:**
```bash
# Verify file exists
ls -la src/lib.rs

# Use relative or absolute path
splice patch --file ./src/lib.rs --symbol func --with new.rs
```

### Error: SPL-E030: Parse error

**Symptom:** Tree-sitter failed to parse file.

**Possible Causes:**

1. **Syntax error in file** — Fix syntax first
2. **Incomplete file** — Ensure file is complete
3. **Language detection failed** — Specify language:
```bash
splice patch --file src/lib.rs --symbol func --with new.rs --language rust
```

---

## Validation Issues

### Error: Compiler validation failed

**Symptom:** Cargo check or compiler reports errors.

**Solution:**

1. **Review the compiler output** — The error message will show what's wrong
2. **Fix the replacement code** — Update the `--with` file
3. **Run compiler manually** — Check if file compiles:
```bash
cd /path/to/project
cargo check
# Or for other languages:
python -m py_compile file.py
gcc -fsyntax-only file.c
```

### Error: SPL-E050: Validation failed

**Symptom:** Post-operation validation (tree-sitter reparse) failed.

**Causes:**
- Replacement created invalid syntax
- Byte spans were incorrect
- UTF-8 boundary violation

**Solution:**
```bash
# 1. Preview the change first
splice patch --file src/lib.rs --symbol func --with new.rs --preview

# 2. Check replacement file syntax
cat new.rs

# 3. Use --no-validate only if you're certain (not recommended)
splice patch --file src/lib.rs --symbol func --with new.rs --no-validate
```

### Error: UTF-8 boundary violation

**Symptom:** Byte split in middle of multi-byte character.

**Solution:**
```bash
# splice automatically detects and prevents this
# If it occurs, it's a bug — report it

# Workaround: Use rename command instead of manual editing
splice rename --symbol <id> --file src/lib.rs --to new_name
```

---

## Performance Issues

### Operation is Slow

**Symptom:** Operations take more than expected.

**Solutions:**

1. **Use Native-V2 backend:**
```bash
# Re-index with Native-V2
magellan watch --root ./src --db .codemcp/codegraph.db --storage native-v2 --scan-initial

# Rebuild splice with native-v2
cargo install splice --features native-v2 --no-default-features --force
```

2. **Skip validation for non-critical changes:**
```bash
splice apply-files --glob "tests/**/*.rs" --find "TODO" --replace "FIXME" --no-validate
```

3. **Limit graph traversal depth:**
```bash
splice reachable --symbol main --path src/main.rs --max-depth 3
```

4. **Use preview mode to avoid writing:**
```bash
splice rename --symbol <id> --file src/lib.rs --to new_name --preview
```

### Database File is Large

**Symptom:** Database file grows beyond expected size.

**Cause:** SQLite backend creates larger databases.

**Comparison:**
| Symbols | SQLite Size | Native-V2 Size |
|---------|-------------|----------------|
| 10,000 | ~5MB | ~2MB |
| 100,000 | ~50MB | ~15MB |

**Solution:** Consider migrating to Native-V2:
```bash
# See NATIVE-V2-MIGRATION.md for detailed instructions
magellan watch --root ./src --db .codemcp/codegraph.db --storage native-v2 --scan-initial
```

---

## Native-V2 Issues

### Error: Native-V2 feature not available

**Symptom:** Native-V2 exclusive commands fail.

**Cause:** splice was built without native-v2 feature.

**Solution:**
```bash
# Rebuild with native-v2 feature
cargo install splice --features native-v2 --no-default-features --force

# Verify
splice --help | grep -E "(snapshot|batch|verify)"
```

### Error: Database format mismatch

**Symptom:** "Database format not recognized" error.

**Cause:** splice built without native-v2 trying to open native-v2 database, or vice versa.

**Solution:**
```bash
# Check backend
splice status --db .codemcp/codegraph.db --detect-backend

# If native-v2, rebuild with native-v2 feature
cargo install splice --features native-v2 --no-default-features --force
```

### Migration Failed

**Symptom:** `splice migrate` fails.

**Solutions:**

1. **Verify source database is SQLite:**
```bash
sqlite3 .codemcp/codegraph.db "PRAGMA integrity_check;"
```

2. **Check available disk space:**
```bash
df -h .
```

3. **Run with progress:**
```bash
splice migrate --source .codemcp/codegraph.db --dest .codemcp/codegraph-native.db --progress
```

---

## Recovery Procedures

### Restore from Backup

```bash
# 1. Find backup manifest
ls -la .splice-backup/

# 2. Restore from manifest
splice undo --manifest .splice-backup/<operation-id>/manifest.json

# 3. Verify checksums
# (automatic during restore)
```

### Restore from Snapshot (Native-V2)

```bash
# 1. List snapshots
splice snapshots list

# 2. Delete current database (careful!)
# mv .codemcp/codegraph.db .codemcp/codegraph.db.broken

# 3. Restore from snapshot
# (native-v2 restore capability - see NATIVE-V2-FEATURES.md)
```

### Partial Recovery

If only some files were affected:

```bash
# 1. List files in backup
cat .splice-backup/<operation-id>/manifest.json | jq '.files'

# 2. Restore individual files
cp .splice-backup/<operation-id>/src/lib.rs.backup src/lib.rs
```

---

## Getting Help

If issues persist:

1. **Enable verbose output:**
```bash
RUST_LOG=debug splice <command>
```

2. **Check database integrity:**
```bash
sqlite3 .codemcp/codegraph.db "PRAGMA integrity_check;"
```

3. **Report issues:** Include database size, splice version, and Magellan version.

4. **Check existing documentation:**
- [README.md](../README.md) - Quick start
- [MANUAL.md](../MANUAL.md) - Command reference
- [PERFORMANCE.md](PERFORMANCE.md) - Performance optimization
- [BEST_PRACTICES.md](BEST_PRACTICES.md) - Recommended workflows

---

## Error Code Reference

| Code | Description | Solution |
|------|-------------|----------|
| SPL-E001 | Symbol not found | Check name, path, or use symbol-id |
| SPL-E002 | Multiple symbols found | Add --path filter or use symbol-id |
| SPL-E010 | File not found | Verify file path |
| SPL-E020 | Parse error | Check file syntax |
| SPL-E030 | Validation failed | Fix replacement code |
| SPL-E040 | Ambiguous symbol | Use more specific path or symbol-id |
| SPL-E050 | UTF-8 boundary error | Report bug |
| SPL-E060 | Backup creation failed | Check disk space |
| SPL-E070 | Backup restoration failed | Verify backup manifest |
| SPL-E080 | Snapshot failed | Check available disk space |
| SPL-E090 | Native-V2 required | Rebuild with --features native-v2 |
| SPL-E091 | Magellan error | Check Magellan installation |
| SPL-E100 | Batch operation failed | Check --continue-on-error |
| SPL-E110 | Proof validation failed | Verify proof file format |

---

## Further Reading

- [README.md](../README.md) - Quick start guide
- [MANUAL.md](../MANUAL.md) - Complete command reference
- [PERFORMANCE.md](PERFORMANCE.md) - Performance optimization
- [BEST_PRACTICES.md](BEST_PRACTICES.md) - Recommended workflows

---

*Created: 2026-02-10*
