# Sprint Removal Ledger

Tracks dead/speculative cleanup removals so deletions stay explicit, reviewable, and reversible.

## 2026-04-23 (RTG-034 Cleanup Wave)

| Removed item | Location | Category | Reason | Follow-up |
| --- | --- | --- | --- | --- |
| Legacy alias fallback (`row.alias`) from Shell transition pair matcher | `tests/tooling/scripts/ci/shell_transition_completion_tracker.ts` | speculative compatibility fallback | Alias matching now requires explicit `compatibility` field, eliminating ambiguous fallback behavior. | Keep strict until all shell/client compatibility bridges retire. |
| Empty `doc_aliases` stub from Shell transition alias map | `client/runtime/config/shell_transition_alias_map.json` | dead config stub | Field carried no semantic value in canonical Shell compatibility bridge flow. | Do not reintroduce empty alias buckets without a guard requirement. |

## 2026-04-24 (USC Cleanup Closure Wave)

| Removed item | Location | Category | Reason | Follow-up |
| --- | --- | --- | --- | --- |
| Shell compatibility alias config for old `client_naming_policy` name | `client/runtime/config/client_naming_policy.json` | retired compatibility config | Shell naming policy is canonical and no active users require a `client_*` compatibility alias. | `ops:shell-transition:tracker` now rejects retired Shell aliases with active bridges. |
| Shell compatibility alias config for old `client_transition_alias_map` name | `client/runtime/config/client_transition_alias_map.json` | retired compatibility config | Shell transition alias map is canonical and compatibility bridge state is retired. | Keep `client/**` as an implementation path only until an explicit path migration exists. |
| Old docs filename for Shell naming policy | `docs/workspace/client_naming_policy.md` | retired compatibility doc path | The document already defined Shell naming; the filename carried the retired public term. | Canonical replacement lives at `docs/workspace/shell_naming_policy.md`. |
