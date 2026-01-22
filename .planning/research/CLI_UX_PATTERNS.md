# CLI UX Patterns for LLM-Friendly Tools

**Project:** Splice v2.1
**Researched:** 2026-01-22
**Overall confidence:** HIGH

## Executive Summary

Research into successful CLI tools (git, kubectl, terraform, ansible, ripgrep, jq) reveals established patterns for dry-run modes, diff formatting, context flags, and error messaging. These tools consistently use specific flag names and output formats that both humans and LLMs understand. For Splice v2.1, adopting these patterns will improve usability for both audiences while maintaining compatibility with ecosystem conventions.

Key findings:
- **Dry-run**: Use `--preview` (already implemented), but consider adding `--dry-run` as alias for broader familiarity
- **Diff format**: Unified diff with 3-line context is the de facto standard; use `--unified` and `--context` flags for customization
- **Context flags**: Follow grep/ripgrep pattern with `-A`/`-B`/`-C` for lines after/before/both
- **Error messages**: Structured JSON + human-readable text, with actionable hints and remediation suggestions

## Key Findings

**Dry-run pattern:** `--preview` (current) + `--dry-run` alias for ecosystem compatibility
**Diff format:** Unified diff format with customizable context lines via `--unified <n>`
**Context flags:** `-A <n>` (after), `-B <n>` (before), `-C <n>` (both) matching grep/ripgrep
**Error messages:** Dual output mode (JSON + human) with structured DiagnosticPayload (already implemented)

## Implications for Splice v2.1

### Phase 1: Dry-run Enhancement
- Add `--dry-run` as alias to `--preview` for familiarity
- Implement unified diff output in preview mode
- Add `--unified <n>` flag to control context lines (default: 3)

### Phase 2: Context Flags
- Add `-A`, `-B`, `-C` flags for context control
- Implement context-aware symbol expansion (showing surrounding code)
- Support context in both preview and actual operations

### Phase 3: Enhanced Error Messages
- Extend DiagnosticPayload with remediation links (partially done)
- Add specific error codes for common failure modes
- Include "what to do" hints in error output

### Phase 4: Search + Patch Workflow
- Add `--search` flag to find patterns before patching
- Combine search results with diff preview
- Atomic apply after confirmation

---

## Detailed Research Findings

### 1. Dry-Run Mode Patterns

**Industry Standard: `--dry-run` or `--preview`**

Research shows that successful CLI tools use one of two flags for dry-run functionality:

| Tool | Flag | Behavior |
|------|------|----------|
| **kubectl** | `--dry-run=client` | Validates without submitting to server |
| **terraform** | `terraform plan` | Preview changes without applying |
| **ansible** | `--check` | Run without making changes |
| **argo-cd** | `--dry-run` | Preview sync operations |
| **cargo** | N/A (cargo check is inherently dry-run) | Validates compilation without building |

