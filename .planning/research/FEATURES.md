# Rich Span Extensions - Feature Categories

**Project:** Splice v2.2
**Researched:** 2026-01-22
**Focus:** LLM-first UX improvements and cross-tool alignment
**Overall Confidence:** HIGH

---

## Table Stakes

### Context Extension

**Must-have for LLM consumption:**

- **Before/After Lines (default: 3)** — Surrounding context lines above and below the span
  - **Complexity:** LOW
  - **Why essential:** LLMs need context to make accurate edits. Without context, LLMs hallucinate surrounding structure, produce malformed patches, or break AST boundaries. Industry standard from VS Code, IntelliJ, and AI code tools.
  - **Dependencies:** Builds on existing span coordinates (line_start, line_end)
  - **LLM use case:** "Replace this function body" → LLM sees imports, the function signature, and what follows to understand scope

- **Selected Lines** — The actual content within the span for verification
  - **Complexity:** LOW
  - **Why essential:** LLMs need to verify they're editing the correct code. Prevents "edit the wrong thing" errors when spans shift due to race conditions.
  - **Dependencies:** Builds on existing span coordinates and byte offsets
  - **LLM use case:** LLM can verify: "This looks like the function I intended to patch"

- **Configurable Line Count** — `--context-lines <n>` flag to control context window
  - **Complexity:** LOW
  - **Why essential:** Different operations need different context sizes. Single-line edits need 3-5 lines; module-level refactors need 20+ lines.
  - **Dependencies:** Builds on context extension
  - **LLM use case:** Adjust context based on token budget and operation complexity

