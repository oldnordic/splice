# Splice

Span-safe refactoring kernel for Rust using tree-sitter and SQLiteGraph.

**Version**: 0.2.1
**License**: GPL-3.0-or-later

## What This Is

Splice is a command-line tool that performs byte-accurate, AST-validated refactoring operations on Rust code. It can replace function bodies and delete symbols along with all their references (across files).

## What This Is NOT

- An IDE - Use Rust Analyzer or IntelliJ Rust
- A general-purpose refactoring tool - Focused on specific operations
- A complete solution - It's a focused tool for specific jobs
- Production-hardened - Use with version control

## What It Does

- **patch**: Replace function bodies, struct definitions, enum variants with validation
- **delete**: Remove symbol definitions and all references (cross-file)
- Validates syntax with tree-sitter after every operation
- Validates compilation with cargo check after every operation
- Rolls back atomically on any failure
- Orchestrates multi-step refactors via JSON plans

## Installation

```bash
cargo install splice
```

Or from source:
```bash
git clone https://github.com/oldnordic/splice.git
cd splice
cargo build --release
cp target/release/splice ~/.local/bin/
```

## Quick Start

### Delete a Symbol

Delete a function and all its references:

```bash
splice delete --file src/lib.rs --symbol helper --kind function
```

Output:
```
Deleted 'helper' (3 references + definition) across 2 file(s).
```

### Patch a Symbol

Replace a function body:

```bash
cat > new_greet.rs << 'EOF'
pub fn greet(name: &str) -> String {
    format!("Hi, {}!", name)
}
EOF

splice patch --file src/lib.rs --symbol greet --kind function --with new_greet.rs
```

### Multi-Step Plan

```bash
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

splice plan --file plan.json
```

## Commands

### splice delete

Remove a symbol definition and all its references.

```bash
splice delete --file <PATH> --symbol <NAME> [--kind <KIND>]
```

Finds references across the entire workspace:
- Same-file references (function calls, type mentions)
- Cross-file references (via imports)
- Handles shadowing (local variables don't count)
- Follows re-export chains

### splice patch

Apply a patch to a symbol's span.

```bash
splice patch --file <PATH> --symbol <NAME> --with <FILE> [--kind <KIND>]
```

### splice plan

Execute a multi-step refactoring plan.

```bash
splice plan --file <PLAN.json>
```

## Documentation

- **manual.md** - Complete user manual
- **CHANGELOG.md** - Version history

## Requirements

- Rust 1.70+ (for building)
- Cargo workspace (for validation)

## Architecture

- **src/cli/** - CLI argument parsing
- **src/ingest/** - File parsing (Rust; import extraction for other languages for future cross-lang support)
- **src/graph/** - SQLiteGraph integration
- **src/resolve/** - Symbol resolution and reference finding
- **src/patch/** - Span-safe replacement + validation
- **src/validate/** - Tree-sitter + cargo check validation
- **src/plan/** - JSON plan orchestration

## Validation Gates

Every operation passes:
1. UTF-8 boundary validation
2. Tree-sitter reparse (syntax check)
3. Cargo check (compilation check)

## Testing

```bash
cargo test
```

Test Coverage: 298/298 tests passing

## Feedback

If you use this tool and find a bug or miss a feature that would be useful, drop a line. It's appreciated.

## License

GPL-3.0-or-later

## Disclaimer

This software modifies source code. Always commit your changes before running Splice.
