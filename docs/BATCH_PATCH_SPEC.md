# Batch Patch Specification

This document records the format consumed by `src/patch/batch_loader.rs::load_batches_from_file` and how `splice patch --batch` executes the manifest (`src/cli/mod.rs::Commands::Patch`, `src/main.rs::execute_patch_batch`).

## JSON Schema

```jsonc
{
  "batches": [
    {
      "replacements": [
        {
          "file": "src/lib.rs",        // relative to the batch file directory or absolute path
          "start": 120,                // byte offsets, inclusive start / exclusive end
          "end": 240,
          "content": "inline text"     // mutually exclusive with `with`
          // "with": "patches/new_impl.rs"
        }
      ]
    }
  ]
}
```

- `batches` – array of operations that must succeed atomically. The CLI currently applies every entry in the file as a single batch for `apply_batch_with_validation`.
- `replacements` – ordered entries resolved per file. Each item requires `file`, `start`, `end`, and either inline `content` or a `with` path (relative or absolute) pointing to a file whose contents will be inlined verbatim.
- Paths are resolved against `batch.json`’s parent directory. This means manifests can live under `plans/` while referencing nearby source files.

## CLI Invocation

```
splice patch --batch plans/batch.json --language rust [--analyzer os]
```

- `--batch <file>` activates the loader. All span-specific flags (`--file`, `--symbol`, `--with`, `--kind`) must be omitted.
- `--language` is required for batch mode so `src/patch/mod.rs::apply_batch_with_validation` can select the correct validation gates.
- `--analyzer` behaves like the single-span command (defaults to OFF; `--analyzer os` runs rust-analyzer once when `language=rust`).

## Validation & Rollback

1. `load_batches_from_file` ensures JSON syntax, non-empty batches, and replacement contents. It surfaces schema errors as `SpliceError::InvalidBatchSchema`.
2. `apply_batch_with_validation` (`src/patch/mod.rs:180`) groups replacements per file, verifies byte spans via `validate_replacements`, applies them in-memory with Ropey, and writes atomically.
3. Every touched file is re-parsed through tree-sitter. For Rust, a single `cargo check` (and optional rust-analyzer run) validates the entire workspace. Non-Rust languages call the per-language compiler gates.
4. Any failure rewrites all files back to their original bytes before returning `CargoCheckFailed`, `CompilerValidationFailed`, or the appropriate validation error (`tests/cli_tests.rs::test_cli_batch_patch_rolls_back_on_failure` proves this end-to-end).

## Success Payload

`src/main.rs::execute_patch_batch` emits a `CliSuccessPayload` with both the human message and structured metadata:

```json
{
  "status": "ok",
  "message": "Patched 2 file(s) across 1 batch(es).",
  "data": {
    "batch_file": "/repo/plans/batch.json",
    "batches_applied": 1,
    "files": [
      {
        "file": "/repo/src/a.rs",
        "before_hash": "59a4…",
        "after_hash": "acd1…"
      }
    ]
  }
}
```

Single-span `splice patch` responses follow the same schema with a `files` array of length 1. Downstream tooling should rely on this metadata (file path + before/after SHA-256) for auditing instead of re-hashing the filesystem.

Keep batch files small enough that humans can review the byte offsets. Future work (Task F) will add metadata hashes to the CLI payload, but the schema above is stable and grounded in the current implementation.
