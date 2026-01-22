# Phase 13: Dry-run & Diff - Research

**Researched:** 2026-01-22
**Domain:** CLI dry-run conventions, diff formatting, terminal output
**Confidence:** HIGH

## Summary

Phase 13 requires implementing standard CLI dry-run functionality with unified diff output. The existing codebase already has `--preview` flag functionality (line 122-124 in `src/cli/mod.rs`) that validates changes in a temporary workspace clone. The new requirements (CLI-01 through CLI-07) focus on making this conform to industry standards: adding `-n, --dry-run` as aliases, outputting unified diff format with red/green colors, respecting `NO_COLOR`, detecting TTY, supporting `--unified <n>` for context, and returning exit code 1 when changes are pending.

The primary technical challenge is integrating Rust's diffing libraries (`similar` crate) with the existing `preview_patch` function to generate unified diff output. The codebase already uses `ropey` for line/column calculations (src/context.rs) and has a `PreviewReport` type (src/patch/mod.rs) that captures line numbers and byte counts. The new diff module will build on this foundation.

**Primary recommendation:** Use the `similar` crate for unified diff generation, `nu-ansi-term` for colored output, `is-terminal` for TTY detection, and respect the `NO_COLOR` environment variable. All are zero-dependency or minimal-dependency crates that align with Splice's existing dependencies.

## Standard Stack

### Core
| Library | Version | Purpose | Why Standard |
|---------|---------|---------|--------------|
| `similar` | 2.6+ | Unified diff generation | De facto standard for Rust diffing, maintained by mitsuhiko, provides `udiff` module for unified diff format |
| `nu-ansi-term` | 0.50+ | Terminal colors/formatting | Maintained by Nushell project, actively developed, supports red/green color convention |
| `is-terminal` | 0.4+ | TTY detection | Simple, cross-platform, widely used in Rust ecosystem |

### Supporting
| Library | Version | Purpose | When to Use |
|---------|---------|---------|-------------|
| `atty` | 0.2+ | Alternative TTY detection | Legacy option, but `is-terminal` is preferred (actively maintained) |
| `no_color` crate | 2.0+ | NO_COLOR detection | Optional; can be implemented manually with `std::env::var` |

### Alternatives Considered
| Instead of | Could Use | Tradeoff |
|------------|-----------|----------|
| `similar` | `patiencediff` | `similar` has more active development, unified diff support out of the box |
| `nu-ansi-term` | `colored`, `ansi_term` | `nu-ansi-term` is actively maintained by Nushell; `ansi_term` is deprecated; `colored` is higher-level |
| `is-terminal` | `atty` | `is-terminal` is cross-platform and more actively maintained |

**Installation:**
```bash
cargo add similar nu-ansi-term is-terminal
```

**Features to enable:**
- `similar` with default features (includes `text` feature for unified diff)
- Optional: `similar` with `bytes` feature if byte-level diffs are needed (not required for line-based unified diff)

## Architecture Patterns

### Recommended Project Structure
```
src/
├── diff/
│   ├── mod.rs          # Diff module entry point
│   ├── unified.rs      # Unified diff generation using similar
│   └── color.rs        # Color output with nu-ansi-term + NO_COLOR/TTY detection
├── cli/
│   └── mod.rs          # Add dry_run alias, --unified flag
└── patch/
    └── mod.rs          # Extend PreviewReport to support diff generation
```

### Pattern 1: Unified Diff Generation
**What:** Generate standard unified diff format using `similar::TextDiff::unified_diff()`
**When to use:** For all dry-run output
**Example:**
```rust
// Source: https://docs.rs/similar/latest/similar/udiff/index.html
use similar::{ChangeTag, TextDiff};

let diff = TextDiff::from_lines(old_text, new_text);
let unified = diff.unified_diff()
    .context_radius(3)  // --unified flag value
    .header("a/file.rs", "b/file.rs");

println!("{}", unified);
```

