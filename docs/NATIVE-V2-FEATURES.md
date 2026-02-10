# Native-V2 Features

**Last Updated:** 2026-02-10

splice offers advanced features when built with the native-v2 backend. These features leverage native-v2's snapshot system and clustered edge storage.

---

## Building with Native-V2

```bash
cargo install splice --features native-v2

# Or build from source
cargo build --release --features native-v2 --no-default-features
```

---

## Feature Overview

### 1. Snapshot Capture (`--snapshot-before`)

Capture graph state before refactoring for verification and rollback.

**Usage:**

```bash
splice rename --symbol "old_name" --to "new_name" \
       --file src/lib.rs --db .codemcp/codegraph.db \
       --snapshot-before

# Output: Snapshot captured: .splice/snapshots/snapshot-rename-1234567890.json
```

**What Gets Captured:**
- All symbols (functions, classes, variables)
- All edges (call relationships, imports)
- Invariant validation results
- Timestamp and operation metadata

**Storage:** Snapshots stored in `.splice/snapshots/` with timestamp filenames

---

### 2. Snapshot Comparison (`verify`)

Compare two snapshots to detect changes.

**Usage:**

```bash
# Human-readable output
splice verify --before snapshot-before.json \
              --after snapshot-after.json \
              --detailed

# JSON for programmatic use
splice verify --before before.json --after after.json --output json
```

**What Gets Compared:**
- Symbol additions/removals/modifications
- Edge changes (new/removed call relationships)
- Invariant violations

**Exit Codes:**
- `0`: Identical (no changes)
- `1`: Differences detected
- `2`: Error

---

### 3. Impact Visualization (`--impact-graph`)

Generate DOT graph showing refactoring impact.

**Usage:**

```bash
# Show what's affected by renaming a function
splice rename --symbol "process" --to "handle" \
       --file src/lib.rs --preview --impact-graph \
       --db .codemcp/codegraph.db > impact.dot

# Render to PNG
dot -Tpng impact.dot -o impact.png
```

**What Gets Visualized:**
- Affected symbols (boxes)
- Call relationships (arrows)
- Symbol kinds (colors/shapes)

**Works With:**
- `splice reachable --impact-graph`
- `splice refs --impact-graph`
- `splice rename --preview --impact-graph`

---

### 4. Batch Operations (`batch`)

Execute multi-file refactors with automatic rollback.

**YAML Specification:**

```yaml
operations:
  - type: rename
    symbol: "old_function"
    file: "src/lib.rs"
    to: "new_function"

  - type: patch
    symbol: "process_data"
    file: "src/process.rs"
    with: "replacements/new_process.rs"

  - type: delete
    symbol: "deprecated_util"
    file: "src/utils.rs"
```

**Usage:**

```bash
# Preview batch changes
splice batch --spec refactor.yaml --dry-run

# Execute with rollback on failure
splice batch --spec refactor.yaml --rollback on-failure --db .codemcp/codegraph.db

# Continue despite errors
splice batch --spec refactor.yaml --continue-on-error
```

**Rollback Behavior:**
- `--rollback on-failure`: Automatic snapshot before batch, restore on any failure
- `--rollback never`: No rollback (default)
- `--rollback always`: Always rollback after execution (testing mode)

---

### 5. Snapshot Management (`snapshots`)

List, delete, and clean up snapshots.

**Commands:**

```bash
# List all snapshots
splice snapshots list

# List by operation type
splice snapshots list --operation rename

# Show disk usage
splice snapshots list --disk-use

# Delete specific snapshot
splice snapshots delete snapshot-1234567890

# Delete with confirmation skip
splice snapshots delete snapshot-1234567890 --force

# Clean up old snapshots (keep 10 most recent)
splice snapshots cleanup --keep 10

# Preview cleanup
splice snapshots cleanup --keep 5 --dry-run
```

---

## Complete Workflow Example

```bash
# 1. Build with native-v2
cargo build --release --features native-v2 --no-default-features

# 2. Create snapshot before refactor
splice rename --symbol "foo" --to "bar" \
       --file src/lib.rs --db .codemcp/codegraph.db \
       --snapshot-before

# 3. Generate impact graph
splice rename --symbol "foo" --to "bar" \
       --file src/lib.rs --preview --impact-graph \
       --db .codemcp/codegraph.db > before.dot

# 4. Apply rename
splice rename --symbol "foo" --to "bar" \
       --file src/lib.rs --db .codemcp/codegraph.db

# 5. Capture after state
splice snapshots list --operation rename

# 6. Compare snapshots
splice verify --before snapshot-rename-before.json \
              --after snapshot-rename-after.json \
              --detailed
```

---

## See Also

- [Command Reference](../MANUAL.md) — Complete CLI documentation
- [Backend Selection](../README.md#which-backend-should-i-use) — Choosing the right backend
- [Migration Guide](NATIVE-V2-MIGRATION.md) — Migrating from SQLite to native-v2
