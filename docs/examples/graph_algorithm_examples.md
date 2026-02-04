# Graph Algorithm Examples

This document provides practical examples for using Splice's graph analysis commands to understand code structure, perform impact analysis, and make safer refactoring decisions.

## Example 1: Impact Analysis Before Refactoring

### Scenario

You want to refactor `process_data()` but need to know what will break.

```bash
# Find all code reachable from process_data
splice reachable \
  --symbol process_data \
  --path src/processor.rs \
  --max-depth 5 \
  --db .codemcp/codegraph.db
```

### Output

```
Reachable symbols (5 levels deep):
Level 0: process_data (src/processor.rs)
Level 1:
  - parse_input (src/parser.rs)
  - validate_output (src/validator.rs)
  - log_results (src/logger.rs)
Level 2:
  - read_file (src/fs.rs)
  - write_file (src/fs.rs)
  - parse_json (src/json.rs)
Level 3:
  - open_stream (src/stream.rs)
  - close_stream (src/stream.rs)
```

### Decision

With this knowledge, you know refactoring `process_data` requires updating:
- `src/parser.rs` (calls parse_input)
- `src/validator.rs` (calls validate_output)
- `src/logger.rs` (calls log_results)

### JSON Output

```bash
splice reachable \
  --symbol process_data \
  --path src/processor.rs \
  --output json \
  --db .codemcp/codegraph.db
```

```json
{
  "symbol": "process_data",
  "file_path": "src/processor.rs",
  "max_depth": 5,
  "levels": [
    {
      "level": 0,
      "symbols": [
        {"name": "process_data", "file": "src/processor.rs"}
      ]
    },
    {
      "level": 1,
      "symbols": [
        {"name": "parse_input", "file": "src/parser.rs"},
        {"name": "validate_output", "file": "src/validator.rs"},
        {"name": "log_results", "file": "src/logger.rs"}
      ]
    }
  ]
}
```

## Example 2: Finding Dead Code

### Scenario

Find unused functions after feature removal.

```bash
# Find dead code from main entry point
splice dead-code \
  --entry main \
  --path src/main.rs \
  --db .codemcp/codegraph.db
```

### Output

```
Dead symbols (unreachable from main):
  - old_feature_a (src/legacy.rs:10)
  - deprecated_handler (src/handlers.rs:45)
  - unused_utility (src/utils.rs:100)

3 dead symbols found
Run with --include-public to include public symbols
```

### Cleanup Workflow

```bash
# Review each symbol before deleting
splice refs --symbol <id> --db .codemcp/codegraph.db

# Delete unused symbols (after verification)
splice delete --symbol old_feature_a --file src/legacy.rs
```

### Including Public Symbols

```bash
# Find all dead code including public API functions
splice dead-code \
  --entry main \
  --path src/main.rs \
  --include-public \
  --db .codemcp/codegraph.db
```

## Example 3: Detecting Circular Dependencies

### Scenario

Investigate build slowness caused by cyclic dependencies.

```bash
# Find all cycles
splice cycles --db .codemcp/codegraph.db
```

### Output

```
Found 3 cycles:
1. [a_calls_b, b_calls_c, c_calls_a] (representative: a_calls_b)
2. [x_imports_y, y_imports_x] (representative: x_imports_y)
3. [self_recursive] (representative: self_recursive)
```

### Analyzing a Specific Cycle

```bash
# Show cycle members
splice cycles \
  --db .codemcp/codegraph.db \
  --show-members

# Inspect a specific cycle
splice find --name "a_calls_b" --path "src/a.rs"
splice refs --symbol <id> --db .codemcp/codegraph.db --direction out
```

### Solutions

Break cycles by:
- Extracting shared logic to separate module
- Introducing dependency inversion
- Splitting circularly-dependent functions
- Using callback patterns instead of direct calls

## Example 4: Dependency Analysis with Condensation

### Scenario

Understand high-level architecture without getting lost in details.

```bash
# Condensed graph analysis
splice condense \
  --db .codemcp/codegraph.db \
  --show-levels \
  --show-members
```

### Output

```
Condensed graph (5 SCCs collapsed to 4 levels):

Level 0 (entry points):
  - main, test_main

Level 1:
  - http_handler (10 edges from level 0)
  - cli_handler (5 edges from level 0)

Level 2:
  - business_logic_scc (15 edges from level 1)
    Contains: [process_order, validate_order, calculate_total]

Level 3:
  - data_access_scc (20 edges from level 2)
    Contains: [db_query, cache_get, cache_set]
```

### Insights

- `business_logic_scc` is a tightly coupled cluster (good candidate for refactoring)
- `data_access_scc` is heavily depended upon (stable abstraction)
- `http_handler` and `cli_handler` are independent (good separation)

### Using Condensation for Refactoring

```bash
# Before refactoring, check SCC structure
splice condense --show-members

# After refactoring, verify improvements
splice condense --show-members

# Look for:
# - Fewer SCC members (less coupling)
# - Clearer level separation (better layering)
```

## Example 5: Forward Slice for Testing

### Scenario

Determine test coverage for a function.

```bash
# Forward slice: what does this function affect?
splice slice \
  --target <id> \
  --direction forward \
  --max-distance 10 \
  --db .codemcp/codegraph.db
```

### Output