### Pattern 2: Color Detection Logic
**What:** Respect NO_COLOR, detect TTY, provide conditional coloring
**When to use:** For all terminal output
**Example:**
```rust
use is_terminal::IsTerminal;
use nu_ansi_term::Color::{Red, Green};

fn should_use_color() -> bool {
    // Check NO_COLOR first (accessibility standard)
    if std::env::var("NO_COLOR").is_ok() {
        return false;
    }
    // Check explicit flags (future: --color/--no-color)
    // Default to TTY detection
    std::io::stdout().is_terminal()
}

// Usage:
if should_use_color() {
    println!("{}- removed line", Red.prefix());
    println!("{}+ added line", Green.prefix());
} else {
    println!("- removed line");
    println!("+ added line");
}
```

### Pattern 3: Exit Code for Dry-Run
**What:** Return exit code 1 if changes would be made, 0 if no changes
**When to use:** In dry-run/preview mode
**Example:**
```rust
// In execute_patch or similar function:
let exit_code = if preview && has_changes {
    ExitCode::from(1)  // Changes pending
} else {
    ExitCode::SUCCESS   // No changes or error
};
```

### Anti-Patterns to Avoid
- **Hardcoding color:** Always respect `NO_COLOR` and detect TTY
- **Ignoring --unified flag:** Make context configurable
- **Returning 0 for changes pending:** Follow `git diff --exit-code` convention
- **Using deprecated `atty` crate:** Use `is-terminal` instead

## Don't Hand-Roll

Problems that look simple but have existing solutions:

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| Unified diff formatting | Manual string formatting with `-`/`+` prefixes | `similar::udiff` module | Handles hunk headers, context ranges, missing newlines, edge cases |
| TTY detection on Windows | Raw `isatty()` syscall | `is-terminal` crate | Cross-platform, handles Windows console quirks |
| ANSI color codes | Manual `\x1b[31m` escape sequences | `nu-ansi-term` crate | Readable API, handles reset codes, optimized output |
| NO_COLOR detection | Custom env var parsing | `std::env::var("NO_COLOR").is_ok()` | Single check, standard defined |

**Key insight:** Unified diff format has surprising edge cases (missing newlines, hunk headers, context boundaries). The `similar` crate has already handled these.

## Common Pitfalls

### Pitfall 1: Ignoring NO_COLOR Environment Variable
**What goes wrong:** Output always colored even when redirected to files or piped
**Why it happens:** Only checking TTY, not respecting NO_COLOR accessibility standard
**How to avoid:** Check `NO_COLOR` first, then fall back to TTY detection
**Warning signs:** CI logs have ANSI escape codes, tools break when parsing output

### Pitfall 2: Exit Code 0 for Pending Changes
**What goes wrong:** Scripts can't detect if dry-run found changes
**Why it happens:** Following "command succeeded" rather than "changes pending" convention
**How to avoid:** Follow `git diff --exit-code` pattern: 0 = no changes, 1 = changes pending
**Warning signs:** Pre-commit hooks can't use `splice --dry-run` to check for changes

### Pitfall 3: Not Detecting Piped Output
**What goes wrong:** ANSI codes in piped/logs break parsing
**Why it happens:** Only checking for TTY, not for redirection
**How to avoid:** Use `is-terminal` crate, check if stdout is a terminal
**Warning signs:** `splice --dry-run | grep` shows escape sequences

### Pitfall 4: Wrong Diff Format
**What goes wrong:** Custom format doesn't work with `patch` command
**Why it happens:** Building custom diff instead of using unified diff
**How to avoid:** Use `similar::TextDiff::unified_diff()` for standard format
**Warning signs:** Output can't be piped to `patch -p1`

### Pitfall 5: Context Lines Not Configurable
**What goes wrong:** Users can't see more/fewer context lines
**Why it happens:** Hardcoding context value
**How to avoid:** Implement `--unified <n>` flag that maps to `context_radius()`
**Warning signs:** Users request more context for complex changes

## Code Examples

Verified patterns from official sources:

### Unified Diff with Configurable Context
```rust
// Source: https://docs.rs/similar/latest/similar/udiff/index.html
use similar::TextDiff;

fn format_unified_diff(old: &str, new: &str, path: &str, context_lines: usize) -> String {
    let diff = TextDiff::from_lines(old, new);
    diff.unified_diff()
        .context_radius(context_lines)
        .header(&format!("a/{}", path), &format!("b/{}", path))
        .to_string()
}
```

