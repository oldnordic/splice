# Cross-File Rename Examples

This document provides comprehensive examples for using Splice's cross-file rename functionality across different programming languages and scenarios.

## Example 1: Simple Function Rename (Rust)

### Setup

Create test project:

```bash
mkdir rename_example && cd rename_example
cargo init --lib
```

**src/lib.rs:**
```rust
pub mod utils;

pub fn main() {
    utils::helper_function();
}
```

**src/utils.rs:**
```rust
pub fn helper_function() {
    println!("Helper called");
}

pub fn caller() {
    helper_function();
}
```

### Ingest and Rename

```bash
# Ingest codebase
splice ingest --root ./src --db .codemcp/codegraph.db

# Find symbol
splice find --name "helper_function" --path "src/utils.rs"

# Output:
# Found symbol "helper_function" in src/utils.rs
# ID: 1a2b3c4d5e6f7g8h
# Kind: function
# Span: (0, 24)

# Preview rename
splice rename \
  --symbol 1a2b3c4d5e6f7g8h \
  --file src/utils.rs \
  --to new_helper_name \
  --preview

# Output:
# Will rename "helper_function" to "new_helper_name"
# Found 2 references:
#   src/lib.rs:3:10
#   src/utils.rs:7:5

# Perform rename
splice rename \
  --symbol 1a2b3c4d5e6f7g8h \
  --file src/utils.rs \
  --to new_helper_name
```

### Result

**src/lib.rs:**
```rust
pub mod utils;

pub fn main() {
    utils::new_helper_name();
}
```

**src/utils.rs:**
```rust
pub fn new_helper_name() {
    println!("Helper called");
}

pub fn caller() {
    new_helper_name();
}
```

## Example 2: Cross-Language Rename (Python)

### Setup

**main.py:**
```python
from utils import process_data

def main():
    result = process_data()
    return result
```

**utils.py:**
```python
def process_data():
    return "processed"

def another_function():
    data = process_data()
    return data
```

### Rename

```bash
# Ingest the project
splice ingest --root . --db .codemcp/codegraph.db

# Find the symbol
splice find --name "process_data" --path "utils.py"

# Rename across all Python files
splice rename \
  --symbol <id> \
  --file utils.py \
  --to transform_data
```

### Result

Both `main.py` and `utils.py` will have `process_data` replaced with `transform_data` at all reference locations.

## Example 3: Rename with Proof

Generate a machine-checkable proof during rename for audit trail:

```bash
# Generate proof during rename
splice rename \
  --symbol <id> \
  --file src/utils.rs \
  --to new_name \
  --proof

# Proof written to .splice/proofs/rename-<timestamp>.json

# Validate proof later
splice validate-proof --proof .splice/proofs/rename-<timestamp>.json
```

### Proof Output

```json
{
  "metadata": {
    "operation": "rename",
    "timestamp": 1736035200,
    "splice_version": "2.3.0",
    "git_commit": "abc123..."
  },
  "before": {
    "symbols": {
      "1a2b3c4d5e6f7g8h": {
        "name": "helper_function",
        "file_path": "src/utils.rs",
        "kind": "function",
        "fan_in": 2,
        "fan_out": 1
      }
    }
  },
  "after": {
    "symbols": {
      "1a2b3c4d5e6f7g8h": {
        "name": "new_name",
        "file_path": "src/utils.rs",
        "kind": "function",
        "fan_in": 2,
        "fan_out": 1
      }
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
    }
  ],
  "checksums": {
    "before_hash": "sha256:...",
    "after_hash": "sha256:...",
    "proof_hash": "sha256:..."
  }
}
```

## Example 4: Handling Ambiguity

When multiple symbols with the same name exist across different files:

### Scenario

You have `helper_function` in both `src/utils.rs` and `src/other.rs`.

```bash
splice rename \
  --symbol helper_function \
  --file src/utils.rs \
  --to new_helper_name
```

### Error Output

```
Error: Multiple symbols found
Please disambiguate:
  src/utils.rs:helper_function:function
  src/other.rs:helper_function:function

Use symbol ID instead:
splice rename \
  --symbol 1a2b3c4d5e6f7g8h \
  --file src/utils.rs \
  --to new_helper_name
```

### Solution

