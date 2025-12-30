# Splice Manual

**Version**: 0.3.0

---

## Overview

Splice is a span-safe refactoring tool that performs byte-accurate, AST-validated operations on code in 7 languages: Rust, Python, C, C++, Java, JavaScript, and TypeScript.

### Core Philosophy

1. **Span Safety**: All operations work with exact byte spans
2. **Validation Gates**: Every operation passes tree-sitter reparse + language compiler check
3. **Atomic Rollback**: Any failure triggers automatic rollback
4. **No Magic**: Explicit parameters only

### What Splice Does

- Replace function bodies, class definitions, interface definitions, enum variants, trait definitions, impl blocks
- Delete symbol definitions and all references (cross-file for Rust, definition-only for other languages)
- Validate syntax with tree-sitter after every operation
- Validate semantics with language-specific compilers (cargo check, python -m py_compile, javac, etc.)
- Rollback atomically if validation fails
- Orchestrate multi-step refactors with JSON plans

### What Splice Does NOT Do

- Automatic symbol discovery
- Configuration files
- Persistent databases
- Resume mode for failed plans
- Cross-file reference finding for non-Rust languages (planned for future)

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

## Supported Languages

| Language | Extensions | Delete | Patch | Compiler |
|----------|-----------|--------|-------|----------|
| Rust | `.rs` | Full | Full | `cargo check` |
| Python | `.py` | Basic | Full | `python -m py_compile` |
| C | `.c`, `.h` | Basic | Full | `gcc -fsyntax-only` |
| C++ | `.cpp`, `.hpp`, `.cc`, `.cxx` | Basic | Full | `g++ -fsyntax-only` |
| Java | `.java` | Basic | Full | `javac` |
| JavaScript | `.js`, `.mjs`, `.cjs` | Basic | Full | `node --check` |
| TypeScript | `.ts`, `.tsx` | Basic | Full | `tsc --noEmit` |

**Delete modes:**
- **Full** (Rust): Finds all references across files via import tracking
- **Basic** (others): Deletes the symbol definition only

---

## Command Reference

### splice delete

Remove a symbol definition and all its references.

```bash
splice delete --file <PATH> --symbol <NAME> [--kind <KIND>] [--language <LANG>]
```

**Required Arguments:**
- `--file <PATH>`: Path to source file containing the symbol
- `--symbol <NAME>`: Symbol name to delete

**Optional Arguments:**
- `--kind <KIND>`: Symbol kind filter (function, method, class, struct, interface, enum, trait, impl, module, variable, constructor, type-alias)
- `--language <LANG>`: Language override (rust, python, c, cpp, java, java-script, type-script)
- `--analyzer <MODE>`: Validation mode (off, os, path)
- `-v, --verbose`: Enable verbose logging

**Rust-specific features:**
1. Finds the symbol definition in the specified file
2. Searches for all references in the workspace:
   - Same-file references (unqualified calls, qualified paths)
   - Cross-file references (via imports)