```
Forward slice (affected by process_data):
Directly affected:
  - process_order (src/order.rs)
  - send_notification (src/notify.rs)

Indirectly affected (2 hops):
  - update_inventory (src/inventory.rs)
  - log_transaction (src/logging.rs)
  - charge_payment (src/payment.rs)

Affected files:
  - src/order.rs (is_root: true)
  - src/notify.rs
  - src/inventory.rs
  - src/logging.rs
  - src/payment.rs
```

### Test Coverage Strategy

Generate tests for all affected functions when changing `process_data`:

```bash
# Get the slice as JSON
splice slice \
  --target <id> \
  --direction forward \
  --output json \
  --db .codemcp/codegraph.db > affected_functions.json

# Use with jq to extract test targets
jq '.affected_symbols[].name' affected_functions.json
```

## Example 6: Backward Slice for Root Cause Analysis

### Scenario

Investigate why `report_error` is being called.

```bash
# Backward slice: what leads to this error?
splice slice \
  --target <id> \
  --direction backward \
  --max-distance 10 \
  --db .codemcp/codegraph.db
```

### Output

```
Backward slice (affects report_error):
Direct callers:
  - handle_failure (src/error.rs)
  - validate_input (src/validator.rs)

Caller chains (2 hops):
  - main -> process_data -> validate_input
  - main -> handle_request -> handle_failure

Root causes:
  - src/main.rs:main
```

### Use Case

Trace error sources to fix bugs at the root:
1. Identify the error reporting function
2. Run backward slice to find all callers
3. Examine caller chains to find root causes
4. Fix at the source rather than symptoms

## Example 7: Multi-Entry Dead Code Analysis

### Scenario

Analyze dead code for a library with multiple entry points.

```bash
# Dead code from main entry
splice dead-code \
  --entry main \
  --path src/main.rs \
  --db .codemcp/codegraph.db

# Dead code from test entry
splice dead-code \
  --entry test_main \
  --path tests/integration.rs \
  --db .codemcp/codegraph.db

# Compare results to find truly unused code
```

## Example 8: Cycle Detection with Specific Symbol

### Scenario

Check if a specific function is part of a cycle.

```bash
# Check for cycles containing a specific symbol
splice cycles \
  --symbol recursive_function \
  --path src/recursive.rs \
  --db .codemcp/codegraph.db
```

### Output

```
Found 1 cycle containing recursive_function:
  [recursive_function, helper_a, helper_b] (representative: recursive_function)
```

## Example 9: Reverse Reachability for API Analysis

### Scenario

Find all code that uses a specific API function.

```bash
# What calls this API function?
splice reachable \
  --symbol api_endpoint \
  --path src/api.rs \
  --direction backward \
  --max-depth 10 \
  --db .codemcp/codegraph.db
```

### Use Cases

- API deprecation planning
- Impact analysis for API changes
- Finding undocumented API usage

## Example 10: Layered Architecture Validation

### Scenario

Verify that your code follows layered architecture.

```bash
# Check condensation for layer violations
splice condense \
  --db .codemcp/codegraph.db \
  --show-levels
```

### Expected Output (Good Architecture)

```
Level 0: Entry points (main, tests)
Level 1: Controllers/API layer
Level 2: Business logic layer
Level 3: Data access layer
Level 4: Utilities/helpers
```

### Warning Sign (Bad Architecture)

```
Level 0: main
Level 1: [mixed: controllers, business_logic, data_access]
Level 2: [mixed: utilities calling business logic]
```

## Workflow: Complete Refactoring Analysis

### Before Refactoring

```bash
# 1. Check impact scope
splice reachable \
  --symbol function_to_change \
  --path src/lib.rs \
  --max-depth 10 \
  --db .codemcp/codegraph.db

# 2. Check for cycles that might complicate refactoring
splice cycles --db .codemcp/codegraph.db

# 3. Identify dead code that could be removed
splice dead-code \
  --entry main \
  --path src/main.rs \
  --db .codemcp/codegraph.db

# 4. Understand architectural layers
splice condense --show-levels --db .codemcp/codegraph.db
```

### During Refactoring

```bash
# Preview changes
splice rename \
  --symbol <id> \
  --file src/lib.rs \
  --to new_name \
  --preview

# Generate proof
splice rename \
  --symbol <id> \
  --file src/lib.rs \
  --to new_name \
  --proof
```

### After Refactoring

```bash
# 1. Verify no new cycles introduced
splice cycles --db .codemcp/codegraph.db

# 2. Verify architectural integrity
splice condense --show-levels --db .codemcp/codegraph.db

# 3. Validate proof
splice validate-proof --proof .splice/proofs/rename-<timestamp>.json
```

## Performance Considerations

### Large Codebases

For large projects, use depth limits to prevent excessive output:

```bash
# Limit depth for faster analysis
splice reachable \
  --symbol main \
  --path src/main.rs \
  --max-depth 3 \
  --db .codemcp/codegraph.db
```

### JSON Output for Automation

Use JSON output for scripting and CI/CD integration:

```bash
# Get JSON for programmatic processing
splice reachable \
  --symbol main \
  --path src/main.rs \
  --output json \
  --db .codemcp/codegraph.db | jq '.levels[] | .symbols[] | .name'
```

## See Also

- [docs/manual.md](../manual.md) - Complete CLI reference
- [docs/examples/rename_examples.md](rename_examples.md) - Cross-file rename examples
- [docs/examples/proof_examples.md](proof_examples.md) - Proof-based refactoring
