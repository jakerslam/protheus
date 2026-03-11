# Data Scope Boundaries

This repository enforces a strict scope split:

- User-specific data:
  - `client/memory/` for user-owned records, preferences, and histories.
  - `client/cognition/adaptive/` for learned heuristics/tuning tied to user behavior.
- Permanent/shared implementation:
  - `client/runtime/systems/` for runtime logic.
  - `client/runtime/config/` for policy contracts.
  - `docs/client/` for operator contracts and runbooks.
- Internal-only local scaffolding:
  - `.internal/` for non-runtime private working material that must not ship.

## Hard Rules

1. `client/memory/` and `client/cognition/adaptive/` must not contain executable `.ts` or `.js` runtime modules.
2. Canonical implementation files must live under `client/runtime/systems/` (with config in `client/runtime/config/`).
3. `.internal/` content is never a source-of-truth runtime path and should remain local-only.
4. New feature lanes must declare:
   - user paths (`client/memory/`, `client/cognition/adaptive/`)
   - permanent runtime paths (`client/runtime/systems/`, `client/runtime/config/`)
   - check coverage in `client/runtime/systems/ops/data_scope_boundary_check.ts`

## Integration Touchpoints (V3-RACE-136)

- Soul vector: `client/runtime/systems/symbiosis/soul_vector_substrate.ts`
- Economy tithe lane: `client/runtime/systems/economy/tithe_engine.ts`
- Spawn broker: `client/runtime/systems/spawn/spawn_broker.ts`
- Guard path: `client/runtime/systems/security/guard.ts`
- Fractal engine + complexity warden:
  - `client/runtime/systems/fractal/engine.ts`
  - `client/runtime/systems/fractal/warden/complexity_warden_meta_organ.ts`
- Jigsaw receipts: `client/runtime/systems/security/jigsaw/attackcinema_replay_theater.ts`

## Enforcement

- `node client/runtime/systems/ops/data_scope_boundary_check.ts check --strict=1`
- Latest receipt: `state/ops/data_scope_boundary_check/latest.json`
