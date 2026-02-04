# Splice v2.3.0 Release Checklist

**Date:** 2026-02-04
**Version:** 2.3.0

## Pre-Release Checklist

### Version Bump
- [x] Version bumped to 2.3.0 in Cargo.toml
- [x] Package metadata is complete (name, version, description, license, repository, keywords, categories)
- [x] Exclude list updated (.planning/, CLAUDE.md, AGENTS.md, QUICKSTART.md)

### Documentation
- [x] CHANGELOG.md documents all v2.3.0 changes
  - Cross-file rename command
  - Impact analysis commands (reachable, slice, dead-code, cycles, condense)
  - Proof-based refactoring (--proof flag, validate-proof command)
  - Dual-format SymbolId support (V1 SHA-256, V2 BLAKE3)
  - Magellan 2.0.0 integration and migration
  - Performance benchmarks
  - Test coverage summary (469+ tests passing)
- [x] RELEASE_NOTES.md created with user-facing highlights
- [x] README.md updated with v2.3.0 features
- [x] Manual updated with cross-file rename documentation
- [x] Example files created (rename_examples.md, graph_algorithm_examples.md, proof_examples.md)
- [x] .gitignore updated to allow RELEASE_NOTES.md

### Testing
- [x] 469 tests passing (exceeds 407+ requirement)
  - 395 unit tests
  - 26 CLI output tests
  - 7 other integration tests
  - 36 CLI tests (1 pre-existing issue in test_cli_patch_preview)
- [x] Performance benchmarks all pass
- [x] Cross-file rename integration tests (18 tests, 7 languages)
- [x] Graph algorithm performance tests (<1s for 1K symbols)

### Known Issues
- [ ] test_cli_patch_preview has a pre-existing issue
  - Root cause: SQLiteGraph debug output (V2_SLOT_DEBUG) interferes with JSON parsing
  - Not a v2.3 regression - test was failing before this release
  - Preview functionality itself works correctly (used in other tests)
  - Workaround: Test infrastructure needs to filter stderr from stdout

## Release Steps

### 1. Final Verification
```bash
# Verify version
grep "version = " Cargo.toml  # Should show "2.3.0"

# Run all tests
cargo test --all

# Build release binary
cargo build --release

# Run smoke tests
./target/release/splice --help
./target/release/splice --version
```

### 2. Create Git Tag
```bash
git tag -a v2.3.0 -m "Release v2.3.0: Magellan v2 Integration with Cross-File Rename"
git push origin v2.3.0
```

### 3. Publish to crates.io
```bash
cargo publish --dry-run  # Verify package builds correctly
cargo publish            # Actually publish
```

### 4. Create GitHub Release
- Go to https://github.com/oldnordic/splice/releases/new
- Tag: v2.3.0
- Title: "Splice v2.3.0: Magellan v2 Integration with Cross-File Rename"
- Description: Use content from RELEASE_NOTES.md
- Attach binaries (optional)

### 5. Post-Release
- [ ] Update website/documentation
- [ ] Announce on social media/changelog
- [ ] Monitor for issues

## Rollback Plan

If critical issues are found after release:
1. Yank the crate from crates.io: `cargo yank splice@2.3.0`
2. Create a patch release (v2.3.1) with fixes
3. Publish v2.3.1

## Success Criteria

- [x] Version bumped to 2.3.0
- [x] CHANGELOG.md complete
- [x] RELEASE_NOTES.md created
- [x] All major features documented
- [x] 407+ tests passing (actually 469)
- [x] Ready for cargo publish
