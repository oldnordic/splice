# Pitfalls Research

**Domain:** Adding dual backend support (SQLite + native-v2) with feature flags to existing Rust CLI tool
**Researched:** 2026-02-09
**Confidence:** MEDIUM

## Executive Summary

Adding native-v2 as an optional backend to splice introduces several categories of pitfalls:

1. **Feature flag configuration errors** - Hardcoded features, dependency resolver confusion
2. **Testing gaps with conditional compilation** - Code paths not tested, CI matrix missing variants
3. **Type signature changes via cfg guards** - Unintended API changes across feature combinations
4. **Documentation confusion for users** - Unclear which backend to use, migration path unclear
5. **Version skew between local path dependencies** - magellan/sqlitegraph/splice version misalignment
6. **Migration path issues** - Breaking existing SQLite users during rollout

The most critical pitfall is **feature unification issues** from Cargo's resolver, which can silently enable the wrong backend or cause duplicate builds with different features.

## Critical Pitfalls

### Pitfall 1: Hardcoded Feature Flags in Dependencies

**What goes wrong:**
The current Cargo.toml hardcodes `features = ["native-v2"]` on the magellan dependency:

```toml
magellan = { version = "2.1", path = "../magellan", features = ["native-v2"] }
```

This forces native-v2 to be enabled regardless of splice's feature flags. When splice is built without `--features native-v2`, magellan still uses native-v2, creating a mismatch between declared and actual backend.

**Why it happens:**
Developers copy dependency declarations from magellan's documentation without considering that features should be **controlled by the consuming crate**, not hardcoded. Path dependencies in workspaces are especially prone to this because local changes don't go through normal version resolution.

**Consequences:**
- splice users get native-v2 backend even when they expect SQLite (default)
- Binary size increases unexpectedly (native-v2 links additional dependencies)
- Tests pass locally but fail in production with different feature combinations
- Documentation claims one backend, code uses another

**Prevention:**

1. **Remove hardcoded features from dependency declaration:**
```toml
# WRONG - hardcoded feature
magellan = { version = "2.1", path = "../magellan", features = ["native-v2"] }

# CORRECT - feature controlled by splice
magellan = { version = "2.1", path = "../magellan", default-features = false, features = [] }

# Then define splice features that pass through to magellan:
[features]
default = []  # SQLite backend is default
native-v2 = ["magellan/native-v2"]  # Enable native-v2 via splice feature
```

2. **Use `default-features = false`** to explicitly control which features are enabled

3. **Document the feature contract** in a comment above the dependency

**Warning signs:**
- `cargo tree -e features` shows `magellan native-v2` even when building without `--features native-v2`
- Binary contains native-v2 symbols when built with default features
- Tests only pass with certain feature combinations

**Detection commands:**
```bash
# Check what features are actually enabled
cargo tree -e features -i magellan

# Verify feature unification
cargo tree -f "{p} {f}"

# Check for duplicate builds
cargo tree --duplicates
```

**Phase to address:** Phase 1 (Feature Flag Setup) - Must be fixed before any other work

---

### Pitfall 2: API Type Changes Across cfg(feature) Guards

**What goes wrong:**
When code is conditionally compiled with `#[cfg(feature = "native-v2")]`, public API types can change depending on which features are enabled. This creates **different APIs for the same version** of splice.

```rust
// WRONG - API changes based on features
#[cfg(feature = "native-v2")]
pub fn open_graph(path: &Path) -> Result<NativeGraph> { ... }

#[cfg(not(feature = "native-v2"))]
pub fn open_graph(path: &Path) -> Result<SQLiteGraph> { ... }
```

**Why it happens:**
Developers use cfg guards to switch implementations without maintaining a stable public API surface. The cfg guard is applied at the function level rather than internally within a stable wrapper.

**Consequences:**
- Downstream code compiled against splice with one feature set breaks when splice is recompiled with different features
- SemVer violation - same version has incompatible API
- `cargo doc` shows different documentation depending on features
- Users cannot switch backends without code changes

