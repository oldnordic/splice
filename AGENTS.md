# Repository Guidelines

## Project Structure & Module Organization
Splice is a Rust binary crate defined in `Cargo.toml`. Core logic sits in `src/`: `src/cli` handles argument parsing, `src/ingest` performs tree-sitter driven symbol extraction, `src/resolve` finds cross-file references, and `src/patch` plus `src/validate` enforce editing and compiler gates. SQLite graph plumbing lives in `src/graph`, while plan orchestration is in `src/plan`. Regression suites live in `tests/` (e.g., `tests/python_patch_tests.rs`). Reference docs are under `docs/`, with hands-on walkthroughs in `manual.md` and `QUICKSTART.md`. Build outputs stay in `target/`; do not commit artifacts.

## Build, Test, and Development Commands
- `cargo fmt --all`: format every crate file per rustfmt before review.
- `cargo clippy --all-targets --all-features -D warnings`: lint with strict deny rule so PRs stay warning-free.
- `cargo test --all`: run the 300+ unit and integration suites covering CLI, validation gates, and per-language ingest logic.
- `cargo run -- --help`: smoke-test the binary and confirm CLI flag wiring.
- `cargo build --release`: produce an optimized binary before publishing or benchmarking.

## Coding Style & Naming Conventions
Use Rust 2021 defaults: four-space indentation, no tabs, and snake_case for functions/modules while keeping CamelCase for types and UpperCamelCase enums. Keep modules focused and prefer `mod.rs` files that re-export cohesive submodules, mirroring the current directory layout. Error messages should come from `thiserror` enums in `src/error.rs`, and logging should use `env_logger` macros with lowercase targets. Always run rustfmt; never hand-align code.

## Testing Guidelines
Every change needs coverage in the matching `tests/*_tests.rs` file; mirror the language-specific suites when touching ingest or resolver logic. Favor table-driven helpers that generate temp projects (see `tests/cli_tests.rs`). Long-running validations (compiler invocations) can be gated behind fixtures, but plain `cargo test` must stay green. If you add multi-step flows, script them in `docs/QUICKSTART.md` so others can reproduce them manually.

## Commit & Pull Request Guidelines
Follow the Conventional Commit style already in history (`fix: parser handles impl_item`, `feat(delete): ...`). Scope prefixes are encouraged when touching a subsystem. Reference issues in the body, describe validation steps, and attach CLI output or logs when behavior changes. PRs should outline reproduction steps, list new test names, and mention language runtimes required for validation.

## Validation & Safety Tips
Splice operations mutate files; always work on a clean branch and keep backups via git. Honor the three validation gates (UTF-8 boundary, tree-sitter parse, compiler check) when adding functionality, and document any temporary bypass in code or PR notes.
