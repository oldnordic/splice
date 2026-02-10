# Performance Guide

**Last Updated:** 2026-02-10
**Version:** v2.5.0

Understanding splice's performance characteristics and optimization strategies.

---

## Table of Contents

1. [Benchmark Overview](#benchmark-overview)
2. [Operation Performance](#operation-performance)
3. [Backend Comparison](#backend-comparison)
4. [Optimization Strategies](#optimization-strategies)
5. [Scaling to Large Codebases](#scaling-to-large-codebases)

---

## Benchmark Overview

splice is optimized for fast, safe refactoring operations with multi-stage validation. Performance varies by operation type, file size, and backend.

| Operation | SQLite (10k symbols) | Native-V2 (10k symbols) | Notes |
|-----------|---------------------|------------------------|-------|
| Patch (single file) | 50-200ms | 40-150ms | Parse + validate + compiler |
| Rename (5 refs) | 100-300ms | 80-200ms | Cross-file replacement |
| Rename (50 refs) | 300-800ms | 200-500ms | Linear to reference count |
| Delete (with cleanup) | 100-400ms | 80-300ms | Cross-file removal |
| Reachability (1K symbols) | 10-30ms | 5-15ms | Graph traversal |
| Dead-code detection | 20-50ms | 10-30ms | BFS from entry point |
| Cycle detection | 30-60ms | 15-40ms | Tarjan's SCC |
| Snapshot | 100-200ms | 50-150ms | Full graph serialization |

**Benchmarks performed on:**
- CPU: 8-core @ 3.0GHz
- RAM: 16GB
- Storage: NVMe SSD
- Database: 10,000 symbols (typical medium project)

---

## Operation Performance

### Factor: File Size

| File Size | Patch Time | Rename Time | Notes |
|-----------|------------|-------------|-------|
| <100 LOC | 20-50ms | 50-100ms | Small utility files |
| 100-500 LOC | 50-150ms | 100-300ms | Typical module files |
| 500-2000 LOC | 150-400ms | 200-500ms | Large modules |
| >2000 LOC | 400ms+ | 500ms+ | Consider splitting |

### Factor: Reference Count

| References | Rename Time | Notes |
|------------|-------------|-------|
| 1-5 | 50-150ms | Typical private function |
| 5-20 | 150-400ms | Popular utility function |
| 20-100 | 400ms-1s | Public API function |
| 100+ | 1s+ | Consider refactoring in stages |

### Factor: Language Validation

| Language | Validation Time | Validator |
|----------|----------------|-----------|
| Rust | 100-500ms | `cargo check` |
| Python | 20-50ms | `python -m py_compile` |
| C/C++ | 50-150ms | `gcc -fsyntax-only` |
| Java | 100-300ms | `javac` |
| JavaScript | 20-40ms | `node --check` |
| TypeScript | 100-400ms | `tsc --noEmit` |

**Note:** Rust validation dominates operation time due to `cargo check` overhead.

---

## Backend Comparison

splice supports both SQLite and Native-V2 backends.

### SQLite Backend

**Advantages:**
- Proven, battle-tested
- Excellent tooling ecosystem (sqlite3, DB Browser)
- Unlimited scale (tested to 1M+ symbols)
- Easy debugging with SQL queries

**Performance:**
- Patch: 50-200ms per file
- Rename: 100-800ms depending on references
- Graph algorithms: 10-60ms for 1K symbols
- Snapshot: Manual export (100-200ms)

**When to use:**
- Default choice for most users
- Need to inspect database directly
- Very large codebases (100k+ symbols)

### Native-V2 Backend

**Advantages:**
- O(1) KV lookups for symbol operations
- Smaller database file sizes (~70% reduction)
- Native snapshot/restore system
- Embedded operations (no subprocess overhead)

**Performance:**
- Patch: 40-150ms per file (20-25% faster)
- Rename: 80-500ms depending on references (20-40% faster)
- Graph algorithms: 5-40ms for 1K symbols (2-3x faster)
- Snapshot: 50-150ms (native, 2-4x faster)

**When to use:**
- Large codebases (100k+ symbols)
- Frequent snapshot/restore workflows
- Batch operations with rollback
- Maximum performance needed

---

## Optimization Strategies

### 1. Use Preview Mode

```bash
# Fast: Preview doesn't write files
splice rename --symbol <id> --file src/lib.rs --to new_name --preview

# Slower: Full operation with backup
splice rename --symbol <id> --file src/lib.rs --to new_name --create-backup
```

### 2. Skip Validation When Safe

```bash
# For non-critical changes, skip validation
splice apply-files --glob "tests/**/*.rs" --find "TODO" --replace "FIXME" --no-validate

# Use with caution: only when you're confident changes are valid
```

### 3. Use Native-V2 for Large Projects

```bash
# Re-index with Native-V2 for better performance
magellan watch --root ./src --db .codemcp/codegraph.db --storage native-v2 --scan-initial

# Rebuild splice with native-v2 support
cargo install splice --features native-v2 --no-default-features
```

### 4. Batch Similar Operations

```bash
# Instead of multiple individual renames:
splice rename --symbol func1 --file src/lib.rs --to new_func1
splice rename --symbol func2 --file src/lib.rs --to new_func2
splice rename --symbol func3 --file src/lib.rs --to new_func3

# Use batch operations:
cat > batch.yaml << 'EOF'
operations:
  - type: rename
    symbol: func1
    file: src/lib.rs
    to: new_func1
  - type: rename
    symbol: func2
    file: src/lib.rs
    to: new_func2
  - type: rename
    symbol: func3
    file: src/lib.rs
    to: new_func3
EOF

splice batch --spec batch.yaml --db .codemcp/codegraph.db
```

### 5. Limit Graph Traversal Depth

```bash
# Slow: Unlimited traversal
splice reachable --symbol main --path src/main.rs

# Fast: Limited depth
splice reachable --symbol main --path src/main.rs --max-depth 3
```

---

## Scaling to Large Codebases

splice scales well to large projects:

| Symbols | Patch Time | Rename Time | Graph Algo | Database Size |
|---------|------------|-------------|------------|---------------|
| 1,000 | 20-50ms | 50-150ms | 5-15ms | 500KB / 200KB |
| 10,000 | 50-200ms | 100-500ms | 10-30ms | 5MB / 2MB |
| 100,000 | 100-400ms | 200-800ms | 20-60ms | 50MB / 15MB |
| 1,000,000 | 200-800ms | 500ms-2s | 50-150ms | 500MB / 150MB |

**Key insights:**
- Patch time scales with file size, not database size
- Rename time scales with reference count, not total symbols
- Graph algorithms scale sub-linearly with optimizations
- Native-V2 database is ~70% smaller than SQLite

---

## Token Efficiency

splice's primary design goal is token efficiency for LLM consumption.

| Task | Source Code | splice JSON | Savings |
|------|-------------|-------------|---------|
| Single file patch | ~5,000 tokens | ~100 tokens | 98% |
| Cross-file rename | ~50,000 tokens | ~200 tokens | 99.6% |
| Impact analysis | ~100,000 tokens | ~300 tokens | 99.7% |

**Why this matters:**
- Less context bloat
- Fewer compactions
- More accurate responses
- Lower API costs

---

## Profiling Operations

Use `--json` output for precise timing:

```bash
$ splice patch --file src/lib.rs --symbol process --with new.rs --json

# Output includes timing information:
{
  "status": "ok",
  "data": {
    "operation_id": "1234567890-abcd",
    "duration_ms": 127,
    ...
  }
}
```

---

## Further Reading

- [README.md](../README.md) - Quick start guide
- [MANUAL.md](../MANUAL.md) - Complete command reference
- [BEST_PRACTICES.md](BEST_PRACTICES.md) - Recommended workflows
- [TROUBLESHOOTING.md](TROUBLESHOOTING.md) - Common issues

---

*Created: 2026-02-10*
