# Proof-Based Refactoring Examples

This document demonstrates how to use Splice's proof-based refactoring system to generate machine-checkable behavioral equivalence proofs for refactoring operations.

## What is a Proof?

A refactoring proof is a machine-checkable record that a refactoring operation preserved structural invariants. It contains:

- **Before/after snapshots** of the code graph
- **Invariant checks** (reference counts, orphan detection, etc.)
- **SHA-256 checksums** for audit trail integrity

## Example 1: Rename with Proof

### Generate Proof

```bash
splice rename \
  --symbol 1a2b3c4d5e6f7g8h \
  --file src/utils.rs \
  --to new_name \
  --proof
```

### Output

```
Proof generated: .splice/proofs/rename-1736035200.json

Summary:
- Operation: rename
- Symbol: helper_function -> new_name
- Files modified: 3
- Invariants: PASSED
- Checksum: a1b2c3d4e5f6...
```

### Proof Content

```json
{
  "metadata": {
    "operation": "rename",
    "timestamp": 1736035200,
    "splice_version": "2.4.0",
    "git_commit": "abc123..."
  },
  "before": {
    "timestamp": 1736035197,
    "symbols": {
      "1a2b3c4d5e6f7g8h": {
        "id": "1a2b3c4d5e6f7g8h",
        "name": "helper_function",
        "file_path": "src/utils.rs",
        "kind": "function",
        "fan_in": 2,
        "fan_out": 1
      }
    },
    "edges": {
      "1a2b3c4d5e6f7g8h": ["9i0j1k2l3m4n5o6p"]
    }
  },
  "after": {
    "timestamp": 1736035200,
    "symbols": {
      "1a2b3c4d5e6f7g8h": {
        "id": "1a2b3c4d5e6f7g8h",
        "name": "new_name",
        "file_path": "src/utils.rs",
        "kind": "function",
        "fan_in": 2,
        "fan_out": 1
      }
    },
    "edges": {
      "1a2b3c4d5e6f7g8h": ["9i0j1k2l3m4n5o6p"]
    }
  },
  "invariants": [
    {
      "invariant_name": "reference_count_preservation",
      "passed": true,
      "violations": []
    },
    {
      "invariant_name": "no_orphaned_symbols",
      "passed": true,
      "violations": []
    },
    {
      "invariant_name": "symbol_id_stability",
      "passed": true,
      "violations": []
    },
    {
      "invariant_name": "entry_point_preservation",
      "passed": true,
      "violations": []
    }
  ],
  "checksums": {
    "before_hash": "sha256:a1b2c3d4e5f6...",
    "after_hash": "sha256:9f8e7d6c5b4a...",
    "proof_hash": "sha256:3210987654321..."
  }
}
```

## Example 2: Validating a Proof

```bash
splice validate-proof --proof .splice/proofs/rename-1736035200.json
```

### Successful Validation

```
Proof Validation: .splice/proofs/rename-1736035200.json

Status: VALID

All SHA-256 checksums verified:
  Before snapshot hash: a1b2c3d4e5f6...
  After snapshot hash: 9f8e7d6c5b4a...
  Overall proof hash: 3210987654321...

Invariant checks:
  reference_count_preservation: PASSED
  no_orphaned_symbols: PASSED
  symbol_id_stability: PASSED
  entry_point_preservation: PASSED

Audit trail integrity is confirmed.
```

### Invalid Proof Example

```bash
splice validate-proof --proof .splice/proofs/rename-tampered.json
```

### Output

```
Proof Validation: .splice/proofs/rename-tampered.json

Status: INVALID

Checksum verification failed:
  Before snapshot hash: MISMATCH
  Expected: a1b2c3d4e5f6...
  Actual: z9y8x7w6v5u4...

Invariant checks:
  reference_count_preservation: PASSED
  no_orphaned_symbols: FAILED
    Violations:
    - orphaned_symbol (src/unused.rs) has no incoming references
  symbol_id_stability: PASSED

1 invariant violation found.
```

## Example 3: CI/CD Integration

### GitHub Actions Workflow

```yaml
name: Refactoring Validation

on:
  pull_request:
    paths:
      - 'src/**/*.rs'
      - '.splice/proofs/*.json'

jobs:
  validate-proofs:
    runs-on: ubuntu-latest
    steps:
      - name: Checkout code
        uses: actions/checkout@v3

      - name: Install Splice
        run: cargo install splice

      - name: Validate refactoring proofs
        run: |
          for proof in .splice/proofs/*.json; do
            echo "Validating $proof"
            splice validate-proof --proof "$proof" || exit 1
          done

      - name: Comment PR with results
        if: always()
        uses: actions/github-script@v6
        with:
          script: |
            const proofCount = $(ls .splice/proofs/*.json 2>/dev/null | wc -l);
            github.rest.issues.createComment({
              issue_number: context.issue.number,
              owner: context.repo.owner,
              repo: context.repo.repo,
              body: `Validated ${proofCount} refactoring proofs.`
            });
```

