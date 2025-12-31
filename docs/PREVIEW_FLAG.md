# `--preview` Flag

`splice patch --preview` lets operators and agents see what Splice would do without touching the actual workspace. The implementation uses:

1. `src/cli/mod.rs::Commands::Patch` adds `--preview` (conflicts with `--batch`).
2. `src/main.rs::execute_patch` delegates preview flows to `src/patch/mod.rs::preview_patch`, which clones the workspace, applies the edit in a temp directory, and runs tree-sitter, cargo, and optional rust-analyzer gates there.
3. `src/patch/mod.rs::PreviewReport` captures the affected file, line span, bytes changed, and line/byte counts; `build_success_payload` adds it to the `CliSuccessPayload::data`.

Preview command example:

```bash
splice patch --file src/lib.rs --symbol greet --with new_greet.rs --preview
```

The CLI prints JSON with `status:"ok"` and a `data` object similar to:

```json
{
  "status": "ok",
  "message": "Previewed patch 'greet' at bytes 120..210 (hash: ...)",
  "data": {
    "files": [{
      "file": "/repo/src/lib.rs",
      "before_hash": "...",
      "after_hash": "..."
    }],
    "preview_report": {
      "file": "/repo/src/lib.rs",
      "line_start": 12,
      "line_end": 18,
      "lines_added": 3,
      "lines_removed": 2,
      "bytes_added": 85,
      "bytes_removed": 40
    }
  }
}
```

Because the preview runs entirely in a temporary copy, the real workspace files remain unchanged even after the validations complete. downstream tooling can rely on the `preview_report` metadata to show diffs or highlight impacted ranges without needing to recompute hashes.