```bash
# Use the exact symbol ID from find command
splice find --name "helper_function" --path "src/utils.rs"

# Then rename with the ID
splice rename \
  --symbol 1a2b3c4d5e6f7g8h \
  --file src/utils.rs \
  --to new_helper_name
```

## Example 5: Batch Renames with Preview

Before performing multiple renames, use preview mode to verify changes:

```bash
# Preview all renames first
splice rename \
  --symbol func_a \
  --file src/lib.rs \
  --to function_a \
  --preview

# Review the diff output
# If satisfied, apply without --preview flag
splice rename \
  --symbol func_a \
  --file src/lib.rs \
  --to function_a
```

## Example 6: Rename in TypeScript Projects

### Setup

**src/utils.ts:**
```typescript
export function calculateTotal(price: number, tax: number): number {
    return price + tax;
}

export function displayTotal(total: number): void {
    console.log(`Total: ${total}`);
}
```

**src/main.ts:**
```typescript
import { calculateTotal, displayTotal } from './utils';

const total = calculateTotal(100, 10);
displayTotal(total);
```

### Rename

```bash
# Ingest TypeScript files
splice ingest --root ./src --db .codemcp/codegraph.db

# Find and rename
splice find --name "calculateTotal" --path "src/utils.ts"
splice rename \
  --symbol <id> \
  --file src/utils.ts \
  --to computeTotal
```

### Result

Both the export in `utils.ts` and the import in `main.ts` are updated.

## Example 7: Rename Class Names (Java)

### Setup

**Calculator.java:**
```java
public class Calculator {
    public int add(int a, int b) {
        return a + b;
    }
}

**Main.java:**
```java
public class Main {
    public static void main(String[] args) {
        Calculator calc = new Calculator();
        System.out.println(calc.add(5, 3));
    }
}
```

### Rename

```bash
splice ingest --root . --db .codemcp/codegraph.db
splice find --name "Calculator" --path "Calculator.java"
splice rename \
  --symbol <id> \
  --file Calculator.java \
  --to MathCalculator
```

## Example 8: CI/CD Integration

### GitHub Actions Workflow

```yaml
name: Rename Validation

on: [pull_request]

jobs:
  validate-rename:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v3
      - name: Install Splice
        run: cargo install splice
      - name: Ingest codebase
        run: splice ingest --root ./src --db .codemcp/codegraph.db
      - name: Preview rename
        run: |
          splice rename \
            --symbol old_name \
            --file src/lib.rs \
            --to new_name \
            --preview
      - name: Validate no breaking changes
        run: cargo check
```

## Common Workflows

### Finding Symbol IDs

```bash
# List all symbols in a file
splice query --db .codemcp/codegraph.db --file src/lib.rs

# Find specific symbol by name
splice find --name "my_function" --path "src/lib.rs"

# Get references to understand impact
splice refs --symbol <id> --db .codemcp/codegraph.db
```

### Impact Analysis Before Rename

```bash
# See what will be affected
splice reachable \
  --symbol my_function \
  --path src/lib.rs \
  --max-depth 3 \
  --db .codemcp/codegraph.db

# Check for circular dependencies
splice cycles --db .codemcp/codegraph.db
```

### Rollback After Failed Rename

If a rename causes issues, restore from backup:

```bash
# Backups are created automatically at:
# .splice/backups/rename-<timestamp>/

# Restore manually if needed
cp -r .splice/backups/rename-<timestamp>/src/* ./src/
```

## Troubleshooting

### UTF-8 Character Handling

Splice validates UTF-8 boundaries to ensure safe multi-byte character handling:

```bash
# Rename with special characters
splice rename \
  --symbol old_name \
  --file src/lib.rs \
  --to "new_name_with_special_chars"

# UTF-8 validation is automatic
```

### Large Codebases

For large projects, use reachability analysis first:

```bash
# Check impact scope before renaming
splice reachable \
  --symbol api_function \
  --path src/api.rs \
  --max-depth 10 \
  --db .codemcp/codegraph.db

# If impact is large, consider phased approach
```

## See Also

- [docs/manual.md](../manual.md) - Complete CLI reference
- [docs/examples/graph_algorithm_examples.md](graph_algorithm_examples.md) - Impact analysis with graph algorithms
- [docs/examples/proof_examples.md](proof_examples.md) - Proof-based refactoring
