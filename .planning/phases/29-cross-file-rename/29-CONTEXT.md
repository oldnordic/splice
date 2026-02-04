# Phase 29: Cross-File Rename Foundation - Context

**Gathered:** 2026-02-04
**Status:** Ready for planning

## Phase Boundary

Implement byte-accurate cross-file rename using ReferenceFact spans from Magellan v2. Users can rename symbols across all files in a multi-language codebase (Rust, Python, C, C++, Java, JavaScript, TypeScript). The rename operation uses exact byte spans from the code graph, avoiding regex false positives.

## Implementation Decisions

### Symbol identification
- **ID-first with fallback**: Primary method is Symbol ID (32-char BLAKE3 v2 or 16-char SHA-256 v1). Optional name+path lookup for convenience.
- **Ambiguous symbols**: Error with list of all ambiguous symbols and their file paths. Interactive prompt is allowed only as opt-in UX when TTY is available (isatty == true), never in CI/scripts/agent mode. Interactive selection resolves to a SymbolId for internal use.
- **Indexed files only**: Rename only works on files in the Magellan database. No auto-discovery or auto-indexing of additional files.
- **Pre-flight validation**: Verify symbol exists and has references before starting the rename operation.

### Preview experience
- **Unified diff format**: Use git-style unified diff format showing all changes in context.
- **Full diff output**: Show complete diff for every file with changes (no truncation by default).
- **Auto-detect TTY**: Colored diff (red/green) only when terminal supports it (follows git conventions).
- **Pure read-only**: Preview mode has NO filesystem mutation, NO side effects, NO backup creation. Must be pure, read-only, and replayable.

### Error handling
- **Abort and rollback**: If any reference can't be safely updated, abort entire operation and restore modified files from backup.
- **Both error types**: Internal typed errors (FileError, ReferenceError, etc.) exposed externally as SpliceError with SPL-E### codes.
- **Preview = pure**: No backup during preview. All validation (symbol resolution, reference collection, span validation, encoding checks, mtimes/hashes) happens first.
- **Transactional apply**: Only after full validation passes, create backups immediately before applying edits. If any write fails, rollback using backups.
- **Zero unnecessary backups**: No stale artifacts, clean transactional semantics, predictable for CI/automation.

### Backup strategy
- **Central location**: Default to `.splice/backups/` directory with optional `--backup-dir` override.
- **Operation + timestamp**: Backup directories named as `rename-{symbol_id}-{timestamp}` (e.g., `rename-a1b2c3d4-20260204-153045`).
- **Never auto-delete**: Keep all backups forever. User manually cleans up.
- **Transaction scope**: Backup created atomically right before apply, after all validation passes.

### Claude's Discretion
- Exact timestamp format in backup directory names
- Internal error type hierarchy design
- Specific validation checks order and optimization
- Backup file format (full copy vs hardlink vs reflink)

## Specific Ideas

- Preview must be like `git diff` — familiar and readable
- Interactive resolution is a "frontend convenience" only, never implicit behavior
- Transaction pattern: validate → backup → apply → rollback on error
- Think of backups as transaction logs, not temporary files

## Deferred Ideas

None — discussion stayed within phase scope.

---

*Phase: 29-cross-file-rename*
*Context gathered: 2026-02-04*