**Prevention:**

1. **Maintain a stable public API regardless of backend:**
```rust
// CORRECT - API is stable, implementation switches internally
pub struct CodeGraph {
    backend: Box<dyn GraphBackend>,  // Trait object, not concrete type
}

pub fn open_graph(path: &Path) -> Result<CodeGraph> {
    // Backend selection is internal
    let backend = if cfg!(feature = "native-v2") {
        Box::new(NativeBackend::open(path)?) as Box<dyn GraphBackend>
    } else {
        Box::new(SQLiteBackend::open(path)?) as Box<dyn GraphBackend>
    };
    Ok(CodeGraph { backend })
}
```

2. **Use trait objects or enums** to hide backend differences from public API

3. **Feature flags should control implementation, not interface**

**Warning signs:**
- Public function signatures change between `cargo doc --no-default-features` and `cargo doc --features native-v2`
- `#[cfg(feature = "...")]` appears on public `pub fn` or `pub struct` declarations
- Tests fail when recompiled with different feature combinations

**Phase to address:** Phase 2 (Backend Abstraction Layer) - API must be designed before implementation

---

### Pitfall 3: Mutually Exclusive Features Without Compile-Time Checks

**What goes wrong:**
If both backends could theoretically be enabled simultaneously, undefined behavior or silent bugs can occur. Without compile-time checks, both might be linked, causing bloat at best and correctness issues at worst.

```toml
# Dangerous: both features could be enabled
[features]
default = []
native-v2 = []
sqlite = []  # Should be mutually exclusive with native-v2
```

**Why it happens:**
Developers assume users will read documentation and only enable one backend. They don't add defensive compile_error! checks because "nobody would do that."

**Consequences:**
- Binary includes both backend code (2x size increase)
- Runtime confusion about which backend is actually being used
- Silent performance degradation
- Confusing error messages when both try to initialize

**Prevention:**

```rust
// lib.rs - compile-time check
#[cfg(all(feature = "native-v2", feature = "sqlite"))]
compile_error!(
    "feature \"native-v2\" and feature \"sqlite\" are mutually exclusive. \
     Enable only one backend feature. \
     Use default features (SQLite) or --features native-v2"
);

// Also provide helpful guidance
#[cfg(not(any(feature = "native-v2", feature = "sqlite")))]
compile_error!(
    "No backend feature enabled. Enable the default (SQLite) or use --features native-v2"
);
```

**Alternative approach (better for some cases):**
Don't make features mutually exclusive. Instead, use **runtime selection** so both can coexist but only one is used per database open:

```rust
pub fn open_with_backend(path: &Path, backend: BackendChoice) -> Result<CodeGraph>
```

**Warning signs:**
- `cargo build --all-features` succeeds but produces unexpected behavior
- Binary size doubles with `--all-features`
- No `compile_error!` guards in lib.rs

**Phase to address:** Phase 1 (Feature Flag Setup) - Add guards before implementing backends

---

### Pitfall 4: Incomplete Test Coverage Across Feature Variants

**What goes wrong:**
Tests only run with default features. Code paths specific to native-v2 or SQLite-only scenarios never execute in CI, leading to regressions that only appear in user environments.

**Why it happens:**
Default `cargo test` only builds with default features. Developers forget to test other feature combinations, or CI isn't configured to test multiple variants.

**Consequences:**
- native-v2 code paths break silently
- SQLite-specific code breaks when native-v2 is added
- Release works for developers (who use one backend) but fails for users (who use another)
- Hyrum's Law: users depend on behavior that wasn't tested across variants

**Prevention:**

1. **Add a CI matrix that tests both backends:**
```yaml
# .github/workflows/test.yml
test:
  strategy:
    matrix:
      backend: [default, native-v2]
  steps:
    - name: Test (default)
      if: matrix.backend == 'default'
      run: cargo test
    - name: Test (native-v2)
      if: matrix.backend == 'native-v2'
      run: cargo test --features native-v2
```

