# Phase 11: Rich Span Core - Research

**Researched:** 2026-01-22
**Domain:** Rust span metadata enrichment for LLM consumption
**Confidence:** HIGH

## Summary

This phase adds rich metadata to span output for LLM consumption, including:
- **Context extraction** (before/selected/after lines)
- **Semantic kind detection** (function, variable, parameter, etc.)
- **Language detection** (from file extension/tree-sitter)
- **Checksums** (span-level and file-level SHA-256)
- **Error codes** (structured with severity, location, hints)

**Key finding:** ZERO new dependencies needed. All required functionality exists in the codebase:
- `ropey 1.6` for efficient line-based context extraction
- `tree-sitter 0.22` for semantic kind detection
- `sha2 0.10` for checksums (already implemented)
- `serde 1.0` for JSON serialization

**Primary recommendation:** Extend existing `SpanResult` struct with optional fields using additive-only schema to avoid breaking 311 existing tests.

## Standard Stack

### Core Dependencies (Already Present)
| Library | Version | Purpose | Why Standard |
|---------|---------|---------|--------------|
| `ropey` | 1.6 | Efficient line/col calculations, context extraction | Industry standard for text editing, already used throughout codebase |
| `tree-sitter` | 0.22 | AST parsing, semantic kind detection | Already integrated for all 7 supported languages |
| `sha2` | 0.10 | SHA-256 checksums | Already implemented in `checksum.rs` |
| `serde` | 1.0 | JSON serialization with optional fields | Enables additive schema evolution |

### Language-Specific Parsers (Already Present)
| Parser | Version | Semantic Kinds Detected |
|--------|---------|------------------------|
| `tree-sitter-rust` | 0.21 | Function, Struct, Enum, Trait, Impl, Module, TypeAlias, Const, Static |
| `tree-sitter-python` | 0.21 | Function, Class, Method, Variable |
| `tree-sitter-c` | 0.21 | Function, Struct, Enum, Field, Variable |
| `tree-sitter-cpp` | 0.21 | Function, Class, Struct, Namespace, Enum, Method, Field, TemplateFunction, TemplateClass |
| `tree-sitter-java` | 0.21 | Class, Interface, Enum, Method, Constructor, Field |
| `tree-sitter-javascript` | 0.21 | Function, Class, Method, Variable, ArrowFunction |
| `tree-sitter-typescript` | 0.21 | Interface, TypeAlias, Enum, Namespace, Function, Class, Method, Variable, ArrowFunction |

### Alternatives Considered
| Instead of | Could Use | Tradeoff |
|------------|-----------|----------|
| ropey | lines-from-bytes manual calculation | Ropey handles UTF-8 grapheme boundaries correctly, manual approach is error-prone |
| Existing checksum functions | New checksum code | Duplicates tested code, inconsistencies |

**Installation:** No new dependencies required. All dependencies already present in `Cargo.toml`.

## Architecture Patterns

### Recommended Project Structure

```
src/
├── context.rs              # NEW - Context extraction using ropey
├── ingest/
│   └── semantic_kind.rs    # NEW - Unified semantic kind detection
├── output.rs               # MODIFY - Extend SpanResult with new fields
├── checksum.rs             # EXTEND - Expose checksum helpers
├── error_codes.rs          # NEW - Error code registry
└── cli/
    └── mod.rs              # MODIFY - Add --context-lines argument
```

### Pattern 1: Context Extraction with Ropey

**What:** Extract `before`, `selected`, `after` line arrays from byte spans.

**When to use:** All span output operations (query, patch, delete, resolve).

**Location:** `src/context.rs` (NEW)

**Example:**

