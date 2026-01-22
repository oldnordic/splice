# Phase 16: Symbol Expansion & Search - Research

**Researched:** 2026-01-22
**Domain:** AST navigation, symbol expansion, code search, atomic replacement
**Confidence:** HIGH

## Summary

Phase 16 requires implementing two major features: (1) AST-aware symbol expansion that retrieves full symbol bodies by walking tree-sitter parent chains, and (2) grep-like code search with atomic find-and-replace. The codebase already has substantial infrastructure for both tasks: tree-sitter parsers for 7 languages, parent chain navigation in the reference resolution module, context extraction with ropey, and a pattern search/replace module that needs to be exposed via CLI.

**Primary recommendations:**
1. **Symbol expansion:** Use tree-sitter's `node.parent()` API to walk parent chains, progressively expanding from identifier → full body → containing block
2. **Search command:** Expose the existing `patch::pattern` module via a new `splice search` CLI subcommand with JSON output support
3. **Atomic apply:** Leverage the existing atomic write pattern (temp file + fsync + rename) from `patch::mod.rs`
4. **Doc comments:** Use tree-sitter's previous sibling navigation to find leading comment nodes
5. **Context flag integration:** Apply existing `extract_context_asymmetric` to expanded symbol boundaries

## Standard Stack

### Core Dependencies (Already in Cargo.toml)
| Library | Version | Purpose | Why Standard |
|---------|---------|---------|--------------|
| `tree-sitter` | 0.22 | AST parsing and parent navigation | Official Rust bindings, already used across codebase |
| `ropey` | 1.6 | UTF-8 aware line/column calculations | Already used in context.rs for efficient line operations |
| `glob` | 0.3 | File pattern matching | Official rust-lang glob crate, already in dependencies |
| `regex` | 1.10 | Pattern matching for search | Already in dependencies for compiler error parsing |

### Tree-Sitter Language Parsers (Already Available)
| Parser | Version | Purpose |
|--------|---------|---------|
| `tree-sitter-rust` | 0.21 | Rust symbol expansion |
| `tree-sitter-python` | 0.21 | Python symbol expansion |
| `tree-sitter-c` | 0.21 | C symbol expansion |
| `tree-sitter-cpp` | 0.21 | C++ symbol expansion |
| `tree-sitter-java` | 0.21 | Java symbol expansion |
| `tree-sitter-javascript` | 0.21 | JavaScript symbol expansion |
| `tree-sitter-typescript` | 0.21 | TypeScript symbol expansion |

### No New Dependencies Required

All required libraries are already in `Cargo.toml`. The existing `patch::pattern` module already implements multi-file search with glob filtering and AST confirmation.

## Architecture Patterns

### Module Structure
```
src/
├── expansion/          # NEW: Symbol expansion module
│   ├── mod.rs          # Public API for symbol expansion
│   ├── parent_walker.rs # Tree-sitter parent chain traversal
│   └── doc_comments.rs  # Documentation comment extraction
├── search/             # NEW: Search command module (or extend patch::pattern)
│   ├── mod.rs          # CLI command handlers
│   └── output.rs       # Search result formatting (text + JSON)
└── cli/
    └── mod.rs          # Add Search subcommand to Commands enum
```

### Pattern 1: AST-Aware Parent Chain Walking

**What:** Traverse tree-sitter's AST from a child node to its containing symbol body and beyond.

**When to use:** When implementing `--expand` and `--expand-level <N>` flags.

**Example:**
```rust
// Source: Based on tree-sitter API docs
// https://tree-sitter.github.io/tree-sitter/using-parsers

use tree_sitter::Node;

/// Walk parent chain to expand a symbol to its containing body.
fn expand_to_symbol_body(node: Node) -> Option<Node> {
    let mut current = node;

    // Walk up the parent chain
    loop {
        let parent = current.parent()?;

        match parent.kind() {
            // Found a function/method definition
            "function_item" | "function_definition" | "method_definition"
            | "function_declaration" => return Some(parent),

            // Found a type definition
            "struct_item" | "class_declaration" | "class_specifier"
            | "enum_item" | "enum_specifier" | "trait_item"
            | "interface_declaration" => return Some(parent),

            // Reached the root - no symbol body found
            "source_file" => return None,

            // Continue walking up
            _ => current = parent,
        }
    }
}

/// Progressive expansion: name → body → containing block
fn expand_symbol(node: Node, level: usize) -> Option<Node> {
    let mut current = node;

    for _ in 0..level {
        match expand_to_symbol_body(current) {
            Some(parent) => current = parent,
            None => break,
        }
    }

    Some(current)
}
```

