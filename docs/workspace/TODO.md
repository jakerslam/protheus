# TODO (Priority + ROI + Dependency Ordered)

Updated: 2026-03-11 (policy-enforcement + SRS normalization + live-metrics refresh)

## Ordering policy
- Priority first (`P0` > `P1` > `P2` > `P3`)
- Then ROI (higher unblock value first)
- Then dependency chain (prerequisites before dependents)

## Current objective
Standardize repo policy boundaries first, then shrink client surface to true DX/UI/app bridge only, then execute SRS backlog by dependency.

## Live baseline (current worktree)
- `client/*.js = 0`
- `client/*.sh = 0`
- `client/*.py = 0`
- `client/*.ps1 = 0`
- `client total ts files = 232`
- `runtime_system_surface = 116`
- `cognition_surface = 0`
- `runtime_sdk_surface = 40`
- `wrapper_count = 109`
- `allowed_non_wrapper_count = 8`
- `repo rust share = 74.946%`
- `verify.sh = PASS`
- `policy target gaps = 0`

## Backlog snapshot
- Source: `docs/workspace/SRS.md` + `client/runtime/config/backlog_registry.json`
- Artifact: `artifacts/backlog_actionable_report_current.json`
- Counts: `actionable=773`, `queued=690`, `in_progress=83`, `blocked=42`, `done=2148`

## Executed first (policy enforcement)

1. `P0-POL-001` Restore TypeScript bootstrap/entrypoint runtime contract. `STATUS: COMPLETE`
- Why first:
- Policy checks were green but execution path was broken due removed `.js` bootstrap entrypoints.
- Completion evidence:
- `client/runtime/lib/ts_bootstrap.ts`
- `client/runtime/lib/ts_entrypoint.ts`
- `client/lib/ts_bootstrap.ts`
- `client/lib/ts_entrypoint.ts`

2. `P0-POL-002` Fix critical policy wrappers to TypeScript-only contract. `STATUS: COMPLETE`
- Why first:
- `ops:dependency-boundary:check` and origin-integrity were failing on stale `.js` script pointers.
- Completion evidence:
- `client/runtime/systems/ops/dependency_boundary_guard.ts`
- `client/runtime/systems/ops/formal_spec_guard.ts`
- `core/layer0/ops/src/origin_integrity.rs`

3. `P0-POL-003` Rewrite stale `client/...*.js` references to `.ts` where `.ts` exists. `STATUS: COMPLETE`
- Why first:
- Required to make no-JS client policy executable, not just declarative.
- Completion evidence:
- `scripts`/`docs`/`config`/`workflows` references updated where valid `.ts` target exists.
- Policy and runtime gates now run end-to-end.

4. `P0-POL-004` Full policy + regression gate pass. `STATUS: COMPLETE`
- Why first:
- No further migration should proceed on a failing baseline.
- Completion evidence:
- `npm run -s ops:dependency-boundary:check`
- `npm run -s ops:formal-spec:check`
- `npm run -s ops:client-layer:boundary`
- `npm run -s ops:repo-surface:audit`
- `npm run -s ops:public-platform:contract`
- `npm run -s ops:client-target:audit`
- `./verify.sh`

5. `P0-POL-005` Recover and harden dependency/formal guards after wrapper collapse. `STATUS: COMPLETE`
- Why first:
- Wrapper pruning surfaced missing authoritative guard entrypoints and stale local import paths.
- Completion evidence:
- `client/runtime/systems/ops/dependency_boundary_guard.ts` (rebuilt deterministic guard shim)
- `client/runtime/systems/ops/formal_spec_guard.ts` (rebuilt formal surface verifier)
- `scripts/memory/skill_runner.ts` (fixed stale `../../lib/*.js` imports to policy-compliant TS paths)
- `client/runtime/config/dependency_boundary_manifest.json` (allowlisted `rust_lane_bridge.ts`)

## Ordered execution queue (next)

