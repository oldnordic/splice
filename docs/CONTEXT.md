# Splice: Context & Current State

**Created**: 2026-02-03
**Version**: v2.2.3
**Purpose**: Quick grounding for AI sessions — read this first

---

## What Splice Is

**Splice is a span-safe refactoring kernel for 7 programming languages.**

It's NOT an AI system. It provides deterministic tools that an AI MUST use instead of guessing about code edits.

```
The AI THINKS           The TOOLS DECIDE
Bad edits BLOCKED       Not "noticed later"
```

---

## Part of The Larger Vision

Splice is one piece of the grounding substrate:

```
┌─────────────────────────────────────────────────────────────┐
│                     OdinCode (Orchestration)                  │
│  Internal prompts: "Use Splice for edits, never manual Edit"│
└─────────────────────────────────────────────────────────────┘
                              │
        ┌─────────────────────┼─────────────────────┐
        │                     │                     │
┌───────────────┐   ┌───────────────┐   ┌───────────────┐
│   Magellan    │   │    llmgrep    │   │    Splice    │
│   (Truth)     │   │    (Search)   │   │    (Edit)     │
│               │   │               │   │               │
│ "Where is X?" │   │ "What calls Y?"│   │ "Edit safely"│
└───────────────┘   └───────────────┘   └───────────────┘
```

**Read `/home/feanor/Projects/VISION.md` for the full context.**

---

## Actual Current State (2026-02-03)

**My previous assessment was WRONG.** Splice is more complete than I thought.

### What Works ✅

| Component | Status | Location |
|-----------|--------|----------|
| **MagellanIngestor** | ✅ Working | `src/ingest/magellan.rs` |
| **extract_symbols()** | ✅ Working | `src/ingest/dispatch.rs` |
| **All 7 language extractors** | ✅ Working | rust.rs, python.rs, cpp.rs, java.rs, javascript.rs, typescript.rs |
| **Import extraction** | ✅ Working | `src/ingest/imports/` |
| **Semantic kind detection** | ✅ Working | `src/ingest/semantic_kind.rs` (15KB) |
| **Span-safe editing** | ✅ Working | `src/patch/` |
| **AST validation** | ✅ Working | tree-sitter |
| **Compiler verification** | ✅ Working | cargo check, python -m py_compile, etc. |
| **Atomic writes** | ✅ Working | Rollback on failure |
| **Backup/undo** | ✅ Working | `.splice/operations.db` |

**Real status: ~85-90% complete, not 70%.**

---

## The Dead Code (Ignore This)

### `Ingestor` struct — REMOVED IN v2.2.4

**Status**: This dead code has been removed from the codebase.

**Why it existed**: Original design vision where Splice would have its own `CodeGraph`.

**Why it was abandoned**: Realized Magellan already has a perfectly good graph. Implemented `MagellanIngestor` instead.

**What to use instead**: `MagellanIngestor` or `extract_symbols()` directly.

**Migration**: If you encounter old documentation referencing the `Ingestor` struct:
- Use `MagellanIngestor` for database-backed indexing
- Use `extract_symbols()` for one-off symbol extraction

---

## What Actually Works (Use This)

### For Ingestion

**Option 1: MagellanIngestor (Recommended)**

```rust
use splice::ingest::magellan::MagellanIngestor;

let mut ingestor = MagellanIngestor::new(&db_path)?;
let count = ingestor.index_file(&file_path)?;  // ← WORKS
```

**Option 2: Direct Extraction**

```rust
use splice::ingest::dispatch::extract_symbols;

let source = std::fs::read(&file_path)?;
let symbols = extract_symbols(&file_path, &source)?;  // ← WORKS
```

### For Editing

```rust
use splice::patch::splice_patch;

// Span-safe, validated, atomic
splice_patch(&file_path, &symbol_name, &replacement)?;
```

---

## Supported Languages (All 7)

| Language | Extractor | Imports | Status |
|----------|-----------|---------|--------|
| Rust | ✅ | ✅ | Full cross-file |
| Python | ✅ | ✅ | Basic |
| C/C++ | ✅ | ✅ | Basic |
| Java | ✅ | ✅ | Basic |
| JavaScript | ✅ | ✅ | Basic |
| TypeScript | ✅ | ✅ | Basic |

---

## Integration with OdinCode

**Location**: `odincode/src/execution_engine/tool_mapper/splice_tools.rs`

OdinCode wraps Splice CLI and provides:

```rust
splice_patch(symbol, with)     // Safe replacement
splice_delete(symbol)          // Safe deletion
splice_plan(plan_file)         // Multi-step refactoring
```

---

## Dependencies

| Tool | Version | For What |
|------|---------|----------|
| Magellan | 1.7.0+ | Symbol database |
| tree-sitter | Latest | AST parsing |
| ropey | Latest | Safe edits |

---

## Known Issue: Magellan Reference Bug

**Splice depends on Magellan for reference finding.**

**Current blocker**: Magellan missing reference byte offsets bug
**Impact**: Rename refactoring limited
**Status**: Another LLM is working on Magellan fix

---

## Code Organization

```
src/
├── ingest/
│   ├── magellan.rs       ← USE THIS (MagellanIngestor)
│   ├── dispatch.rs       ← USE THIS (extract_symbols)
│   ├── mod.rs            ← Ingestor stub (dead code)
│   ├── rust.rs           ← Language extractor
│   ├── python.rs         ← Language extractor
│   ├── cpp.rs            ← Language extractor
│   ├── java.rs           ← Language extractor
│   ├── javascript.rs     ← Language extractor
│   ├── typescript.rs     ← Language extractor
│   ├── imports/          ← Import tracking
│   └── semantic_kind.rs  ← Kind detection
├── patch/                ← Editing logic
├── resolve/              ← Reference resolution
├── relationships/        ← Cross-file analysis
└── graph/                ← Database integration
```

---

## For AI Sessions

**When working on Splice:**

1. **Never guess** — Use tools to verify
2. **Use MagellanIngestor** — Not the `Ingestor` stub
3. **Test with actual files** — Don't assume parsers work
4. **Validate after edits** — Compilation, AST reparse

**When I say "ingestion is missing":**
- Point me to this file
- Remind me: `MagellanIngestor` works
- The `Ingestor` stub is dead code

**When I say "read that file":**
- Stop me
- Say "use Magellan first"
- Say "use extract_symbols"

---

## The Real Gap

Splice is mostly complete. The main gaps are:

1. **Cross-file delete** — Rust only, other languages are basic
2. **Resume mode** — Failed plans restart from beginning
3. **Cleanup** — Remove dead `Ingestor` stub code

**None of these are blockers.** Splice works today.

---

## Quick Reference

```bash
# Index a file
splice magellan index /path/to/file.rs

# Find symbol
splice find --name my_function

# Edit symbol
splice patch --name my_function --with "new_name"

# Delete symbol
splice delete --name unused_function

# Show references
splice refs --name my_function
```

---

*Last updated: 2026-02-03*
*Read `/home/feanor/Projects/VISION.md` for the full ecosystem context*
