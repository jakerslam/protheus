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