**Sources:**
- [VS Code Semantic Highlight Guide](https://code.visualstudio.com/api/language-extensions/semantic-highlight-guide) - Industry standard for context in code tools
- [Addy Osmani's LLM Coding Workflow 2026](https://medium.com/@addyosmani/my-llm-coding-workflow-going-into-2026-52fe1681325e) - "carefully managing your context" for LLM code editing
- [Nuanced Call Graph Context Layer](https://www.nuanced.dev/blog/python-open-source-launch) - Context provision for AI coding tools

### Semantic Kind Detection

**Must-have for AST-aware operations:**

- **Per-Language Symbol Kinds** — Standardized semantic kinds across 7 supported languages
  - **Complexity:** MEDIUM
  - **Why essential:** Enables symbol-level operations instead of text-level operations. LLMs can infer correct patch structure (function body vs entire function). Prevents breaking AST boundaries.
  - **Dependencies:** Builds on existing tree-sitter parsing infrastructure
  - **LLM use case:** "Patch this function" → LLM knows to preserve signature, replace body only

**Supported semantic kinds by language:**

| Language | Required Kinds | Complexity |
|----------|----------------|------------|
| Rust | function, method, struct, enum, trait, impl, mod, const, static, type, macro | MEDIUM |
| Python | function, method, class, async_function, decorator, module, variable | MEDIUM |
| TypeScript | function, method, class, interface, type, enum, namespace, variable | MEDIUM |
| JavaScript | function, method, class, variable, statement | LOW |
| C/C++ | function, method, struct, enum, union, namespace, variable | MEDIUM |
| Java | function, method, class, interface, enum, variable | MEDIUM |

**Sources:**
- [Tree-sitter AST Parsing at Scale (40 Languages)](https://www.dropstone.io/blog/ast-parsing-tree-sitter-40-languages) - Tree-sitter as parsing backbone for semantic analysis
- [MCP Server Tree-sitter Features](https://github.com/wrale/mcp-server-tree-sitter/blob/main/FEATURES.md) - Tree-sitter AST parsing capabilities
- [How to Chunk Code at Semantic Boundaries](https://stackoverflow.com/questions/79849934/how-to-chunk-code-at-semantic-boundaries-when-a-single-ast-node-exceeds-the-chun) - Semantic boundary detection for LLM code indexing

### Checksums for Race Condition Protection

**Must-have for production safety:**

- **Content Checksum Before** — SHA-256 hash of span content before patch
  - **Complexity:** LOW
  - **Why essential:** Prevents applying patches to shifted spans. If code changed after Magellan query, checksum mismatch catches it. Same pattern used by Google/Meta for automated refactoring.
  - **Dependencies:** Builds on existing SHA-256 checksum infrastructure
  - **LLM use case:** Prevents "I patched the wrong thing because someone else edited the file"

- **File Checksum Before** — SHA-256 hash of entire file before operation
  - **Complexity:** LOW
  - **Why essential:** Detects external modifications between query and patch. Prevents corrupting files with race conditions.
  - **Dependencies:** Builds on existing file checksum infrastructure
  - **LLM use case:** Multi-agent scenarios where multiple LLMs edit the same codebase

**Splice validation flow:**
1. Read file, compute SHA-256 → Compare with `file_checksum_before`
2. Read span, compute SHA-256 → Compare with `checksum_before`
3. If mismatch → reject patch (file/symbol shifted)
4. Apply patch
5. Compute `checksum_after` for verification

**Sources:**
- [Security-Focused Guide for AI Code Assistant Instructions](https://best.openssf.org/Security-Focused-Guide-for-AI-Code-Assistant-Instructions.html) - Checksum verification for external resources
- [Generative File System with SysSpec](https://arxiv.org/html/2512.13047v1) - LLM tools for avoiding race conditions

---

## Differentiators

### Relationships Block

**Competitive advantages:**

- **Callers/Callees** — Full codebase call graph embedded in spans
  - **Value Proposition:** LLMs can perform impact analysis before patching. "If I change this function, what breaks?" No separate Magellan queries needed.
  - **Complexity:** HIGH
  - **Dependencies:** Requires integration with code graph database (SQLiteGraph/Magellan)
  - **LLM use case:** "Rename this function" → LLM sees all callers, updates them automatically

- **Imports/Exports** — Cross-file module dependencies
  - **Value Proposition:** Enables safe module refactoring. LLMs can see what imports a symbol and update import statements.
  - **Complexity:** MEDIUM
  - **Dependencies:** Requires import/export tracking in code graph
  - **LLM use case:** "Delete this symbol" → LLM sees all imports, removes them

- **Blast Radius Analysis** — Aggregate impact metrics
  - **Value Proposition:** "This change affects 47 files across 3 modules" → LLM can warn user or break into smaller operations.
  - **Complexity:** MEDIUM
  - **Dependencies:** Builds on callers/callees/imports/exports
  - **LLM use case:** "This is a big change, are you sure?" (safety intervention)

**Sources:**
- [Nuanced Call Graph Context Layer](https://www.nuanced.dev/blog/python-open-source-launch) - Call graph context for AI coding tools
- [Code Graph Analysis: Visualize Source Code Structure](https://www.falkordb.com/blog/code-graph-analysis-visualize-source-code/) - Visualizes function calls, class inheritance, module imports
- [Tools to Trace Caller/Callee Information from C/C++](https://medium.com/@sapna.sinha_3763/tools-to-trace-caller-callee-information-from-c-c-application-f86564004c6d) - Caller/callee tracing tools

### Tool Hints (Behavior Guidance)

**Competitive advantages:**

- **Requires Full Context** — Boolean hint for operations needing entire file context
  - **Value Proposition:** Rust macros, C++ templates, and Python decorators require full file context for safe patching. LLMs can't patch these correctly without seeing the entire file.
  - **Complexity:** LOW
  - **Dependencies:** Language-specific analysis
  - **LLM use case:** "This is a Rust macro, I need full file context to patch it safely"

- **Apply Atomically** — Boolean hint for all-or-nothing operations
  - **Value Proposition:** Some operations must be atomic (e.g., swapping two function definitions). Partial application is corruption.
  - **Complexity:** LOW
  - **Dependencies:** Existing atomic rollback infrastructure
  - **LLM use case:** "Swap these two functions" → Fails if either function fails

- **Search Case Sensitivity** — Boolean hint for text search operations
  - **Value Proposition:** Enables case-sensitive search in case-insensitive languages (e.g., SQL, Fortran).
  - **Complexity:** LOW
  - **Dependencies:** None (new flag)
  - **LLM use case:** "Find only 'Foo', not 'foo' or 'FOO'"

- **Language-Specific Hints** — Extensible JSON for language-specific behavior
  - **Value Proposition:** Turns schema into shared contract between tools. Magellan can tell Splice "this is a Rust macro" and Splice adapts.
  - **Complexity:** MEDIUM
  - **Dependencies:** Per-language hint definitions
  - **LLM use case:** Cross-tool coordination without LLM needing to understand language specifics

**Sources:**
- [IntelliJ IDEA Blog: Coding Guidelines for AI Agents](https://blog.jetbrains.com/idea/2025/05/coding-guidelines-for-your-ai-agents/) - AI tool coordination and behavior guidance
- [From JetBrains to Cursor: Ultimate VS Code Setup](https://medium.com/israeli-tech-radar/from-jetbrains-to-ultimate-vs-code-setup-for-jetbrains-refugees-fb6291bbff6e) - Tool behavior comparison

### Suggested Action Metadata

**Competitive advantages:**

- **Action Type** — "rename", "delete", "extract", "inline", etc.
  - **Value Proposition:** Enables intelligent batching and self-repair. LLM can suggest "rename this" and Splice can auto-generate the correct operation.
  - **Complexity:** MEDIUM
  - **Dependencies:** Action type taxonomy
  - **LLM use case:** "I need to extract this into a function" → Splice knows what "extract" means

- **Action Parameters** — Structured params for each action type
  - **Value Proposition:** Automatic merge of multi-step refactors. "Rename foo to bar everywhere" → Single operation, not N separate patches.
  - **Complexity:** HIGH
  - **Dependencies:** Per-action parameter schemas
  - **LLM use case:** "Rename 'process_data' to 'handle_request' across all files"

**Future-proofing (not immediate MVP):**
- Automatic merge of multi-step refactors
- Intelligent batching of operations
- LLM self-repair suggestions
- Opportunistic optimizations

**Sources:**
- [Prompting LLMs for Code Editing: Struggles and Remedies](https://www.researchgate.net/publication/391282210_Prompting_LLMs_for_Code_Editing_Struggles_and_Remedies) - AutoPrompter tool for inferring missing information

### Unified Error Codes

**Competitive advantages:**

- **Machine-Readable Error Codes** — `{TOOL}-{CATEGORY}-{NUMBER}` format
  - **Value Proposition:** Automatic repair strategies. LLM retry logic without hallucination. Splice ↔ Magellan ↔ LLM integrated debugging.
  - **Complexity:** LOW
  - **Dependencies:** Error code taxonomy
  - **LLM use case:** "SPL-E001: Symbol not found" → LLM knows exactly what to do

- **Error Categories** — IO, QUERY, REF, VALIDATION, AST
  - **Value Proposition:** Structured error handling. LLM can map errors to repair strategies automatically.
  - **Complexity:** LOW
  - **Dependencies:** Error categorization
  - **LLM use case:** "IO error" → Check file permissions; "REF error" → Re-query symbol

**Error code format:**

| Tool | Prefix | Examples |
|------|--------|----------|
| Splice | `SPL` | `SPL-E001`, `SPL-V002` |
| Magellan | `MAG` | `MAG-REF-001`, `MAG-QRY-002` |
| llmsearch | `LMS` | `LMS-IO-001`, `LMS-QRY-002` |

**Sources:**
- [Better JSON Schema Errors (GSoC 2025)](https://json-schema.org/blog/posts/gsoc25-wrapup) - Error code standardization efforts
- [Complete JSON Validation Guide: Best Practices 2025](https://dataformatterpro.com/blog/complete-json-validation-guide-2025/) - JSON schema error handling patterns

---

## Anti-Features

### Explicitly NOT Building:

- **Full LSP Integration** — Direct LSP server integration for real-time diagnostics
  - **Why avoid:** LSP is editor-specific. Splice is a CLI tool. LSP integration belongs in llmfilewrite, not Splice.
  - **Alternative:** Splice uses tree-sitter for parsing and language compilers for validation. No LSP dependency.

- **Fuzzy Symbol Matching** — "Did you mean...?" suggestions for typos
  - **Why avoid:** LLMs should handle fuzzy matching. Adding fuzzy matching to Splice duplicates LLM capabilities and adds complexity.
  - **Alternative:** Return structured error "SPL-REF-001: Symbol not found" and let LLM retry with different spelling.

- **Incremental Patch Application** — Apply patches as they're generated, fail fast
  - **Why avoid:** Violates atomic operation principle. Partial patch application leaves codebase in inconsistent state.
  - **Alternative:** Collect all patches, validate all, apply all atomically. Roll back on any failure.

- **Automatic Conflict Resolution** — Merge conflicting patches automatically
  - **Why avoid:** Automatic conflict resolution is error-prone. Better to fail explicitly and let LLM retry.
  - **Alternative:** Detect conflicts, return error with conflict details, let LLM generate new patches.

- **In-Place File Modification** — Modify files without backup
  - **Why avoid:** Violates safety principles. No recovery path if operation fails.
  - **Alternative:** Always create backup, apply to copy, atomic rename on success.

- **Context-Free Operations** -- Allow operations without context/semantics
  - **Why avoid:** LLMs will default to context-free operations to save tokens, leading to errors.
  - **Alternative:** Context and semantics are opt-in via flags, but recommended for LLM use.

---

## Feature Interdependencies

```
[Context Extension]
    └─→ [Selected Lines] ──→ [Checksum Validation]

[Semantic Kind Detection]
    └─→ [Tool Hints] ──→ [Apply Atomically]
    └─→ [Suggested Action Metadata]

[Checksums]
    ├─→ [Content Checksum Before]
    └─→ [File Checksum Before] ──→ [Race Condition Protection]

[Relationships Block]
    ├─→ [Callers/Callees]
    ├─→ [Imports/Exports]
    └─→ [Blast Radius Analysis]

[Error Codes]
    └─→ [Error Categories] ──→ [Automatic Repair Strategies]
```

### Dependency Chain for MVP:

1. **Phase 1: Context & Semantics** (Foundation)
   - Context extension (before/after/selected lines)
   - Semantic kind detection
   - Configurable context lines

2. **Phase 2: Safety & Validation** (Trust)
   - Checksums (content + file)
   - Race condition protection
   - Unified error codes

3. **Phase 3: Intelligence** (Power)
   - Relationships (callers/callees)
   - Tool hints (full context, atomic)
   - Suggested actions (future)

---

## Implementation Complexity by Feature

| Feature | Complexity | Dependencies | Risk Level |
|---------|------------|--------------|------------|
| Before/After Lines | LOW | Span coordinates | LOW |
| Selected Lines | LOW | Byte offsets | LOW |
| Configurable Context | LOW | Context extension | LOW |
| Semantic Kinds | MEDIUM | Tree-sitter parsing | MEDIUM |
| Content Checksum | LOW | SHA-256 infrastructure | LOW |
| File Checksum | LOW | File I/O | LOW |
| Callers/Callees | HIGH | Code graph database | HIGH |
| Imports/Exports | MEDIUM | Import tracking | MEDIUM |
| Tool Hints | LOW-MEDIUM | Language analysis | LOW |
| Suggested Actions | MEDIUM | Action taxonomy | MEDIUM |
| Error Codes | LOW | Error handling | LOW |

---

## Cross-Tool Alignment Impact

### Unified Schema Benefits:

1. **Magellan → Splice Flow**
   - Magellan query returns spans with context
   - Splice patches without re-reading files
   - No token waste on format translation

2. **Splice → llmtransform Flow**
   - Splice generates spans with checksums
   - llmtransform applies edits with verification
   - Consistent error codes across tools

3. **All Tools → LLM Flow**
   - LLM consumes output from any tool
   - No format-specific parsing
   - Consistent field names: `file_path`, `start_line`, `start_col`

**Sources:**
- [LLM Tool Ecosystem Alignment](https://github.com/oldnordic/splice/blob/main/docs/LLM_TOOL_ECOSYSTEM_ALIGNMENT.md) - Cross-tool schema alignment strategy

---

## Open Questions & Research Flags

### Phase-Specific Research Needed:

1. **Semantic Kind Detection (Phase 1)**
   - **Flag:** Tree-sitter node type → semantic kind mapping for each language
   - **Research needed:** "What are the exact tree-sitter node types for Rust macros?"
   - **Confidence:** MEDIUM (Tree-sitter docs exist, but per-language mappings needed)

2. **Callers/Callees (Phase 3)**
   - **Flag:** SQLiteGraph/Magellan query performance for large codebases
   - **Research needed:** "What's the query latency for callers in 100K+ LOC codebase?"
   - **Confidence:** LOW (No benchmarks yet)

3. **Suggested Actions (Future)**
   - **Flag:** Action type taxonomy completeness
   - **Research needed:** "What other action types beyond rename/delete/extract/inline?"
   - **Confidence:** LOW (Exploratory feature)

### Verified (No Research Needed):

- Context extension patterns (VS Code, IntelliJ standards)
- Checksum validation (Splice v2.0 already has this)
- Error code formats (JSON Schema community standards)

---

## MVP Recommendation

For **Splice v2.2 MVP**, prioritize in this order:

### Must-Have (Blocker if missing):
1. **Context extension** — Before/after lines (3 default, configurable)
2. **Selected lines** — Content verification for LLMs
3. **Semantic kind detection** — Per-language symbol kinds
4. **Checksums** — Content + file (already exists, just expose in JSON)

### Nice-to-Have (Stretch goals):
5. **Tool hints** — Full context, atomic flags
6. **Error codes** — Unified error format

### Defer to v2.3+:
- **Relationships** (callers/callees/imports/exports) — Requires Magellan integration work
- **Suggested actions** — Future-proofing, not immediate LLM need

**Rationale:** Context + semantics + checksums = 80% of LLM value with 20% of implementation effort. Relationships and suggested actions are power features that can be added after foundation is solid.

---

## Sources

### HIGH Confidence (Official/Primary Sources):
- [VS Code Semantic Highlight Guide](https://code.visualstudio.com/api/language-extensions/semantic-highlight-guide) - Official VS Code documentation
- [Splice v2.0 README](https://github.com/oldnordic/splice/blob/main/README.md) - Project documentation (verified)
- [Unified JSON Schema for Splice](https://github.com/oldnordic/splice/blob/main/docs/UNIFIED_JSON_SCHEMA.md) - Design specification (verified)
- [LLM Tool Ecosystem Alignment](https://github.com/oldnordic/splice/blob/main/docs/LLM_TOOL_ECOSYSTEM_ALIGNMENT.md) - Cross-tool alignment (verified)

### MEDIUM Confidence (Verified with Community Sources):
- [Addy Osmani's LLM Coding Workflow 2026](https://medium.com/@addyosmani/my-llm-coding-workflow-going-into-2026-52fe1681325e) - Industry expert perspective
- [Tree-sitter AST Parsing at Scale](https://www.dropstone.io/blog/ast-parsing-tree-sitter-40-languages) - Tree-sitter best practices
- [Nuanced Call Graph Context Layer](https://www.nuanced.dev/blog/python-open-source-launch) - Call graph tooling
- [Better JSON Schema Errors (GSoC 2025)](https://json-schema.org/blog/posts/gsoc25-wrapup) - Error code standards

### LOW Confidence (Web Search Only, Needs Verification):
- Semantic kind detection per-language mappings (need tree-sitter grammar verification)
- Callers/callees query performance (need benchmarking)
- Suggested action taxonomy completeness (exploratory)

---

*Document created: 2026-01-22*
*Status: Ready for roadmap creation*
*Confidence: HIGH (table stakes), MEDIUM (differentiators), LOW (future features)*
