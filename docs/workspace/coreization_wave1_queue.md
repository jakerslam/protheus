# Coreization Wave 1 Queue (Acceleration Mode)

Purpose: maximize Rust share velocity by converting the highest-LOC TS authority surfaces to Rust-lane ownership first.

## Completed In This Wave

- Security wave-1 authority lanes moved to `core/layer1/security` with thin wrappers.
- Spine scheduler/local fallback authority removed; wrappers route to Rust lanes.
- `pain_signal`, `protheusctl` cut to Rust authority lanes.
- Non-yield/autophagy family moved behind `core/layer0/ops::autonomy-controller`.
- `organ_atrophy_controller`, `narrow_agent_parity_harness` moved to Rust authority.
- `offsite_backup`, `settlement_program` moved to Rust authority.
- `llm_economy_organ`, `backlog_queue_executor` moved to Rust authority.

## Current Rule (Strict)

- Layer 0: authoritative deterministic control/gates and domain command lanes.
- Layer 1: security/memory policy + receipts and state contracts.
- Layer 2: orchestration/scheduling/coordination engines.
- Client: thin wrappers only; no truth/authority logic.

## Acceleration Queue (Top ROI Remaining)

1. `client/runtime/systems/continuum/continuum_core.ts` -> `core/layer2/execution` + L0 command lane.
2. `client/runtime/systems/sensory/focus_controller.ts` -> `core/layer2/sensory` + L0 command lane.
3. `client/runtime/systems/weaver/weaver_core.ts` -> `core/layer2/execution`.
4. `client/runtime/systems/identity/identity_anchor.ts` -> `core/layer1/security|identity`.
5. `client/runtime/systems/dual_brain/coordinator.ts` -> `core/layer2/autonomy`.
6. `client/runtime/systems/budget/system_budget.ts` -> `core/layer1/resource`.
7. `client/runtime/systems/routing/llm_gateway.ts` -> `core/layer2/routing`.
8. `client/runtime/systems/adaptive/strategy/strategy_store.ts` -> `core/layer1/storage`.
9. `client/runtime/systems/echo/heroic_echo_controller.ts` -> `core/layer2/autonomy`.
10. `client/runtime/systems/helix/helix_controller.ts` -> `core/layer2/execution`.
11. `client/runtime/systems/routing/provider_readiness.ts` -> `core/layer1/observability`.
12. `client/runtime/systems/redteam/ant_colony_controller.ts` -> `core/layer2/autonomy`.
13. `client/runtime/systems/primitives/explanation_primitive.ts` -> `core/layer2/execution`.
14. `client/runtime/systems/attribution/value_attribution_primitive.ts` -> `core/layer1/observability`.
15. `client/runtime/systems/assimilation/capability_profile_compiler.ts` -> `core/layer2/execution`.
16. `client/runtime/systems/migration/core_migration_bridge.ts` -> `core/layer0/ops`.
17. `client/runtime/systems/primitives/effect_type_system.ts` -> `core/layer1/policy`.
18. `client/runtime/systems/primitives/emergent_primitive_synthesis.ts` -> `core/layer2/execution`.
19. `client/runtime/systems/primitives/long_horizon_planning_primitive.ts` -> `core/layer2/execution`.
20. `client/runtime/systems/primitives/runtime_scheduler.ts` -> `core/layer2/execution`.

## Process Optimizers (Now Active)

1. Largest-LOC-first batching (not subsystem-by-subsystem).
2. Batch conversion templates for wrappers/shims to cut manual churn.
3. Commit/push every stable batch.
4. No new feature backlog implementation before Rust gate is met.