2. **Add conditional tests:**
```rust
#[cfg(feature = "native-v2")]
#[test]
fn test_native_v2_specific_behavior() {
    // Tests only run when native-v2 is enabled
}

#[cfg(not(feature = "native-v2"))]
#[test]
fn test_sqlite_fallback() {
    // Tests only run with SQLite backend
}
```

3. **Use trybuild for compilation testing:**
```rust
// tests/ui/compile_fail_backend_conflict.rs
// This should fail to compile if both features are enabled
#[cfg(all(feature = "native-v2", feature = "sqlite"))]
fn should_fail() {}
```

**Warning signs:**
- CI only runs `cargo test` without feature variations
- `#[cfg(test)]` blocks don't have conditional tests for each backend
- No tests in `tests/` directory (integration tests with different features)

**Phase to address:** Phase 4 (Testing Infrastructure) - CI must test both backends before release

---

### Pitfall 5: Path Dependency Version Skew

**What goes wrong:**
splice uses a path dependency on magellan (`path = "../magellan"`). When magellan's API or feature set changes, splice continues to compile but may have subtle incompatibilities. This is worse when sqlitegraph is also a path dependency with version skew.

**Why it happens:**
Path dependencies bypass Cargo's version resolution. Local development changes don't trigger version bumps, so the lockfile doesn't catch API changes.

**Consequences:**
- splice compiles but fails at runtime due to API mismatch
- Features that existed in local magellan disappear in published version
- Schema version mismatches between splice and magellan's expectations
- "Works on my machine" syndrome that only appears in CI or users' systems

**Prevention:**

1. **Use version constraints for path dependencies:**
```toml
[dependencies]
magellan = { version = "2.1", path = "../magellan" }
#                                    ^^^^^^ allows local dev
#              ^^^^^^^ enforces API compatibility
```

2. **Run cargo-update against crates.io periodically:**
```bash
# Temporarily switch to published version to test compatibility
cargo update --package magellan

# Or test in CI against published version
cargo test --no-dev-dependencies
```

3. **Add version assert in build.rs or tests:**
```rust
#[test]
fn test_magellan_version_compatibility() {
    let magellan_version = magellan::VERSION;
    assert!(
        magellan_version.starts_with("2.1"),
        "magellan version {} is not compatible with expected 2.1.x",
        magellan_version
    );
}
```

4. **Document required versions together:**
```toml
# In a comment or separate workspace Cargo.toml:
# Version lock: splice 2.5.0 requires:
#   - magellan 2.1.x
#   - sqlitegraph 1.3.x
#   - native-v2 schema version 3
```

**Warning signs:**
- `cargo tree` shows different versions of magellan in different contexts
- Changes to magellan break splice but tests still pass
- CI and local development behave differently

**Detection commands:**
```bash
# Check for version inconsistencies
cargo tree -p magellan --depth 0

# Compare local vs published
cargo tree | grep magellan
```

**Phase to address:** Phase 1 (Dependency Setup) - Version constraints must be established

---

### Pitfall 6: Silent Feature Unification from Multiple Dependencies

**What goes wrong:**
When multiple dependencies in the dependency graph enable different features on magellan, Cargo's **feature unification** combines them. This can silently enable native-v2 even when splice doesn't request it.

