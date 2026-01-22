# Phase 12: Rich Span Advanced - Context

**Gathered:** 2026-01-22
**Status:** Ready for planning

<domain>
## Phase Boundary

Spans include relationships (callers, callees, imports, exports), tool hints (behavioral flags), and suggested actions (delete, replace, expand) for advanced LLM workflows. Relationships are lazy (only via --relationships flag) and span full codebase via CodeGraph. All advanced fields are optional and don't impact performance when not requested.

</domain>

<decisions>
## Implementation Decisions

### Relationship Granularity
- **Direct relationships only** — no transitive traversal. A calls B, B calls C (two separate queries). LLM can chain if needed.
- **Module-level imports** — use/import statements only (e.g., `use crate::foo` → `foo.rs`). No symbol-level type resolution.
- **Ambiguous references** — return all candidates with locations. Let LLM disambiguate based on context.
- **Lazy evaluation** — relationships only computed when `--relationships` flag is provided.

### Tool Hint Derivation
- **requires_full_context** — symbol-dependent. True for closures/impl blocks that capture outer scope.
- **may_break_tests** — static heuristic. True when deleting public functions or changing trait signatures.
- **requires_compilation** — true for type-changing operations (struct/enum modifications, trait changes, public signatures).
- **apply_atomically** — always true. All splice operations are atomic by design.

### Suggested Action Logic
- **Action type** — based on user intent. delete for --delete, replace for --patch, expand for --expand.
- **Action params** — contextual: action_type + confidence + reason (why this action).
- **Expand levels** — 2 levels: name → full body → containing block.
- **Confidence** — categorical: High/Medium/Low based on uniqueness, ambiguity, type info.

### Performance Boundary
- **Caller threshold** — skip relationships if symbol has > 100 direct callers.
- **Circular dependencies** — return all relationships with `cycle_detected: true` marker.
- **Caching** — session-based. Cache within single splice invocation, clear after.
- **Query failure** — return empty `relationships: {}` with `error_code` explaining failure. Don't fail entire operation.

### Claude's Discretion
- Exact confidence thresholds (what makes something High vs Medium vs Low)
- Cycle detection algorithm details
- Cache key structure for session-based caching

</decisions>

<specifics>
## Specific Ideas

- ROADMAP.md explicitly requires lazy evaluation: "only executed with --relationships flag"
- STATE.md identified concern about "Relationship graph schema — need to define edge types" — module-level addresses this by keeping it simple
- Error code integration from Phase 11 should be reused for relationship query failures

</specifics>

<deferred>
## Deferred Ideas

None — discussion stayed within phase scope.

</deferred>

---

*Phase: 12-rich-span-advanced*
*Context gathered: 2026-01-22*
