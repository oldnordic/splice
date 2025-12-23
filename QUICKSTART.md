# Splice Quick Reference

**Binary**: `splice` (install from source)
**Version**: 0.1.0

## Most Common Commands

```bash
# Single patch
splice patch --file src/lib.rs --symbol foo --kind function --with new_foo.rs

# Multi-step plan
splice plan --file plan.json
```

## Symbol Kinds

| Kind | Use For |
|------|---------|
| `function` | Functions |
| `struct` | Structs |
| `enum` | Enums |
| `trait` | Traits |
| `impl` | Impl blocks |

## Quick Example

```bash
# 1. Create replacement file
cat > new_greet.rs << 'EOF'
pub fn greet(name: &str) -> String {
    format!("Hi, {}!", name)
}
EOF

# 2. Apply patch
splice patch --file src/lib.rs --symbol greet --kind function --with new_greet.rs
```

## Plan Example

```json
{
  "steps": [
    {"file": "src/a.rs", "symbol": "foo", "kind": "function", "with": "patches/foo.rs"},
    {"file": "src/b.rs", "symbol": "bar", "kind": "function", "with": "patches/bar.rs"}
  ]
}
```

## Common Gotchas

❌ Forgot `--file` → Ambiguous symbol error
✅ Add `--file path/to/file.rs`

❌ Syntax error in patch → Parse validation failed
✅ Run `cargo check` on patch file first

❌ Type error → Cargo check failed
✅ Fix types in patch file

## What Gets Validated

Every patch passes:
1. UTF-8 boundary check
2. Tree-sitter reparse (syntax)
3. Cargo check (compilation)
4. Atomic rollback if ANY fail

## Gotchas

- Doesn't find all references (use `rg` or IDE)
- Doesn't auto-discover symbols (you must know exact names)
- Doesn't update call sites (manual step)
- No resume mode (plan fails = partial state)

## For Full Documentation

See `manual.md` for complete guide with examples and troubleshooting.
