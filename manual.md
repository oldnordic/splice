# Splice Manual

**Version**: 0.2.0

---

## Overview

Splice is a span-safe refactoring tool for Rust code that performs byte-accurate, AST-validated operations.

### Core Philosophy

1. **Span Safety**: All operations work with exact byte spans
2. **Validation Gates**: Every operation passes tree-sitter reparse + cargo check
3. **Atomic Rollback**: Any failure triggers automatic rollback
4. **No Magic**: Explicit parameters only

### What Splice Does

- Replace function bodies, struct definitions, enum variants, trait definitions, impl blocks
- Delete symbol definitions and all references (cross-file)
- Validate syntax with tree-sitter after every operation
- Validate semantics with cargo check after every operation
- Rollback atomically if validation fails
- Orchestrate multi-step refactors with JSON plans

### What Splice Does NOT Do

- Automatic symbol discovery
- Configuration files
- Persistent databases
- Resume mode for failed plans

---

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

---

## Command Reference

### splice delete

Remove a symbol definition and all its references.

```bash
splice delete --file <PATH> --symbol <NAME> [--kind <KIND>]
```

**Required Arguments:**
- `--file <PATH>`: Path to source file containing the symbol
- `--symbol <NAME>`: Symbol name to delete

**Optional Arguments:**
- `--kind <KIND>`: Symbol kind (function, struct, enum, trait, impl)
- `--analyzer <MODE>`: rust-analyzer mode (off, os, path)
- `-v, --verbose`: Enable verbose logging

**How It Works:**
1. Finds the symbol definition in the specified file
2. Searches for all references in the workspace:
   - Same-file references (unqualified calls, qualified paths)
   - Cross-file references (via imports)
3. Handles shadowing correctly (local variables don't count as references)
4. Follows re-export chains to find indirect references
5. Deletes references first (reverse byte order per file to keep offsets valid)
6. Deletes the definition last

**Example:**
```bash
splice delete --file src/lib.rs --symbol helper --kind function
```

Output:
```
Deleted 'helper' (3 references + definition) across 2 file(s).
```

### splice patch

Apply a single patch to a symbol's span.

```bash
splice patch --file <PATH> --symbol <NAME> --with <FILE> [--kind <KIND>]
```

**Required Arguments:**
- `--file <PATH>`: Path to source file
- `--symbol <NAME>`: Symbol name to patch
- `--with <FILE>`: Path to replacement file

**Optional Arguments:**
- `--kind <KIND>`: Symbol kind (function, struct, enum, trait, impl)
- `--analyzer <MODE>`: rust-analyzer mode (off, os, path)
- `-v, --verbose`: Enable verbose logging

**Symbol Kinds:**

| Kind | Example |
|------|---------|
| `function` | `pub fn foo() {}` |
| `struct` | `pub struct Foo;` |
| `enum` | `pub enum Bar {}` |
| `trait` | `pub trait Baz {}` |
| `impl` | `impl Foo {}` |

### splice plan

Execute a multi-step refactoring plan.

```bash
splice plan --file <PLAN.json>
```

**Execution Behavior:**
1. Steps execute sequentially
2. Stops on first failure
3. Previous successful steps remain applied
4. Each step has atomic rollback

---

## Quick Start Examples

### Delete a Function

**Source** (`src/lib.rs`):
```rust
pub fn helper() -> i32 {
    42
}

pub fn main() {
    let x = helper();
    println!("{}", x);
}
```

**Command:**
```bash
splice delete --file src/lib.rs --symbol helper --kind function
```

**Result:**
```rust
pub fn main() {
    let x = ();
    println!("{}", x);
}
```

### Patch a Function

**Original** (`src/lib.rs`):
```rust
pub fn greet(name: &str) -> String {
    format!("Hello, {}!", name)
}
```

**Patch** (`new_greet.rs`):
```rust
pub fn greet(name: &str) -> String {
    format!("Greetings, {}!", name)
}
```

**Command:**
```bash
splice patch --file src/lib.rs --symbol greet --kind function --with new_greet.rs
```

### Multi-Step Plan

Create `plan.json`:
```json
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
```

Execute:
```bash
splice plan --file plan.json
```

---

## Reference Finding Details

### Same-File References

The delete command finds:
- Unqualified function calls: `foo()`
- Qualified paths: `crate::module::foo()`
- Method calls: `obj.method()`
- Trait methods: `Trait::method(obj)`

### Cross-File References

For public symbols, searches workspace files:
1. Extracts imports from all Rust files
2. Matches imports to the target symbol
3. Finds references in files that import the symbol

### Shadowing Detection

Local definitions correctly shadow imports:

```rust
use crate::utils::helper;  // Import

fn main() {
    helper();  // This IS a reference (to import)

    fn helper() {  // Shadows the import
        println!("local");
    }

    helper();  // This is NOT a reference (local, not import)
}
```

### Re-Export Chains

Follows `pub use` re-exports:

```rust
// utils.rs
pub fn helper() -> i32 { 42 }

// mod_a.rs
pub use crate::utils::helper;  // Re-export

// main.rs
use crate::mod_a::helper;  // Imports via re-export
helper();  // Found when deleting utils::helper
```

---

## Error Handling

### Common Errors

**Symbol Not Found:**
```
Error: Symbol not found: nonexistent
```
Check symbol name and verify `--file` path.

**Ambiguous Symbol:**
```
Error: Ambiguous symbol 'foo': found in multiple files
```
Add `--file` to disambiguate.

**Parse Validation Failed:**
```
Error: Parse validation failed - Tree-sitter detected syntax errors
```
Check patch file for syntax errors.

**Cargo Check Failed:**
```
Error: Cargo check failed - mismatched types
```
Fix type errors in patch file.

---

## Validation Gates

Every operation passes:
1. UTF-8 boundary validation
2. Tree-sitter reparse
3. Cargo check

**Rollback Behavior:**
- Automatic on any failure
- Atomic (temp + fsync + rename)
- No partial patch states

---

## Best Practices

**DO:**
- Run `cargo check` on patch files before using splice
- Use `--kind` to disambiguate when needed
- Test patches in git repos
- Use verbose mode for debugging
- Create backup branches

**DON'T:**
- Manually edit files after splice starts
- Skip `cargo check` on patch files
- Use patch files with syntax errors
- Run delete on symbols without committing first

---

## Technical Details

**How Splice Works:**

**Patch:**
1. Extract symbols via tree-sitter
2. Resolve symbol byte span
3. Read replacement file
4. Replace span with ropey
5. Validate with tree-sitter + cargo check
6. Commit or rollback atomically

**Delete:**
1. Extract symbols via tree-sitter
2. Find symbol definition
3. Build workspace import index
4. Find all references (same-file + cross-file)
5. Delete references (reverse byte order per file)
6. Delete definition
7. Validate each file with tree-sitter + cargo check
8. Rollback on any failure

**Why Byte Spans:**
- Deterministic (independent of line endings)
- Exact (no ambiguity)
- Fast (no conversion overhead)

---

## License

GPL-3.0-or-later

---

**End of Manual**

For quick help:
```bash
splice --help
splice delete --help
splice patch --help
splice plan --help
```
