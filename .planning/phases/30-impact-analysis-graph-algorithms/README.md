# Phase 30: Impact Analysis & Graph Algorithms

## Overview

Phase 30 adds graph algorithm integration and impact analysis for safer refactoring.
All commands delegate to Magellan 2.0.0 library API (not subprocess).

## Commands Added

| Command | Purpose | Plan |
|---------|---------|------|
| `reachable` | Caller/callee chain analysis | 30-01 |
| `dead-code` | Dead code detection from entry points | 30-02 |
| `cycles` | Cycle detection in call graph | 30-03 |
| `condense` | Condensation graph (SCC collapse to DAG) | 30-04 |
| `slice` | Forward/backward program slicing | 30-05 |

## Success Criteria (all met)

- [x] User can see impact analysis before refactoring (reachable command)
- [x] User can detect dead code from entry points (dead-code command)
- [x] User can find cycles in call graph (cycles command)
- [x] User can analyze condensation graph (condense command)
- [x] User can perform forward/backward slicing (slice command)

## Integration Points

- **Magellan 2.0.0**: All graph algorithms use the library API directly
- **CLI Pattern**: Consistent with existing commands (--db, --output flags)
- **Output Types**: All support JSON/human/pretty formats
- **Error Handling**: Uses existing SpliceError types

## Testing

Integration tests located in `tests/integration/graph_algorithms_tests.rs`.

All 16 tests pass:
- Command argument parsing tests
- Direction tests (forward/reverse/both)
- Output format tests (human/json/pretty)
- Parameter validation tests

## Documentation

Updated in `docs/manual.md` with usage examples for all commands:
- Reachability analysis for impact tracking
- Dead code detection for cleanup
- Cycle detection for debugging
- Condensation graph for architecture analysis
- Program slicing for refactoring support
