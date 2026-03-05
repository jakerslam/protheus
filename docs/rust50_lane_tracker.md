# Rust50 Lane Tracker

Last updated: 2026-03-05 (America/Denver, late run)
Branch target: `main`

## Purpose
Persistent lane-by-lane migration log so progress is preserved outside chat context.

## Gate Contract (per lane)
1. `cargo test` (lane-appropriate crate/manifest)
2. `cargo clippy -- -D warnings` (lane-appropriate crate/manifest)
3. `npm run -s formal:invariants:run` with `NODE_PATH=.../node_modules`
4. Commit + push to `origin/main`

## Completed In This Run
- [x] `1dd15784` retire generic-json legacy fallback
- [x] `42c3b4d1` retire generic-yaml legacy fallback
- [x] `aa1b060a` retire openfang legacy fallback
- [x] `3246b0b5` retire workflow-graph legacy fallback
- [x] `697a4928` retire autotest-controller legacy TypeScript lane
- [x] `8434df99` retire autotest-doctor legacy TypeScript lane
- [x] `3cb7304b` retire spine legacy TypeScript lane
- [x] `f000496e` retire idle-dream-cycle legacy TypeScript lane
- [x] `e6b73a53` retire memory-transition legacy TypeScript lane
- [x] `8f4cccb0` retire strategy-mode-governor legacy TypeScript lane
- [x] `30d756e6` retire contract-check legacy TypeScript lane
- [x] `02093c59` retire model-router legacy TypeScript lane
- [x] `34bfe2b7` retire foundation-contract-gate legacy TypeScript lane
- [x] `fd350b16` retire state-kernel legacy TypeScript lane
- [x] `6f06441f` retire personas-cli legacy TypeScript lane
- [x] `ef061725` retire workflow-executor legacy TypeScript lane
- [x] `a064cf98` retire autonomy-controller legacy TypeScript lane
- [x] `c41fddd1` retire inversion-controller legacy TypeScript lane
- [x] `3ea8e1ea` retire proposal-enricher legacy TypeScript lane
- [x] `b1444a14` retire health-status legacy TypeScript lane

## Completed In This Continuation (Wrapper Runtime + Source Cutover)
- [x] `880e4bc4` harden health-status JavaScript rust wrapper
- [x] `c8de50fe` harden inversion-controller JavaScript rust wrapper
- [x] `3a5b7fb8` harden proposal-enricher JavaScript rust wrapper
- [x] `3d406659` harden strategy-mode-governor JavaScript rust wrapper
- [x] `7226554b` migrate autotest-controller wrapper source to JavaScript
- [x] `a180eeeb` migrate autotest-doctor wrapper source to JavaScript
- [x] `56dfec10` migrate foundation-contract-gate wrapper source to JavaScript
- [x] `888c8390` migrate state-kernel wrapper source to JavaScript
- [x] `5143267f` migrate personas-cli wrapper source to JavaScript
- [x] `60dbff35` migrate model-router wrapper source to JavaScript
- [x] `fcf63d08` migrate contract-check wrapper source to JavaScript
- [x] `f0d9b0b8` migrate spine wrapper source to JavaScript
- [x] `ca667c2f` migrate workflow-executor wrapper source to JavaScript
- [x] `760f2bc7` migrate idle-dream-cycle wrapper source to JavaScript
- [x] `d622ecfc` migrate rust-memory-transition-lane wrapper source to JavaScript
- [x] `b296c600` migrate fluxlattice-program wrapper source to JavaScript
- [x] `7a06cc8e` migrate perception-polish-program wrapper source to JavaScript
- [x] `c3ea2e1a` migrate protheusctl wrapper source to JavaScript
- [x] `88397540` migrate runtime-efficiency-floor wrapper source to JavaScript
- [x] `8272d457` migrate scale-readiness-program wrapper source to JavaScript

## Remaining Legacy TS Lanes (Current Queue)
- [x] `systems/autonomy/strategy_mode_governor_legacy.ts`
- [x] `systems/spine/contract_check_legacy.ts`
- [x] `systems/routing/model_router_legacy.ts`
- [x] `systems/ops/foundation_contract_gate_legacy.ts`
- [x] `systems/ops/state_kernel_legacy.ts`
- [x] `systems/personas/cli_legacy.ts`
- [x] `systems/workflow/workflow_executor_legacy.ts`
- [x] `systems/autonomy/autonomy_controller_legacy.ts`
- [x] `systems/autonomy/inversion_controller_legacy.ts`
- [x] `systems/autonomy/proposal_enricher_legacy.ts`
- [x] `systems/autonomy/health_status_legacy.ts`

## Notes
- Some Rust lane entrypoints still route through legacy script adapters in `crates/ops/src/*`.
- Retirement stubs are fail-closed and emit deterministic JSON error payloads.
- Full functional replacement for those lanes requires replacing `legacy_bridge::run_passthrough` / `run_legacy_script_compat` in Rust entrypoints.
- Wrapper source `.ts` files for the above lanes have been removed and replaced by committed `.js` runtime wrappers.

