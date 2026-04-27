# Splice Quick Reference

**Binary**: `splice` (install from source)
**Version**: 0.3.0

## Supported Languages

Rust, Python, C, C++, Java, JavaScript, TypeScript

## Most Common Commands

```bash
# Rust: Patch a function
splice patch --file src/lib.rs --symbol foo --kind function --with new_foo.rs

# Python: Patch a function
splice patch --file utils.py --symbol bar --language python --with new_bar.py

# TypeScript: Patch a function
splice patch --file src/math.ts --symbol calculate --language type-script --with new_calc.ts

# Rust: Delete a function (with all references)
splice delete --file src/lib.rs --symbol old_func --kind function

# Multi-step plan
splice plan --file plan.json
```

## Symbol Kinds

| Kind | Use For |
|------|---------|
| `function` | Functions (all languages) |
| `method` | Methods (all languages) |
| `class` | Classes (Python, JS, TS) |
| `struct` | Structs (Rust, C++) |
| `interface` | Interfaces (Java, TS) |
| `enum` | Enums (all languages) |
| `trait` | Traits (Rust) |
| `impl` | Impl blocks (Rust) |
| `module` | Modules/namespaces |
| `variable` | Variables (JS, TS) |
| `constructor` | Constructors (Java, C++) |
| `type-alias` | Type aliases (Rust, TS, Python) |

## Language Flags

| Flag | Language |
|------|----------|
| `rust` | Rust (`.rs`) |
| `python` | Python (`.py`) |
| `c` | C (`.c`, `.h`) |
| `cpp` | C++ (`.cpp`, `.hpp`) |
| `java` | Java (`.java`) |
| `java-script` | JavaScript (`.js`) |
| `type-script` | TypeScript (`.ts`, `.tsx`) |

## Quick Examples

### Rust

```bash
# Create replacement
cat > new_greet.rs << 'EOF'
pub fn greet(name: &str) -> String {
    format!("Hi, {}!", name)
}
EOF

# Apply patch
splice patch --file src/lib.rs --symbol greet --kind function --with new_greet.rs
```

### Python

```bash
# Create replacement
cat > new_calc.py << 'EOF'
def calculate(x: int, y: int) -> int:
    return x * y
EOF

# Apply patch
splice patch --file utils.py --symbol calculate --language python --with new_calc.py
```

### JavaScript

```bash
# Create replacement
cat > new_func.js << 'EOF'
function processData(data) {
    return data.map(x => x * 2);
}
EOF

# Apply patch
splice patch --file src/utils.js --symbol processData --with new_func.js
```

### TypeScript

```bash
# Create replacement
cat > new_iface.ts << 'EOF'
interface User {
    name: string;
    email: string;
}
EOF

# Apply patch
splice patch --file src/types.ts --symbol User --kind interface --with new_iface.ts
```

### Java

```bash
# Create replacement
cat > new_method.java << 'EOF'
    public int compute(int x, int y) {
        return x * y;
    }
EOF

# Apply patch
splice patch --file Calculator.java --symbol compute --kind method --language java --with new_method.java
```

### C++

```bash
# Create replacement
cat > new_func.cpp << 'EOF'
int process(int value) {
    return value * 2;
}
EOF

# Apply patch
splice patch --file utils.cpp --symbol process --language cpp --with new_func.cpp
```

## Plan Example

```json
{
  "steps": [
    {"file": "src/lib.rs", "symbol": "foo", "kind": "function", "with": "patches/foo.rs"},
    {"file": "utils.py", "symbol": "bar", "language": "python", "with": "patches/bar.py"},
    {"file": "src/math.ts", "symbol": "calc", "language": "type-script", "with": "patches/calc.ts"}
  ]
}
```

## Common Gotchas

### Wrong language
```bash
# Wrong: Auto-detects .ts as TypeScript when you want JavaScript
splice patch --file file.ts --symbol foo --with patch.js

# Right: Specify language explicitly
splice patch --file file.ts --symbol foo --language java-script --with patch.js
```

### Symbol not found
```bash
# Wrong: Multiple symbols with same name
splice patch --file src/lib.rs --symbol foo --with patch.rs

# Right: Specify kind or file
splice patch --file src/lib.rs --symbol foo --kind function --with patch.rs
```

### Syntax error in patch
```bash
# Wrong: Patch has syntax error
splice patch --file src/lib.rs --symbol foo --with broken.rs

# Right: Validate patch first
cargo check --bin broken.rs  # or python -m py_compile, etc.
```

## What Gets Validated

Every patch passes:
1. UTF-8 boundary check
2. Tree-sitter reparse (syntax)
3. Language compiler check (cargo check, python -m py_compile, node --check, etc.)
4. Atomic rollback if ANY fail

## Language-Specific Validation

| Language | Compiler |
|----------|----------|
| Rust | `cargo check` |
| Python | `python -m py_compile` |
| C | `gcc -fsyntax-only` |
| C++ | `g++ -fsyntax-only` |
| Java | `javac` |
| JavaScript | `node --check` |
| TypeScript | `tsc --noEmit` |

## Gotchas

- **Rust delete**: Finds all references (cross-file)
- **Other languages**: Delete definition only (no reference finding yet)
- **Auto-detection**: Works from file extension
- **Override**: Use `--language` for ambiguous cases

## For Full Documentation

See `manual.md` for complete guide with examples and troubleshooting.

## Get Help

```bash
splice --help
splice delete --help
splice patch --help
splice plan --help
```
