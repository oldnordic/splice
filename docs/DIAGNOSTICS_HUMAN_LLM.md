# Diagnostics Output for Humans and LLMs

This note captures the decision we made while keeping `rust-analyzer` output aligned with the rest of Splice and ensuring non-Rust languages emit the same JSON that downstream agents and humans expect.

## CLI JSON First

The CLI always prints `CliErrorPayload`/`CliSuccessPayload` JSON (see `src/cli/mod.rs::CliErrorPayload`) on stderr/stdout. Every payload includes `status`, `message`, and, when present, structured `diagnostics` entries. Each diagnostic is already normalized (`DiagnosticPayload` fields such as `tool`, `level`, `message`, `file`, `line`, `column`, `code`, `note`, `tool_path`, `tool_version`, `remediation`). Agents SHOULD parse the JSON and use the `diagnostics` array verbatim; humans CAN trust the short text message on stdout for quick confirmation but should quote the JSON fields when sharing code or filing bugs.

## Rust Analyzer Consistency

Rust analyzer is opt-in via `validate::gate_rust_analyzer` (`src/validate/mod.rs::gate_rust_analyzer`). We treat ANY stdout/stderr output as a failure, parse it with `parse_rust_analyzer_output`, and materialize `Diagnostic` objects before serializing through `CliErrorPayload`. There is no bespoke format exposed to consumers; every diagnostic is annotated with `tool_metadata` and remediation links the same way cargo diagnostics (`parse_cargo_output`) already are. The LLM MUST keep relying on the JSON `tool`, `message`, `file`, and `line` fields — that structure is stable and human-readable when quoted verbatim.

## Multi-language Gateways

Splice supports Rust plus six other languages (`src/ingest/detect.rs::Language` enum: Rust, Python, C, C++, Java, JavaScript, TypeScript). The per-language validators in `src/validate/gates.rs::validate_file` call each native compiler (Python: `python -m py_compile`, C: `gcc -fsyntax-only`, C++: `g++ -fsyntax-only`, Java: `javac`, JavaScript: `node --check`, TypeScript: `tsc --noEmit`). Each validator normalizes a `ValidationOutcome` whose `errors`/`warnings` lists are converted into CLI diagnostics with the same JSON contract described above. Non-Rust diagnostics therefore look identical in the payload (`tool`, `level`, `message`, `file`, `line`, `column`, etc.), and agents do not need per-language branches.

## Consumption Guidelines

1. **LLMs**: Always ingest the JSON payload, not prose. Use the `diagnostics` array for severity and location, and only display the short `message` when summarizing. Do not paraphrase the compiler text — quote the `message`, `file`, and `line` fields verbatim so users can copy/paste.
2. **Humans**: Read the CLI message for a quick sense of success or failure, but inspect the JSON when you need precise spans/code references. When referencing diagnostics in docs or chat, copy the `tool`, `code`, `message`, `file`, and `line` fields exactly.
3. **Future languages**: Whenever a new `Language` is added, ensure its validator writes to `ValidationError`/`Diagnostic` so the same JSON fields reach consumers; the doc here should be updated immediately after that change.

This doc ensures we keep a single contract for Rust and every other supported language, making both LLMs and humans rely on the same structured facts.