```rust
// Source: Existing ropey usage in src/ingest/rust.rs:160-164
use ropey::Rope;
use std::path::Path;

/// Context lines surrounding a span.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpanContext {
    /// Lines before the span (default: 3)
    pub before: Vec<String>,
    /// Lines within the span
    pub selected: Vec<String>,
    /// Lines after the span (default: 3)
    pub after: Vec<String>,
}

/// Extract context lines for a byte span.
///
/// # Arguments
///
/// * `path` - File path
/// * `byte_start` - Start byte offset
/// * `byte_end` - End byte offset
/// * `context_lines` - Number of context lines before/after (default: 3)
///
/// # Returns
///
/// * `Ok(SpanContext)` - Extracted context with UTF-8 byte offsets
/// * `Err(SpliceError)` - If file cannot be read or span is invalid
pub fn extract_context(
    path: &Path,
    byte_start: usize,
    byte_end: usize,
    context_lines: usize,
) -> Result<SpanContext> {
    use crate::error::{Result, SpliceError};

    // Read file
    let contents = std::fs::read(path)
        .map_err(|e| SpliceError::IoContext {
            context: format!("Failed to read file for context extraction: {}", path.display()),
            source: e,
        })?;

    // Create Rope for efficient line operations
    let rope = Rope::from_str(std::str::from_utf8(&contents)?);

    // Convert byte offsets to line numbers
    let start_line = rope.byte_to_line(byte_start);  // 0-based
    let end_line = rope.byte_to_line(byte_end);      // 0-based

    // Calculate context boundaries
    let context_start = start_line.saturating_sub(context_lines);
    let context_end = (end_line + context_lines + 1).min(rope.len_lines());

    // Extract before, selected, after lines
    let before: Vec<String> = (context_start..start_line)
        .map(|i| rope.line(i).to_string())
        .collect();

    let selected: Vec<String> = (start_line..=end_line)
        .map(|i| rope.line(i).to_string())
        .collect();

    let after: Vec<String> = (end_line + 1..context_end)
        .map(|i| rope.line(i).to_string())
        .collect();

    Ok(SpanContext { before, selected, after })
}
```

**Performance optimization:** Cache `Rope` instance per file for multiple spans in same file (e.g., query results).

### Pattern 2: Semantic Kind Detection

**What:** Map language-specific tree-sitter node types to standardized semantic kinds.

**When to use:** All span operations that return symbol information.

**Location:** `src/ingest/semantic_kind.rs` (NEW)

**Example:**

```rust
// Source: Existing RustSymbolKind in src/ingest/rust.rs:256-275
use crate::ingest::detect::Language;

/// Standardized semantic kinds across all languages.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SemanticKind {
    /// Function or method definition
    Function,
    /// Class, struct, or interface definition
    Type,
    /// Variable, field, or parameter
    Variable,
    /// Module or namespace
    Module,
    /// Enum or enumerator
    Enum,
    /// Trait or interface
    Trait,
    /// Type alias
    TypeAlias,
    /// Constant or static
    Constant,
    /// Unknown kind (fallback)
    Unknown,
}

/// Detect semantic kind from tree-sitter node type and language.
///
/// # Arguments
///
/// * `node_type` - Tree-sitter node kind (e.g., "function_item", "struct_item")
/// * `language` - Programming language
///
/// # Returns
///
/// Standardized `SemanticKind` or `SemanticKind::Unknown` for unmapped types
pub fn detect_semantic_kind(node_type: &str, language: Language) -> SemanticKind {
    match language {
        Language::Rust => match node_type {
            "function_item" => SemanticKind::Function,
            "struct_item" | "enum_item" => SemanticKind::Type,
            "impl_item" => SemanticKind::Trait,
            "mod_item" => SemanticKind::Module,
            "trait_item" => SemanticKind::Trait,
            "type_item" => SemanticKind::TypeAlias,
            "const_item" | "static_item" => SemanticKind::Constant,
            _ => SemanticKind::Unknown,
        },
        Language::Python => match node_type {
            "function_definition" => SemanticKind::Function,
            "class_definition" => SemanticKind::Type,
            "expression_statement" => SemanticKind::Variable,
            _ => SemanticKind::Unknown,
        },
        Language::JavaScript | Language::TypeScript => match node_type {
            "function_declaration" | "function_expression" | "arrow_function" => SemanticKind::Function,
            "class_declaration" | "class_expression" | "interface_declaration" => SemanticKind::Type,
            "variable_declaration" => SemanticKind::Variable,
            "enum_declaration" => SemanticKind::Enum,
            "type_alias_declaration" => SemanticKind::TypeAlias,
            "namespace_declaration" => SemanticKind::Module,
            _ => SemanticKind::Unknown,
        },
        Language::Java => match node_type {
            "method_declaration" => SemanticKind::Function,
            "class_declaration" | "interface_declaration" => SemanticKind::Type,
            "field_declaration" => SemanticKind::Variable,
            "enum_declaration" => SemanticKind::Enum,
            _ => SemanticKind::Unknown,
        },
        Language::C | Language::Cpp => match node_type {
            "function_definition" => SemanticKind::Function,
            "struct_specifier" | "class_specifier" => SemanticKind::Type,
            "declaration" => SemanticKind::Variable,
            "enum_specifier" => SemanticKind::Enum,
            "namespace_definition" => SemanticKind::Module,
            _ => SemanticKind::Unknown,
        },
    }
}
```