6. `P1-CLIENT-001` Collapse duplicate wrapper families behind generic entrypoints. `STATUS: COMPLETE`
- ROI: 10/10
- Dependency: `P0-POL-004`
- Target outcome:
- Reduce `wrapper_count` from `546` toward `< 200` target.
- Current tranche evidence:
- 182 unreferenced wrapper files pruned from `client/runtime/systems/**` (round 1).
- 425 additional unreferenced runtime system files pruned (round 2).
- `move_to_apps` tranche: 60 files moved from `client/cognition/**` to `apps/_shared/**`.
- `move_to_adapters` tranche: 8 integration files moved from `client/cognition/skills/**` to `adapters/**`.
- Current metrics:
- `total_ts_files: 232`
- `runtime_system_surface: 116`
- `wrapper_count: 109`
- `cognition_surface: 0` (target exceeded)

7. `P1-CLIENT-002` Reduce `client/runtime/systems` to public bridge surfaces only. `STATUS: COMPLETE`
- ROI: 10/10
- Dependency: `P1-CLIENT-001`
- Target outcome:
- Reduced `runtime_system_surface` to `116` (target met).

8. `P1-CLIENT-003` Promote residual authority logic from client TS to Rust core. `STATUS: IN_PROGRESS`
- ROI: 10/10
- Dependency: `P1-CLIENT-002`
- Target outcome:
- Burn down residual `promote_to_core` recommendations (`108` currently flagged by disposition audit).

9. `P1-CLIENT-004` Move workflow/product-specific surfaces from client to `/apps`. `STATUS: COMPLETE`
- ROI: 9/10
- Dependency: `P1-CLIENT-002`
- Target outcome:
- `move_to_apps` bucket is `0`.

10. `P1-CLIENT-005` Move integration-specific bridges to `/adapters`. `STATUS: COMPLETE`
- ROI: 9/10
- Dependency: `P1-CLIENT-002`
- Target outcome:
- `move_to_adapters` bucket is `0`.

11. `P1-CLIENT-006` Consolidate runtime SDK alias surface. `STATUS: COMPLETE`
- ROI: 8/10
- Dependency: `P1-CLIENT-001`
- Target outcome:
- `runtime_sdk_surface` reduced to `40` (target met).

12. `P1-CLIENT-007` Reduce cognition surface to direct DX/operator surfaces only. `STATUS: COMPLETE`
- ROI: 8/10
- Dependency: `P1-CLIENT-003`
- Target outcome:
- `cognition_surface` reduced to `0` (target exceeded).

13. `P1-CLIENT-008` Refresh metrics and checkpoint commit discipline after each tranche. `STATUS: IN_PROGRESS`
- ROI: 8/10
- Dependency: each completed tranche
- Target outcome:
- Every tranche ends with refreshed artifacts + verify pass + checkpoint commit.

14. `P2-SRS-001` Execute SRS backlog in dependency order starting with policy/core primitives. `STATUS: IN_PROGRESS`
- ROI: 10/10
- Dependency: `P1-CLIENT-003`
- Target outcome:
- Continue closing highest-ROI queued items in `docs/workspace/SRS.md` while keeping client thin by default.

15. `P2-SRS-002` Run ongoing regression against top SRS critical set after each SRS tranche. `STATUS: IN_PROGRESS`
- ROI: 9/10
- Dependency: `P2-SRS-001`
- Target outcome:
- No SRS execution merges without deterministic regression evidence.
- Current evidence:
- Full sweep artifact `artifacts/srs_full_regression_current.json` + `docs/workspace/SRS_FULL_REGRESSION_CURRENT.md` generated from all SRS rows (`1998`) with fail-class regressions closed (`fail=0`, `warn=27`, `pass=1971` in the latest sweep).

16. `P3-BLOCKED-001` External/human-gated items. `STATUS: BLOCKED`
- Examples:
- Independent audit publication, human-authorized release publication, weekly human cadence evidence.

## Commands used in this checkpoint
- `npm run -s ops:client-layer:boundary`
- `npm run -s ops:repo-surface:audit`
- `npm run -s ops:public-platform:contract`
- `npm run -s ops:client-target:audit`
- `npm run -s ops:layer-placement:check`
- `npm run -s ops:dependency-boundary:check`
- `npm run -s ops:formal-spec:check`
- `node scripts/ci/client_scope_inventory.mjs`
- `node scripts/ci/client_surface_disposition.mjs`
- `node scripts/ci/backlog_actionable_report.mjs`
- `npm run -s metrics:rust-share`
- `cargo build --manifest-path core/layer0/ops/Cargo.toml --bin protheus-ops`
- `./verify.sh`