### Colored Diff Output
```rust
// Source: https://docs.rs/nu-ansi-term/latest/nu_ansi_term/
use nu_ansi_term::Color::{Red, Green};
use similar::{ChangeTag, TextDiff};

fn format_colored_diff(old: &str, new: &str, use_color: bool) -> String {
    let diff = TextDiff::from_lines(old, new);
    let mut output = String::new();

    for change in diff.iter_all_changes() {
        let sign = match change.tag() {
            ChangeTag::Delete => {
                if use_color {
                    format!("{}{}", Red.paint("-"), change)
                } else {
                    format!("-{}", change)
                }
            }
            ChangeTag::Insert => {
                if use_color {
                    format!("{}{}", Green.paint("+"), change)
                } else {
                    format!("+{}", change)
                }
            }
            ChangeTag::Equal => format!(" {}", change),
        };
        output.push_str(&sign);
    }
    output
}
```

### Cross-Platform TTY Detection
```rust
// Source: https://docs.rs/is-terminal/latest/is_terminal/
use is_terminal::IsTerminal;

fn should_use_color() -> bool {
    // NO_COLOR takes precedence (accessibility standard)
    // Source: https://no-color.org/
    if std::env::var("NO_COLOR").is_ok() {
        return false;
    }

    // Check if stdout is a terminal
    std::io::stdout().is_terminal()
}
```

### Exit Code Convention (like git diff --exit-code)
```rust
// Based on: https://git-scm.com/docs/git-diff
// Returns 0 if no changes, 1 if changes detected

enum DryRunResult {
    NoChanges,     // Exit code 0
    HasChanges,    // Exit code 1
    Error,         // Exit code != 0, 1
}

fn get_exit_code(result: &DryRunResult) -> std::process::ExitCode {
    match result {
        DryRunResult::NoChanges => ExitCode::SUCCESS,
        DryRunResult::HasChanges => ExitCode::from(1),
        DryRunResult::Error => ExitCode::from(2),
    }
}
```

## State of the Art

| Old Approach | Current Approach | When Changed | Impact |
|--------------|------------------|--------------|--------|
| `atty` crate | `is-terminal` crate | 2023+ | Better cross-platform support, actively maintained |
| `ansi_term` | `nu-ansi-term` | 2020+ | Original `ansi_term` deprecated, Nushell fork is active |
| Manual diff logic | `similar` crate | 2021+ | Unified diff, multiple algorithms, better handling of edge cases |

**Deprecated/outdated:**
- `atty` crate: Unmaintained, use `is-terminal` instead
- `ansi_term` crate: Deprecated, replaced by `nu-ansi-term`
- Custom diff formatting: Use `similar::udiff` instead

## Existing Codebase Integration

### Current Preview Functionality
**Location:** `src/patch/mod.rs:356-391`
**Function:** `preview_patch()`
**Behavior:** Clones workspace, applies change in temp, validates, returns `PreviewReport`

**Current output (JSON):**
```json
{
  "preview_report": {
    "file": "path/to/file.rs",
    "line_start": 10,
    "line_end": 15,
    "lines_added": 2,
    "lines_removed": 3,
    "bytes_added": 50,
    "bytes_removed": 75
  }
}
```

### Current CLI Flag
**Location:** `src/cli/mod.rs:122-124`
```rust
/// Run in preview mode without mutating the workspace.
#[arg(long, conflicts_with = "batch")]
preview: bool,
```

### Integration Strategy
1. Add `-n, --dry-run` as alias to existing `preview` flag
2. Create new `src/diff/mod.rs` module with unified diff generation
3. Add `--unified <n>` flag to CLI (default 3, matches existing `--context-lines`)
4. Modify `execute_patch` in `src/main.rs` to output unified diff when dry-run
5. Add exit code logic based on whether changes would be made

## Exit Code Pattern Analysis

Research into established tools reveals two competing conventions for dry-run exit codes:

### Convention 1: Success-focused (AWS, cargo, JFrog CLI)
- Exit code 0: Command succeeded (even if changes pending)
- Exit code 1: Error occurred
- **Rationale:** Dry-run itself succeeded, CI-friendly

### Convention 2: Change-focused (git diff --exit-code, rectorphp)
- Exit code 0: No changes would be made
- Exit code 1: Changes would be made
- Exit code 2+: Error occurred
- **Rationale:** Allows scripts to detect pending changes

**Splice should follow Convention 2** (git diff pattern) because:
1. Splice is a code transformation tool (like git)
2. Pre-commit hooks need to detect changes (CLI-07 requirement)
3. Consistent with diff-oriented tools

**Implementation:**
```rust
// In main.rs, modify execute_patch return handling:
let exit_code = if preview {
    if report.lines_added > 0 || report.lines_removed > 0 {
        ExitCode::from(1)  // Changes pending
    } else {
        ExitCode::SUCCESS   // No changes
    }
} else {
    match result {
        Ok(_) => ExitCode::SUCCESS,
        Err(_) => ExitCode::from(1),
    }
};
```

## Open Questions

1. **Batch operations with dry-run**
   - What we know: `--preview` conflicts with `--batch` (src/cli/mod.rs:124)
   - What's unclear: Should dry-run support batch operations with multi-file diffs?
   - Recommendation: Add batch diff support if time permits, otherwise defer to later phase

2. **Delete command dry-run**
   - What we know: `delete` command has no `--preview` flag currently
   - What's unclear: Should `--dry-run` work for delete too?
   - Recommendation: Add `--dry-run` to delete for consistency (CLI-07 implies universal)

3. **Colored output in JSON mode**
   - What we know: `--json` flag exists globally
   - What's unclear: Should `--json --dry-run` include color? (JSON should never have ANSI)
   - Recommendation: JSON mode disables colors unconditionally

## Sources

### Primary (HIGH confidence)
- [similar crate documentation](https://docs.rs/similar/latest/similar/) - Unified diff functionality, TextDiff API
- [similar::udiff module](https://docs.rs/similar/latest/similar/udiff/index.html) - Unified diff generation
- [nu-ansi-term documentation](https://docs.rs/nu-ansi-term/latest/nu_ansi_term/) - Color output API
- [is-terminal documentation](https://docs.rs/is-terminal/latest/is_terminal/) - TTY detection
- [NO_COLOR standard](https://no-color.org/) - Official NO_COLOR specification
- [Git diff documentation](https://git-scm.com/docs/git-diff) - Exit code convention for --exit-code

### Secondary (MEDIUM confidence)
- [DRYRUN_CONVENTIONS.md](/home/feanor/Projects/splice/.planning/research/DRYRUN_CONVENTIONS.md) - Prior project research, HIGH confidence internally
- [CLI_UX_PATTERNS.md](/home/feanor/Projects/splice/.planning/research/CLI_UX_PATTERNS.md) - Prior project research on CLI patterns
- [GitHub Issue: rectorphp dry-run exit code](https://github.com/rectorphp/rector/issues/800) - Discussion of dry-run exit code conventions
- [Stack Overflow: git diff exit code](https://stackoverflow.com/questions/69615398/how-to-use-exit-code-of-git-diff-in-pre-commit-hook) - Pre-commit usage patterns

### Tertiary (LOW confidence)
- [sitkevij/no_color crate](https://github.com/sitkevij/no_color) - Rust crate for NO_COLOR (not needed, can use std::env)
- [JFrog CLI Issue #682](https://github.com/jfrog/jfrog-cli/issues/682) - Debate about dry-run exit codes

## Metadata

**Confidence breakdown:**
- Standard stack: HIGH - Verified with official docs.rs documentation
- Architecture: HIGH - Based on verified crate capabilities
- Pitfalls: HIGH - Cross-referenced with prior research (DRYRUN_CONVENTIONS.md)
- Exit code convention: MEDIUM - Competing conventions exist, git diff pattern is relevant match

**Research date:** 2026-01-22
**Valid until:** 90 days (stable ecosystem, standard crates)