### Pattern 3: Extending SpanResult Additively

**What:** Add optional fields to `SpanResult` without breaking existing tests.

**When to use:** All span output operations.

**Location:** `src/output.rs` (MODIFY lines 234-273)

**Example:**

```rust
// Source: Existing SpanResult in src/output.rs:234-273
/// Unified span result with byte and line/column information.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpanResult {
    // ... EXISTING FIELDS (lines 237-272) ...

    /// NEW: Context lines before/selected/after (optional)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub context: Option<SpanContext>,

    /// NEW: Standardized semantic kind (optional)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub semantic_kind: Option<String>,  // Serialized as lowercase string

    /// NEW: Programming language (optional)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub language: Option<String>,  // e.g., "rust", "python"

    /// NEW: Checksum of span content before modification (optional)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub checksum_before: Option<String>,  // SHA-256 hex

    /// NEW: Checksum of entire file before modification (optional)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub file_checksum_before: Option<String>,  // SHA-256 hex

    /// NEW: Error code with severity, location, hint (optional)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error_code: Option<ErrorCode>,
}

/// NEW: Error code structure for diagnostics.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ErrorCode {
    /// Error code (e.g., "SPL-E001")
    pub code: String,
    /// Severity level (error/warning/note)
    pub severity: String,
    /// Precise location (file:line:column)
    pub location: String,
    /// What to do hint
    pub hint: String,
}

impl SpanResult {
    /// NEW: Add context to span result.
    pub fn with_context(mut self, context: SpanContext) -> Self {
        self.context = Some(context);
        self
    }

    /// NEW: Add semantic kind and language.
    pub fn with_semantic_info(mut self, kind: SemanticKind, language: Language) -> Self {
        self.semantic_kind = Some(kind.as_str().to_string());
        self.language = Some(language.as_str().to_string());
        self
    }

    /// NEW: Add checksums.
    pub fn with_checksums(mut self, checksum_before: String, file_checksum_before: String) -> Self {
        self.checksum_before = Some(checksum_before);
        self.file_checksum_before = Some(file_checksum_before);
        self
    }

    /// NEW: Add error code.
    pub fn with_error_code(mut self, error_code: ErrorCode) -> Self {
        self.error_code = Some(error_code);
        self
    }
}
```

### Anti-Patterns to Avoid

- **Breaking existing tests:** Do NOT change required fields in `SpanResult`. Always use `#[serde(skip_serializing_if = "Option::is_none")]` for new fields.
- **Re-parsing files:** Do NOT parse the same file multiple times for different spans. Cache the `Rope` instance.
- **Hand-rolled UTF-8 handling:** Do NOT manually calculate line/col offsets. Use `ropey` API.
- **Inconsistent checksums:** Do NOT use different hash algorithms. Always use SHA-256 via existing `checksum::checksum_span()` and `checksum::checksum_file()`.
- **Missing language detection:** Do NOT assume Rust language. Always detect from file extension using `detect_language()`.