**Why it happens:**
From [The Cargo Book](https://doc.rust-lang.org/cargo/reference/features.html#feature-unification):
> "When a dependency is used by multiple packages, Cargo will use the union of all features enabled on that dependency when building it."

Example scenario:
- splice enables magellan with default features (SQLite)
- a new dependency `foo` enables magellan's native-v2 feature
- Cargo builds magellan with BOTH features enabled

**Consequences:**
- Binary includes native-v2 code that splice didn't request
- Potential behavior changes if features aren't purely additive
- Increased binary size
- Confusion when `cargo tree` shows features that weren't explicitly enabled

**Prevention:**

1. **Check feature flow with cargo tree:**
```bash
# See which package enabled which feature
cargo tree -e features -i magellan

# Inverted view: what enables magellan features
cargo tree -e features --invert magellan
```

2. **Use resolver = "2"** to avoid unwanted unification:
```toml
[package]
name = "splice"
version = "2.5.0"
resolver = "2"  # Use version 2 resolver
```

Resolver 2 avoids unification in:
- Platform-specific dependencies for unused targets
- Build-dependencies vs regular dependencies
- Dev-dependencies (unless building tests)

3. **Document feature expectations:**
```toml
# splice expects magellan with exactly one backend:
# - default features (SQLite) OR
# - native-v2 feature (native backend)
#
# If both are enabled via dependency unification, behavior is undefined.
```

**Warning signs:**
- `cargo tree -e features` shows unexpected features enabled
- Binary size differs between `cargo build` and `cargo build --release`
- Behavior changes when adding new dependencies

**Phase to address:** Phase 1 (Feature Flag Setup) - Must use resolver "2" and verify feature flow

---

### Pitfall 7: Documentation Confusion - Which Backend Should I Use?

**What goes wrong:**
Users don't know which backend to choose, when to use each, or how to switch. Documentation either doesn't mention backends at all, or presents the information in a way that requires deep reading to understand.

**Why it happens:**
Backend choice is an implementation detail that developers forget to expose to users. Documentation is written from the implementer's perspective, not the user's perspective.

**Consequences:**
- Users stick with default (SQLite) even when native-v2 would be better
- Users enable native-v2 unnecessarily, missing migration steps
- Bug reports from using wrong backend for use case
- Support burden explaining backend choice repeatedly

**Prevention:**

1. **Add a "Quick Start" section that explains backend choice clearly:**
```markdown
## Which Backend Should I Use?

### SQLite Backend (default)
**Use when:**
- You're just getting started with splice
- You need maximum compatibility
- You want to inspect the database with standard SQLite tools
- Database size < 100K symbols

**Enable:** `cargo install splice` (default)

### Native-v2 Backend
**Use when:**
- You have > 100K symbols in your codebase
- You need faster graph operations
- You don't need to inspect the database with external tools
- You're doing bulk refactoring operations

**Enable:** `cargo install splice --features native-v2`

### Switching Backends
```bash
# From SQLite to Native-v2
splice migrate --to native-v2 --db .codemcp/codegraph.db

# From Native-v2 to SQLite
splice migrate --to sqlite --db .codemcp/codegraph.db
```
```

2. **Add runtime backend detection in CLI:**
```bash
$ splice status --db .codemcp/codegraph.db
Backend: native-v2
Format version: 3
Node count: 42,156
```

3. **Document performance characteristics:**
| Operation | SQLite | Native-v2 | Speedup |
|-----------|--------|-----------|---------|
| Open database | 50ms | 10ms | 5x |
| Get neighbors | 5ms | 0.5ms | 10x |
| Reachability query | 200ms | 50ms | 4x |

**Warning signs:**
- README doesn't mention backend choice
- No performance comparison table
- Users asking "which backend should I use?" in issues
- Migration commands not documented

**Phase to address:** Phase 5 (Documentation) - Must be complete before v2.5.0 release

---

## Moderate Pitfalls

### Pitfall 8: Database Auto-Detection Without Explicit Backend Selection

**What goes wrong:**
splice's `CodeGraph::open()` currently auto-detects database format by checking file headers:

```rust
let cfg = if Self::is_sqlite_db(path)? {
    sqlitegraph::GraphConfig::sqlite()
} else {
    sqlitegraph::GraphConfig::native()
};
```

This is convenient but can mask issues when the wrong backend is compiled.

**Why it happens:**
Developers prioritize convenience over explicitness. Auto-detection seems user-friendly.

**Consequences:**
- User opens a native-v2 database with a splice build that doesn't have native-v2 enabled → cryptic error
- User accidentally creates wrong format database due to typo in path
- Debugging difficulty when backend/format mismatch occurs

**Prevention:**

Keep auto-detection but provide **clear error messages:**
```rust
pub fn open(path: &Path) -> Result<Self> {
    let is_native = Self::is_native_db(path)?;
    let is_sqlite = Self::is_sqlite_db(path)?;

    match (is_native, is_sqlite) {
        (true, false) => {
            if !cfg!(feature = "native-v2") {
                return Err(SpliceError::Other(
                    "Database is native-v2 format, but splice was built without native-v2 feature. \
                     Rebuild with: --features native-v2".to_string()
                ));
            }
            // ... open native
        }
        (false, true) => {
            // ... open SQLite
        }
        (false, false) => {
            // Create new - use configured default
        }
        (true, true) => {
            return Err(SpliceError::Other("File appears to be both formats? Corrupted?".to_string()));
        }
    }
}
```

**Phase to address:** Phase 2 (Backend Abstraction Layer)

---

### Pitfall 9: Breaking Change When Moving Code Behind Features

**What goes wrong:**
During native-v2 implementation, developers might move existing SQLite-specific code behind `#[cfg(feature = "sqlite")]`. This is a **breaking SemVer change** because users who were using that code can no longer access it without enabling the feature.

**Why it happens:**
Developers think "I'm just organizing code" without realizing that adding cfg guards to existing public APIs is SemVer-breaking.

**Consequences:**
- User code breaks when upgrading splice
- SemVer violation (should be major bump, not minor)
- Downstream crates need changes to upgrade

**Prevention:**

**Do NOT move existing code behind feature flags.** Only add NEW code behind features.

```rust
// WRONG - moving existing API behind feature (breaking)
#[cfg(feature = "native-v2")]
pub fn existing_function() { ... }

// CORRECT - new API behind feature (additive)
#[cfg(feature = "native-v2")]
pub fn new_native_v2_function() { ... }

// EXISTING API - remains available (backwards compatible)
pub fn existing_function() { ... }
```

From [The Cargo Book](https://doc.rust-lang.org/cargo/reference/features.html#semver-compatibility):
> "Moving existing public code behind a feature" should usually NOT be done in a minor release.

**Phase to address:** Phase 3 (Implementation) - Review all cfg changes for SemVar compliance

---

### Pitfall 10: Missing cfg_attr Tests

**What goes wrong:**
Tests are only compiled with default features. Test-specific code that should run with different backends doesn't get covered.

**Why it happens:**
Developers use `#[cfg(test)]` without considering backend-specific test scenarios.

**Prevention:**

Use combined cfg attributes:
```rust
// Test only runs when both test AND native-v2 are enabled
#[cfg(all(test, feature = "native-v2"))]
#[test]
fn test_native_v2_behavior() {
    // ...
}

// Test only runs when test is enabled BUT native-v2 is NOT
#[cfg(all(test, not(feature = "native-v2")))]
#[test]
fn test_sqlite_only_behavior() {
    // ...
}
```

**Phase to address:** Phase 4 (Testing Infrastructure)

---

## Minor Pitfalls

### Pitfall 11: Dead Code Detection with cfg Removes

**What goes wrong:**
When old SQLite-specific code is replaced or made redundant by native-v2, developers might `#[cfg]` it out rather than removing it. This dead code still compiles in some configurations, potentially hiding bugs.

**Prevention:**
Remove dead code instead of hiding it behind cfg. Use git history to recover if needed.

**Phase to address:** Phase 3 (Implementation)

---

### Pitfall 12: Inconsistent Error Messages Across Backends

**What goes wrong:**
SQLite and native-v2 backends return different error messages for the same failure condition, confusing users who switch backends.

**Prevention:**
Use a unified error type that maps backend-specific errors to splice-specific codes.

**Phase to address:** Phase 2 (Backend Abstraction Layer)

---

## Technical Debt Patterns

| Shortcut | Immediate Benefit | Long-term Cost | When Acceptable |
|----------|-------------------|----------------|-----------------|
| Hardcoded feature flag in Cargo.toml | Works immediately | Users can't control backend | Never |
| Only testing default features | Faster CI development | Regressions in other backends | Only during initial spike |
| Backend-specific public APIs | Simpler implementation | Users can't switch backends | Never - public API must be stable |
| No compile_error! guards | Less code to write | Silent bugs when features conflict | Never - add guards upfront |
| Duplicating tests with cfg | Tests run for each backend | Test maintenance burden | Acceptable if truly needed |

---

## Integration Gotchas

| Integration | Common Mistake | Correct Approach |
|-------------|----------------|------------------|
| magellan dependency | Hardcoding `features = ["native-v2"]` in splice's Cargo.toml | Use `features = []` and control via splice's own features |
| sqlitegraph dependency | Not using `default-features = false` | Explicitly control which backend is enabled |
| CI testing | Only testing `cargo test` | Test matrix with `--features native-v2` variant |
| Documentation | Not explaining backend tradeoffs | Add "Which Backend?" section with performance table |
| Error messages | Backend-specific errors leaking to users | Map all errors to splice error codes |

---

## Performance Traps

| Trap | Symptoms | Prevention | When It Breaks |
|------|----------|------------|----------------|
| Feature unification overhead | Binary bloat, slower compiles | Use resolver = "2", check cargo tree | At 5+ dependencies |
| Duplicate backend builds | Long build times | Avoid enabling both backends, use cargo-hakari | In large workspaces |
| Runtime backend checks | Every operation has conditional | Use trait objects, check once at open | At any scale (micro-optimization) |

---

## UX Pitfalls

| Pitfall | User Impact | Better Approach |
|---------|-------------|-----------------|
| No backend choice documentation | Users use wrong backend for needs | Add decision table with use cases |
| Silent auto-detection failures | Cryptic errors when format mismatches | Explicit error messages with rebuild instructions |
| No migration tooling | Users stuck on old backend | Provide `splice migrate --to {backend}` command |
| No runtime backend indication | Users don't know which backend is active | Add `splice status --db` showing backend |

---

## "Looks Done But Isn't" Checklist

- [ ] **Backend selection:** Often missing compile_error! guards — verify with `cargo build --all-features`
- [ ] **Feature propagation:** Often missing cargo tree verification — verify features flow correctly
- [ ] **Test coverage:** Often missing backend-specific tests — verify both backends have passing tests
- [ ] **Documentation:** Often missing backend choice guidance — verify README explains when to use each
- [ ] **Error messages:** Often missing backend-specific error context — verify errors mention required feature
- [ ] **Migration path:** Often missing upgrade/downgrade tooling — verify users can convert databases

---

## Recovery Strategies

| Pitfall | Recovery Cost | Recovery Steps |
|---------|---------------|----------------|
| Hardcoded feature flag | LOW | Remove from Cargo.toml, add splice feature, verify with cargo tree |
| API changes via cfg | HIGH | Redesign public API to use trait objects, this is a breaking change |
| Missing test coverage | MEDIUM | Add CI matrix for both backends, write conditional tests |
| Documentation confusion | LOW | Add backend choice section, performance comparison table |
| Version skew | MEDIUM | Pin dependency versions, add version assert in build.rs or tests |
| Silent feature unification | LOW | Add resolver = "2", verify with cargo tree -e features |

---

## Pitfall-to-Phase Mapping

| Pitfall | Prevention Phase | Verification |
|---------|------------------|--------------|
| Hardcoded feature flags | Phase 1: Feature Flag Setup | `cargo tree -e features -i magellan` |
| API type changes via cfg | Phase 2: Backend Abstraction | Compare `cargo doc` output across features |
| Missing compile_error! guards | Phase 1: Feature Flag Setup | `cargo build --all-features` should fail |
| Incomplete test coverage | Phase 4: Testing Infrastructure | CI matrix tests both backends |
| Path dependency version skew | Phase 1: Dependency Setup | `cargo tree -p magellan` shows consistent version |
| Silent feature unification | Phase 1: Feature Flag Setup | `cargo tree -e features --invert magellan` |
| Documentation confusion | Phase 5: Documentation | README answers "which backend should I use?" |
| Auto-detection without errors | Phase 2: Backend Abstraction | Test opening wrong format with wrong build |
| Moving code behind features | Phase 3: Implementation | Review SemVer compliance before merge |
| Missing cfg_attr tests | Phase 4: Testing Infrastructure | `cargo test --features native-v2` runs backend-specific tests |

---

## Sources

| Source | Confidence | Notes |
|--------|------------|-------|
| [The Cargo Book - Features](https://doc.rust-lang.org/cargo/reference/features.html) | HIGH | Official Cargo documentation on features, feature unification, SemVer compatibility |
| [RFC 3013 - Conditional Compilation Checking](https://rust-lang.github.io/rfcs/3013-conditional-compilation-checking.html) | MEDIUM | Detecting dead code from removed feature flags |
| [Best practices for features and dependencies](https://users.rust-lang.org/t/best-practices-features-and-dependencies-in-a-crate/125626) | MEDIUM | Community discussion on feature flag best practices (Feb 2025) |
| [Conditional Compilation in Rust with Feature Flags](https://midnightprogrammer.net/post/conditional-compilation-in-rust-with-feature-flags/) | LOW-MEDIUM | Blog post on cfg(feature) patterns (Nov 2025) |
| [Cargo Issue #5210 - Feature unification](https://github.com/rust-lang/cargo/issues/5210) | MEDIUM | Ongoing issue about feature unification in workspaces |
| [Guidance on optional functionality: crates vs features](https://users.rust-lang.org/t/guidance-on-optional-functionality-crates-vs-features/85694) | MEDIUM | When to split crates vs use features |
| [A standard schema for crate feature flags documentation](https://users.rust-lang.org/t/cargo-a-standard-schema-for-crate-feature-flags-documentation/132869) | LOW | 2025 discussion on documentation standards |
| [Multiple versions of local dependency in workspace](https://stackoverflow.com/questions/64200237/multiple-versions-of-local-dependency-in-cargo-workspace-with-different-features) | MEDIUM | Path dependency feature conflicts |
| [Dependency ranges, workspaces, and breaking changes](https://internals.rust-lang.org/t/dependency-ranges-workspaces-and-breaking-changes/22394) | MEDIUM | February 2025 discussion on version skew |
| [Conditional compilation - The Rust Reference](https://doc.rust-lang.org/reference/conditional-compilation.html) | HIGH | Official reference on cfg attributes |
| /home/feanor/Projects/splice/Cargo.toml | HIGH | Verified current hardcoded native-v2 feature |
| /home/feanor/Projects/splice/src/graph/mod.rs | HIGH | Verified current auto-detection implementation |
| /home/feanor/Projects/splice/src/platform.rs | HIGH | Verified existing platform feature pattern |
| /home/feanor/Projects/splice/.planning/research/native-v2-binary-format.md | HIGH | Native-v2 implementation details |
| /home/feanor/Projects/splice/.planning/research/STACK_MAGELLAN_V2.md | HIGH | Magellan integration context |

---

## Open Questions Requiring Phase-Specific Research

1. **Exact feature flag naming:** Should splice expose `native-v2` or rename to something more user-friendly? Research Phase 1

2. **Migration tooling:** Does sqlitegraph provide built-in migration between formats, or does splice need to implement it? Research Phase 2

3. **Performance benchmarks:** What are the actual performance differences between backends at various scales? Research Phase 3

4. **Downstream consumers:** Are there crates depending on splice that will be affected by API changes? Research Phase 2

5. **Windows compatibility:** Does native-v2 work on Windows (splice has Windows support via features)? Research Phase 4

---

*Pitfalls research for: Dual backend support with feature flags in Rust CLI tool*
*Researched: 2026-02-09*
*Confidence: MEDIUM - Research based on official documentation and community discussion, but native-v2 is new and unproven in production*
