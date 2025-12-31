# Diagnostics Output Contract

This note archives how validation diagnostics leave Splice today so both humans and LLM agents consume the same facts.

## Structured Payload

`src/cli/mod.rs::CliErrorPayload` is the canonical schema emitted on stderr. Every failure prints JSON with:

```json
{
  "status": "error",
  "error": {
    "kind": "CargoCheckFailed",
    "message": "...",
    "file": "/repo",
    "hint": "…",
    "diagnostics": [
      {
        "tool": "cargo-check",
        "level": "error",
        "message": "cannot find value `x`",
        "file": "/repo/src/lib.rs",
        "line": 12,
        "column": 5,
        "code": "E0425",
        "note": "consider importing ...",
        "tool_path": "/usr/bin/cargo",
        "tool_version": "cargo 1.75.0",
        "remediation": "https://doc.rust-lang.org/error-index.html#E0425"
      }
    ]
  }
}
```

The CLI also prints a concise human sentence via `CliSuccessPayload` on stdout, so scripts can ignore it and rely only on JSON.

## Rust Analyzer Gate

`src/validate/mod.rs::gate_rust_analyzer` runs `rust-analyzer check --workspace` whenever the CLI flag `--analyzer=os` is enabled. Any stdout/stderr triggers failure; diagnostics are parsed via `parse_rust_analyzer_output`, down-leveled into the same `Diagnostic` structs used everywhere else, and annotated with `tool_path`, `tool_version`, and remediation URLs when a code exists. That keeps Rust analyzer output JSON-identical to cargo-check diagnostics and avoids bespoke formats.

## Non-Rust Compilers

`src/patch/mod.rs::gate_compiler_validation` routes every non-Rust file through `validate::gates::validate_file`. That dispatcher calls `python -m py_compile`, `gcc`, `g++`, `javac`, `node --check`, or `tsc` (see `src/validate/gates.rs`). Each validator parses its native stderr and converts it into `ValidationError` structs before we wrap them in `Diagnostic` objects. If a compiler is missing we log and continue, but once it runs every diagnostic follows the same JSON contract. Tool metadata comes from `tool_invocation_for_language` so consumers always get `{tool, level, message, file, line, column, code?, note?, tool_path?, tool_version?}` regardless of language.

## Guidance for Agents and Humans

- Treat the CLI JSON as the source of truth; textual summaries are for humans who do not parse JSON.
- Diagnostics arrays may contain errors and warnings together. Always inspect `level`.
- Remediation links are optional; when absent, reference the upstream compiler manual using `tool` and `code`.
- For logs or chat responses, quote the `message`, `file`, and `line` fields verbatim to avoid paraphrasing compiler output.

With this contract codemcp and downstream LLMs can display actionable diagnostics without per-language hacks while humans still see readable descriptions.