## Don't Hand-Roll

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| Line/col calculations | Manual byte-to-line conversion | `ropey::Rope::byte_to_line()`, `ropey::line()` | Handles UTF-8 grapheme clusters, tested throughout codebase |
| Checksums | New SHA-256 implementation | `checksum::checksum_span()`, `checksum::checksum_file()` | Already implemented with SHA-256, tested, consistent |
| Language detection | File extension parsing logic | `detect_language()` from `src/ingest/detect.rs` | Table-driven, handles all 7 languages, tested |
| Semantic kind mapping | Duplicate per-language enums | `detect_semantic_kind()` in new `semantic_kind.rs` | Unified mapping, avoids code duplication |
| Error code formatting | Printf-style string construction | Structured `ErrorCode` type + `serde` | Consistent JSON output, LLM-consumable |

**Key insight:** The codebase already has production-ready implementations for all required functionality. The phase is about integration and exposure, not new algorithms.

## Common Pitfalls

### Pitfall 1: Breaking Existing Tests with Schema Changes

**What goes wrong:** Adding new required fields to `SpanResult` breaks 311 existing tests that expect the old schema.

**Why it happens:** Serde deserialization fails if JSON doesn't contain new required fields.

**How to avoid:**
1. Always add fields as `Option<T>` with `#[serde(skip_serializing_if = "Option::is_none")]`
2. Provide builder methods (`with_context()`, `with_semantic_info()`) for optional population
3. Run full test suite after changes: `cargo test`

**Warning signs:** Test failures in `patch_tests`, `resolve_tests`, `integration_refactor` after schema changes.

### Pitfall 2: Large File Performance Degradation

**What goes wrong:** Context extraction is slow for files >32KB when re-parsing for each span.

**Why it happens:** Creating a new `Rope` instance for each span involves re-reading the file.

**How to avoid:**
1. Cache `Rope` instance per file when extracting context for multiple spans
2. Use span-based batching: extract all contexts for a file in one pass
3. Consider lazy loading: only extract context when JSON output is requested

**Warning signs:** Slow query results for files with many symbols (>100 spans).

### Pitfall 3: Semantic Kind Edge Cases

**What goes wrong:** Anonymous functions, macros, and language-specific constructs return `Unknown`.

**Why it happens:** Tree-sitter node types vary across languages for edge cases.

**How to avoid:**
1. Start with known node types (see "Standard Stack" table above)
2. Use `SemanticKind::Unknown` as safe fallback
3. Log unknown node types for future taxonomy extension
4. Test with real codebases (Rust standard library, Python projects, etc.)

**Warning signs:** Many spans with `"semantic_kind": "unknown"` in output.

### Pitfall 4: Checksum Calculation Timing

**What goes wrong:** Checksums calculated after file modification are incorrect.

**Why it happens:** File content changes between span identification and checksum calculation.

**How to avoid:**
1. Calculate checksums BEFORE applying patches
2. Use `checksum_span(path, start, end)` with original byte offsets
3. Store checksums in `SpanResult` before any modifications
4. Verify checksums match when applying patches (race condition protection)

**Warning signs:** Patch failures due to checksum mismatches, "File externally modified" errors.

### Pitfall 5: Inconsistent UTF-8 Offset Handling

**What goes wrong:** Context line offsets don't match span byte offsets.

**Why it happens:** Mixing byte offsets with character offsets (ropey uses char indices internally).

**How to avoid:**
1. Always use `rope.byte_to_line()` and `rope.line_to_byte()` for conversions
2. Never mix `byte_start`/`byte_end` with `rope.char_to_line()` (wrong!)
3. Verify context lines by checking byte ranges match expected spans
4. Test with multi-byte UTF-8 characters (emoji, CJK characters)

**Warning signs:** Context lines don't contain the expected span content.

## Code Examples

### Example 1: Context Extraction with Error Handling

```rust
// Source: Based on existing ropey patterns in src/ingest/rust.rs:51
use crate::context::{extract_context, SpanContext};
use crate::error::Result;
use std::path::Path;

fn main() -> Result<()> {
    let path = Path::new("src/main.rs");
    let byte_start = 100;
    let byte_end = 200;
    let context_lines = 3;

    let context = extract_context(path, byte_start, byte_end, context_lines)?;

    println!("Before: {} lines", context.before.len());
    println!("Selected: {} lines", context.selected.len());
    println!("After: {} lines", context.after.len());

    Ok(())
}
```