### GitLab CI Pipeline

```yaml
validate-proofs:
  stage: test
  image: rust:latest
  script:
    - cargo install splice
    - |
      for proof in .splice/proofs/*.json; do
        echo "Validating $proof"
        splice validate-proof --proof "$proof" || exit 1
      done
  artifacts:
    paths:
      - .splice/proofs/*.json
    reports:
      junit: proof-validation.xml
```

## Example 4: Audit Trail

### List All Proofs

```bash
ls -la .splice/proofs/
# -rw-r--r-- 1 user user 2.3K Jan 15 10:30 rename-1736035200.json
# -rw-r--r-- 1 user user 2.1K Jan 15 11:45 rename-1736039345.json
```

### Search Proofs by Symbol

```bash
# Search for proofs mentioning a symbol
grep -r "helper_function" .splice/proofs/
# .splice/proofs/rename-1736035200.json:      "name": "helper_function",
```

### Verify Proof Chain

```bash
# Ensure all proofs in a chain are valid
for proof in .splice/proofs/*.json; do
  echo "Validating $proof"
  splice validate-proof --proof "$proof" || exit 1
done
```

### Export Proof Summary

```bash
# Get summary of all proofs
for proof in .splice/proofs/*.json; do
  echo "=== $proof ==="
  jq -r '.metadata | "Operation: \(.operation)\nTimestamp: \(.timestamp)"' "$proof"
  jq -r '.invariants[] | "\(.invariant_name): \(.passed)"' "$proof"
  echo
done
```

## Example 5: Programmatic Usage

### Rust API Usage

```rust
use splice::proof::{
    generate_proof, generate_snapshot,
    validate_proof_file, write_proof, RefactoringProof
};
use std::path::Path;

fn main() -> splice::error::Result<()> {
    let db_path = Path::new(".codemcp/codegraph.db");

    // Capture before state
    let before = generate_snapshot(db_path)?;
    println!("Captured before state: {} symbols", before.symbols.len());

    // Perform refactoring...
    println!("Performing refactoring...");

    // Capture after state
    let after = generate_snapshot(db_path)?;
    println!("Captured after state: {} symbols", after.symbols.len());

    // Generate proof with checksums
    let proof = generate_proof("rename", db_path, before, after)?;

    // Validate invariants
    for check in &proof.invariants {
        if !check.passed {
            eprintln!("Invariant violation: {}", check.invariant_name);
            for violation in &check.violations {
                eprintln!("  - {}: {}", violation.subject, violation.message);
            }
        }
    }

    // Write proof to file
    let output_dir = Path::new(".splice/proofs/");
    let proof_path = write_proof(&proof, output_dir)?;
    println!("Proof written to: {}", proof_path.display());

    // Later, validate the proof file
    let is_valid = validate_proof_file(&proof_path)?;
    assert!(is_valid, "Proof checksums are invalid!");

    Ok(())
}
```

### Python Script for Validation

```python
import json
import subprocess
import hashlib
from pathlib import Path

def validate_proof(proof_path: Path) -> bool:
    """Validate a Splice proof file."""
    result = subprocess.run(
        ["splice", "validate-proof", "--proof", str(proof_path)],
        capture_output=True,
        text=True
    )
    return result.returncode == 0

def get_proof_summary(proof_path: Path) -> dict:
    """Extract summary from proof file."""
    with open(proof_path) as f:
        proof = json.load(f)

    return {
        "operation": proof["metadata"]["operation"],
        "timestamp": proof["metadata"]["timestamp"],
        "invariants_passed": sum(
            1 for inv in proof["invariants"] if inv["passed"]
        ),
        "total_invariants": len(pro["invariants"])
    }

# Validate all proofs
proofs_dir = Path(".splice/proofs")
for proof_file in proofs_dir.glob("*.json"):
    if validate_proof(proof_file):
        summary = get_proof_summary(proof_file)
        print(f"✓ {proof_file.name}: {summary}")
    else:
        print(f"✗ {proof_file.name}: INVALID")
```

## Use Cases

### Critical Refactorings

Generate proofs for sensitive changes:

```bash
# API rename with proof
splice rename \
  --symbol api_endpoint \
  --file src/api.rs \
  --to new_api_endpoint \
  --proof

# Database schema migration rename
splice rename \
  --symbol user_table \
  --file src/schema.rs \
  --to user_v2_table \
  --proof
```

