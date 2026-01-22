# Dry-Run and Diff Conventions Research

**Project:** Splice
**Researched:** 2026-01-22
**Mode:** Ecosystem Research
**Confidence:** HIGH

## Executive Summary

Based on comprehensive research of established CLI tools (git, kubectl, rsync, cargo), the standard conventions for dry-run mode are well-established and consistent. The universally accepted pattern is **`-n, --dry-run`** flag naming with **unified diff format** output using **red for deletions, green for additions**.

For Splice's refactoring tools, we should follow these conventions rather than invent new ones. Users already have mental models from git and other tools, and diverging from these patterns creates friction.

## Key Findings

### Flag Naming: `-n, --dry-run` is Universal

**Confidence:** HIGH (verified with multiple authoritative sources)

The standard across all major CLI tools:
- **Short flag:** `-n`
- **Long flag:** `--dry-run`
- **Rationale:** The letter "n" appears in "dry-run" and is not commonly used for other purposes

**Examples from established tools:**
- `rsync -n, --dry-run` - Preview changes without syncing
- `git add -n, --dry-run` - Show what would be added without staging
- `kubectl apply --dry-run=server` - Preview Kubernetes changes
- `make -n` - Print commands without executing

**Alternative flag names found in research:**
- `--preview`: Used by some tools, but less common
- `--what-if`: Rare, mostly in Microsoft tools
- `--diff`: Confusing (implies showing diff between two existing states)
- `--check`: Used by tools like cargo-rail for validation-only mode