## Completed In This Continuation (Top-100 Wrapper Source Cutover)
- Timestamp: 2026-03-05 14:05 
- Result: Ranked top-100 queue now has `0` remaining `.ts + .js` wrapper pairs.
- Execution mode: lane-by-lane (`test` + `clippy` + `invariants` + commit + push per lane).

| Rank | Path | Commit |
|---:|---|---|
| 52 | `systems/autonomy/self_improvement_cadence_orchestrator.ts` | `facc5866` |
| 53 | `systems/autonomy/improvement_orchestrator.ts` | `b2ebf98f` |
| 54 | `systems/security/guard.ts` | `e069e308` |
| 55 | `systems/autonomy/receipt_summary.ts` | `6ced2d93` |
| 56 | `systems/ops/llm_economy_organ.ts` | `cb5627d5` |
| 57 | `systems/security/remote_emergency_halt.ts` | `6edba656` |
| 58 | `systems/autonomy/pain_adaptive_router.ts` | `7f9f0439` |
| 59 | `systems/autonomy/hold_remediation_engine.ts` | `4df28077` |
| 60 | `systems/autonomy/collective_shadow.ts` | `bf1f6f97` |
| 61 | `systems/autonomy/tier1_governance.ts` | `f277041f` |
| 62 | `systems/workflow/learning_conduit.ts` | `3e0008a3` |
| 63 | `systems/security/anti_sabotage_shield.ts` | `10e44a75` |
| 64 | `systems/security/alias_verification_vault.ts` | `bf4cecdd` |
| 65 | `systems/ops/offsite_backup.ts` | `78c11303` |
| 66 | `systems/routing/route_task.ts` | `f03dda42` |
| 67 | `systems/workflow/data_rights_engine.ts` | `11778462` |
| 68 | `systems/autonomy/lever_experiment_gate.ts` | `be557b3a` |
| 69 | `systems/security/soul_token_guard.ts` | `c28ae72c` |
| 70 | `systems/autonomy/autonomy_rollout_controller.ts` | `427e8862` |
| 71 | `systems/autonomy/self_documentation_closeout.ts` | `422df322` |
| 72 | `systems/security/delegated_authority_branching.ts` | `beeb339e` |
| 73 | `systems/ops/settlement_program.ts` | `e4c145a1` |
| 74 | `systems/routing/router_budget_calibration.ts` | `2f2d27ba` |
| 75 | `systems/sensory/cross_signal_engine.ts` | `f39cc1a2` |
| 76 | `systems/memory/creative_links.ts` | `ca7b4493` |
| 77 | `systems/ops/narrow_agent_parity_harness.ts` | `4132dbf7` |
| 78 | `systems/memory/cryonics_tier.ts` | `aff238bd` |
| 79 | `systems/strategy/strategy_controller.ts` | `efc8ec1c` |
| 80 | `systems/security/secure_heartbeat_endpoint.ts` | `10a81f83` |
| 81 | `systems/autonomy/multi_agent_debate_orchestrator.ts` | `c1df98f1` |
| 82 | `systems/autonomy/background_persistent_agent_runtime.ts` | `d886e7c3` |
| 83 | `systems/tools/assimilate.ts` | `4d4e02de` |
| 84 | `systems/routing/provider_readiness.ts` | `4d9e0b8c` |
| 85 | `systems/autonomy/strategy_mode.ts` | `7be54028` |
| 86 | `systems/tools/cli_suggestion_engine.ts` | `0394006d` |
| 87 | `systems/autonomy/self_code_evolution_sandbox.ts` | `b512f280` |
| 88 | `systems/security/organ_state_encryption_plane.ts` | `32c8d680` |
| 89 | `systems/autonomy/proactive_t1_initiative_engine.ts` | `2825bb3a` |
| 90 | `systems/nursery/specialist_training.ts` | `e942242e` |
| 91 | `systems/actuation/disposable_infrastructure_organ.ts` | `f10f3469` |
| 92 | `systems/memory/memory_federation_plane.ts` | `e3f19491` |
| 93 | `systems/ops/productized_suite_program.ts` | `aedf10c5` |
| 94 | `systems/autonomy/trit_shadow_report.ts` | `a910e519` |
| 95 | `systems/security/dream_warden_guard.ts` | `73424799` |
| 96 | `systems/autonomy/ethical_reasoning_organ.ts` | `5d47a564` |
| 97 | `systems/ops/rust_hybrid_migration_program.ts` | `bad36563` |
| 98 | `systems/security/skin_protection_layer.ts` | `df83b00a` |
| 99 | `systems/fractal/regime_organ.ts` | `9a539c02` |
| 100 | `systems/tools/research.ts` | `1274cc60` |
