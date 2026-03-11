# Architecture Ownership

Purpose: define which layer owns which decisions, and which modules are allowed to mutate adaptive state.

Companion docs:
- `docs/client/architecture/ROOT_OWNERSHIP_MAP.md` (root ownership + allowed surfaces)
- `docs/client/architecture/MIGRATION_LEDGER.md` (temporary compatibility lanes and removal triggers)

## Ownership Matrix

| Layer/Path | Ownership | May Mutate | Mutation Channel |
|---|---|---|---|
| `client/runtime/systems/` | Control plane + safety infrastructure | No direct adaptive writes except controller stores | N/A |
| `client/cognition/adaptive/` | Runtime-adaptive policy/state data | Data only (no arbitrary writes) | `client/runtime/systems/adaptive/*/*_store.js` |
| `client/cognition/habits/` | Dynamic routine execution and generation | Habit runtime/state only | Habit scripts + adaptive habit store |
| `client/cognition/skills/` | Task-specific integrations | Skill-local files and allowed state receipts | Skill wrappers + guards |
| `client/runtime/config/` | Static policy/config contracts | Only approved governance flows | Guarded writes |
| `client/runtime/local/state/` + `core/local/state/` | Runtime outputs and ledgers | Runtime emitters only | Append/log + bounded writers |

## Adaptive Store Controllers (Single Writer Channels)

Only these modules are canonical adaptive mutators:

- `client/runtime/systems/adaptive/core/layer_store.ts`
- `client/runtime/systems/adaptive/sensory/eyes/catalog_store.ts`
- `client/runtime/systems/adaptive/sensory/eyes/focus_trigger_store.ts`
- `client/runtime/systems/adaptive/habits/habit_store.ts`
- `client/runtime/systems/adaptive/reflex/reflex_store.ts`
- `client/runtime/systems/adaptive/strategy/strategy_store.ts`

Guard policy:

- `client/runtime/systems/sensory/adaptive_layer_guard.ts`
- `client/runtime/config/adaptive_layer_guard_policy.json`

CI enforces this in strict mode.

## Schema Contracts (Single Source)

Versioned contracts live in:

- `client/runtime/config/contracts/autonomy_receipt.schema.json`
- `client/runtime/config/contracts/proposal_admission.schema.json`
- `client/runtime/config/contracts/adaptive_store.schema.json`

Validation entrypoint:

- `node client/runtime/systems/security/schema_contract_check.ts run`

CI executes this check before general test execution.

## Design Rules

1. `client/runtime/systems/` should remain broadly reusable; no business specialization in system modules.
2. `client/cognition/adaptive/` is resettable; deleting adaptive data should return the system to a blank-slate learning posture.
3. All adaptive writes must go through store getters/setters/mutators.
4. Contract changes require a schema version bump and CI passing against updated contracts.
5. Runtime churn in local state mirrors should not be treated as source-of-truth for code review.

## Incident Boundaries

If behavior drifts:

1. Check `schema_contract_check` output.
2. Check adaptive guard strict output.
3. Check recent store mutation logs in `client/runtime/local/state/security/adaptive_mutations.jsonl`.
4. Roll back by commit boundary, not ad-hoc file edits.
