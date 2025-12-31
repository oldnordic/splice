# Splice

Span-safe refactoring kernel for 7 languages using tree-sitter and SQLiteGraph.

**Version**: 0.4.0
**License**: GPL-3.0-or-later

## What This Is

Splice is a command-line tool that performs byte-accurate, AST-validated refactoring operations on code in 7 languages: Rust, Python, C, C++, Java, JavaScript, and TypeScript. It can replace function bodies and delete symbols along with all their references.

## What This Is NOT

- An IDE or LSP server - Use Rust Analyzer, IntelliJ, PyCharm, VSCode, or your editor
- A general-purpose refactoring tool - Focused on specific operations
- A complete solution - It's a focused tool for specific jobs
- Production-hardened - Use with version control

## What It Does

- **patch**: Replace function bodies, class definitions, enum variants with validation
- **delete**: Remove symbol definitions and all references (cross-file, Rust-only)
- Validates syntax with tree-sitter after every operation
- Validates compilation with language-specific compilers
- Rolls back atomically on any failure
- Orchestrates multi-step refactors via JSON plans

## Supported Languages

| Language | Extensions | Delete | Patch | Validation |
|----------|-----------|--------|-------|------------|
| Rust | `.rs` | Full | Full | `cargo check` |
| Python | `.py` | Basic | Full | `python -m py_compile` |
| C | `.c`, `.h` | Basic | Full | `gcc -fsyntax-only` |
| C++ | `.cpp`, `.hpp`, `.cc`, `.cxx` | Basic | Full | `g++ -fsyntax-only` |
| Java | `.java` | Basic | Full | `javac` |
| JavaScript | `.js`, `.mjs`, `.cjs` | Basic | Full | `node --check` |
| TypeScript | `.ts`, `.tsx` | Basic | Full | `tsc --noEmit` |

**Delete modes:**
- **Full**: Finds all references across files (Rust only)
- **Basic**: Deletes definition only, no reference finding (other languages)

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

### Delete a Symbol (Rust)

Delete a function and all its references:

```bash
splice delete --file src/lib.rs --symbol helper --kind function
```

Output:
```
Deleted 'helper' (3 references + definition) across 2 file(s).
```

### Patch a Symbol (Rust)

Replace a function body:

```bash
cat > new_greet.rs << 'EOF'
pub fn greet(name: &str) -> String {
    format!("Hi, {}!", name)
}
EOF

splice patch --file src/lib.rs --symbol greet --kind function --with new_greet.rs
```

### Patch a Symbol (Python)

```bash
cat > new_calc.py << 'EOF'
def calculate(x: int, y: int) -> int:
    return x * y
EOF

splice patch --file utils.py --symbol calculate --language python --with new_calc.py
```

### Patch a Symbol (TypeScript)

```bash
cat > new_fn.ts << 'EOF'
function calculate(x: number, y: number): number {
    return x * y;
}
EOF

splice patch --file src/math.ts --symbol calculate --language type-script --with new_fn.ts
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

**Rust-specific features:**
- Finds references across the entire workspace
- Tracks imports and re-exports
- Handles shadowing correctly
- Cross-file reference resolution

**Other languages:**
- Deletes the symbol definition only
- Use with `--language` flag or auto-detection from file extension

### splice patch

Apply a patch to a symbol's span.

```bash
splice patch --file <PATH> --symbol <NAME> --with <FILE> [--kind <KIND>] [--language <LANG>]
```

**Optional Arguments:**
- `--kind <KIND>`: Symbol kind filter (function, method, class, struct, interface, enum, trait, impl, module, variable, constructor, type-alias)
- `--language <LANG>`: Language override (rust, python, c, cpp, java, java-script, type-script)
- `--analyzer <MODE>`: Validation mode (off, os, path)

### splice plan

Execute a multi-step refactoring plan.

```bash
splice plan --file <PLAN.json>
```

## Documentation

- **manual.md** - Complete user manual
- **CHANGELOG.md** - Version history
- **docs/DIAGNOSTICS_HUMAN_LLM.md** - CLI diagnostics contract for humans and LLMs, including rust-analyzer and multi-language validation output

## Requirements

- Rust 1.70+ (for building)
- Language-specific compilers (for validation):
  - Rust: `cargo`
  - Python: `python`
  - C/C++: `gcc`/`g++`
  - Java: `javac`
  - JavaScript: `node`
  - TypeScript: `tsc`

## Architecture

- **src/cli/** - CLI argument parsing
- **src/ingest/** - Symbol parsing for 7 languages
- **src/graph/** - SQLiteGraph integration
- **src/resolve/** - Symbol resolution and reference finding
- **src/patch/** - Span-safe replacement + validation
- **src/validate/** - Tree-sitter + compiler validation gates
- **src/plan/** - JSON plan orchestration

## Validation Gates

Every operation passes:
1. UTF-8 boundary validation
2. Tree-sitter reparse (syntax check)
3. Language-specific compiler check (cargo check, python -m py_compile, etc.)

## Testing

```bash
cargo test
```

Test Coverage: 339/339 tests passing

## Feedback

If you use this tool and find a bug or miss a feature that would be useful, drop a line. It's appreciated.

## License

GPL-3.0-or-later

## Disclaimer

This software modifies source code. Always commit your changes before running Splice.
