# Performance Guide

**Last Updated:** 2026-04-09  
**Version:** v2.5.4

This guide documents current `splice` performance behavior for safe refactoring workflows.

## Current Backend Status

`splice` supports:

- **SQLite (`.db`)**: compatibility backend
- **Native-V3 (`.v3`)**: high-performance backend (recommended path)

Legacy references to `Native-V2` are historical and should not be treated as the current target architecture.

## Cost Drivers

Most `splice` latency is dominated by one or more of:

1. Parse + span validation complexity
2. Number of references touched in cross-file edits
3. Validation command cost (`cargo check`, `tsc`, `javac`, etc.)
4. File-system I/O and backup behavior

## Directional Operation Ranges

Directional (environment-dependent) values for medium projects:

| Operation | Directional Range |
|---|---:|
| Single-file patch | 50-200 ms |
| Rename (small fanout) | 100-300 ms |
| Rename (large fanout) | 300-800 ms |
| Reachability / dead-code graph query | 10-60 ms |

Use repository benchmarks/tests for exact local measurements:

- `benches/graph_benchmarks.rs`
- `tests/graph_algorithm_performance_tests.rs`
- `tests/performance_context_tests.rs`
- `tests/performance_relationship_tests.rs`

## Validation Cost by Language

| Language | Typical Validation Driver |
|---|---|
| Rust | `cargo check` |
| TypeScript | `tsc --noEmit` |
| Java | `javac` |
| Python | `python -m py_compile` |
| C/C++ | `gcc -fsyntax-only` / `clang -fsyntax-only` |

Validation often dominates runtime on safe-edit operations. If you are patching generated or non-critical files and have separate CI guarantees, disabling validation can materially reduce latency.

## Optimization Playbook

### 1) Preview First

```bash
splice rename --symbol <id> --file src/lib.rs --to new_name --preview
```

### 2) Bound Edit Scope

Prefer targeted files/symbols and avoid broad wildcard operations without filters.

### 3) Limit Traversal Depth

```bash
splice reachable --symbol main --path src/main.rs --max-depth 3
```

### 4) Batch Related Edits

Combine related symbol operations in a single batch run to reduce repeated startup/validation overhead.

### 5) Prefer JSON for Automation

```bash
splice patch --file src/lib.rs --symbol process --with new.rs --json
```

JSON output is typically faster and easier to post-process than human-formatted output in automated pipelines.

## Large Codebase Scaling Notes

- Patch cost scales mainly with touched-file size and validation mode.
- Rename cost scales with reference fanout, not just total symbol count.
- Graph queries usually remain fast when constrained by symbol/path filters.

## Backend Choice Guidance

Use **Native-V3** when:

- You need faster graph-heavy operations.
- You want a single high-throughput path aligned with current Magellan ecosystem direction.

Use **SQLite** when:

- You need direct SQL inspection and interoperability.
- You are migrating from existing `.db`-centric workflows.

## Related Docs

- [README.md](../README.md)
- [MANUAL.md](../MANUAL.md)
- [TROUBLESHOOTING.md](TROUBLESHOOTING.md)

