# Sprint Removal Ledger

Tracks dead/speculative cleanup removals so deletions stay explicit, reviewable, and reversible.

## 2026-04-23 (RTG-034 Cleanup Wave)

| Removed item | Location | Category | Reason | Follow-up |
| --- | --- | --- | --- | --- |
| Legacy alias fallback (`row.alias`) from Shell transition pair matcher | `tests/tooling/scripts/ci/shell_transition_completion_tracker.ts` | speculative compatibility fallback | Alias matching now requires explicit `compatibility` field, eliminating ambiguous fallback behavior. | Keep strict until all shell/client compatibility bridges retire. |
| Empty `doc_aliases` stub from Shell transition alias map | `client/runtime/config/shell_transition_alias_map.json` | dead config stub | Field carried no semantic value in canonical Shell compatibility bridge flow. | Do not reintroduce empty alias buckets without a guard requirement. |
