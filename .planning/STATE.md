# State: Splice

**Current Milestone:** v2.5.0 - Native-V2 Backend Support

## Project Reference

See: `.planning/PROJECT.md` (updated 2026-02-09)

**Core value:** Safe code transformation with byte-level precision and rollback support
**Current focus:** Phase 38 - Documentation

---

## Current Position

**Phase:** 38 of 38 (Documentation)
**Plan:** 2 of 4 in current phase
**Status:** In Progress
**Last activity:** 2026-02-10 — Plan 38-01 completed: Backend decision guide in README (work completed with 38-02)

---

## Performance Metrics

**Velocity:**
- Total plans completed: 19 (v2.5.0)
- Average duration: 5.0 min
- Total execution time: ~1.6 hours

**By Phase:**

| Phase | Plans | Total | Avg/Plan |
|-------|-------|-------|----------|
| 33 | 4 | 4 | 6 min |
| 34 | 4 | 4 | 5 min |
| 35 | 5 | 20 min | 4 min |
| 36 | 4 | 8 min | 2 min |
| 37 | 3 | 27 min | 9 min |
| 38 | 2 | 6 min | 3 min |

---
| Phase 37 P03 | 799s | 2 tasks | 4 files |
| Phase 38-documentation P01 | 247s | 1 tasks | 1 files |
| Phase 38-documentation P02 | 123s | 1 tasks | 1 files |

## Key Decisions

- Sequential backend testing to avoid database conflicts (2026-02-10)
- Shell script test orchestration with feature flag selection (2026-02-10)
- Backend-agnostic integration testing pattern (2026-02-10)
- Feature-gated testing with #[cfg(feature = "...")] attributes for backend-specific code (2026-02-10)
- New databases default to native-v2 format even with sqlite feature enabled (2026-02-10)
- Added 'migration' feature combining both sqlite and native-v2 backends for cross-backend testing (2026-02-10)
- Migration tests use #[ignore] to document known snapshot format incompatibility bug (2026-02-10)
- Section placement in README: "Which Backend Should I Use?" placed after Installation, before Quick Start for logical flow (2026-02-10)
- Comparison table with 11 aspects covering feature flags, format, size, performance, and tooling (2026-02-10)
- Feature availability matrix distinguishing between both backends vs native-v2 exclusives (2026-02-10)
- Recommendation guidance for SQLite vs native-v2 based on codebase size and performance needs (2026-02-10)
- Integrated platform features into Installation section to reduce documentation duplication (2026-02-10)

---

## Issues Encountered

**sccache compilation error:** Initial cargo test runs failed with "No such file or directory" error for sccache. Worked around by setting `RUSTC_WRAPPER=""` and `SCCACHE_DISABLE=1` environment variables (2026-02-10).

**Migration snapshot format incompatibility:** Discovered that SQLite and native-v2 backends use incompatible snapshot formats (snapshot.json vs export.manifest). Migration tests document this bug but cannot run until implementation is fixed (2026-02-10).

---

*State updated: 2026-02-10*
*Last session: 2026-02-10T06:38:22Z - Completed Phase 38-01*
