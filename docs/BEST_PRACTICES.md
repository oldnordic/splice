# Best Practices

**Last Updated:** 2026-02-10
**Version:** v2.5.0

Recommended workflows and patterns for using splice effectively.

---

## Table of Contents

1. [LLM Integration](#llm-integration)
2. [Command Patterns](#command-patterns)
3. [Refactoring Workflows](#refactoring-workflows)
4. [Safety Practices](#safety-practices)
5. [Anti-Patterns](#anti-patterns)

---

## LLM Integration

splice is designed specifically for LLM-consumed refactoring. Follow these patterns:

### 1. Always Preview First

```bash
# Good: Preview before applying
splice rename --symbol <id> --file src/lib.rs --to new_name --preview --json

# Bad: Apply without seeing impact
splice rename --symbol <id> --file src/lib.rs --to new_name
```

### 2. Use Impact Analysis

```bash
# Good: Check impact before refactoring
splice reachable --symbol function_name --path src/lib.rs --max-depth 3 --output json

# Good: Find what will be affected
splice refs --db .codemcp/codegraph.db --name function_name --path src/lib.rs --direction in
```

### 3. Generate Proofs for Audit Trail

```bash
# Good: Create proof for verification
splice rename --symbol <id> --file src/lib.rs --to new_name --proof

# Later: Verify the proof
splice validate-proof --proof .splice/proofs/rename-{timestamp}.json
```

### 4. Use Snapshots for Rollback (Native-V2)

```bash
# Good: Capture snapshot before major changes
splice patch --file src/lib.rs --symbol process --with new.rs \
  --snapshot-before --db .codemcp/codegraph.db

# If needed: Restore from snapshot
# (native-v2 restore capability)
```

---

## Command Patterns

### Pattern: Safe Rename Workflow

```bash
# Step 1: Find the symbol
splice find --db .codemcp/codegraph.db --name function_name --path src/lib.rs

# Step 2: Check references
splice refs --db .codemcp/codegraph.db --name function_name --path src/lib.rs --direction in

# Step 3: Analyze impact
splice reachable --symbol function_name --path src/lib.rs --max-depth 3 --output json

# Step 4: Preview rename
splice rename --symbol <id> --file src/lib.rs --to new_name --preview --json

# Step 5: Apply with proof
splice rename --symbol <id> --file src/lib.rs --to new_name --proof
```

### Pattern: Batch Similar Changes

```bash
# Create batch specification
cat > refactor.yaml << 'EOF'
operations:
  - type: rename
    symbol: old_function_1
    file: src/lib.rs
    to: new_function_1
  - type: rename
    symbol: old_function_2
    file: src/lib.rs
    to: new_function_2
  - type: patch
    symbol: process_data
    file: src/process.rs
    with: replacements/new_process.rs
EOF

# Execute with rollback on failure
splice batch --spec refactor.yaml --db .codemcp/codegraph.db --rollback on-failure
```

### Pattern: Dead Code Cleanup

```bash
# Step 1: Find dead code
splice dead-code --entry main --path src/main.rs --exclude-public --output json

# Step 2: Review candidates (manual review recommended)
# Step 3: Delete with backup
splice delete --file src/unused.rs --symbol unused_function --create-backup
```

### Pattern: Cycle Detection and Resolution

```bash
# Step 1: Find cycles
splice cycles --show-members --max-cycles 20 --output json

# Step 2: Analyze specific cycle
splice cycles --symbol problematic_function --path src/lib.rs --show-members

# Step 3: Break cycle (manual intervention required)
# Use splice patch to refactor one of the functions in the cycle
```

---

## Refactoring Workflows

### Workflow: Extract Function

```bash
# 1. Find the function to extract from
splice find --db .codemcp/codegraph.db --name large_function --path src/lib.rs

# 2. Check what calls it
splice refs --db .codemcp/codegraph.db --name large_function --path src/lib.rs --direction in

# 3. Create new extracted function
cat > extracted.rs << 'EOF'
pub fn extracted_logic(data: &str) -> Result<String> {
    // New implementation
}
EOF

# 4. Patch original to call extracted
cat > updated.rs << 'EOF'
pub fn large_function(data: &str) -> Result<String> {
    extracted_logic(data)
}
EOF

# 5. Apply changes
splice patch --file src/extracted.rs --symbol extracted_logic --with extracted.rs
splice patch --file src/lib.rs --symbol large_function --with updated.rs
```

### Workflow: Safe Library Rename

```bash
# 1. Impact analysis
splice reachable --symbol old_name --path src/lib.rs --max-depth 5 --output json

# 2. Count references
splice refs --db .codemcp/codegraph.db --name old_name --path src/lib.rs --direction in --output json

# 3. Preview with impact graph
splice rename --symbol <id> --file src/lib.rs --to new_name --preview --impact-graph

# 4. Apply with proof and backup
splice rename --symbol <id> --file src/lib.rs --to new_name --proof --create-backup

# 5. Verify proof
splice validate-proof --proof .splice/proofs/rename-*.json
```

### Workflow: Module Refactoring

```bash
# 1. Find all symbols in module
llmgrep --db .codemcp/codegraph.db search --path "src/old_module/" --output json

# 2. Detect cycles within module
splice cycles --show-members --output json | grep "src/old_module"

# 3. Find external dependencies
splice refs --db .codemcp/codegraph.db --path "src/old_module/" --direction out

# 4. Plan refactoring (manual)
# 5. Execute with batch operations
splice batch --spec module_refactor.yaml --db .codemcp/codegraph.db

# 6. Verify with snapshots
splice verify --before .splice/snapshots/before.json \
              --after .splice/snapshots/after.json --detailed
```

---

## Safety Practices

### 1. Always Use Backups

```bash
# For critical operations, always create backup
splice patch --file src/lib.rs --symbol critical_func \
  --with new.rs --create-backup

# Restore if needed
splice undo --manifest .splice-backup/<operation-id>/manifest.json
```

### 2. Verify with Compiler

```bash
# Don't skip validation unless you're certain
splice patch --file src/lib.rs --symbol func --with new.rs
# Validation runs automatically via cargo check

# Only skip for non-critical changes
splice apply-files --glob "tests/**/*.rs" --find "TODO" --replace "FIXME" --no-validate
```

### 3. Use Incremental Refactoring

```bash
# Bad: Rename everything at once
splice rename --symbol old1 --file src/lib.rs --to new1
splice rename --symbol old2 --file src/lib.rs --to new2
splice rename --symbol old3 --file src/lib.rs --to new3

# Good: Test after each change
splice rename --symbol old1 --file src/lib.rs --to new1 --proof
# Run tests, verify
splice rename --symbol old2 --file src/lib.rs --to new2 --proof
# Run tests, verify
```

### 4. Leverage Graph Analysis

```bash
# Before refactoring, understand dependencies
splice reachable --symbol target --path src/lib.rs --max-depth 5 --output json

# Check for circular dependencies
splice cycles --symbol target --path src/lib.rs --show-members

# Find dead code to remove first
splice dead-code --entry main --path src/main.rs --exclude-public
```

### 5. Use Snapshots for Major Changes

```bash
# Before major refactoring, capture snapshot
splice status --db .codemcp/codegraph.db > before_state.txt

# Or use native-v2 snapshot
splice patch --file src/lib.rs --symbol func --with new.rs \
  --snapshot-before --db .codemcp/codegraph.db

# After refactoring, compare
splice verify --before .splice/snapshots/before-*.json \
              --after .splice/snapshots/after-*.json
```

---

## Anti-Patterns

### Don't: Skip Preview for Complex Changes

```bash
# Wrong: Apply complex rename without preview
splice rename --symbol <id> --file src/lib.rs --to new_name

# Right: Preview first to understand impact
splice rename --symbol <id> --file src/lib.rs --to new_name --preview --json
```

### Don't: Ignore Validation Errors

```bash
# Wrong: Use --no-validate to bypass errors
splice patch --file src/lib.rs --symbol func --with new.rs --no-validate

# Right: Fix validation errors before applying
# Review the compiler output and fix the replacement code
```

### Don't: Rename Without Checking References

```bash
# Wrong: Rename without understanding impact
splice rename --symbol <id> --file src/lib.rs --to new_name

# Right: Check references first
splice refs --db .codemcp/codegraph.db --name func --path src/lib.rs --direction in
```

### Don't: Delete Without Dead Code Analysis

```bash
# Wrong: Assume function is unused
splice delete --file src/lib.rs --symbol maybe_unused

# Right: Verify it's dead code first
splice dead-code --entry main --path src/main.rs --output json | grep "maybe_unused"
```

### Don't: Use Splice for Text Replacement

```bash
# Wrong: Use splice for simple text replacement
splice apply-files --glob "src/**/*.rs" --find "TODO" --replace "FIXME"

# Right: Use standard tools for text replacement
sed -i 's/TODO/FIXME/g' src/**/*.rs

# Use splice only when AST validation is needed
```

---

## Quick Reference Card

| Task | Command |
|------|---------|
| Preview change | `--preview --json` |
| Find references | `refs --direction in` |
| Impact analysis | `reachable --max-depth N` |
| Find dead code | `dead-code --entry main` |
| Find cycles | `cycles --show-members` |
| Backup | `--create-backup` |
| Audit trail | `--proof` |
| Snapshot | `--snapshot-before` |
| Verify snapshot | `verify --before X --after Y` |

---

## Further Reading

- [README.md](../README.md) - Quick start guide
- [MANUAL.md](../MANUAL.md) - Complete command reference
- [PERFORMANCE.md](PERFORMANCE.md) - Benchmarks and optimization
- [TROUBLESHOOTING.md](TROUBLESHOOTING.md) - Common issues

---

*Created: 2026-02-10*