### Pattern 2: Documentation Comment Extraction

**What:** Extract leading doc comments (`///`, `/** ... */`, `#`) before a symbol.

**When to use:** When implementing expansion that includes documentation (Success Criteria #4).

**Example:**
```rust
// Source: Based on tree-sitter sibling navigation
// https://docs.rs/tree-sitter/0.22.0/tree_sitter/

use tree_sitter::Node;

/// Extract leading doc comments for a symbol node.
fn extract_leading_doc_comments(node: Node, source: &[u8]) -> Vec<String> {
    let mut comments = Vec::new();
    let mut prev_sibling = node.prev_sibling();

    // Walk backwards through previous siblings
    while let Some(sibling) = prev_sibling {
        let kind = sibling.kind();

        // Check if this is a comment node
        let is_doc_comment = kind.contains("comment")
            && (kind.starts_with("doc") || kind == "line_comment" || kind == "block_comment");

        if is_doc_comment {
            let text = sibling.utf8_text(source).unwrap_or("");
            comments.push(text.to_string());
            prev_sibling = sibling.prev_sibling();
        } else {
            // Stop at non-comment sibling
            break;
        }
    }

    // Reverse to get correct order (top to bottom)
    comments.reverse();
    comments
}
```

### Pattern 3: Search Command with JSON Output

**What:** Expose existing pattern search functionality via CLI with both human-readable and JSON output.

**When to use:** Implementing `splice search --pattern <text>` command.

**Example:**
```rust
// Source: Extend existing patch::pattern module
use serde::Serialize;

/// Search result with location and context.
#[derive(Debug, Serialize)]
pub struct SearchResult {
    pub file: PathBuf,
    pub line: usize,
    pub column: usize,
    pub byte_start: usize,
    pub byte_end: usize,
    pub matched_text: String,
    pub context_before: Vec<String>,
    pub context_after: Vec<String>,
}

/// CLI handler for search command.
pub fn handle_search(
    pattern: &str,
    glob_pattern: &str,
    context_before: usize,
    context_after: usize,
    json_output: bool,
) -> Result<Vec<SearchResult>> {
    // Use existing find_pattern_in_files
    let config = PatternReplaceConfig {
        glob_pattern: glob_pattern.to_string(),
        find_pattern: pattern.to_string(),
        replace_pattern: String::new(), // Not needed for search-only
        language: None,
        validate: false,
    };

    let matches = find_pattern_in_files(&config)?;

    // Add context using existing extract_context_asymmetric
    let results: Vec<SearchResult> = matches.into_iter().map(|m| {
        let ctx = extract_context_asymmetric(
            &m.file,
            m.byte_start,
            m.byte_end,
            context_before,
            context_after,
        ).unwrap_or_default();

        SearchResult {
            file: m.file,
            line: m.line,
            column: m.column,
            byte_start: m.byte_start,
            byte_end: m.byte_end,
            matched_text: m.matched_text,
            context_before: ctx.before,
            context_after: ctx.after,
        }
    }).collect();

    if json_output {
        println!("{}", serde_json::to_string_pretty(&results)?);
    } else {
        // Human-readable output
        for result in &results {
            println!("{}:{}:{}: {}", result.file.display(), result.line, result.column, result.matched_text);
        }
    }

    Ok(results)
}
```

### Pattern 4: Atomic Apply with Rollback

**What:** Apply search-and-replace atomically with automatic rollback on failure.

**When to use:** Implementing `splice search --apply` flag.

**Example:**
```rust
// Source: Based on existing atomic write pattern in patch::mod.rs
use std::fs;
use std::io::Write;
use tempfile::NamedTempFile;

/// Atomic file replacement with rollback on failure.
pub fn apply_atomic_replace(
    file_path: &Path,
    replacements: &[(usize, usize, &str)], // (start, end, replacement)
) -> Result<()> {
    // Read original content
    let mut content = fs::read_to_string(file_path)?;

    // Create backup
    let backup = NamedTempFile::new()?;

    // Apply replacements in reverse byte order
    let mut sorted: Vec<_> = replacements.to_vec();
    sorted.sort_by_key(|(start, _, _)| std::cmp::Reverse(*start));

    for (start, end, replacement) in sorted {
        content.replace_range(*start..*end, replacement);
    }

    // Write to temp file first
    let temp_file = NamedTempFile::new()?;
    temp_file.as_file().write_all(content.as_bytes())?;
    temp_file.as_file().sync_all()?;

    // Atomic rename
    fs::rename(temp_file.path(), file_path)?;

    // Persist backup for rollback
    backup.persist(file_path.with_extension("bak"))?;

    Ok(())
}
```

### Anti-Patterns to Avoid

- **Line-based expansion:** Don't use line numbers for expansion boundaries - always use byte offsets from tree-sitter nodes.
- **String searching for symbols:** Don't search for symbol bodies with text patterns - use tree-sitter AST navigation.
- **Non-atomic writes:** Never write directly to files - use temp file + rename pattern.
- **Hardcoded language logic:** Don't duplicate expansion logic per language - use generic tree-sitter node kind matching.
- **Ignoring UTF-8:** Don't assume single-byte characters - always use ropey for line/column conversion.

## Don't Hand-Roll

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| Symbol body expansion | Custom AST traversal | tree-sitter `node.parent()` | Handles all 7 languages correctly, respects grammar structure |
| UTF-8 line/column | Manual byte counting | ropey `Rope::byte_to_line` | Handles multi-byte UTF-8 correctly, already used in context.rs |
| File pattern matching | Custom glob logic | `glob::glob()` | Standard crate, already in dependencies, handles **, ?, [] patterns |
| Atomic writes | Manual fsync + rename | `tempfile::NamedTempFile` + `persist()` | Cross-platform, handles cleanup automatically |
| JSON serialization | Manual formatting | `serde_json::to_string_pretty()` | Consistent with other JSON output in codebase |
| Search with AST confirmation | Text-only search | Existing `patch::pattern::find_pattern_in_files()` | Already implements AST node filtering to avoid comment matches |

**Key insight:** The `patch::pattern` module already implements 80% of the search functionality needed. The primary work is exposing it via CLI and adding JSON output, not rebuilding search from scratch.

## Common Pitfalls

### Pitfall 1: Expanding to Wrong Node Type

**What goes wrong:** Expansion walks parent chain but stops at too narrow or too broad a node (e.g., stops at parameter list instead of function body).

**Why it happens:** Tree-sitter grammars have nested nodes (e.g., `function_item` contains `parameters` contains `parameter`). Without careful node kind checking, expansion might stop at the wrong level.

**How to avoid:**
- Always check `node.kind()` against language-specific symbol node types
- Define a mapping of "expansion targets" per language (e.g., for Rust: `function_item`, `struct_item`, `impl_item`)
- Test expansion on each of the 7 supported languages

**Warning signs:** Expansion returns only the symbol signature, not the body. Expansion includes unrelated sibling nodes.

### Pitfall 2: Doc Comments Not Found

**What goes wrong:** Leading doc comments are not included in expanded output because they're not children of the symbol node.

**Why it happens:** In tree-sitter, comments are typically siblings or preceding nodes, not children. Walking only the parent chain won't find them.

**How to avoid:**
- Use `node.prev_sibling()` to walk backwards from the symbol node
- Check sibling node kinds for comment types
- Stop at the first non-comment sibling

**Warning signs:** Tests for doc comment inclusion fail. Expanded output missing `///` or `/**` comments.

### Pitfall 3: Context Flag Boundary Violation

**What goes wrong:** When `--expand` is used with `-A/-B/-C` flags, context lines might overlap with the expanded symbol body.

**Why it happens:** Expansion might change the effective "selected" region, but context extraction still uses the original symbol's byte offsets.

**How to avoid:**
- When `--expand` is used, re-calculate the selected region to be the expanded symbol's span
- Apply context flags to the expanded boundaries, not the original symbol's boundaries
- Document that context flags are relative to the expanded symbol when `--expand` is present

**Warning signs:** Duplicate lines in output. Context lines include parts of the symbol body.

### Pitfall 4: Non-Atomic Replacement Corruption

**What goes wrong:** If `splice search --apply` crashes mid-replacement, files are left in a corrupted state.

**Why it happens:** Writing directly to target files instead of using atomic temp file + rename.

**How to avoid:**
- Always write to a temp file first
- Use `fs::rename()` which is atomic on POSIX and Windows
- Keep backups until the operation completes successfully
- Use `tempfile::NamedTempFile::persist()` for automatic cleanup

**Warning signs:** Files contain partial replacements after crashes. Test files corrupted during test runs.

### Pitfall 5: Breaking Multi-File Replacements

**What goes wrong:** Replacing the same pattern across multiple files breaks if the process fails on file 3 of 10.

**Why it happens:** Not collecting all replacements before applying, or not having a rollback plan.

**How to avoid:**
- Use the existing `SpanBatch` pattern from `patch::mod.rs`
- Validate all replacements before applying any
- On failure, roll back already-applied changes from backups
- Return an error indicating which file failed

**Warning signs:** Some files updated, others not. Inconsistent state after errors.

### Pitfall 6: Inconsistent Expansion Across Languages

**What goes wrong:** `--expand` works for Rust functions but fails for Python classes or Java methods.

**Why it happens:** Hardcoding language-specific logic instead of using generic tree-sitter patterns.

**How to avoid:**
- Define a trait `SymbolExpander` with language-specific implementations
- Use language-specific node kind mappings (stored in constants, not hardcoded in logic)
- Test each supported language with the same test cases

**Warning signs:** Tests pass for Rust but fail for Python/Java/JavaScript. Different expansion behavior per language.

## Code Examples

### Multi-Language Expansion Dispatch

```rust
// Source: Based on existing dispatch pattern in ingest::dispatch.rs
use crate::ingest::Language;

/// Expand a symbol to its containing body based on language.
pub fn expand_symbol_to_body(
    node: tree_sitter::Node,
    language: Language,
) -> Option<tree_sitter::Node> {
    match language {
        Language::Rust => expand_rust_symbol(node),
        Language::Python => expand_python_symbol(node),
        Language::JavaScript => expand_javascript_symbol(node),
        Language::TypeScript => expand_typescript_symbol(node),
        Language::Java => expand_java_symbol(node),
        Language::C => expand_c_symbol(node),
        Language::Cpp => expand_cpp_symbol(node),
    }
}

fn expand_rust_symbol(node: tree_sitter::Node) -> Option<tree_sitter::Node> {
    let mut current = node;

    loop {
        let parent = current.parent()?;

        match parent.kind() {
            "function_item" | "struct_item" | "enum_item"
            | "trait_item" | "impl_item" | "mod_item"
            | "const_item" | "static_item" => return Some(parent),
            "source_file" => return None,
            _ => current = parent,
        }
    }
}
```

### Context Flag Integration with Expanded Boundaries

```rust
// Source: Based on existing context extraction in context.rs
use crate::context::extract_context_asymmetric;

/// Extract context respecting expanded symbol boundaries.
pub fn extract_expanded_context(
    path: &Path,
    original_start: usize,
    original_end: usize,
    context_before: usize,
    context_after: usize,
    expand_level: usize,
    language: Language,
) -> Result<(SpanContext, usize, usize)> {
    // Parse the file to get tree-sitter tree
    let source = std::fs::read(path)?;
    let mut parser = tree_sitter::Parser::new();
    parser.set_language(&language_to_ts_language(language))?;
    let tree = parser.parse(&source, None)?;

    // Find the node at the original location
    let root = tree.root_node();
    let node = root.descendant_for_byte_range(original_start, original_end)
        .ok_or_else(|| SpliceError::Other("No node at location".to_string()))?;

    // Expand the symbol
    let expanded_node = expand_symbol_level(node, expand_level, language)?;

    // Get expanded byte range
    let expanded_start = expanded_node.start_byte() as usize;
    let expanded_end = expanded_node.end_byte() as usize;

    // Extract context using expanded boundaries
    let context = extract_context_asymmetric(
        path,
        expanded_start,
        expanded_end,
        context_before,
        context_after,
    )?;

    Ok((context, expanded_start, expanded_end))
}
```

## State of the Art

| Old Approach | Current Approach | When Changed | Impact |
|--------------|------------------|--------------|--------|
| Manual AST parsing | tree-sitter parsers | 2024 | Industry standard for code analysis tools |
| Line-based context | Byte-exact with ropey | Existing | Handles UTF-8 correctly, already in codebase |
| Non-atomic writes | temp file + rename | Existing | Used in patch::mod.rs, prevents corruption |
| String-based search | AST-confirmed search | Existing | Implemented in patch::pattern.rs |

**Deprecated/outdated:**
- Line numbers for span boundaries: Use byte offsets from tree-sitter nodes
- Manual file locking: Use atomic rename pattern instead
- Text-only search: Use AST node filtering to avoid comment matches

## Open Questions

1. **Expansion level semantics:** Should `--expand-level 0` mean "no expansion" (just the identifier) or "expand to immediate parent" (full symbol body)?

   **Recommendation:** `--expand-level 0` = no expansion (current behavior), `--expand-level 1` = expand to symbol body, `--expand-level 2` = expand to containing block.

2. **Should `--expand` imply expansion in search results?** When using `splice search --expand`, should search results show the expanded symbol containing the match?

   **Recommendation:** No - `--expand` applies to get/query commands only. Search shows the matched pattern, not the containing symbol. This keeps search focused on pattern matching.

3. **JSON output format for search:** Should JSON output include the full matched text with context, or just byte offsets?

   **Recommendation:** Include both: byte offsets for programmatic use, and `matched_text`, `context_before`, `context_after` arrays for human/LLM consumption.

## Sources

### Primary (HIGH confidence)

- **[tree-sitter Rust API docs](https://docs.rs/tree-sitter/0.22.0/tree_sitter/)** - Official Rust bindings documentation for node navigation (parent, children, siblings)
- **[tree-sitter using-parsers guide](https://tree-sitter.github.io/tree-sitter/using-parsers)** - Official tree-sitter documentation on node traversal and tree walking patterns
- **[Existing codebase pattern module](https://github.com/oldnordic/splice/blob/main/src/patch/pattern.rs)** - Multi-file search with glob filtering and AST confirmation (already implemented)
- **[Existing codebase context module](https://github.com/oldnordic/splice/blob/main/src/context.rs)** - Asymmetric context extraction with ropey (already implemented)
- **[Existing codebase resolve module](https://github.com/oldnordic/splice/blob/main/src/resolve/mod.rs)** - Symbol resolution with graph lookup (already implemented)

### Secondary (MEDIUM confidence)

- **[Atomic file operations in Rust](https://stackoverflow.com/questions/70362352/atomic-file-create-write)** - Stack Overflow discussion on atomic writes (2022)
- **[How to write/replace files atomically](https://users.rust-lang.org/t/how-to-write-replace-files-atomically/42821)** - Rust forum discussion (2020)
- **[rust-lang/glob crate](https://github.com/rust-lang/glob)** - Official glob pattern matching crate
- **[Medium: Building a grep-like tool in Rust](https://medium.com/@PocketJack/how-to-build-a-grep-like-tool-in-rust-a-step-by-step-tutorial-43e23813630e)** - Tutorial on grep implementation patterns (2025)
- **[Tree-sitter GitHub issue #192 - Comments](https://github.com/tree-sitter/tree-sitter/issues/192)** - Discussion on comment handling in tree-sitter (2018)

### Tertiary (LOW confidence)

- **[Dev.to: Getting Started with Tree-sitter](https://dev.to/lovestaco/getting-started-with-tree-sitter-syntax-trees-and-express-api-parsing-5c2d)** - Tree-sitter tutorial (April 2025)
- **[Reddit: rep Rust find & replace utility](https://www.reddit.com/r/rust/comments/18rxz9z/rep_is_a_new_rust_find_replace_utility_that_takes/)** - Discussion of rust-based search/replace tools (2024)
- **[Emacs SE: Documentation comments with tree-sitter](https://emacs.stackexchange.com/questions/78274/how-to-highlight-documentation-comments-with-tree-sitter)** - Discussion on doc comment extraction (2024)

## Metadata

**Confidence breakdown:**
- Standard stack: HIGH - All libraries already in codebase and proven working
- Architecture patterns: HIGH - Based on existing patterns in codebase (patch::pattern, context::extract_context_asymmetric)
- Pitfalls: HIGH - Identified from existing code patterns and common tree-sitter/Rust mistakes

**Research date:** 2026-01-22
**Valid until:** 2026-02-22 (30 days - stable domain, tree-sitter API is stable)