### Example 2: Semantic Kind Detection

```rust
// Source: Based on existing RustSymbolKind in src/ingest/rust.rs:256
use crate::ingest::semantic_kind::{detect_semantic_kind, SemanticKind};
use crate::ingest::detect::Language;

fn main() {
    let node_type = "function_item";
    let language = Language::Rust;

    let kind = detect_semantic_kind(node_type, language);
    assert_eq!(kind, SemanticKind::Function);

    println!("Semantic kind: {}", kind.as_str());  // "function"
}
```

### Example 3: Checksum Calculation

```rust
// Source: Existing checksum functions in src/checksum.rs:64-88
use crate::checksum::{checksum_span, checksum_file};
use crate::error::Result;
use std::path::Path;

fn main() -> Result<()> {
    let path = Path::new("src/main.rs");

    // Compute file-level checksum
    let file_checksum = checksum_file(path)?;
    println!("File checksum: {}", file_checksum.as_hex());

    // Compute span-level checksum
    let span_checksum = checksum_span(path, 100, 200)?;
    println!("Span checksum: {}", span_checksum.as_hex());

    Ok(())
}
```

### Example 4: Rich Span Result Construction

```rust
// Source: Extending existing SpanResult in src/output.rs:275-333
use crate::output::{SpanResult, ErrorCode};
use crate::context::SpanContext;
use crate::ingest::semantic_kind::SemanticKind;
use crate::ingest::detect::Language;

fn main() {
    let mut span = SpanResult::from_byte_span("src/main.rs".to_string(), 100, 200);

    // Add context
    let context = SpanContext {
        before: vec!["line 1".to_string()],
        selected: vec!["line 2".to_string()],
        after: vec!["line 3".to_string()],
    };
    span = span.with_context(context);

    // Add semantic info
    span = span.with_semantic_info(SemanticKind::Function, Language::Rust);

    // Add checksums
    span = span.with_checksums(
        "abc123...".to_string(),
        "def456...".to_string(),
    );

    // Add error code
    let error_code = ErrorCode {
        code: "SPL-E001".to_string(),
        severity: "error".to_string(),
        location: "src/main.rs:10:5".to_string(),
        hint: "Add missing semicolon".to_string(),
    };
    span = span.with_error_code(error_code);

    // Serialize to JSON
    let json = serde_json::to_string_pretty(&span).unwrap();
    println!("{}", json);
}
```

### Example 5: CLI Integration for --context-lines

```rust
// Source: Based on existing CLI structure in src/cli/mod.rs:76-173
use clap::Parser;

#[derive(clap::Subcommand, Debug)]
pub enum Commands {
    Query {
        /// ... EXISTING FIELDS ...

        /// NEW: Number of context lines before/after spans (default: 3)
        #[arg(long, default_value = "3")]
        context_lines: usize,

        /// ... REMAINING FIELDS ...
    },

    Patch {
        /// ... EXISTING FIELDS ...

        /// NEW: Number of context lines before/after spans (default: 3)
        #[arg(long, default_value = "3")]
        context_lines: usize,

        /// ... REMAINING FIELDS ...
    },
}
```

## State of the Art

### As of Phase 10 (v2.0 Documentation)

| Feature | Status | Implementation |
|---------|--------|----------------|
| Span output with byte/line/col | ✅ Complete | `SpanResult` in `src/output.rs` |
| SHA-256 checksums | ✅ Complete | `checksum_span()`, `checksum_file()` in `src/checksum.rs` |
| Language detection | ✅ Complete | `detect_language()` in `src/ingest/detect.rs` |
| Semantic kinds (per-language) | ✅ Complete | `RustSymbolKind`, `PythonSymbolKind`, etc. |
| Tree-sitter integration | ✅ Complete | All 7 languages supported |
| Structured JSON output | ✅ Complete | `OperationResult` with serde |

### Phase 11 Additions