3. Handles shadowing correctly (local variables don't count as references)
4. Follows re-export chains to find indirect references
5. Deletes references first (reverse byte order per file to keep offsets valid)
6. Deletes the definition last

**Other languages:**
- Deletes the symbol definition only
- Language auto-detected from file extension

**Example (Rust):**
```bash
splice delete --file src/lib.rs --symbol helper --kind function
```

Output:
```
Deleted 'helper' (3 references + definition) across 2 file(s).
```

**Example (Python):**
```bash
splice delete --file utils.py --symbol old_function --language python
```

### splice patch

Apply a single patch to a symbol's span.

```bash
splice patch --file <PATH> --symbol <NAME> --with <FILE> [--kind <KIND>] [--language <LANG>]
```

**Required Arguments:**
- `--file <PATH>`: Path to source file
- `--symbol <NAME>`: Symbol name to patch
- `--with <FILE>`: Path to replacement file

**Optional Arguments:**
- `--kind <KIND>`: Symbol kind filter
- `--language <LANG>`: Language override (auto-detected from file extension by default)
- `--analyzer <MODE>`: Validation mode (off, os, path)
- `-v, --verbose`: Enable verbose logging

**Symbol Kinds:**

| Kind | Languages | Example |
|------|-----------|---------|
| `function` | All | `pub fn foo() {}`, `def foo():`, `function foo() {}` |
| `method` | All | `pub fn foo(&self) {}`, `def foo(self):`, `foo() {}` |
| `class` | Python, JS, TS | `class Foo:`, `class Foo {}` |
| `struct` | Rust, C++ | `pub struct Foo;`, `struct Foo {}` |
| `interface` | Java, TS | `interface Foo {}` |
| `enum` | All | `pub enum Bar {}`, `enum Bar {}` |
| `trait` | Rust | `pub trait Baz {}` |
| `impl` | Rust | `impl Foo {}` |
| `module` | Rust, Python | `mod foo;`, `import foo` |
| `variable` | JS, TS | `const foo = ...` |
| `constructor` | Java, C++ | `public Foo() {}` |
| `type-alias` | Rust, TS, Python | `type Foo = Bar;`, `type Foo = Bar;`, `Foo = Bar` |

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

### Delete a Function (Rust)

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

### Patch a Function (Rust)

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

### Patch a Function (Python)

**Original** (`utils.py`):
```python
def calculate(x: int, y: int) -> int:
    return x + y
```

**Patch** (`new_calc.py`):
```python
def calculate(x: int, y: int) -> int:
    return x * y
```

**Command:**
```bash
splice patch --file utils.py --symbol calculate --language python --with new_calc.py
```

### Patch a Method (Java)

**Original** (`Calculator.java`):
```java
public int add(int a, int b) {
    return a + b;
}
```

**Patch** (`new_add.java`):
```java
public int add(int a, int b) {
    return a + b + 1;
}
```

**Command:**
```bash
splice patch --file Calculator.java --symbol add --kind method --language java --with new_add.java
```

### Patch an Interface (TypeScript)

**Original** (`User.ts`):
```typescript
interface User {
    name: string;
}
```

**Patch** (`new_user.ts`):
```typescript
interface User {
    name: string;
    age: number;
}
```

**Command:**
```bash
splice patch --file User.ts --symbol User --kind interface --with new_user.ts
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

## Reference Finding Details (Rust Only)

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

**Compiler Check Failed (Rust):**
```
Error: Cargo check failed - mismatched types
```
Fix type errors in patch file.

**Python Validation Failed:**
```
Error: Python compilation failed - SyntaxError
```
Fix Python syntax in patch file.

**TypeScript Validation Failed:**
```
Error: tsc validation failed - Type 'number' is not assignable to type 'string'
```
Fix type errors in patch file.

---

## Validation Gates

Every operation passes:
1. UTF-8 boundary validation
2. Tree-sitter reparse (syntax check)
3. Language-specific compiler check

**Compiler by Language:**
- Rust: `cargo check`
- Python: `python -m py_compile`
- C: `gcc -fsyntax-only`
- C++: `g++ -fsyntax-only`
- Java: `javac`
- JavaScript: `node --check`
- TypeScript: `tsc --noEmit`

**Rollback Behavior:**
- Automatic on any failure
- Atomic (temp + fsync + rename)
- No partial patch states

---

## Best Practices

**DO:**
- Run compiler checks on patch files before using splice
- Use `--kind` to disambiguate when needed
- Test patches in git repos
- Use verbose mode for debugging
- Create backup branches
- Specify `--language` for ambiguous file extensions

**DON'T:**
- Manually edit files after splice starts
- Skip compiler validation on patch files
- Use patch files with syntax errors
- Run delete on symbols without committing first

---

## Technical Details

**How Splice Works:**

**Patch:**
1. Detect language from file extension or `--language` flag
2. Extract symbols via tree-sitter (language-specific parser)
3. Resolve symbol byte span
4. Read replacement file
5. Replace span with ropey
6. Validate with tree-sitter reparse
7. Validate with language compiler
8. Commit or rollback atomically

**Delete (Rust):**
1. Extract symbols via tree-sitter
2. Find symbol definition
3. Build workspace import index
4. Find all references (same-file + cross-file)
5. Delete references (reverse byte order per file)
6. Delete definition
7. Validate each file with tree-sitter + cargo check
8. Rollback on any failure

**Delete (other languages):**
1. Extract symbols via tree-sitter
2. Find symbol definition
3. Delete definition span
4. Validate with tree-sitter + language compiler
5. Rollback on any failure

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
