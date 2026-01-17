# Unwrap() Call Audit

**Generated:** 2026-01-17
**Total unwrap() calls found:** 114

## Summary

| Category | Count | Status |
|----------|-------|--------|
| Test code (`#[cfg(test)]`) | ~44 | Safe - Keep as-is |
| Safe context (after validation) | ~3 | Safe - Document |
| Production paths (needs fix) | ~67 | **Fix required** |

---

## Category 1: Test Code (Safe - Keep as-is)

### Files: `src/ingest/magellan.rs`
Lines 68-72 (3 calls):
- `TempDir::new().unwrap()`
- `MagellanIngestor::new(&db_path).unwrap()`
- `query_by_labels(&["rust"]).unwrap()`

**Reason:** Inside `#[cfg(test)]` module. Test-only unwrap is acceptable.

### Files: `src/graph/magellan_integration.rs`
Lines 183, 187, 190, 194, 200, 203, 206 (7 calls):
- `TempDir::new().unwrap()`
- `MagellanIntegration::open(&db_path).unwrap()`
- `query_by_labels(&["rust"]).unwrap()`
- `get_all_labels().unwrap()`
- `count_by_label("rust").unwrap()`

**Reason:** Inside `#[cfg(test)]` module. Test-only unwrap is acceptable.

### Files: `src/resolve/references/rust.rs`
Lines 1117-1378 (34 calls):
- All in `#[cfg(test)]` module starting at line 1110
- Include `NamedTempFile::new().unwrap()`, `write!(...).unwrap()`, etc.

**Reason:** Inside `#[cfg(test)]` module. Test-only unwrap is acceptable.

---

## Category 2: Safe Context (After Validation - Document only)

### File: `src/resolve/mod.rs`
**Line 111:**
```rust
let (node_id, file_path) = all_matches.into_iter().next().unwrap();
```
**Reason:** Safe - Only called after:
1. Early return if `all_matches.is_empty()` at line ~84
2. Early return if `all_matches.len() > 1` at lines 97-108
3. Therefore, exactly one match is guaranteed at this point

**Action:** Add explanatory comment: `// Safe: validated exactly one match above`

### File: `src/ingest/imports/python.rs`
**Line 94:**
```rust
let imported_name = path.first().unwrap().clone();
```
**Reason:** Safe - Check `if !path.is_empty()` on line 92 ensures path has at least one element

**Action:** Add explanatory comment: `// Safe: path.is_empty() check above`

**Line 199:**
```rust
imported_names.push(name_path.last().unwrap().clone());
```
**Reason:** Safe - Check `if name_path.is_empty()` with `continue` on lines 189-191

**Action:** Add explanatory comment: `// Safe: name_path.is_empty() check above`

### File: `src/patch/backup.rs`
**Line 423:**
```rust
fs::create_dir_all(manifest_path.parent().unwrap())
```
**Reason:** Need to verify if parent() could return None. Path from a file should have a parent, but edge cases exist (root paths).

**Action:** Convert to proper error handling

---

## Category 3: Production Paths (Fix Required)

### Core Files (Plan 01-02)

#### File: `src/main.rs`
- **Line 196:** `file_path.parent().unwrap().join(".splice_graph.db")`
- **Line 459:** `file_path.parent().unwrap().join(".splice_graph.db")`

**Fix:**
```rust
let graph_db_path = file_path
    .parent()
    .ok_or_else(|| SpliceError::Other(format!(
        "File path has no parent: {}",
        file_path.display()
    )))?
    .join(".splice_graph.db");
```

#### File: `src/plan/mod.rs`
- **Line 204:** `file_path.parent().unwrap().join(".splice_graph.db")`
- **Line 253:** `let plan: Plan = serde_json::from_str(json).unwrap()`
- **Line 262:** `serde_json::from_str::<Plan>(r#"{"steps": []}"#).unwrap()`

**Fix:**
```rust
// Line 204 - same as main.rs
// Line 253 - add error context
let plan: Plan = serde_json::from_str(json).map_err(|e| {
    SpliceError::InvalidPlanSchema {
        message: format!("JSON parse error: {}", e),
    }
})?;
```

#### File: `src/patch/pattern.rs`
- **Line 165:** `ropey::Rope::from_reader(content.as_bytes()).unwrap()`

**Fix:**
```rust
let rope = ropey::Rope::from_reader(content.as_bytes()).map_err(|e| {
    SpliceError::Other(format!("Failed to create rope: {}", e))
})?;
```

#### File: `src/validate/gates.rs`
- **Line 93:** `path.to_str().unwrap()` - Python validation
- **Line 135:** `path.to_str().unwrap()` - C++ validation
- **Line 177:** `path.to_str().unwrap()` - Objective-C validation
- **Line 219:** `path.to_str().unwrap()` - Java validation
- **Line 262:** `path.to_str().unwrap()` - JavaScript validation
- **Line 315:** `path.to_str().unwrap()` - TypeScript validation