| Feature | Status | Implementation |
|---------|--------|----------------|
| Context extraction (before/selected/after) | 🔨 In Progress | NEW: `src/context.rs` |
| Unified semantic kinds | 🔨 In Progress | NEW: `src/ingest/semantic_kind.rs` |
| Checksums in JSON output | 🔨 In Progress | MODIFY: `SpanResult` fields |
| Error codes with hints | 🔨 In Progress | NEW: `src/error_codes.rs` |
| CLI --context-lines flag | 🔨 In Progress | MODIFY: `src/cli/mod.rs` |

**Deprecated/outdated:** None. This is additive-only functionality.

## Open Questions

### Question 1: Error Code Registry Design

**What we know:**
- Error codes need format "SPL-E001" (SPL-{E|W|N}-{number})
- Need severity (error/warning/note), location, hint
- No existing error code system in codebase

**What's unclear:**
- Should error codes be auto-generated or manually assigned?
- How many error codes should be defined in Phase 11 (seed set vs. full taxonomy)?

**Recommendation:** Start with 10-20 seed error codes for common scenarios (parse errors, invalid spans, checksum mismatches). Auto-generate codes from error kinds using format: `{kind}-{severity}-{number}`.

### Question 2: Context Extraction for Multi-Span Results

**What we know:**
- Query operations return multiple spans from the same file
- Each span needs context extracted
- Large files (>32KB) may have performance issues

**What's unclear:**
- Should context extraction batch per-file or per-span?
- Should context be optional in JSON (only when requested)?

**Recommendation:** Extract context per-span initially (simplest), optimize to per-file caching if performance tests show >100ms slowdown for 100 spans. Make context opt-in via CLI flag to avoid overhead for non-LLLM use cases.

### Question 3: Test Strategy for Additive Schema

**What we know:**
- 311 existing tests must not break
- New fields need test coverage
- JSON serialization/deserialization needs verification

**What's unclear:**
- Should new tests verify backward compatibility (old JSON parses into new struct)?
- Should tests cover all 7 languages for semantic kind detection?

**Recommendation:** Add `context_tests.rs` with ~20 tests covering:
- Context extraction (empty files, single line, multi-byte UTF-8)
- Semantic kind detection (all 7 languages, unknown node types)
- Checksum integration (before/after patches)
- Error code formatting
- Backward compatibility ( deserialize old JSON into new `SpanResult`)

## Sources

### Primary (HIGH confidence)

- `/home/feanor/Projects/splice/src/checksum.rs` - Checksum implementation (SHA-256, verified)
- `/home/feanor/Projects/splice/src/output.rs` - `SpanResult` struct, serde serialization
- `/home/feanor/Projects/splice/src/ingest/detect.rs` - Language detection (7 languages)
- `/home/feanor/Projects/splice/src/ingest/rust.rs` - Tree-sitter integration, `RustSymbolKind`
- `/home/feanor/Projects/splice/Cargo.toml` - Dependency versions (ropey 1.6, tree-sitter 0.22, sha2 0.10)

### Secondary (MEDIUM confidence)

- `/home/feanor/Projects/splice/src/ingest/python.rs` - Python semantic kinds
- `/home/feanor/Projects/splice/src/ingest/cpp.rs` - C++ semantic kinds
- `/home/feanor/Projects/splice/src/ingest/java.rs` - Java semantic kinds
- `/home/feanor/Projects/splice/src/ingest/javascript.rs` - JavaScript semantic kinds
- `/home/feanor/Projects/splice/src/ingest/typescript.rs` - TypeScript semantic kinds
- `/home/feanor/Projects/splice/src/patch/pattern.rs` - Ropey API usage patterns
- `/home/feanor/Projects/splice/src/resolve/references/rust.rs` - Ropey line/col calculations

### Tertiary (LOW confidence)

- None. All findings verified from source code.

## Metadata

**Confidence breakdown:**
- Standard stack: **HIGH** - Verified from `Cargo.toml` and source code
- Architecture patterns: **HIGH** - Based on existing patterns in codebase (ropey usage, checksum functions)
- Pitfalls: **HIGH** - Inferred from existing test structure (311 tests) and UTF-8 handling patterns

**Research date:** 2026-01-22
**Valid until:** 2026-02-22 (30 days - stable dependencies, additive-only changes)

**Baseline test count:** 311 tests passing (verified via `cargo test`)