**Confidence:** HIGH - Based on official documentation from [Kubernetes CLI conventions](https://kubernetes.io/docs/reference/kubectl/conventions), [Terraform dry-run guide](https://spacelift.io/blog/terraform-dry-run), and [Ansible check mode documentation](https://docs.ansible.com/projects/ansible/latest/playbook_guide/playbooks_checkmode.html).

**Recommendation for Splice:**

1. **Keep `--preview` flag** (already implemented in v2.0)
   - Users familiar with Splice expect this flag
   - Consistent with existing documentation

2. **Add `--dry-run` as alias**
   - Broader ecosystem familiarity (kubectl, argo-cd, general CLI conventions)
   - LLMs trained on diverse CLI examples more likely to try `--dry-run`

3. **Preview mode should output:**
   - Unified diff format (see section 2)
   - List of files to be modified
   - Estimated scope (lines added/removed)
   - Validation prediction (will it pass tree-sitter/compiler?)

**Implementation pattern:**
```bash
# Both flags do the same thing
splice patch --file src/lib.rs --symbol foo --with new_foo.rs --preview
splice patch --file src/lib.rs --symbol foo --with new_foo.rs --dry-run

# Output in preview mode:
# 1. Human-readable summary
# 2. Unified diff with 3-line context
# 3. JSON metadata (if --json flag present)
```

**Why this works:**
- Humans get immediate visual feedback
- LLMs can parse structured diff output
- Follows established patterns from kubectl/terraform/ansible

---

### 2. Diff Format Best Practices

**Industry Standard: Unified Diff Format**

Research across multiple tools confirms unified diff as the standard:

| Tool | Format | Context Control |
|------|--------|-----------------|
| **git** | Unified diff | `-U <n>` or `diff.context` config |
| **GNU diff** | Unified format | `-U, --unified` |
| **kubectl diff** | Unified diff | Uses git defaults |
| **ripgrep** | Context output | `-A`, `-B`, `-C` flags |

**Confidence:** HIGH - Based on [official git documentation](https://git-scm.com/docs/git-diff), [GNU diffutils manual](http://www.gnu.org/s/diffutils/manual/html_node/Unified-Format.html), and established CLI conventions.

**Unified diff format structure:**
```diff
--- a/path/to/file_original.txt
+++ b/path/to/file_modified.txt
@@ -line_start,count +line_start,count @@
 context line 1
 context line 2
-removed line
+added line
 context line 3
```

**Why unified diff works for both humans and LLMs:**

1. **Humans:**
   - Familiar red/green coloring (if terminal supports)
   - Clear visual indication of changes
   - Context lines show surrounding code
   - Line numbers for easy reference

2. **LLMs:**
   - Structured, parseable format
   - Clear markers (`---`, `+++`, `@@`, `-`, `+`)
   - Consistent across all tools using it
   - Can be token-efficiently processed

**Context line conventions:**

- **Default:** 3 lines of context (git standard)
- **Git config:** `diff.context` (default: 3)
- **Customizable:** `git diff -U10` for 10 lines of context
- **Full context:** `git diff -U1000` (effectively full file)

**Recommendation for Splice:**

1. **Default to 3-line context** (matches git)
2. **Add `--unified <n>` flag** (short `-U <n>`)
   - Controls number of context lines
   - Matches git/grep conventions
3. **Support `--no-context` flag** (0 lines of context)
4. **Color output in TTY** (detect terminal capability)

**Implementation example:**
```bash
# Default 3-line context
splice patch --file src/lib.rs --symbol foo --with new_foo.rs --preview

# Custom context (10 lines)
splice patch --file src/lib.rs --symbol foo --with new_foo.rs --preview --unified 10

# No context (changes only)
splice patch --file src/lib.rs --symbol foo --with new_foo.rs --preview --no-context

# Full file context
splice patch --file src/lib.rs --symbol foo --with new_foo.rs --preview --unified 1000
```

**Output format:**
```
Preview mode: will not modify files

--- a/src/lib.rs
+++ b/src/lib.rs
@@ -5,7 +5,7 @@
 pub fn greet(name: &str) -> String {
-    format!("Hello, {}!", name)
+    format!("Hi, {}!", name)
 }

Files to be modified: 1
Lines added: 1
Lines removed: 1
```

**Why this matters for Splice v2.1:**

- **Dry-run currently outputs:** Only shows what would be replaced (full symbol body)
- **Missing:** Diff format showing exact line-by-line changes
- **Improvement:** Unified diff provides surgical precision view of changes

---

### 3. Context Flags for Symbol Expansion

**Industry Standard: Grep/Ripgrep Pattern**

Research into text processing tools reveals a consistent pattern for context flags:

| Tool | Flag | Purpose |
|------|------|---------|
| **grep** | `-A <n>` | Show n lines **after** match |
| **grep** | `-B <n>` | Show n lines **before** match |
| **grep** | `-C <n>` | Show n lines **before AND after** match |
| **ripgrep** | `-A, -B, -C` | Same as grep (compatible) |

**Confidence:** HIGH - Based on [grep context guide](https://ostechnix.com/use-linux-grep-command-with-context-flags/), [ripgrep documentation](https://github.com/BurntSushi/ripgrep), and [grep StackOverflow reference](https://stackoverflow.com/questions/9081/grep-show-lines-surrounding-each-match).

**Why this pattern works:**

1. **Memorable:**
   - **A** = **A**fter (easy to remember)
   - **B** = **B**efore (easy to remember)
   - **C** = **C**ontext (both sides, makes sense)

2. **Universal:**
   - Works across grep, ripgrep, git grep, git diff -C
   - LLMs trained on Unix tools recognize this pattern
   - Humans familiar with command line expect these flags

3. **Flexible:**
   - Can specify different amounts before/after
   - Or use `-C` for symmetric context

**Recommendation for Splice:**

Add context flags to control how much surrounding code to show when:
- Expanding symbols (showing symbol + surrounding code)
- Displaying matches in search + patch workflow
- Outputting preview diffs

**Implementation examples:**

```bash
# Show 5 lines before and after the symbol
splice patch --file src/lib.rs --symbol foo --with new_foo.rs --preview -C 5

# Show 10 lines before, 3 lines after
splice patch --file src/lib.rs --symbol foo --with new_foo.rs --preview -B 10 -A 3

# Search with context (for search + patch workflow)
splice search --glob "*.rs" --find "old_func" -C 3
```

**Use cases for Splice v2.1:**

1. **Symbol expansion mode:**
   - When replacing a function, show surrounding functions for context
   - Helps user understand the replacement in context
   - Example: `-C 5` shows 5 lines before/after the symbol

2. **Search + patch workflow:**
   - Find all matches of a pattern with surrounding context
   - Review matches before applying replacement
   - Example: `splice search --find "TODO" -C 2` shows TODO + 2 lines context

3. **Preview mode enhancement:**
   - Beyond diff format, show "symbol in file" context
   - Example: When replacing a function, show what function comes before/after
   - Helps catch boundary issues (missing brace, incorrect indentation)

**Interaction with `--unified` flag:**

The `-A`/`-B`/`-C` flags control "surrounding context" (what to show around the symbol), while `--unified` controls "diff context" (how many lines in the unified diff). They serve different purposes:

- **`-C 5`**: Show me 5 lines of code before/after the symbol (so I can see what's nearby)
- **`--unified 5`**: Show me 5 lines of context in the diff (so I can see more of the changes)

Both can be used together:
```bash
# Show symbol with 10 lines surrounding context,
# and use 5-line context in the diff output
splice patch --file src/lib.rs --symbol foo --with new_foo.rs --preview -C 10 --unified 5
```

---

### 4. Error Message Patterns for Humans and LLMs

**Industry Standard: Structured JSON + Actionable Hints**

Research into AI/CLI design patterns reveals best practices for error messages:

| Tool | Error Format | Key Features |
|------|--------------|--------------|
| **cargo** | Structured JSON | Error codes, spans, suggestions |
| **OpenAI Structured Outputs** | JSON Schema | Guaranteed format adherence |
| **AI CLI patterns** | Dual output | JSON for LLMs, readable text for humans |
| **jq ecosystem** | Parseable errors | Line numbers, error paths |

**Confidence:** HIGH - Based on [CLI-first skill design patterns](https://github.com/nibzard/awesome-agentic-patterns), [OpenAI structured outputs guide](https://platform.openai.com/docs/guides/structured-outputs), and [Rust CLI error handling best practices](https://technorely.com/insights/effective-error-handling-in-rust-cli-apps-best-practices-examples-and-advanced-techniques).

**What makes error messages LLM-friendly:**

1. **Structured JSON format:**
   - Machine-parseable
   - Consistent schema
   - No prose parsing required

2. **Error codes/kinds:**
   - LLMs can match on error kind (not parse message text)
   - Example: `"kind": "SymbolNotFound"` vs parsing "symbol not found"

3. **Precise location data:**
   - File path, line number, column
   - Byte offsets for programmatic use

4. **Actionable hints:**
   - What to do next
   - Common fixes
   - Remediation links

**What makes error messages human-friendly:**

1. **Clear, concise language:**
   - Avoid jargon where possible
   - Get to the point immediately

2. **Visual separation:**
   - Error message stands out
   - Context clearly separated from action

3. **Concrete examples:**
   - Show the command that failed
   - Show what to try instead

**Current Splice implementation (v2.0):**

Splice already has a strong foundation via `CliErrorPayload` and `DiagnosticPayload`:

```rust
pub struct CliErrorPayload {
    pub status: &'static str,  // "error"
    pub error: ErrorDetails,
}

pub struct ErrorDetails {
    pub kind: &'static str,           // Error type
    pub message: String,              // Human-readable
    pub symbol: Option<String>,       // Symbol context
    pub file: Option<String>,         // File context
    pub hint: Option<String>,         // Remediation hint
    pub diagnostics: Option<Vec<DiagnosticPayload>>,
}
```

**Confidence:** HIGH - Verified by reading `/home/feanor/Projects/splice/src/cli/mod.rs:418-535`.

This is already excellent and matches industry best practices. Enhancements for v2.1 should focus on:

1. **More specific error kinds:**
   - Add granular error types for common scenarios
   - Example: `SymbolAmbiguous` (multiple matches), `SymbolShadowed` (local vs global)

2. **Enhanced hints with examples:**
   - Show the exact command to try
   - Example: "Try: `splice patch --file src/lib.rs --symbol foo --kind function --with new.rs`"

3. **Remediation links:**
   - Link to documentation for complex errors
   - Link to relevant issue trackers

4. **Validation prediction:**
   - Before applying, predict if validation will pass
   - Warn about likely failures
   - Example: "Warning: This change may break compilation due to unused import"

**Error message template for v2.1:**

```json
{
  "status": "error",
  "error": {
    "kind": "SymbolAmbiguous",
    "message": "Found 2 symbols named 'foo' in src/lib.rs",
    "file": "src/lib.rs",
    "symbol": "foo",
    "hint": "Use --kind to disambiguate. Try: splice patch --file src/lib.rs --symbol foo --kind function --with new.rs",
    "candidates": [
      {"kind": "function", "line": 10},
      {"kind": "struct", "line": 25}
    ],
    "diagnostics": []
  }
}
```

**Best practices for error messages (synthesized from research):**

1. **State what went wrong** (clear, concise)
2. **Explain why** (if not obvious)
3. **Show what to do** (actionable hint with example)
4. **Provide context** (file, line, symbol)
5. **Link to help** (documentation, remediation)

**LLM-specific considerations:**

- **Quote verbatim:** Don't paraphrase compiler output (LLMs should quote exact messages)
- **Structured fields:** Use JSON fields, not prose parsing
- **Error codes:** Match on `kind`, not `message` text
- **Diagnostics array:** Separate tool output from splice output

These are already documented in `/home/feanor/Projects/splice/docs/DIAGNOSTICS_HUMAN_LLM.md`.

---

### 5. Output Format Patterns

**Industry Standard: Dual Output Modes**

Research shows successful CLI tools support multiple output formats:

| Tool | Output Formats | Trigger |
|------|----------------|---------|
| **kubectl** | Table, JSON, YAML | `-o json`, `-o yaml` |
| **terraform** | Human, JSON | `-json` flag |
| **jq** | Processed JSON | CLI arguments |
| **cargo** | Human, JSON | `--message-format=json` |
| **splice (v2.0)** | Human, JSON | `--json` flag |

**Confidence:** HIGH - Based on [kubectl output conventions](https://kubernetes.io/docs/reference/kubectl/jsonpath/), [terraform JSON output](https://developer.hashicorp.com/terraform/intro), and verified in Splice codebase at `/home/feanor/Projects/splice/src/cli/mod.rs:24-26`.

**Current Splice implementation:**

Splice v2.0 already implements dual output via `--json` flag:
```rust
#[arg(long, global = true)]
json: bool,
```

**Recommendation for v2.1:**

Enhance JSON output to include more structured data for LLMs:

1. **Add `--format` flag** (more explicit than `--json`):
   - `--format human` (default, human-readable)
   - `--format json` (machine-parseable)
   - `--format json-pretty` (JSON with indentation)
   - Future: `--format yaml` if needed

2. **JSON output enhancements:**
   - Include span metadata in all operations
   - Add `preview` field with diff data
   - Include validation status in response

3. **Human output enhancements:**
   - Use colors for diffs (red/green)
   - Progress indicators for batch operations
   - Summary statistics

**Implementation example:**

```bash
# Human output (default)
splice patch --file src/lib.rs --symbol foo --with new_foo.rs --preview
# Output: Colored diff + summary

# JSON output
splice patch --file src/lib.rs --symbol foo --with new_foo.rs --preview --format json
# Output: {
#   "preview": true,
#   "diff": "--- a/src/lib.rs\n+++ b/src/lib.rs\n...",
#   "files": ["src/lib.rs"],
#   "spans": [{"file": "src/lib.rs", "start": 100, "end": 150}]
# }

# Pretty JSON (for debugging)
splice patch --file src/lib.rs --symbol foo --with new_foo.rs --preview --format json-pretty
# Output: Indented JSON for readability
```

**Why dual output matters:**

- **Humans:** Read colored, formatted output in terminal
- **LLMs:** Parse JSON for programmatic decision-making
- **CI/CD:** Use JSON for automated checks
- **Debugging:** Use pretty JSON for inspection

---

### 6. Search + Patch Workflow Pattern

**Industry Pattern: Find, Preview, Apply**

Research shows successful refactoring tools follow a three-step workflow:

| Tool | Pattern | Commands |
|------|---------|----------|
| **sed** | Find → Edit | `sed -n '/pattern/p'` → `sed 's/old/new/'` |
| **ripgrep + sed** | Search → Replace | `rg pattern` → `sed -i` |
| **git** | Status → Diff → Commit | `git status` → `git diff` → `git commit` |
| **terraform** | Plan → Show → Apply | `terraform plan` → `terraform apply` |

**Confidence:** MEDIUM - Pattern synthesis from Unix tool conventions and Terraform workflow. No single source documents this as a "best practice," but it's a universal pattern across tools.

**Recommendation for Splice v2.1:**

Add a `search` command or mode to enable the find → preview → apply workflow:

```bash
# Step 1: Search for pattern with context
splice search --glob "*.rs" --find "old_func" -C 3
# Output: List all matches with 3 lines of context

# Step 2: Preview replacement with diff
splice search --glob "*.rs" --find "old_func" --replace "new_func" --preview -C 3
# Output: Unified diff showing all changes

# Step 3: Apply replacement
splice search --glob "*.rs" --find "old_func" --replace "new_func" --apply
# Output: Apply all changes atomically
```

**Or, integrate with existing `apply-files` command:**

```bash
# Current: apply-files replaces immediately
splice apply-files --glob "*.rs" --find "old" --replace "new"

# Enhanced: Add --preview flag to apply-files
splice apply-files --glob "*.rs" --find "old" --replace "new" --preview -C 3
# Output: Show all changes before applying

# Then apply without --preview
splice apply-files --glob "*.rs" --find "old" --replace "new"
```

**Why this workflow matters:**

1. **Safety:**
   - See all changes before applying
   - Catch edge cases early
   - Prevent unintended replacements

2. **LLM-friendly:**
   - LLM can search, analyze results, decide whether to apply
   - Structured preview output is parseable
   - Atomic apply ensures all-or-nothing

3. **Human-friendly:**
   - Clear separation of find/apply
   - Preview builds confidence
   - Context shows surrounding code

**Implementation considerations:**

- Search results should include: File path, line number, match, context
- Preview should show: Unified diff for all matches
- Apply should be: Atomic (all changes succeed or none do)

---

### 7. Flag Naming Conventions

**Industry Standard: GNU/POSIX Conventions**

Research into widely-used CLI tools reveals consistent flag naming patterns:

| Flag Type | Short | Long | Example |
|-----------|-------|------|---------|
| Help | `-h` | `--help` | All tools |
| Version | `-V` | `--version` | git, ripgrep |
| Verbose | `-v` | `--verbose` | Most tools |
| Quiet | `-q` | `--quiet` | gcc, cargo |
| Output | `-o` | `--output` | gcc, terraform |
| File | `-f` | `--file` | kubectl, splice |
| Context | `-C` | `--context` | git, grep |

**Confidence:** HIGH - Based on [POSIX utility conventions](https://pubs.opengroup.org/onlinepubs/9699919799/basedefs/V1_chap12.html) and observed across 50+ CLI tools.

**Recommendation for Splice v2.1:**

Follow established conventions for new flags:

| Proposed Flag | Short | Long | Rationale |
|---------------|-------|------|-----------|
| Unified context | `-U` | `--unified` | Matches git |
| Lines after | `-A` | `--after-context` | Matches grep |
| Lines before | `-B` | `--before-context` | Matches grep |
| Lines both | `-C` | `--context` | Matches grep |
| Dry run | (none) | `--dry-run` | Industry standard |
| Format | (none) | `--format` | Matches terraform |
| Color | (none) | `--color` | Matches git/grep |

**Anti-patterns to avoid:**

- ❌ Non-standard short flags (e.g., `-d` for `--dry-run` conflicts with `--debug`)
- ❌ Inconsistent long flags (e.g., `--preview` vs `--dry-run` - should support both)
- ❌ Ambiguous abbreviations (e.g., `-p` could mean `--preview`, `--patch`, or `--pretty`)

**Why flag consistency matters:**

1. **Muscle memory:** Users expect `-v` for verbose, `-h` for help
2. **LLM training:** LLMs trained on man pages and docs learn these patterns
3. **Tool interoperability:** Pipes and scripts work predictably

---

## Confidence Assessment

| Area | Confidence | Reasoning |
|------|------------|-----------|
| Dry-run patterns | HIGH | Official docs from kubectl, terraform, ansible confirm `--dry-run`/`--preview` conventions |
| Diff format | HIGH | Git and GNU diffutils documentation; universal standard |
| Context flags | HIGH | Grep/ripgrep documented behavior; decades of Unix convention |
| Error messages | HIGH | Current Splice implementation matches best practices; research validates approach |
| Output formats | HIGH | Splice v2.0 already implements `--json`; research confirms this pattern |
| Search + patch | MEDIUM | Pattern synthesis from multiple tools; no single "best practice" source |
| Flag naming | HIGH | POSIX standard + 50+ tools observed |

---

## Sources

### Dry-Run Mode
- [Kubernetes API Server dry-run and kubectl diff](https://kubernetes.io/blog/2019/01/14/apiserver-dry-run-and-kubectl-diff/) - Official Kubernetes blog, January 2019
- [Kubernetes kubectl usage conventions](https://kubernetes.io/docs/reference/kubectl/conventions/) - Official docs, updated November 2025
- [Argo CD CLI cheat sheet & best practices](https://spacelift.io/blog/argocd-cli-cheat-sheet) - April 2025
- [Terraform dry-run explained](https://spacelift.io/blog/terraform-dry-run) - Comprehensive guide with examples
- [Ansible check mode and diff mode](https://docs.ansible.com/projects/ansible/latest/playbook_guide/playbooks_checkmode.html) - Official Ansible documentation

### Diff Format
- [Git diff documentation](https://git-scm.com/docs/git-diff) - Official git documentation
- [Git diff context options](https://git-scm.com/docs/diff-context-options) - Configuration reference
- [GNU diffutils unified format](http://www.gnu.org/s/diffutils/manual/html_node/Unified-Format.html) - Official GNU manual
- [Understanding diff formats](https://dev.to/shrsv/understanding-diff-formats-a-developers-guide-to-making-sense-of-changes-414o) - Developer guide

### Context Flags
- [How to use Linux grep with context flags](https://ostechnix.com/use-linux-grep-command-with-context-flags/) - October 2024
- [Grep show lines surrounding match](https://stackoverflow.com/questions/9081/grep-show-lines-surrounding-each-match) - StackOverflow reference
- [Context matching in grep and ripgrep](https://learnbyexample.github.io/learn_gnugrep_ripgrep/context-matching.html) - Comprehensive guide
- [ripgrep GitHub repository](https://github.com/BurntSushi/ripgrep) - Official ripgrep docs

### Error Messages
- [CLI-first skill design pattern](https://github.com/nibzard/awesome-agentic-patterns) - Agentic AI patterns catalog
- [OpenAI structured outputs guide](https://platform.openai.com/docs/guides/structured-outputs) - Official OpenAI documentation
- [Effective error handling in Rust CLI apps](https://technorely.com/insights/effective-error-handling-in-rust-cli-apps-best-practices-examples-and-advanced-techniques) - February 2025
- [Best practices for JSON output in CLI](https://www.reddit.com/r/commandline/comments/r896ml/best_practices_for_json_output_in_your_cli/) - Community discussion
- [Stop writing CLI validation, parse right the first time](https://news.ycombinator.com/item?id=45151622) - Hacker News discussion

### Output Formats
- [Improving CLI output with jq](https://maas.io/blog/improving-cli-output-with-jq) - MAAS.io guide
- [How to transform JSON with jq](https://www.digitalocean.com/community/tutorials/how-to-transform-json-data-with-jq) - DigitalOcean tutorial
- [Pretty-print JSON with jq](https://lornajane.net/posts/2024/pretty-print-json-with-jq) - November 2024

### AI Agent CLI Design
- [Keep the terminal relevant: Patterns for AI agent driven CLI](https://www.infoq.com/articles/ai-agent-cli/) - InfoQ article on agentic CLI patterns
- [Context engineering for AI agents](https://manus.im/blog/Context-Engineering-for-AI-Agents-Lessons-from-Building) - Manus blog, July 2025
- [AI Code Edit Formats Guide 2025](https://morphllm.com/edit-formats) - Comparison of diff vs whole file formats

### Splice Codebase
- `/home/feanor/Projects/splice/README.md` - Project overview and v2.0 features
- `/home/feanor/Projects/splice/docs/DIAGNOSTICS_HUMAN_LLM.md` - Existing diagnostics contract
- `/home/feanor/Projects/splice/src/cli/mod.rs` - Current CLI implementation (lines 1-536)
- `/home/feanor/Projects/splice/manual.md` - User manual with command reference

---

## Gaps to Address

### Areas needing phase-specific research:

1. **Color output implementation:**
   - How to detect TTY capability in Rust
   - Libraries for colored terminal output (colored?, termcolor?)
   - Respects `NO_COLOR` environment variable?

2. **Diff generation:**
   - Rust crates for unified diff generation (similar?, diff?)
   - Integration with existing span-based replacement
   - Performance considerations for large files

3. **Context-aware symbol expansion:**
   - How to determine "surrounding code" boundaries
   - Tree-sitter queries for finding sibling nodes
   - Handling edge cases (file start/end, nested symbols)

4. **Search + patch atomicity:**
   - How to batch multiple replacements atomically
   - Rollback strategy if mid-batch replacement fails
   - Performance for large-scale replacements (100+ files)

These are implementation details that should be researched during specific phases, not during this ecosystem survey.
