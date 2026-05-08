# Splice v2.6.4 Release Notes

**Release Date:** 2026-05-08

Splice v2.6.4 is a bug-fix release that resolves two critical issues: cross-file rename not updating the definition site, and `splice undo` failing to restore rename-format backup manifests.

---

## What's Fixed

### Cross-File Rename Definition Bug

`splice rename` now correctly renames both the **definition site** (the symbol declaration) and all **call sites** across all indexed files.

**Previously:** Rename updated only references (call sites), leaving `fn old_name()` unchanged while all callers referenced `fn new_name()`. The resulting code would not compile.

**Now:** The definition name is extracted using a name-only byte span inside the declaration and included in the replacement set. The fix also enables renaming symbols with zero callers (previously blocked by an empty-reference guard).

**Example:**
```bash
splice rename --db .magellan/splice.db --name "foo" --file src/main.rs --to "bar"
```
This now renames `fn foo()` to `fn bar()` AND every `foo()` call site in the same transaction.

### Undo Manifest Format Mismatch

`splice undo --manifest <path>` now works for both patch-format and rename-format backup manifests.

**Previously:** The undo reader only understood `BackupManifest` (patch format, `files: Vec<BackupEntry>`). Restoring a rename backup produced: `Failed to parse manifest: expected a sequence at line 4 column 11`.

**Now:** `restore_from_manifest()` attempts both formats, converting rename manifests (`HashMap<String, String>`) internally so the same restore logic handles all backups.

---

## Installation

```bash
cargo install splice
```

Or build from source:

```bash
git clone https://github.com/oldnordic/splice
cd splice
cargo build --release
cp target/release/splice ~/.local/bin/
```

## Upgrade from v2.6.x

```bash
cargo install splice --force
```

## Full Changelog

See [CHANGELOG.md](CHANGELOG.md) for complete version history.