**Fix:**
```rust
.args(["-m", "py_compile", path.to_str().ok_or_else(|| SpliceError::Other(
    format!("Invalid UTF-8 path: {}", path.display())
))?
])
```

**Note:** All 6 are Command arg conversions. Need UTF-8 validation error.

### Language Ingest Files (Plan 01-03)

#### File: `src/ingest/dispatch.rs`
Lines 144, 157, 170, 183, 196, 209, 230, 242 (8 calls):
```rust
let symbols = result.unwrap();
```
**Pattern:** Each is from a language-specific ingest function

**Fix:**
```rust
let symbols = result.map_err(|e| SpliceError::Parse {
    file: file.clone(),
    message: format!("Ingest failed: {}", e),
})?;
```

#### File: `src/ingest/typescript.rs`
Lines 384, 396, 408, 420, 432, 444, 457, 469, 481, 493, 507 (11 calls):
All `let symbols = result.unwrap();`

**Fix:** Same as dispatch.rs

#### File: `src/ingest/java.rs`
Lines 322, 334, 349, 362, 376, 388, 402, 413, 425 (9 calls):
All `let symbols = result.unwrap();`

**Fix:** Same as dispatch.rs

#### File: `src/ingest/javascript.rs`
Lines 356, 368, 381, 393, 405, 417 (6 calls):
All `let symbols = result.unwrap();`

**Fix:** Same as dispatch.rs

#### File: `src/ingest/python.rs`
Line 272 (1 call):
```rust
let symbols = result.unwrap();
```

**Fix:** Same as dispatch.rs

#### File: `src/ingest/cpp.rs`
Line 508 (1 call):
```rust
let symbols = result.unwrap();
```

**Fix:** Same as dispatch.rs

### Import Resolution Files (Plan 01-03)

#### File: `src/ingest/imports/typescript.rs`
Lines 303, 316, 328, 341, 353, 366, 378, 388, 400 (9 calls):
All `let imports = result.unwrap();`

**Fix:**
```rust
let imports = result.map_err(|e| SpliceError::Other(format!(
    "Import resolution failed: {}", e
)))?;
```

#### File: `src/ingest/imports/java.rs`
Lines 143, 155, 167, 179, 191, 203 (6 calls):
All `let imports = result.unwrap();`

**Fix:** Same as typescript imports

#### File: `src/ingest/imports/cpp.rs`
Lines 139, 151 (2 calls):
All `let imports = result.unwrap();`

**Fix:** Same as typescript imports

#### File: `src/ingest/imports/javascript.rs`
Lines 255, 268, 280, 293, 305, 318, 328 (7 calls):
All `let imports = result.unwrap();`

**Fix:** Same as typescript imports

#### File: `src/ingest/imports/python.rs`
Line 308 (1 call):
```rust
let imports = result.unwrap();
```

**Fix:** Same as typescript imports

---

## Breakdown by Module

| Module | Test | Safe | Production | Total |
|--------|------|------|------------|-------|
| src/ingest/magellan.rs | 3 | 0 | 0 | 3 |
| src/graph/magellan_integration.rs | 7 | 0 | 0 | 7 |
| src/resolve/references/rust.rs | 34 | 0 | 0 | 34 |
| src/resolve/mod.rs | 0 | 1 | 0 | 1 |
| src/ingest/imports/python.rs | 0 | 2 | 1 | 3 |
| src/main.rs | 0 | 0 | 2 | 2 |
| src/plan/mod.rs | 0 | 0 | 3 | 3 |
| src/patch/pattern.rs | 0 | 0 | 1 | 1 |
| src/patch/backup.rs | 0 | 1 | 0 | 1 |
| src/validate/gates.rs | 0 | 0 | 6 | 6 |
| src/ingest/dispatch.rs | 0 | 0 | 8 | 8 |
| src/ingest/typescript.rs | 0 | 0 | 11 | 11 |
| src/ingest/java.rs | 0 | 0 | 9 | 9 |
| src/ingest/javascript.rs | 0 | 0 | 6 | 6 |
| src/ingest/python.rs | 0 | 0 | 1 | 1 |
| src/ingest/cpp.rs | 0 | 0 | 1 | 1 |
| src/ingest/imports/typescript.rs | 0 | 0 | 9 | 9 |
| src/ingest/imports/java.rs | 0 | 0 | 6 | 6 |
| src/ingest/imports/cpp.rs | 0 | 0 | 2 | 2 |
| src/ingest/imports/javascript.rs | 0 | 0 | 7 | 7 |

---

## New Helper Methods Available

The following methods are now available on `SpliceError` (added in Plan 01-01):

1. **`.with_context(msg)`** - Add context to error messages
2. **`.with_path(path)`** - Attach a file path to errors
3. **`SpliceError::io_with_path(path, source)`** - Create I/O error with path
4. **`SpliceError::parse_with_file(file, message)`** - Create parse error with file
5. **`SpliceError::other(msg)`** - Create generic error

These helpers make it easier to convert unwrap() calls to proper error handling.