**Recommendation:** Use `-n, --dry-run` for Splice. This matches user expectations from git and follows [CLI Guidelines](https://clig.dev/).

### Output Format: Unified Diff is Standard

**Confidence:** HIGH (official Git documentation verified)

The unified diff format is the de facto standard for showing changes. According to [Git's official diff-format documentation](https://git-scm.com/docs/diff-format):

```
diff --git a/file.rs b/file.rs
index abc123..def456 100644
--- a/file.rs
+++ b/file.rs
@@ -10,5 +10,5 @@
 fn old_name() {
-    println!("old");
+    println!("new");
 }
```

**Key characteristics:**
- `---` marks the original file (deletions)
- `+++` marks the modified file (additions)
- `-` prefix for removed lines
- `+` prefix for added lines
- Hunk headers `@@ -start,count +start,count @@` show location
- Compatible with `patch` command for application

**Kubernetes approach:** [kubectl diff](https://kubernetes.io/blog/2019/01/14/apiserver-dry-run-and-kubectl-diff/) shows differences between "live" and "dry-run" objects, making it easy to focus on actual changes.

### Color Convention: Red/Green is Universal

**Confidence:** HIGH (verified across multiple sources)

The universal color convention for diffs:
- **Red** = Removed/deleted lines (marked with `-`)
- **Green** = Added/inserted lines (marked with `+`)

This convention is used by:
- Git (most VCS tools)
- Linux `diff --color` command
- GitHub, GitLab, and other code review platforms
- Most modern diff visualization tools

**Rationale:** Red intuitively means "bad/removal" while green means "good/addition" in UI design.

### Essential Information for Dry-Run Output

**Confidence:** HIGH (based on established tool patterns)

Based on analysis of git, kubectl, and other tools, dry-run output should include:

1. **Summary header** - What operation would be performed
2. **File list** - Which files would be modified
3. **Line counts** - Number of additions/deletions per file (like `git --stat`)
4. **Actual diff** - Unified diff format showing exact changes
5. **Exit code** - Return 0 if no changes, 1 if changes would be made

**Example structure:**
```
Would refactor 3 files:

 src/lib.rs    | 4 +--
 src/main.rs   | 12 ++++++------
 src/utils.rs  | 2 +-

 3 files changed, 9 insertions(+), 9 deletions(-)

diff --git a/src/lib.rs b/src/lib.rs
...
```

### Output Mode Variations

**Confidence:** MEDIUM (observed across tools)

Common output format flags:

1. **`--color` / `--no-color`** - Enable/disable colored output
   - Respect `NO_COLOR` environment variable
   - Auto-detect TTY (disable colors when piped)
   - Follow [no-color.org](https://no-color.org/) guidelines

2. **`--json`** - Machine-readable JSON output
   - Structure diff data for programmatic consumption
   - Used by modern tools like `gh` (GitHub CLI)

3. **`--stat`** - Summary statistics only
   - Like `git diff --stat`
   - Shows files changed and line counts without full diff

4. **`--quiet` / `-q`** - Minimal output
   - Only show errors, suppress normal output
   - Useful for scripts

## Implementation Recommendations for Splice

### 1. Flag Design

```bash
# Recommended
splice refactor rename --old=foo --new=bar -n          # Short form
splice refactor rename --old=foo --new=bar --dry-run   # Long form

# Avoid (non-standard)
splice refactor rename --preview      # Confusing
splice refactor rename --what-if      # Uncommon
```

### 2. Output Format

**Default (TTY):** Colored unified diff with summary
```bash
$ splice refactor rename old_func new_func --dry-run

Would rename 1 function in 2 files:

 src/lib.rs    | 2 +-
 src/main.rs   | 2 +-

 2 files changed, 2 insertions(+), 2 deletions(-)

diff --git a/src/lib.rs b/src/lib.rs
@@ -10,7 +10,7 @@
 mod utils;

-fn old_func() -> String {
+fn new_func() -> String {
     "hello".to_string()
 }
```

**Piped/File (non-TTY):** Plain text without color
```bash
$ splice refactor rename old_func new_func --dry-run | grep "src/lib.rs"
# Still produces unified diff, but without ANSI color codes
```

**JSON mode (optional):**
```bash
$ splice refactor rename old_func new_func --dry-run --json
{
  "files": [
    {
      "path": "src/lib.rs",
      "additions": 1,
      "deletions": 1,
      "changes": [...]
    }
  ]
}
```

### 3. Color Detection Logic

```rust
// Pseudo-code for color detection
fn should_use_color() -> bool {
    // Check explicit flag
    if args.contains("--no-color") { return false; }
    if args.contains("--color") { return true; }

    // Check environment variables
    if env::var("NO_COLOR").is_ok() { return false; }
    if env::var("TERM").unwrap_or_default() == "dumb" { return false; }

    // Check if stdout is a TTY
    atty::is(atty::Stream::Stdout)
}
```

### 4. Exit Codes

```bash
# No changes would be made
$ splice refactor rename foo bar --dry-run
echo $?  # 0

# Changes would be made
$ splice refactor rename foo bar --dry-run
# (shows diff)
echo $?  # 1

# Error occurred
$ splice refactor rename foo bar --dry-run
# (shows error)
echo $?  # 2
```

## Anti-Patterns to Avoid

1. **Don't invent new flag names**
   - ❌ `--preview`, `--check`, `--simulate`
   - ✅ `-n, --dry-run`

2. **Don't use non-standard diff formats**
   - ❌ Side-by-side by default, custom formats
   - ✅ Unified diff (git-style)

3. **Don't use non-standard colors**
   - ❌ Blue/yellow for additions/deletions
   - ✅ Red/green (universally understood)

4. **Don't suppress essential information**
   - ❌ Only showing "would apply 3 changes" without details
   - ✅ Show summary + full diff

5. **Don't ignore TTY detection**
   - ❌ Always using colors (breaks piping, logs)
   - ✅ Auto-detect and respect NO_COLOR

## Sources

### HIGH Confidence (Official Documentation)
- [Git - diff-format Documentation](https://git-scm.com/docs/diff-format) - Official unified diff specification
- [APIServer dry-run and kubectl diff | Kubernetes](https://kubernetes.io/blog/2019/01/14/apiserver-dry-run-and-kubectl-diff/) - Official Kubernetes blog
- [Command Line Interface Guidelines](https://clig.dev/) - CLI conventions including `-n, --dry-run`

### MEDIUM Confidence (Verified Sources)
- [Stack Overflow: Why "-n" is commonly used for dry-run](https://stackoverflow.com/questions/64534753/why-n-is-commonly-used-for-dry-run) - Explanation of flag naming
- [Stack Overflow: git-diff console default colors](https://stackoverflow.com/questions/69166648/git-diff-console-default-colors-meanings) - Color conventions
- [Unix StackExchange: How to color diff output](https://unix.stackexchange.com/questions/196565/how-to-color-diff-output) - Color discussion

### Additional References
- [How to Use diff --color: 7 Practical Examples](https://medium.com/@redswitches/how-to-use-diff-color-7-practical-examples-46de448e46a6)
- [Understanding Diff Formats: A Developer's Guide](https://dev.to/shrsv/understanding-diff-formats-a-developers-guide-to-making-sense-of-changes-414o)
- [Diff Formats - Mercurial SCM](https://mercurial-scm.org/help/topics/diffs.html)

## Confidence Assessment

| Area | Confidence | Reason |
|------|------------|--------|
| Flag naming (-n, --dry-run) | HIGH | Verified with clig.dev and multiple tool examples |
| Unified diff format | HIGH | Official Git documentation verified |
| Color conventions (red/green) | HIGH | Cross-verified across multiple sources |
| Essential output info | HIGH | Based on established tool patterns (git, kubectl) |
| Output mode variations | MEDIUM | Observed across tools, but less standardized |

## Gaps and Open Questions

None significant. The conventions are well-established and consistent across the ecosystem. The only area requiring Splice-specific decisions is:

1. **JSON schema** - If providing `--json` output, need to design the structure (no standard exists)
2. **Stat format** - Whether to show git-style stat summary (recommended) or custom metrics

## Roadmap Implications

Based on this research, the dry-run implementation should:

1. **Start with standard `-n, --dry-run` flag** - Matches user expectations
2. **Output unified diff format** - Leverage existing Rust diff libraries (similar/imprint)
3. **Implement color detection** - Respect NO_COLOR, detect TTY
4. **Provide summary stats** - Files changed, line counts
5. **Exit code 1 if changes** - Standard pattern for "changes detected"

No significant research flags for phases - dry-run is a well-understood pattern with established conventions.
