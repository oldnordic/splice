# Research Summary: Splice v2.1 - LLM & Human Usability

**Research Date:** 2026-01-22
**Milestone:** v2.1 - LLM & Human Usability

---

## Key Findings

### 1. CLI UX Patterns

**Dry-run mode:**
- Standard flag: `-n, --dry-run` (not `--preview`, `--what-if`)
- The `-n` convention comes from "dry-run" containing "n"
- Used by: git, rsync, kubectl, make, terraform, ansible
- Splice currently has `--preview` — should add `--dry-run` as alias

**Context flags:**
- Unix standard: `-A` (after), `-B` (before), `-C` (both)
- Default context = 3 lines (from git diff unified format)
- Used by: grep, ripgrep, git diff
- Highly memorable and universal

**Output formats:**
- Unified diff is universal standard (with `---`/`+++`, `-` for deletions, `+` for additions)
- Red for deletions, green for additions (Git, GitHub, GitLab convention)
- Respect `NO_COLOR` environment variable and TTY detection
- Splice v2.0 already implements `--json` — this is correct

### 2. Dry-run Conventions

**Essential dry-run output:**
1. Summary header (what would change)
2. File list
3. Line counts (like `git --stat`)
4. Actual unified diff
5. Exit code 1 if changes would be made, 0 if no changes

**Implementation recommendations:**
- Add `-n, --dry-run` as alias to existing `--preview`
- Implement unified diff output (currently only shows full symbol body)
- Add `--unified <n>` flag for context control
- Color detection is critical: respect `NO_COLOR`, auto-detect TTY

### 3. Error Message Patterns

**What makes errors actionable:**
1. **Severity level** - error/warning/note
2. **Precise location** - file:line:col
3. **Primary message** - what went wrong
4. **Error code** - stable identifier (rustc E0XXX, Swift #DiagnosticName)
5. **Suggestion/note** - how to fix

**Best practices from rustc/Swift:**
- Rust's `rustc --explain` for detailed documentation
- Swift's educational notes (#DiagnosticName) with markdown docs
- Structured JSON for LLM consumption (prevents hallucination)

**For Splice:**
- Define SPLICE_E001, SPLICE_E002 error codes
- Add `splice explain <code>` command
- Extract error codes from compiler output
- Enhance SymbolNotFound with Levenshtein distance suggestions

### 4. Context & Symbol Expansion Patterns

**Context flag conventions:**
- `-A <lines>` — lines after match (grep/ripgrep standard)
- `-B <lines>` — lines before match
- `-C <lines>` — context lines both sides
- Default to 3 lines when `-C` specified without value

**Symbol expansion strategies:**
- LSP defines: `range` (full extent) vs `selectionRange` (name only)
- AST-aware expansion preferred (VS Code/JetBrains use tree-based)
- Tree-sitter integration: find node containing span, walk up parent chain
- First expansion: name only, Second: full body, Third: containing block

---

## Phase Implications

Based on research, the recommended phase structure:

| Phase | Focus | Key Features |
|-------|-------|--------------|
| 1 | Dry-run Enhancement | `--dry-run` alias, unified diff output, `--unified <n>` |
| 2 | Context Flags | `-A`/`-B`/`-C` flags, symbol expansion with AST-aware strategy |
| 3 | Enhanced Error Messages | Granular error kinds, actionable hints, error codes |
| 4 | Search + Patch Workflow | `splice search` command, atomic find-and-replace |

---

## Confidence Levels

| Area | Level | Source |
|------|-------|--------|
| Dry-run flag naming | HIGH | kubectl, terraform, ansible official docs |
| Unified diff format | HIGH | Git official documentation |
| Context flags (-A/-B/-C) | HIGH | grep, ripgrep, git diff standards |
| Error message structure | HIGH | rustc, Swift, GCC official docs |
| LSP range semantics | HIGH | LSP 3.17 specification |
| AST expansion strategies | MEDIUM | Tree-sitter API, needs testing |

---

## Sources

**Official Documentation:**
- [Git diff-format Documentation](https://git-scm.com/docs/diff-format)
- [Kubernetes dry-run and kubectl diff](https://kubernetes.io/blog/2019/01/14/apiserver-dry-run-and-kubectl-diff/)
- [Command Line Interface Guidelines](https://clig.dev/)
- [Evolution of Rust compiler errors](https://kobzol.github.io/rust/rustc/2025/05/16/evolution-of-rustc-errors.html)
- [Swift educational notes](https://forums.swift.org/t/surfacing-educational-notes-for-compiler-errors/78023)
- [GCC 15 usability improvements](https://developers.redhat.com/articles/2025/04/10/6-usability-improvements-gcc-15)

**Research Files:**
- `.planning/research/CLI_UX_PATTERNS.md` — Comprehensive CLI patterns
- `.planning/research/DRYRUN_CONVENTIONS.md` — Dry-run and diff conventions
- `.planning/research/ERROR_PATTERNS.md` — Error message templates and examples
- `.planning/research/CONTEXT_PATTERNS.md` — Context flags and symbol expansion

---

*Research synthesized: 2026-01-22*
