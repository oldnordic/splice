# Context Patterns Research: Context Flags and Symbol Expansion

**Project:** Splice
**Research Date:** 2026-01-22
**Overall Confidence:** HIGH

## Executive Summary

This research analyzes industry patterns for retrieving surrounding context and expanding symbols in code tools. The findings will inform Splice's implementation of `--context-before` and `--context-after` flags, along with a `--expand` / `--full-block` flag for complete symbol retrieval.

**Key recommendations:**
1. **Context flags:** Follow grep/ripgrep conventions: `-C` (combined), `-A` (after), `-B` (before) with 3-line default
2. **Symbol expansion:** AST-aware expansion using tree-sitter, starting at identifier and progressively expanding to containing nodes
3. **Range semantics:** LSP's `range` (full extent including children) vs `selectionRange` (name only) pattern

## Table of Contents

1. [Context Flag Conventions](#context-flag-conventions)
2. [Symbol Expansion Patterns](#symbol-expansion-patterns)
3. [Flag Naming Recommendations](#flag-naming-recommendations)
4. [Line Number Defaults](#line-number-defaults)
5. [Implementation Considerations](#implementation-considerations)

---

## Context Flag Conventions

### Unix Standard: grep, ripgrep, git diff

The established pattern across Unix tools:

| Tool | Flag | Purpose | Default |
|------|------|---------|---------|
| **grep** | `-C NUM` | Context both before and after | Required (no default) |
| **grep** | `-A NUM` | Context after match | Required (no default) |
| **grep** | `-B NUM` | Context before match | Required (no default) |
| **ripgrep** | `-C NUM` | Context both before and after | Required (no default) |
| **ripgrep** | `-A NUM` | Context after match | Required (no default) |
| **ripgrep** | `-B NUM` | Context before match | Required (no default) |
| **git diff** | `-U NUM` | Unified diff context lines | **3 lines** |

**Confidence:** HIGH - Based on official documentation ([ripgrep/GUIDE.md](https://github.com/BurntSushi/ripgrep/blob/master/GUIDE.md), [git-diff man page](https://manpages.debian.org/testing/ripgrep/rg.1.en.html))

### Key Patterns

1. **Short flags are single letters:** `-C`, `-A`, `-B`
2. **Long flags use descriptive names:** `--context`, `--after-context`, `--before-context`
3. **Capitalization matters:** `-C` is context, `-c` is count
4. **`-C` is shorthand for `-B` + `-A` with same value**

---

## Symbol Expansion Patterns

### LSP: Range vs SelectionRange

The Language Server Protocol defines two key range types for symbols:

**Source:** [LSP 3.17 Specification - DocumentSymbol](https://microsoft.github.io/language-server-protocol/specifications/lsp/3.17/specification/)

```typescript
interface DocumentSymbol {
    name: string;
    kind: SymbolKind;
    range: Range;              // Full extent including children
    selectionRange: Range;      // Name only - for cursor positioning
    children?: DocumentSymbol[];
}
```

| Range Type | Purpose | Example for `fn foo() {}` |
|------------|---------|--------------------------|
| `range` | Full symbol extent including nested content | From `fn` to closing `}` |
| `selectionRange` | Identifier name for cursor positioning | Just `foo` |

**Confidence:** HIGH - Directly from LSP specification

### VS Code Smart Select

**Source:** [VS Code Basic Editing Docs](https://code.visualstudio.com/docs/editing/codebasics)

VS Code's expand selection (keyboard: `Alt+Shift+Right` / `Ctrl+Shift+Alt+Right`):

1. **First activation:** Select camelCase sub-word under cursor
2. **Subsequent activations:** Progressively expand to:
   - Entire identifier
   - Containing statement/expression
   - Containing block/function
   - Containing class/module
   - Entire file

**Implementation:** AST-based using language-specific grammar definitions (not text-based)

**Confidence:** MEDIUM - Based on documentation, though implementation details are internal

### JetBrains IDEs

**Source:** [IntelliJ Platform SDK - PSI](https://plugins.jetbrains.com/docs/intellij/implementing-parser-and-psi.html)

JetBrains uses **PSI (Program Structure Interface)** - AST with:
- `PsiElement` tree structure
- `TextRange` for spans (start offset, end offset)
- Parent-child navigation for expansion

**Expansion pattern:**
1. Start at `PsiElement` under cursor
2. Navigate up tree: `element.getParent()`
3. Each level expands to containing node's `getTextRange()`

**Confidence:** MEDIUM - Based on SDK documentation, though specific "extend selection" implementation details are not fully documented

---

## Flag Naming Recommendations

### Primary Recommendations

Based on industry standards, for Splice we recommend:

| Flag | Type | Purpose | Recommendation |
|------|------|---------|----------------|
| `--context-before` / `-B` | Optional | Lines before symbol | **Adopt** - Unix standard |
| `--context-after` / `-A` | Optional | Lines after symbol | **Adopt** - Unix standard |
| `--context` / `-C` | Optional | Combined context | **Adopt** - Unix standard |
| `--expand` | Optional | Full symbol expansion | **Adopt** - Intuitive |
| `--full-block` | Optional | Alias for --expand | **Consider** - Alternative naming |

### Short Flag Justification

**`-B` (before), `-A` (after):** These are de facto standards across grep/ripgrep/git. Users familiar with Unix tools will recognize them immediately.

**Alternative considered:** `-b` (bytes) - BUT this conflicts with grep's `--byte-offset`. Use `-B` for lines-before.

**Long flag naming:** `--context-before` is clearer than `--before-context` (both are used in different tools). We follow ripgrep's convention: `--before-context` / `--after-context`.

---

## Line Number Defaults

### Industry Practice

| Tool | Default Context | Rationale |
|------|-----------------|-----------|
| **git diff** | **3 lines** | Enough context to understand change |
| **unified diff** | 3 lines | POSIX standard |
| **grep -C** | Required (no default) | User must specify |
| **ripgrep -C** | Required (no default) | Follows grep |

**Recommendation for Splice:**

1. **No default context** when context flags not specified
2. **When `--context` flag used without value:** Default to 3 lines (follow git diff)
3. **Allow explicit `0`:** `--context-before 0` for "no lines before"

**Rationale:** Git diff's 3-line default has stood the test of time - enough to see surrounding structure without overwhelming output.

**Confidence:** HIGH - Based on git diff documentation and decades of use

---

## Implementation Considerations

### Context Boundary Inclusion/Exclusion

**Pattern from grep/ripgrep:**

```
Line 1    [context-before]
Line 2    [context-before]
Line 3    [MATCHED LINE]      <-- inclusive
Line 4    [context-after]
Line 5    [context-after]
```

**Recommendation:** Follow grep - matched line is included in output, context lines are surrounding.

### AST-Aware Expansion Strategy

For `--expand` / `--full-block`:

**Option 1: Text-based (Simple, but brittle)**
- Scan outward from byte span
- Detect braces/parentheses
- Risk: Fails on comments, strings, edge cases

**Option 2: AST-aware using tree-sitter (Recommended)**

```
1. Parse file with tree-sitter
2. Find smallest node containing span
3. Expand to parent node's range
4. Repeat until at desired level
```

**Example expansion levels:**

```rust
fn foo() {                    // Level 0: cursor on identifier
    let x = 1;                // Level 1: expand to statement
    if x > 0 {                // Level 2: expand to block
        println!("{}", x);    // Level 3: expand to function
    }
}                             // Level 4: expand to module
```

**Splice-specific consideration:** Use tree-sitter's `Node` API:
- `node.range()` gives start/end bytes
- `node.parent()` walks up tree
- Stop at "logical boundaries" (function, class, impl block)

**Confidence:** MEDIUM - Tree-sitter approach is widely used, but specific implementation needs validation

### Handling Nested Symbols

**Challenge:** When cursor is inside nested function:

```rust
fn outer() {
    fn inner() {      // <-- Cursor here
        // ...
    }
}
```

**Expansion behavior:**

1. First `--expand`: `inner` function only (selectionRange)
2. Second `--expand`: `inner` + body (range)
3. Third `--expand`: `outer` function body

**Implementation:** Walk parent chain, skipping intermediate nodes that don't represent "symbol boundaries"

**Confidence:** LOW - Requires testing across language grammars

---

## Recommended Splice CLI Integration

### Phase 1: Context Flags

```bash
# Show 3 lines before and after symbol
splice get db.spice SymbolName --context 3

# Show 5 lines before, 2 after
splice get db.spice SymbolName --context-before 5 --context-after 2

# Short flags
splice get db.spice SymbolName -C 3
splice get db.spice SymbolName -B 5 -A 2
```

### Phase 2: Symbol Expansion

```bash
# Get full function body (AST-aware)
splice get db.spice SymbolName --expand

# Get full block including nested functions
splice get db.spice SymbolName --expand --expand-level 2

# Alias
splice get db.spice SymbolName --full-block
```

### Output Format

Context should be included in JSON output:

```json
{
  "span": {
    "file": "src/main.rs",
    "byte_start": 1234,
    "byte_end": 5678,
    "content": "fn foo() {\n    ...\n}"
  },
  "context_before": [
    "/// Documentation comment",
    "use std::collections::HashMap;"
  ],
  "context_after": [
    "let x = 1;",
    "}"
  ]
}
```

---

## Sources

### High Confidence (Official Documentation)

- [ripgrep User Guide](https://github.com/BurntSushi/ripgrep/blob/master/GUIDE.md) - Context flag specifications
- [ripgrep Man Page](https://manpages.debian.org/testing/ripgrep/rg.1.en.html) - Official documentation for `-C`, `-A`, `-B` flags
- [git diff Documentation](https://manpages.debian.org/testing/git/git-diff.1.en.html) - Unified diff context default of 3 lines
- [LSP 3.17 Specification](https://microsoft.github.io/language-server-protocol/specifications/lsp/3.17/specification/) - `range` vs `selectionRange` definitions

### Medium Confidence (Official but Secondary)

- [VS Code Basic Editing](https://code.visualstudio.com/docs/editing/codebasics) - Smart select behavior description
- [IntelliJ Platform SDK - PSI](https://plugins.jetbrains.com/docs/intellij/implementing-parser-and-psi.html) - PSI tree and TextRange API
- [Tree-sitter Documentation](https://tree-sitter.github.io/tree-sitter/using-parsers/6-static-node-types) - Node navigation API

### Low Confidence (Community/Indirect)

- [GitHub Issue: VS Code Expand Selection #4795](https://github.com/microsoft/vscode/issues/4795) - Discussion of AST + word separator approach (2019)
- [Stack Overflow: Smart Select camelCase](https://stackoverflow.com/questions/61164281/change-editor-action-smartselect-grow-to-not-treat-camelcase-words-as-boundari) - VS Code configuration (2020)

---

## Gaps and Open Questions

1. **Symbol expansion levels:** Should `--expand-level` be exposed as flag, or always expand to "natural" boundary?
   - **Recommendation:** Start with single-level `--expand`, add `--expand-level` if needed

2. **Context with expanded symbols:** If `--expand` includes full function, should `--context` still apply?
   - **Recommendation:** Yes, context applies to the expanded span's boundaries

3. **Language-specific behavior:** Tree-sitter grammars vary in node structure
   - **Recommendation:** Test across Rust, Python, TypeScript first

4. **Performance impact:** AST parsing for every `--expand` call
   - **Recommendation:** Cache tree-sitter trees per file session

---

## Confidence Assessment

| Area | Confidence | Reasoning |
|------|------------|-----------|
| Context flag conventions | **HIGH** | Based on official grep/ripgrep/git documentation |
| Line number defaults (3) | **HIGH** | Git diff standard, decades of Unix practice |
| LSP range semantics | **HIGH** | Directly from LSP 3.17 specification |
| AST expansion patterns | **MEDIUM** | Tree-sitter API docs are clear, but implementation details untested |
| VS Code/JetBrains internals | **MEDIUM** | Official docs describe behavior, but not implementation details |
| Language-specific edge cases | **LOW** | Requires testing across tree-sitter grammars |

---

## Open Questions for Phase-Specific Research

1. **Rust-specific:** How does tree-sitter-rust structure function bodies?
   - **Research in:** Phase 4 (Multi-language support)

2. **Python-specific:** How to handle indentation-based blocks without braces?
   - **Research in:** Phase 4 (Multi-language support)

3. **Macro expansion:** Should `--expand` include macro-generated code?
   - **Research in:** Phase 5 (Advanced symbol resolution)

4. **Comments and docstrings:** Should they be included in expanded span?
   - **Recommendation:** Include leading doc comments, exclude trailing comments
