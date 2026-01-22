# Phase 15: Enhanced Errors - Research

**Researched:** 2026-01-22
**Domain:** Error reporting, diagnostics, CLI UX
**Confidence:** HIGH

## Summary

Phase 15 enhances Splice's error reporting with structured, actionable diagnostics. The research reveals a well-established ecosystem of Rust error handling libraries and patterns that can be leveraged rather than hand-rolling solutions.

**Key findings:**
1. **Fuzzy matching for symbol suggestions:** Use the `strsim` crate (standard, mature) or `fuzzy-cmp` (more recent, active 2026 development)
2. **`splice explain` command:** Follow the pattern established by `rustc --explain` and `cargo --explain` - embedded documentation is the standard approach
3. **Structured diagnostics:** The codebase already has excellent infrastructure (error_codes.rs, Diagnostic struct) - enhancement is about completing missing fields, not rebuilding
4. **Compiler error parsing:** Existing `parse_rust_style_output()` already handles E0XXX codes; TypeScript uses TSXXXX format with a well-defined pattern

**Primary recommendation:** Build incrementally on existing infrastructure. Don't introduce miette (too heavy, requires refactoring). Use strsim for fuzzy matching. Follow rustc's embedded documentation pattern for `splice explain`.

## Standard Stack

### Core Libraries

| Library | Version | Purpose | Why Standard |
|---------|---------|---------|--------------|
| `strsim` | 0.11+ | Levenshtein distance and string similarity metrics | Mature, widely-used (10k+ crates depend on it), part of the standard Rust error suggestion ecosystem |
| | | | **HIGH confidence** - Official docs.rs, used by ripgrep and other core tools |

### Supporting Libraries

| Library | Version | Purpose | When to Use |
|---------|---------|---------|-------------|
| `fuzzy-cmp` | Latest 2026 | Alternative fuzzy matching with similarity coefficients (0.0-1.0) | If normalized similarity scores are needed beyond distance |
| | | | **MEDIUM confidence** - Recent 2026 releases, less proven than strsim |

### Alternatives Considered

| Instead of | Could Use | Tradeoff |
|------------|-----------|----------|
| `strsim` | `fuzzy-cmp` | strsim is more mature and widely-adopted; fuzzy-cmp offers normalized scores but is newer |
| `strsim` | `did-you-mean` crate | did-you-mean is higher-level but less flexible; strsim gives direct control over similarity algorithm |
| Embedded docs | External markdown files | Embedded = simpler distribution, no file I/O, matches rustc pattern. External = easier to update without recompiling |

**Installation:**
```bash
# Core dependency (HIGH confidence)
cargo add strsim

# Alternative if normalized scores needed (MEDIUM confidence)
cargo add fuzzy-cmp
```

## Architecture Patterns

### Error Code Structure (Already Implemented)

The codebase already has excellent error infrastructure:

```rust
// src/error_codes.rs - SPL-E### format
pub enum SpliceErrorCode {
    SymbolNotFound,  // SPL-E001
    AmbiguousSymbol, // SPL-E002
    // ... 20+ codes already defined
}

impl SpliceErrorCode {
    pub fn code(&self) -> String { /* "SPL-E001" */ }
    pub fn severity(&self) -> String { /* "error" */ }
    pub fn hint(&self) -> String { /* actionable guidance */ }
}

// src/error.rs - Diagnostic struct
pub struct Diagnostic {
    pub tool: String,
    pub level: DiagnosticLevel,  // Error, Warning, Note, Help
    pub message: String,
    pub file: Option<PathBuf>,
    pub line: Option<usize>,
    pub column: Option<usize>,
    pub code: Option<String>,    // E0XXX, TSXXXX, etc.
    pub note: Option<String>,    // Compiler hints
    pub remediation: Option<String>,
}
```

### Pattern 1: Fuzzy Symbol Suggestions (NEW)

**What:** Use Levenshtein distance to suggest similar symbol names when SymbolNotFound occurs.

