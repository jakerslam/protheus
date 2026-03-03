# Data Scope Boundaries

This repository enforces a strict scope split:

- User-specific data:
  - `memory/` for user-owned records, preferences, and histories.
  - `adaptive/` for learned heuristics/tuning tied to user behavior.
- Permanent/shared implementation:
  - `systems/` for runtime logic.
  - `config/` for policy contracts.
  - `docs/` for operator contracts and runbooks.
- Internal-only local scaffolding:
  - `.internal/` for non-runtime private working material that must not ship.

## Hard Rules

1. `memory/` and `adaptive/` must not contain executable `.ts` or `.js` runtime modules.
2. Canonical implementation files must live under `systems/` (with config in `config/`).
3. `.internal/` content is never a source-of-truth runtime path and should remain local-only.
4. New feature lanes must declare:
   - user paths (`memory/`, `adaptive/`)
   - permanent runtime paths (`systems/`, `config/`)
   - check coverage in `systems/ops/data_scope_boundary_check.ts`

## Integration Touchpoints (V3-RACE-136)

- Soul vector: `systems/symbiosis/soul_vector_substrate.ts`
- Economy tithe lane: `systems/economy/tithe_engine.ts`
- Spawn broker: `systems/spawn/spawn_broker.ts`
- Guard path: `systems/security/guard.ts`
- Fractal engine + complexity warden:
  - `systems/fractal/engine.ts`
  - `systems/fractal/warden/complexity_warden_meta_organ.ts`
- Jigsaw receipts: `systems/security/jigsaw/attackcinema_replay_theater.ts`

## Enforcement

- `node systems/ops/data_scope_boundary_check.js check --strict=1`
- Latest receipt: `state/ops/data_scope_boundary_check/latest.json`