### Compliance and Auditing

Proof of correct refactoring in regulated industries:

```bash
# Generate proof for regulated code
splice rename \
  --symbol transaction_processor \
  --file src/financial.rs \
  --to payment_processor \
  --proof

# Store proof with version control
cp .splice/proofs/rename-*.json docs/audits/
git add docs/audits/
```

### Reproducibility

Share proofs with team for review:

```bash
# Archive proof for code review
tar -czf refactor-proof-$(date +%Y%m%d).tar.gz .splice/proofs/*.json

# Upload to code review system
# (implementation depends on your review tool)
```

### Regression Prevention

Store proofs in version control and verify in CI:

```bash
# Add proofs to repository
git add .splice/proofs/*.json
git commit -m "Add refactoring proofs for v2.4.0"

# Verify in CI
make validate-proofs  # runs splice validate-proof for all
```

## Example 6: Automated Proof Generation

### Pre-commit Hook

```bash
#!/bin/bash
# .git/hooks/pre-commit

# Generate proof for any renames in this commit
if git diff --cached --name-only | grep -q "src/"; then
  echo "Checking for rename operations..."

  # Detect if any rename commands were used
  if git diff --cached | grep -q "splice rename"; then
    echo "Generating proof for rename..."
    splice rename \
      --symbol <id> \
      --file <path> \
      --to <new_name> \
      --proof
  fi
fi
```

### Post-refactor Script

```bash
#!/bin/bash
# scripts/generate-proof.sh

# Automatically generate proof after refactoring
SYMBOL_NAME=$1
FILE_PATH=$2
NEW_NAME=$3

# Find symbol ID
SYMBOL_ID=$(splice find --name "$SYMBOL_NAME" --path "$FILE_PATH" \
  --output json | jq -r '.symbols[0].id')

# Perform rename with proof
splice rename \
  --symbol "$SYMBOL_ID" \
  --file "$FILE_PATH" \
  --to "$NEW_NAME" \
  --proof

echo "Proof generated: .splice/proofs/rename-$(date +%s).json"
```

## Example 7: Proof Analysis

### Compare Before/After Snapshots

```bash
# Extract before and after symbols
jq '.before.symbols' .splice/proofs/rename-1736035200.json > before.json
jq '.after.symbols' .splice/proofs/rename-1736035200.json > after.json

# Compare differences
diff before.json after.json
```

### Check Invariant History

```bash
# Track invariant status over time
for proof in .splice/proofs/*.json; do
  timestamp=$(jq -r '.metadata.timestamp' "$proof")
  passed=$(jq '[.invariants[] | .passed] | add' "$proof")
  total=$(jq '.invariants | length' "$proof")
  echo "$timestamp: $passed/$total invariants passed"
done
```

### Verify Checksum Chain

```bash
# Verify that proof checksums are consistent
for proof in .splice/proofs/*.json; do
  before_hash=$(jq -r '.checksums.before_hash' "$proof" | cut -d: -f2)
  after_hash=$(jq -r '.checksums.after_hash' "$proof" | cut -d: -f2)
  proof_hash=$(jq -r '.checksums.proof_hash' "$proof" | cut -d: -f2)

  echo "=== $proof ==="
  echo "Before: $before_hash"
  echo "After:  $after_hash"
  echo "Proof:  $proof_hash"
  echo
done
```

## Best Practices

1. **Always generate proofs for critical refactorings** - API changes, database migrations, security-sensitive operations

2. **Store proofs in version control** - Track proof history alongside code changes

3. **Validate proofs in CI/CD** - Automated proof validation catches issues early

4. **Review invariant violations** - Failed invariants indicate incomplete or incorrect refactoring

5. **Keep proof files organized** - Use consistent naming and directory structure

6. **Document proof context** - Include proof files in code review comments

## Troubleshooting

### Proof validation fails

```bash
# Check which invariants failed
splice validate-proof --proof .splice/proofs/rename-xxx.json --output json | \
  jq '.invariants[] | select(.passed == false)'
```

### Checksum mismatch

```bash
# Verify file integrity
sha256sum .splice/proofs/rename-xxx.json

# Re-generate proof if needed
splice rename --symbol <id> --file <path> --to <name> --proof
```

### Missing proof directory

```bash
# Create proof directory if it doesn't exist
mkdir -p .splice/proofs

# Re-run rename with proof generation
splice rename --symbol <id> --file <path> --to <name> --proof
```

## See Also

- [docs/manual.md](../manual.md) - Complete CLI reference
- [docs/examples/rename_examples.md](rename_examples.md) - Cross-file rename examples
- [docs/examples/graph_algorithm_examples.md](graph_algorithm_examples.md) - Graph algorithm examples