**When to use:** Symbol lookup fails in symbol finding commands.

**Example:**
```rust
use strsim::levenshtein;

fn find_similar_symbols(
    target: &str,
    candidates: &[String],
    max_distance: usize
) -> Vec<String> {
    let mut similar: Vec<_> = candidates
        .iter()
        .map(|name| (name.clone(), levenshtein(target, name)))
        .filter(|(_, dist)| *dist <= max_distance)
        .collect();

    similar.sort_by_key(|(_, dist)| *dist);
    similar.into_iter()
        .take(5)  // Top 5 suggestions
        .map(|(name, _)| name)
        .collect()
}

// Integration with existing SymbolNotFound error
SpliceError::SymbolNotFound {
    message: format!("Symbol '{}' not found", target),
    hint: format!(
        "Did you mean: {}?",
        similar.join(", ")
    ),
    // ...
}
```

**Source:** Based on strsim documentation (https://docs.rs/strsim/) and patterns from ripgrep's "did you mean" suggestions.

### Pattern 2: `splice explain` Command (NEW)

**What:** Follow rustc's pattern of embedding error documentation.

**When to use:** Users run `splice explain SPL-E001` to get detailed error explanations.

**Example:**
```rust
// CLI command
fn cmd_explain(matches: &ArgMatches) -> Result<()> {
    let code = matches.value_of("code").unwrap();

    match get_error_explanation(code) {
        Some(explanation) => println!("{}", explanation),
        None => println!("Unknown error code: {}", code),
    }
    Ok(())
}

// Embedded documentation (don't use external files)
fn get_error_explanation(code: &str) -> Option<&'static str> {
    match code {
        "SPL-E001" => Some("
Symbol Not Found (SPL-E001)

The specified symbol could not be found in the codebase.

Possible causes:
- The symbol name is misspelled
- The symbol hasn't been ingested into the code graph
- The symbol exists in multiple files (use --file to disambiguate)

What to do:
1. Check the symbol name is spelled correctly
2. Run `splice ingest` to ensure the codebase is indexed
3. Use `splice find-symbol --file <path>` to specify which definition
4. Use `splice explain SPL-E002` for help with ambiguous symbols
        "),
        "SPL-E002" => Some("Ambiguous Symbol..."),
        // ... one match arm per error code
        _ => None,
    }
}
```

**Source:** Pattern established by `rustc --explain` (verified via rustc-dev-guide and error-index.html docs).

### Pattern 3: Extract Line/Column from All Errors (ENHANCEMENT)

**What:** Attach precise location (file:line:column) to all errors, not just some.

**When to use:** Currently, line/column extraction is TODO (src/cli/mod.rs:578). Need to extract from:
- Tree-sitter nodes (already have byte spans)
- Compiler output (already parsed in `parse_rust_style_output()`)
- Error variants that have path but no line/col

**Example:**
```rust
// Enhance ErrorCode::from_splice_code to extract location from errors
impl SpliceError {
    /// Extract (file, line, column) if available from error context
    pub fn location(&self) -> (Option<&str>, Option<usize>, Option<usize>) {
        match self {
            SpliceError::Parse { file, .. } => {
                // Tree-sitter errors: extract from node if available
                (Some(file.to_str()?), None, None)
            }
            SpliceError::AmbiguousReference { file, line, col, .. } => {
                (Some(file.as_str()), Some(*line), Some(*col))
            }
            SpliceError::SymbolNotFound { file, .. } => {
                (file.and_then(|f| f.to_str()).map(|s| (s, None, None)))
            }
            // ... handle all variants
            _ => (None, None, None),
        }
    }
}

// Tree-sitter integration: convert byte span to line/column
use tree_sitter::Node;

fn node_to_location(node: &Node, source: &str) -> (usize, usize) {
    let byte_offset = node.start_byte();
    let line = source[..byte_offset]
        .lines()
        .count();
    let column = source[..byte_offset]
        .lines()
        .last()
        .map(|l| l.len())
        .unwrap_or(0);
    (line, column)
}
```

### Pattern 4: Multi-Language Compiler Error Code Extraction (ALREADY PARTIALLY IMPLEMENTED)

**What:** Parse and extract error codes from different compiler outputs.

**When to use:** Validation gates run compilers (Rust, TypeScript, Python, etc.).

**Status:** Already implemented for Rust (E0XXX). TypeScript uses TSXXXX format.

**Example (TypeScript):**
```rust
// TypeScript error format: file.ts(line,col): error TSXXXX: message
// Example: test.ts(2,5): error TS1002: ...

fn parse_typescript_output(output: &str) -> Vec<CompilerError> {
    let re = Regex::new(
        r"^(.+)\((\d+),(\d+)\): error (TS\d+): (.+)$"
    ).unwrap();

    output.lines()
        .filter_map(|line| re.captures(line))
        .map(|caps| CompilerError {
            level: ErrorLevel::Error,
            file: caps[1].to_string(),
            line: caps[2].parse().unwrap(),
            column: caps[3].parse().unwrap(),
            code: Some(caps[4].to_string()),
            message: caps[5].to_string(),
            note: None,
        })
        .collect()
}
```

**Source:** Based on existing `parse_rust_style_output()` implementation (verified in src/validate/mod.rs:210-398) and TypeScript error format research.

### Anti-Patterns to Avoid

- **Don't use miette:** Too heavy, requires refactoring all error types. Codebase already has Diagnostic struct and ErrorCode infrastructure.
- **Don't use external markdown files for error docs:** Distribution complexity, file I/O overhead. Follow rustc's embedded pattern.
- **Don't write custom Levenshtein implementation:** strsim is mature, tested, optimized. Hand-rolling is reinventing the wheel.
- **Don't ignore line/column extraction:** CLI already has TODO for this (line 578). Location is required by CLI-16 success criterion.

## Don't Hand-Roll

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| Levenshtein distance | Custom edit distance calculation | `strsim::levenshtein()` | Well-tested, optimized, handles edge cases (Unicode, empty strings) |
| Fuzzy matching ranking | Custom similarity scoring | `strsim::damerau_levenshtein()` or `fuzzy-cmp` | Normalized scores (0.0-1.0), handles transpositions |
| Error code formatting | Manual string formatting | Existing `SpliceErrorCode` enum | Already implements SPL-E### format, severity(), hint() |
| Diagnostic structure | Custom error structs | Existing `Diagnostic` from src/error.rs | Already has all fields: tool, level, file, line, column, code, note, remediation |
| Compiler error parsing | Custom regex for each compiler | Extend `parse_rust_style_output()` pattern | Already handles E0XXX, location extraction, multi-line diagnostics |

**Key insight:** The codebase has 80% of the needed infrastructure. Don't rebuild - complete and enhance what exists.

## Common Pitfalls

### Pitfall 1: Incomplete Location Extraction

**What goes wrong:** Only some errors have line/column information, others just have file path or nothing.

**Why it happens:** Tree-sitter provides byte spans, not line/column. Some error variants don't track location.

**How to avoid:**
- Implement `node_to_line_column()` helper for tree-sitter nodes
- Add line/column fields to error variants that lack them
- Use `source.lines().count()` pattern to convert byte offset to line number

**Warning signs:** Error output shows "file.rs" instead of "file.rs:10:5", tests for location fail.

### Pitfall 2: Over-Loading Error Code Registry

**What goes wrong:** Trying to map every compiler error (E0XXX, TSXXXX) to Splice error codes.

**Why it happens:** Desire for "complete" error code coverage.

**How to avoid:**
- Splice error codes (SPL-E###) are for Splice-specific errors
- Compiler error codes (E0XXX, TSXXXX) are preserved in Diagnostic.code field
- `splice explain` explains SPL-E### codes, not compiler codes

**Warning signs:** SpliceErrorCode enum grows to hundreds of variants, mapping logic becomes complex.

### Pitfall 3: Ignoring Severity Levels

**What goes wrong:** All errors reported at same severity level (usually "error").

**Why it happens:** All current SpliceErrorCode variants return "error" from severity() (line 199).

**How to avoid:**
- Add Warning/Note variants to SpliceErrorCode enum
- Implement proper severity(): match self { WarningCode => "warning", ... }
- Use DiagnosticLevel::Warning/Note in validation gates

**Warning signs:** Users can't distinguish between blocking errors and warnings, severity field is always "error".

### Pitfall 4: Fuzzy Match Performance

**What goes wrong:** Symbol lookup becomes slow when computing Levenshtein distance against thousands of symbols.

**Why it happens:** O(n*m) complexity where n = target length, m = candidate count. Computing distance for all symbols in codebase.

**How to avoid:**
- Filter candidates by prefix match before computing distance
- Limit to top 5-10 suggestions
- Use max_distance threshold (e.g., 3 edits) to early-exclude distant matches
- Consider caching if symbol set is large

**Warning signs:** `splice find-symbol` takes >1s, CPU usage spikes during symbol lookup.

### Pitfall 5: Hard-Coded Error Documentation

**What goes wrong:** Error explanations become stale, hard to update, or inconsistent with actual errors.

**Why it happens:** Embedding documentation in code, not keeping it in sync with error definitions.

**How to avoid:**
- Keep error hints in SpliceErrorCode::hint() (single source of truth)
- `splice explain` can reference the same hint() method
- Document near the code: put explanation in module docs above error definition
- Test: ensure code in explain command matches variant in SpliceErrorCode

**Warning signs:** Running `splice explain SPL-E001` shows different hint than error output.

## Code Examples

### Fuzzy Symbol Suggestions with strsim

```rust
// Source: https://docs.rs/strsim/ (HIGH confidence)

use strsim::levenshtein;

/// Find symbols similar to target within max edit distance.
/// Returns up to 5 suggestions sorted by similarity.
pub fn suggest_similar_symbols(
    target: &str,
    all_symbols: &[String],
    max_distance: usize
) -> Vec<String> {
    let mut suggestions: Vec<_> = all_symbols
        .iter()
        .filter(|symbol| {
            // Quick prefix check to avoid expensive distance calc
            symbol.chars().next() == target.chars().next()
        })
        .map(|symbol| {
            let dist = levenshtein(target, symbol);
            (symbol.clone(), dist)
        })
        .filter(|(_, dist)| *dist <= max_distance && *dist > 0)
        .collect();

    suggestions.sort_by_key(|(_, dist)| *dist);
    suggestions.truncate(5);
    suggestions.into_iter()
        .map(|(name, _)| name)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_suggest_similar_symbols() {
        let symbols = vec![
            "foo".to_string(),
            "foobar".to_string(),
            "bar".to_string(),
            "baz".to_string(),
        ];

        let suggestions = suggest_similar_symbols("fooo", &symbols, 2);
        assert_eq!(suggestions, vec!["foo".to_string()]);
    }
}
```

### Enhanced Error with Location and Suggestions

```rust
// Enhanced SymbolNotFound with location and suggestions
use crate::error_codes::SpliceErrorCode;
use crate::graph::CodeGraph;

pub fn find_symbol_or_suggest(
    graph: &CodeGraph,
    name: &str,
    file: Option<&Path>
) -> Result<Symbol> {
    match graph.find_symbol(name, file) {
        Ok(symbol) => Ok(symbol),
        Err(_) => {
            // Get all symbol names from graph
            let all_symbols = graph.all_symbol_names();

            // Find similar names
            let suggestions = suggest_similar_symbols(name, &all_symbols, 3);

            let hint = if suggestions.is_empty() {
                format!("Symbol '{}' not found. Run `splice ingest` to index the codebase.", name)
            } else {
                format!("Did you mean: {}?", suggestions.join(", "))
            };

            Err(SpliceError::SymbolNotFound {
                message: format!("Symbol '{}' not found", name),
                symbol: name.to_string(),
                file: file.map(|p| p.to_path_buf()),
                hint,
            })
        }
    }
}
```

### TypeScript Error Code Parsing

```rust
// Extend src/validate/mod.rs pattern to TypeScript
use regex::Regex;

fn parse_typescript_output(output: &str) -> Vec<CompilerError> {
    // TypeScript format: file.ts(line,col): error TSXXXX: message
    let re = Regex::new(
        r"^(.+?)\((\d+),(\d+)\): error (TS\d+): (.+)$"
    ).unwrap();

    let mut errors = Vec::new();

    for line in output.lines() {
        if let Some(caps) = re.captures(line) {
            errors.push(CompilerError {
                level: ErrorLevel::Error,
                file: caps[1].to_string(),
                line: caps[2].parse().unwrap(),
                column: caps[3].parse().unwrap(),
                code: Some(caps[4].to_string()),  // "TS1002", "TS2304", etc.
                message: caps[5].to_string(),
                note: None,
            });
        }
    }

    errors
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_typescript_error() {
        let output = r#"
test.ts(2,5): error TS1002: Unterminated string literal
another.ts(10,12): error TS2304: Cannot find name 'foo'
"#;

        let errors = parse_typescript_output(output);
        assert_eq!(errors.len(), 2);

        assert_eq!(errors[0].file, "test.ts");
        assert_eq!(errors[0].line, 2);
        assert_eq!(errors[0].column, 5);
        assert_eq!(errors[0].code, Some("TS1002".to_string()));
    }
}
```

### Line/Column from Tree-Sitter Node

```rust
// Convert tree-sitter byte offset to line:column
use tree_sitter::Node;

pub fn byte_offset_to_line_column(source: &str, byte_offset: usize) -> (usize, usize) {
    let text = &source[..byte_offset.min(source.len())];

    let line = text.lines().count();  // 1-based
    let column = text.lines()
        .last()
        .map(|l| l.chars().count())
        .unwrap_or(0);  // 0-based

    (line, column)
}

pub fn node_location(node: &Node, source: &str) -> (usize, usize) {
    byte_offset_to_line_column(source, node.start_byte())
}

// Usage in error creation
let node = parse_result.node;
let (line, column) = node_location(&node, source_code);

Err(SpliceError::Parse {
    file: path.to_path_buf(),
    message: "Invalid syntax".to_string(),
})
.with_location(line, column)  // New method to add location context
```

## State of the Art

| Old Approach | Current Approach | When Changed | Impact |
|--------------|------------------|--------------|--------|
| Unstructured error messages | Structured Diagnostic with codes, severity, locations | 2024-2025 (miette, ariadne) | LLMs can parse and act on errors |
| Generic "syntax error" | Specific error codes (E0XXX, TSXXXX) with explanations | 2010s (rustc --explain) | Users can self-service error resolution |
| No "did you mean" suggestions | Fuzzy matching with Levenshtein distance | 2010s (ripgrep, git) | Faster typo recovery, better UX |
| External error docs | Embedded documentation in binary | 2010s (rustc pattern) | Simpler distribution, offline access |

**Deprecated/outdated:**
- `color-eyre`: Replaced by miette for structured diagnostics
- `anyhow` for libraries: Use thiserror + Diagnostic (miette) instead
- Hand-rolled error formatting: Use Diagnostic trait and ReportHandler

**Standard for 2026:**
- Structured diagnostics with codes, severity, location, hints
- "did you mean" suggestions for not-found errors
- `tool --explain <code>` pattern for documentation
- Preserving compiler error codes (E0XXX, TSXXXX) in output

## Open Questions

1. **Error documentation storage format**
   - What we know: rustc uses embedded documentation, miette uses derive macro attributes
   - What's unclear: Whether to use static strings, macro attributes, or build-time codegen
   - Recommendation: Start with static &str in match statement (simplest, matches rustc pattern)

2. **Severity classification criteria**
   - What we know: rustc distinguishes error/warning/note/help based on impact
   - What's unclear: When Splice should emit warnings vs errors (e.g., parse validation = error, but what about ambiguous symbol?)
   - Recommendation: Use error for blocking issues, warning for non-blocking issues, note for informational context

3. **Symbol suggestion performance at scale**
   - What we know: strsim is O(n*m), prefixes can filter candidates
   - What's unclear: How many symbols in "typical" codebase, whether caching is needed
   - Recommendation: Measure performance with 1k, 10k, 100k symbols, add caching if needed

4. **Multi-language compiler error formats**
   - What we know: Rust (E0XXX), TypeScript (TSXXXX), Python (no error codes)
   - What's unclear: Formats for C/C++ (gcc/clang), Java, Go
   - Recommendation: Implement Rust/TypeScript first (already have tests), add others as needed

## Sources

### Primary (HIGH confidence)

- [strsim - Rust string similarity library](https://docs.rs/strsim/) - Official documentation for Levenshtein distance algorithms
- [miette GitHub repository](https://github.com/zkat/miette) - Diagnostic library patterns (structural reference, not for adoption)
- [Rust Error Codes Index](https://doc.rust-lang.org/error-index.html) - Official rustc error code reference
- [Error codes - Rust Compiler Development Guide](https://rustc-dev-guide.rust-lang.org/diagnostics/error-codes.html) - rustc --explain implementation details
- [ts-error-parser GitHub](https://github.com/yuichkun/ts-error-parser) - TypeScript error format documentation (verified via webReader)

### Secondary (MEDIUM confidence)

- [RapidFuzz strsim-rs](https://github.com/rapidfuzz/strsim-rs) - Alternative string similarity implementation
- [fuzzy-cmp crate](https://crates.io/crates/fuzzy-cmp/versions) - Recent 2026 fuzzy matching library
- [The Rover explain Command (Apollo GraphQL)](https://www.apollographql.com/docs/rover/commands/explain) - CLI explain pattern example
- [Implementing Levenshtein Distance in Go CLIs](https://prabeshthapa.medium.com/from-frustrating-typos-to-smart-suggestions-implementing-levenshtein-distance-in-go-clis-3708c0a3b4e1) - Pattern reference for "did you mean" implementations

### Tertiary (LOW confidence)

- [did-you-mean crate](https://lib.rs/data-structures) - Mentioned in search results but not verified
- WebSearch results for Python, gcc/clang error formats - No definitive error code standards found

### Codebase Verification (HIGH confidence)

- `/home/feanor/Projects/splice/src/error_codes.rs` - Verified existing SpliceErrorCode enum with 20+ codes
- `/home/feanor/Projects/splice/src/error.rs` - Verified Diagnostic struct and SpliceError enum
- `/home/feanor/Projects/splice/src/validate/mod.rs` - Verified parse_rust_style_output() implementation
- `/home/feanor/Projects/splice/src/cli/mod.rs` - Verified TODO at line 578 for line/column extraction

## Metadata

**Confidence breakdown:**
- Standard stack: HIGH - strsim is mature, widely-used, documented on official crates.io
- Architecture: HIGH - Based on verified existing codebase patterns and rustc/cargo precedents
- Pitfalls: HIGH - Derived from existing TODO items and common error-handling anti-patterns

**Research date:** 2026-01-22
**Valid until:** 2026-02-22 (30 days - stable ecosystem)

**Next steps for planner:**
1. Plan strsim integration for fuzzy symbol suggestions
2. Plan `splice explain` command following rustc pattern
3. Plan line/column extraction completion (TODO at line 578)
4. Plan TypeScript error code parser extension
5. Plan severity level expansion beyond just "error"
