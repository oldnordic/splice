# Splice

Span-safe refactoring kernel for Rust using SQLiteGraph.

**Status**: MVP / Proof of Concept
**Version**: 0.1.0
**License**: GPL-3.0-or-later

## What This Is

Splice is a command-line tool that performs byte-accurate, AST-validated replacements of Rust code. Think of it as `sed` that:
- Understands Rust syntax via tree-sitter
- Validates replacements with tree-sitter reparse
- Validates replacements with cargo check
- Performs atomic rollback on validation failures

## What This Is NOT

❌ An IDE - Use Rust Analyzer or IntelliJ Rust
❌ A semantic refactoring tool - It doesn't track cross-file references
❌ A complete solution - It's a focused tool for one specific job
❌ Production-hardened - It's an MVP with known limitations

## What It Does Well

✅ Replaces function bodies, struct definitions, enum variants
✅ Validates syntax after every patch
✅ Validates compilation after every patch
✅ Rolls back atomically on any failure
✅ Orchestrates multi-step refactors via JSON plans

## Known Limitations

- No cross-file reference tracking (won't find all call sites)
- No persistent database (creates graph on-the-fly for each patch)
- No resume mode for failed plans (partial state must be manually cleaned up)
- No dry-run mode (can't preview without applying)
- No auto-discovery of symbols (you must know exact names)
- Single-file symbol resolution only

## Installation

### From Source

```bash
# Clone repository
git clone https://github.com/oldnordic/splice.git
cd splice

# Build release binary
cargo build --release

# Install to user bin
mkdir -p ~/.local/bin
cp target/release/splice ~/.local/bin/splice

# Add to PATH (add to ~/.bashrc)
echo 'export PATH="$HOME/.local/bin:$PATH"' >> ~/.bashrc
source ~/.bashrc
```

### System-Wide Install (Optional)

```bash
sudo cp target/release/splice /usr/local/bin/splice
```

## Quick Start

### Single Patch

```bash
# Create replacement file
cat > new_greet.rs << 'EOF'
pub fn greet(name: &str) -> String {
    format!("Hi, {}!", name)
}
EOF

# Apply patch
splice patch \
  --file src/lib.rs \
  --symbol greet \
  --kind function \
  --with new_greet.rs
```

### Multi-Step Plan

```bash
# Create plan.json
cat > plan.json << 'EOF'
{
  "steps": [
    {
      "file": "src/lib.rs",
      "symbol": "foo",
      "kind": "function",
      "with": "patches/foo.rs"
    },
    {
      "file": "src/lib.rs",
      "symbol": "bar",
      "kind": "function",
      "with": "patches/bar.rs"
    }
  ]
}
EOF

# Execute plan
splice plan --file plan.json
```

## Documentation

- **manual.md** - Complete user manual with examples and troubleshooting
- **QUICKSTART.md** - Quick reference card

## Requirements

- Rust 1.70+ (for building)
- Cargo workspace (for validation)
- tree-sitter-rust (bundled)
- SQLiteGraph 0.2.4 (local dependency at `../sqlitegraph/sqlitegraph`)

## Architecture

### Modules

- **src/cli/** - CLI argument parsing (clap)
- **src/ingest/** - Rust file parsing (tree-sitter-rust)
- **src/graph/** - SQLiteGraph integration (symbol storage, resolution)
- **src/resolve/** - Symbol → byte span resolution
- **src/patch/** - Span-safe replacement + validation gates
- **src/validate/** - Tree-sitter + cargo check validation
- **src/plan/** - JSON plan orchestration
- **src/error.rs** - Typed error hierarchy

### Validation Gates

Every patch passes:
1. UTF-8 boundary validation
2. Tree-sitter reparse (syntax check)
3. Cargo check (compilation check)
4. [Optional] rust-analyzer (lint check)

## Development Status

**Complete Features**:
- ✅ Span-safe byte replacement
- ✅ Tree-sitter validation
- ✅ Cargo check validation
- ✅ Atomic rollback
- ✅ Multi-step plan execution
- ✅ CLI interface

**Not Implemented**:
- ⛔ Cross-file reference tracking
- ⛔ Persistent symbol database
- ⛔ Resume mode for failed plans
- ⛔ Dry-run mode
- ⛔ Auto-discovery of symbols
- ⛔ Incremental validation

## Testing

```bash
# Run all tests
cargo test

# Run specific test suite
cargo test --test cli_tests
cargo test --test patch_tests
```

**Test Coverage**: 22/22 tests passing
- Plan unit tests: 2/2
- CLI integration tests: 9/9
- Ingest tests: 3/3
- Integration refactor tests: 3/3
- Patch tests: 3/3
- Resolve tests: 2/2

## Examples

See the test files for examples:
- `tests/cli_tests.rs` - CLI integration examples
- `tests/patch_tests.rs` - Patch validation examples
- `tests/integration_refactor.rs` - Symbol resolution examples

## Contributing

This is an MVP / proof-of-concept. The codebase is feature-complete as of v0.1.0.

Bug reports and PRs are welcome, but please note:
- No new features are planned
- Scope is intentionally limited
- This is not a production refactoring tool

## License

GPL-3.0-or-later - See LICENSE file for details

## Acknowledgments

Built with:
- [tree-sitter](https://tree-sitter.github.io/) - AST parsing
- [tree-sitter-rust](https://github.com/tree-sitter/tree-sitter-rust) - Rust grammar
- [SQLiteGraph](https://github.com/oldnordic/sqlitegraph) - Code graph storage
- [ropey](https://github.com/ceedubs/ropey) - Safe text editing
- [clap](https://github.com/clap-rs/clap) - CLI argument parsing

## Author

oldnordic

## Disclaimer

This is experimental software. Use at your own risk. Always commit your changes before running Splice.
