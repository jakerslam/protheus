# Coreization Next Modules (Post-checkpoint)

## Objective
Continue Hard Coreization Wave 1 by prioritizing high-ROI authority lanes and keeping `client/` as thin wrappers.

## Ordered Queue (ROI + Dependencies)

1. `routing/model_router` -> `core/layer0/ops::model_router` + `core/layer2/autonomy` policy helpers
- Why first: central dependency for spawn, autonomy, and workflow lane selection.
- Dependency note: unlocks clean cutover for `spawn_broker` and `llm_gateway` guard logic.
- Current state: `client/runtime/systems/routing/model_router.ts` exists and is core-first with TS fallback; deeper function-by-function parity still pending.

2. `autonomy/pain_signal` -> `core/layer2/autonomy::pain_signal` + `core/layer1/policy` escalation gates
- Why second: shared failure/escalation contract used across multiple autonomy and ops lanes.
- Dependency note: needed before stricter no-fallback autonomy cutover.

3. `spawn/spawn_broker` -> `core/layer2/ops::spawn_broker`
- Why third: controls module-cell budgets and is consumed by multiple scheduler/runtime lanes.
- Dependency note: depends on stable core model router signals.

4. `assimilation/assimilation_controller` -> `core/layer0/ops::assimilation_controller` + `core/layer1/storage`
- Why fourth: authoritative ingest and state mutation plane; currently a large TS holdout.
- Dependency note: depends on memory/runtime receipts and policy contracts.

5. `sensory/focus_controller` -> `core/layer0/ops::attention_queue` + `core/layer1/task`
- Why fifth: critical for low-burn, high-signal selection and context hydration discipline.

6. `continuum/continuum_core` + `weaver/weaver_core` -> `core/layer2/autonomy`
- Why sixth: advanced adaptation/cognition orchestration should not live in client authority paths.

7. `workflow/orchestron/*` (`adaptive_controller`, `candidate_generator`) -> `core/layer2/autonomy`
- Why seventh: workflow intelligence currently TS-heavy; move orchestration truth to Rust.

8. `identity/identity_anchor` -> `core/layer1/security` + `core/layer1/storage`
- Why eighth: source-of-truth identity anchoring belongs in protected core state.

## Layer Assignment Rule (enforced)
- Layer 0 (`core/layer0/ops`): command authority, command dispatch, receipts, process control.
- Layer 1 (`core/layer1/*`): policy/state/integrity surfaces.
- Layer 2 (`core/layer2/*`): planning/orchestration/scheduling/autonomy logic.
- Client: wrappers, SDK ergonomics, UI-facing utilities only.

## Immediate Validation Gate (per module)
- Kernel lane command exists and returns deterministic receipt.
- Client command runs core-first with fallback only until parity closes.
- Legacy TS logic is deleted or strongly reduced after parity.
- Existing regression lane remains green.
