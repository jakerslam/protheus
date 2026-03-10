# Coreization Wave 1 Queue (Next Modules + Layer Targets)

Purpose: define the concrete next migration set from `client/runtime/systems/*` into Rust core layers, in dependency order.

## Order (ROI + dependency aware)

| Order | Module | Client Source | Core Target Layer | Notes |
|---|---|---|---|---|
| 1 | Security planes (Wave 1 hard set) | `client/runtime/systems/security/{directive_hierarchy_controller,capability_switchboard,black_box_ledger,goal_preservation_kernel,dream_warden_guard}.*` | `core/layer1/security` (+ `core/layer0/ops/security_plane.rs`) | Already core-authoritative; wrappers remain thin. |
| 2 | Spine authority | `client/runtime/systems/spine/*` | `core/layer2/spine` (+ `core/layer0/ops/spine.rs`) | Mostly migrated; remaining JS local fallbacks should be folded into Rust lane behavior. |
| 3 | Memory runtime + recall authority | `client/runtime/systems/memory/{memory_recall,memory_matrix,memory_auto_recall,dream_sequencer,legacy/*}` | `core/layer1/memory_runtime` (+ `core/layer0/ops/memory_ambient.rs`) | Highest TS mass in Wave 1; requires parity contract for `memory_recall_*` output schema and cache-clear behavior. |
| 4 | Autonomy authority | `client/runtime/systems/autonomy/{pain_signal,autonomy_simulation_harness,multi_agent_debate_orchestrator,ethical_reasoning_organ,...}` | `core/layer2/autonomy` (+ `core/layer0/ops/autonomy_controller.rs`) | Start with `pain_signal` contract path, then simulation/debate/ethics lanes. |
| 5 | Workflow/orchestron authority | `client/runtime/systems/workflow/orchestron/{candidate_generator,adaptive_controller,nursery_tester,...}` | `core/layer2/execution` + `core/layer0/ops/{workflow_controller,workflow_executor}.rs` | Move planner/executor logic into core; keep only thin TS wrappers in client. |
| 6 | Daemon control authority | `client/runtime/systems/ops/protheusd.ts` + daemon control support | `core/layer0/ops/daemon_control.rs` + `core/layer2/conduit` | Keep client attach/CLI wrapper only. |

## Layer placement rule (strict)

- Layer 0: deterministic enforcement and command gates (`ops/*`), no probabilistic policy.
- Layer 1: memory/security policy + receipt/state authority.
- Layer 2: orchestration/scheduling/coordination logic.
- Client: wrappers, SDK ergonomics, DX only (no authority logic).

## Current immediate blocker observed

- `protheus-ops` lane calls intermittently time out in this environment under conduit startup/probe pressure.
- This does not block code migration itself, but it blocks reliable parity validation until lane stability is restored.

## Next execution batch

1. Finish memory authority cutover: remove legacy TS fallback from `memory_recall/matrix/auto_recall/dream_sequencer` after Rust parity fields are present.
2. Move autonomy `pain_signal` contract to core-only path and keep TS shim only.
3. Port `workflow/orchestron` candidate/adaptive controllers into Rust lane contracts and replace TS authority with wrappers.
