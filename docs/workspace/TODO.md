# TODO (Priority + ROI + Dependency Ordered)

Updated: 2026-03-10 (policy enforcement tranche closed + Protheus 2.0 intake applied)

## Ordering policy
- Priority first (`P0` > `P1` > `P2` > `P3`)
- Then ROI (higher unblock value first)
- Then dependency chain (prerequisites before dependents)

## Backlog snapshot
- Source: `docs/workspace/SRS.md` + `client/runtime/config/backlog_registry.json`
- Latest actionable report: `artifacts/backlog_actionable_report_2026-03-10_policy_enforcement.json`
- Counts: `actionable=380`, `queued=378`, `in_progress=2`, `blocked=42`, `done=2228`

## Current Objective: Client Boundary Completion

Goal:
- `client` contains only developer-facing SDK/CLI/UI/app-bridge surfaces.
- `client` contains no JS/Shell/Python/PowerShell.
- `client` contains no source-of-truth logic or hidden internal runtime truth.
- authority/system truth stays in `core`; apps/connectors/integration-specific logic live in `apps` or `adapters`.

Live baseline from current worktree:
- `client/*.js = 0`
- `client/*.sh = 0`
- `client/*.py = 0`
- `client/*.ps1 = 0`
- `client total ts files = 926`
- `client/runtime/systems = 679`
- `wrapper_count = 673`
- `allowed_non_wrapper_count = 15`
- `cognition_surface = 100`
- `runtime_sdk_surface = 59`
- hard client target violations: `0`
- repo language share: `Rust = 65.744%`

Execution list for this objective only:

1. `OBJ-CLIENT-001` `P0` Capture and refresh live baselines after each tranche. `STATUS: COMPLETE`
- Exit criteria:
- `client_scope_inventory_current.json`, wrapper inventory, and rust-share artifact are regenerated from the current worktree, not stale `HEAD`.
- Purpose:
- Prevent false progress from stale audits and keep the execution queue tied to real files.
- Completion evidence:
- `scripts/ci/client_scope_inventory.mjs`
- `scripts/ci/client_legacy_debt_report.mjs`
- `artifacts/client_scope_inventory_current.json`
- `artifacts/client_legacy_debt_report_current.json`

2. `OBJ-CLIENT-002` `P0` Partition the remaining `client` TS surface by disposition. `STATUS: COMPLETE`
- Exit criteria:
- Every remaining `client` TS file is assigned to exactly one bucket:
  - `keep_public_client`
  - `collapse_to_generic_wrapper`
  - `promote_to_core`
  - `move_to_apps`
  - `move_to_adapters`
  - `move_to_tests`
  - `delete_dead_surface`
- Purpose:
- Stop treating all TS as equal; only public DX/UX surfaces should survive in `client`.
- Completion evidence:
- `scripts/ci/client_surface_disposition.mjs`
- `client/runtime/config/client_target_contract_policy.json`
- `artifacts/client_surface_disposition_current.json`

3. `OBJ-CLIENT-003` `P0` Collapse dead and duplicate wrapper families. `STATUS: QUEUED`
- Exit criteria:
- legacy-retired wrapper families are reduced behind a small number of generic entrypoints.
- dead alias shells are deleted instead of renamed.
- wrapper inventory count drops materially from the current `667`.
- Purpose:
- Most of the remaining client bloat is wrapper duplication, not useful public surface.

4. `OBJ-CLIENT-004` `P0` Audit the remaining allowed non-wrapper files one by one. `STATUS: IN_PROGRESS`
- Exit criteria:
- each allowlisted non-wrapper is explicitly justified as one of:
  - public SDK/DX
  - UI/operator surface
  - app bridge
  - temporary debt with named migration target
- any file that is actually authority/runtime truth is moved to `core` or another correct surface.
- Purpose:
- The allowlist must become a principled public contract, not a permanent escape hatch.
- Progress evidence:
- `client/runtime/config/client_layer_boundary_policy.json`
- `client/runtime/config/client_target_contract_policy.json`
- allowlist reduced `21 -> 15` in the current worktree

5. `OBJ-CLIENT-005` `P1` Shrink `client/runtime/systems` to true public bridges only. `STATUS: QUEUED`
- Exit criteria:
- runtime-system files that only front Rust receipts are collapsed or removed.
- non-user-facing internal compatibility scaffolding is promoted to `core`, `adapters`, `scripts`, or deleted.
- `client/runtime/systems` count is driven down substantially from the current `679`.
- Purpose:
- This directory is still far too large relative to the intended policy.

6. `OBJ-CLIENT-006` `P1` Shrink `client/cognition/*` to true user/dev-facing cognition surfaces only. `STATUS: QUEUED`
- Exit criteria:
- cognition wrappers that do not directly help users/devs or app integration are moved out or collapsed.
- retained cognition files are limited to operator-facing or app-building surfaces.
- Purpose:
- cognition compatibility shims are still inflating client scope without providing public value.

7. `OBJ-CLIENT-007` `P1` Consolidate `client/lib` and `client/runtime/lib` into a minimal public SDK bridge layer. `STATUS: QUEUED`
- Exit criteria:
- duplicate alias modules are removed.
- retained bridge modules have explicit public purpose and stable import paths.
- hidden internal glue no longer lives in public-facing `client/lib`.
- Purpose:
- current alias stacks make the client look larger and blur public/private boundaries.

8. `OBJ-CLIENT-008` `P1` Promote remaining runtime truth to Rust core wherever wrappers still front non-core behavior. `STATUS: QUEUED`
- Exit criteria:
- any file in `client` that still implements real decision/state/policy logic is ported to `core` and reduced to a thin wrapper.
- updated parity and receipt tests prove core is authoritative.
- Purpose:
- directory cleanup alone will not satisfy the policy if truth remains in TS.

9. `OBJ-CLIENT-009` `P1` Re-home non-client surfaces that still sit under `client`. `STATUS: IN_PROGRESS`
- Exit criteria:
- installer/setup helpers live under `scripts` or root bootstrap surfaces.
- workflow/product surfaces live under `apps`.
- external-system or polyglot bridges live under `adapters`.
- tests remain outside `client`.
- Purpose:
- keep the client as the public platform, not a dumping ground for everything TS-adjacent.
- Progress evidence:
- `apps/personas/rohan-kapoor/projects/*`
- `adapters/skills/*`
- `tmp/client-runtime/*`
- `apps/examples/singularity-seed-demo/orchestrator.ts`
- `adapters/economy/smart_contract_bridge.ts`
- `adapters/importers/*`

10. `OBJ-CLIENT-010` `P1` Set and enforce a minimal client target contract. `STATUS: IN_PROGRESS`
- Exit criteria:
- a checked policy artifact defines what categories are allowed in `client` and by file pattern/path.
- CI fails if new files enter `client` outside approved categories.
- Purpose:
- prevent the same regression after cleanup is complete.
- Progress evidence:
- `scripts/ci/client_target_contract_audit.mjs`
- `client/runtime/config/client_target_contract_policy.json`
- `artifacts/client_target_contract_audit_current.json`

11. `OBJ-CLIENT-011` `P1` Recover Rust share while reducing client TS surface. `STATUS: QUEUED`
- Exit criteria:
- Rust share trends upward from the current `65.744%`.
- TS reductions come from real removals/collapses, not file shuffling.
- core-authority migrations continue wherever client shrink reveals non-core truth.
- Purpose:
- architecture quality and repo language mix need to improve together, not trade off.

12. `OBJ-CLIENT-012` `P0` End-to-end regression after each objective tranche. `STATUS: IN_PROGRESS`
- Exit criteria:
- wrapper smoke commands, policy audits, Rust gates, and `./verify.sh` pass after each batch.
- no client-boundary cleanup lands without runtime verification.
- Purpose:
- avoid breaking the public platform while shrinking it.
- Progress evidence:
- `npm run -s ops:client-target:audit`
- `./verify.sh`

13. `OBJ-CLIENT-013` `P0` Stable checkpointing discipline. `STATUS: QUEUED`
- Exit criteria:
- each completed tranche ends with TODO refresh, artifact refresh, clean checkpoint commit, and push.
- Purpose:
- keep progress resumable without needing chat-state babysitting.

## Ordered execution queue

1. `MAINT-001` `P0` `ROI=10/10` `DEP=none` Refresh TODO from live SRS/backlog state. `STATUS: COMPLETE`
- Exit criteria:
- TODO reflects current SRS statuses and dependency-aware ordering.
- Completion evidence:
- `docs/workspace/TODO.md`
- `artifacts/backlog_actionable_report_2026-03-10_todo_refresh.json`

2. `V6-SEC-008` `P0` `ROI=10/10` `DEP=V6-SEC-003` Continuous Fuzzing + Chaos Suite closure. `STATUS: COMPLETE`
- Exit criteria:
- Nightly workflow emits deterministic fuzz/chaos report artifacts.
- Triage policy exists and is linked in security policy.
- Completion evidence:
- `.github/workflows/nightly-fuzz-chaos.yml`
- `scripts/ci/nightly_fuzz_chaos_report.mjs`
- `docs/client/FUZZ_CHAOS_TRIAGE.md`
- `SECURITY.md`
- `artifacts/nightly_fuzz_chaos_report_latest.json`

3. `MAINT-002` `P0` `ROI=9/10` `DEP=001,002` Post-change gate/regression pass. `STATUS: COMPLETE`
- Exit criteria:
- Primitive wrapper contract gate passes.
- Coreization static audit passes.
- Rust-share gate remains above 60%.
- `verify.sh` passes.
- Completion evidence:
- `./target/debug/protheus-ops contract-check --rust-contract-check-ids=primitive_ts_wrapper_contract`
- `artifacts/coreization_wave1_static_audit_2026-03-10_todo_refresh.json`
- `npm run -s metrics:rust-share:gate` (`64.849%`)
- `./verify.sh`

4. `MAINT-003` `P1` `ROI=8/10` `DEP=003` Refresh actionable backlog artifact after tranche execution. `STATUS: COMPLETE`
- Exit criteria:
- New actionable artifact generated from current SRS/TODO.
- Completion evidence:
- `artifacts/backlog_actionable_report_2026-03-10_todo_refresh.json`

5. `MAINT-004` `P1` `ROI=9/10` `DEP=coreization+security` Client layer boundary lock (wrapper-only runtime systems + explicit residual allowlist). `STATUS: COMPLETE`
- Exit criteria:
- Full `client/runtime/systems` source scan has zero unexpected non-wrapper files.
- Explicit policy tracks residual developer/app surfaces still in client.
- Completion evidence:
- `client/runtime/config/client_layer_boundary_policy.json`
- `scripts/ci/client_layer_boundary_audit.mjs`
- `artifacts/client_layer_boundary_audit_2026-03-10_policy_enforcement.json`
- `npm run -s ops:client-layer:boundary`

6. `MAINT-005` `P1` `ROI=8/10` `DEP=004` Repo surface policy codified (`core/client/apps/adapters/tests`). `STATUS: COMPLETE`
- Exit criteria:
- Repo topology and language policy are documented and enforced by audit.
- `/apps`, `/adapters`, and `/tests` surfaces are explicitly defined.
- Completion evidence:
- `docs/client/architecture/LAYER_RULEBOOK.md`
- `client/runtime/config/repo_surface_policy.json`
- `scripts/ci/repo_surface_policy_audit.mjs`
- `apps/README.md`
- `adapters/README.md`
- `tests/README.md`
- `artifacts/repo_surface_policy_audit_2026-03-10_policy_enforcement.json`

7. `MAINT-007` `P0` `ROI=9/10` `DEP=005` Bind policy enforcement into default verification paths (`verify.sh` + CI). `STATUS: COMPLETE`
- Exit criteria:
- Local verification runs client-boundary, repo-surface, and public-platform contract audits before origin integrity.
- GitHub Actions enforces the same boundary checks on push/PR.
- Completion evidence:
- `verify.sh`
- `.github/workflows/formal-spec-guard.yml`
- `npm run -s ops:public-platform:contract`
- `./verify.sh`

8. `MAINT-008` `P0` `ROI=8/10` `DEP=007` Public platform contract audit for `apps/` + `adapters/`. `STATUS: COMPLETE`
- Exit criteria:
- Apps/adapters fail closed if they reach private `core/` or deep `client/runtime|cognition|memory` surfaces.
- Public app/adaptor surfaces are forced through explicit client contracts only.
- Completion evidence:
- `client/runtime/config/public_platform_contract_policy.json`
- `scripts/ci/public_platform_contract_audit.mjs`
- `artifacts/public_platform_contract_audit_2026-03-10_policy_enforcement.json`

9. `MAINT-009` `P1` `ROI=8/10` `DEP=005` Client legacy debt inventory + migration ledger. `STATUS: COMPLETE`
- Exit criteria:
- Non-TS client files are classified by recommended target (`apps`, `tests`, runtime debt, installer/developer shim, etc.).
- TODO queue has a current baseline for the remaining burn-down.
- Completion evidence:
- `scripts/ci/client_legacy_debt_report.mjs`
- `artifacts/client_legacy_debt_report_2026-03-10_policy_enforcement.json`
- Current baseline summary:
  - `total=4288`
  - `js=4257`
  - `sh=19`
  - `py=11`
  - `ps1=1`

10. `MAINT-010` `P1` `ROI=7/10` `DEP=008,009` Move public example apps out of `client` into `/apps/examples`. `STATUS: COMPLETE`
- Exit criteria:
- Public runnable demos no longer live under `client/cli/apps/examples`.
- Demos invoke the public CLI/binary contract instead of private `client/runtime/systems/*` internals.
- Completion evidence:
- `apps/examples/_shared/run_protheus_toolkit.js`
- `apps/examples/personas-demo/run.js`
- `apps/examples/dictionary-demo/run.js`
- `apps/examples/orchestration-demo/run.js`
- `apps/examples/blob-morphing-demo/run.js`
- `apps/examples/comment-mapper-demo/run.js`
- `docs/client/cognitive_toolkit.md`
- `README.md`
- Smoke evidence:
  - `node apps/examples/dictionary-demo/run.js`
  - `node apps/examples/personas-demo/run.js`

11. `V6-ALIVE-001.2` `P1` `ROI=10/10` `DEP=V3-RACE-180,007` Confidence-gated autophagy auto-approval + rollback window. `STATUS: COMPLETE`
- Source:
- `proposals/protheus_optimization_v2.md`
- `AGE-10`
- Exit criteria:
- High-confidence bounded proposals auto-execute under policy thresholds with delayed commit, rollback window, and explicit regret/remediation path on degradation.
- Human review shifts from per-proposal blocking to exception/batch approval for low-confidence or excluded proposal classes.
- Completion evidence:
- `core/layer2/ops/src/autophagy_auto_approval.rs`
- `core/layer0/ops/src/main.rs`
- `client/runtime/config/autophagy_auto_approval_policy.json`
- `client/runtime/systems/autonomy/autophagy_auto_approval.ts`
- `docs/client/AUTOPHAGY_AUTO_APPROVAL.md`

12. `V6-ALIVE-001.1` `P1` `ROI=9/10` `DEP=007` Micro-dopamine events + objective auto-verification. `STATUS: QUEUED`
- Source:
- `proposals/protheus_optimization_v2.md`
- `AGE-10`
- Exit criteria:
- Objective work classes (maintenance, health checks, documentation, issue creation, anomaly logging) accrue reward without manual thumbs-up.
- Deterministic dopamine ledger + weekly roll-up surfaces exist and are receipt-backed.

13. `V6-ALIVE-001.4` `P1` `ROI=8/10` `DEP=007` Async health monitoring envelope. `STATUS: QUEUED`
- Source:
- `proposals/protheus_optimization_v2.md`
- `AGE-10`
- Exit criteria:
- Routine health polling becomes background/non-blocking with bounded escalation thresholds and aggregated receipts.
- Normal operations are not blocked by non-critical health checks.

14. `V6-ALIVE-001.5` `P1` `ROI=8/10` `DEP=001.4` Tiered verbosity + deep-dive instrumentation mode. `STATUS: QUEUED`
- Source:
- `proposals/protheus_optimization_v2.md`
- `AGE-10`
- Exit criteria:
- Severity-based instrumentation tiers are active (`critical/full`, `normal/lightweight`, `routine/sampled`, `noise/suppressed`).
- Deep-dive mode can be enabled temporarily for anomaly investigation and auto-reverts after bounded duration.

15. `V6-ALIVE-001.3` `P1` `ROI=8/10` `DEP=V6-LLMN-004,007` Progressive right-hemisphere task classifier + synthesis windows. `STATUS: QUEUED`
- Source:
- `proposals/protheus_optimization_v2.md`
- `AGE-10`
- Exit criteria:
- Right-brain eligibility is class-scoped (`synthesis`, `strategy`, `design`, `meta-analysis`, etc.) and fail-closed for execution/health/alert handling.
- Scheduled low-risk synthesis windows exist with rollback triggers and measurable right-brain usage telemetry.

16. `V6-ALIVE-001.6` `P2` `ROI=7/10` `DEP=001.1,001.4,001.5` Unified daily standup + critical update bridge. `STATUS: QUEUED`
- Source:
- `proposals/protheus_optimization_v2.md`
- `AGE-10`
- Exit criteria:
- A single daily briefing summarizes yesterday/today/blockers/proposed actions from receipts/state.
- Governed operator surfaces (Linear/Slack/internal dashboard) can receive the summary and critical update mirrors with deterministic delivery receipts.

17. `V6-ALIVE-001.7` `P2` `ROI=6/10` `DEP=001.1` Weekly cohort dopamine scoring + trend view. `STATUS: QUEUED`
- Source:
- `proposals/protheus_optimization_v2.md`
- `AGE-10`
- Exit criteria:
- Dopamine scoring rolls up to weekly cohort targets/bonuses instead of daily pressure loops.
- Trend reporting is visible in operator surfaces and backed by deterministic ledger state.

18. `MAINT-006` `P1` `ROI=9/10` `DEP=009,010` Client legacy surface burn-down (language debt closed, wrapper volume still oversized). `STATUS: IN_PROGRESS`
- Current baseline:
- Language/format debt is now closed in the live worktree:
  - `client/*.js = 0`
  - `client/*.sh = 0`
  - `client/*.py = 0`
  - `client/*.ps1 = 0`
- Remaining oversized TS surface:
  - `client total ts files = 898`
  - `runtime_system_surface = 679`
  - `cognition_surface = 83`
  - `runtime_sdk_surface = 58`
  - `wrapper_count = 667`
  - `allowed_non_wrapper_count = 21`
- Latest tranche evidence:
  - `tests/client-memory-tools/`
  - `tests/websocket-stability-test.js`
  - `packages/README.md`
  - `packages/lensmap/`
  - `packages/protheus-core/`
  - `packages/protheus-edge/`
  - `packages/protheus-py/`
  - `artifacts/client_legacy_debt_report_2026-03-10_policy_enforcement.json`
  - `artifacts/repo_surface_policy_audit_2026-03-10_policy_enforcement.json`
  - removed legacy shim roots:
    - `client/systems/security/`
    - `client/systems/memory/`
    - `client/systems/audit/`
    - `client/systems/spine/`
    - `client/runtime/systems/lib/`
  - evicted untracked runtime-state debris from `client/runtime/state/state` into ignored top-level `state/local/`
  - moved package distribution surfaces out of `client/cli/packages/` into top-level `/packages`
  - reclassified thin JS runtime wrappers as `runtime_wrapper_debt` instead of authority debt
  - package smoke evidence:
    - `node packages/protheus-core/starter.js --mode=contract --spine=0 --reflex=0 --gates=0 --max-mb=5 --max-ms=200`
  - `node packages/protheus-core/core_profile_contract.js status`
  - `node packages/protheus-edge/starter.js --mode=status`
  - `node packages/lensmap/lensmap_cli.js status`
  - `apps/_shared/run_protheus_ops.js`
  - `apps/habits/scripts/spine_daily.js`
  - `apps/habits/scripts/spine_eyes.js`
  - `adapters/polyglot/pilot_task_classifier.py`
  - `scripts/skills/imap-smtp-email/setup.sh`
  - `artifacts/client_scope_inventory_current.json`
  - `artifacts/public_platform_contract_audit_2026-03-10_policy_enforcement.json`
  - `npm run -s ops:public-platform:contract`
  - direct wrapper smoke evidence:
    - `node client/runtime/systems/tools/research.ts status`
    - `node client/runtime/systems/autonomy/optimization_aperture_controller.ts verify`
    - `node client/cognition/adaptive/sensory/eyes/collectors/google_trends.ts status`
    - `node client/runtime/systems/compat/legacy_retired_lane.ts status --lane-id=RUNTIME-SYSTEMS-TOOLS-RESEARCH`
- Exit criteria:
- Client reaches TS/TSX + HTML/CSS target state except explicitly-approved installer/package shims.
- App/adaptor/test candidates are relocated out of `client`.
- Remaining closure work:
  - collapse dead or duplicate legacy-retired wrappers behind fewer generic compatibility entrypoints
  - reduce `client/runtime/systems` to true DX/app bridge surfaces only
  - move non-user-facing cognition/runtime TS that is still acting as internal compatibility scaffolding out of `client`

19. `MAINT-011` `P1` `ROI=7/10` `DEP=008,009,010` Expose a public contract for `singularity_seed_demo` and move the last client example app to `/apps`. `STATUS: COMPLETE`
- Exit criteria:
- Demo routes through a public CLI/SDK contract and no runnable example app remains under `client/cli/apps/examples`.
- Completion evidence:
- `apps/examples/singularity-seed-demo/run.js`
- `apps/examples/singularity-seed-demo/README.md`
- `artifacts/repo_surface_policy_audit_2026-03-10_policy_enforcement.json`
- `artifacts/client_legacy_debt_report_2026-03-10_policy_enforcement.json`
- `node apps/examples/singularity-seed-demo/run.js`

20. `V6-SEC-001` `P1` `ROI=9/10` `DEP=V6-F100-003` Audited Release + SBOM bundle (`v0.2.0`). `STATUS: IN_PROGRESS`
- Current state:
- Required scaffolding already exists:
  - `.github/workflows/release-security-artifacts.yml`
  - `docs/client/RELEASE_SECURITY_CHECKLIST.md`
  - `docs/client/releases/v0.2.0_migration_guide.md`
- Readiness evidence:
  - `artifacts/release_security_readiness_latest.json`
- Remaining closure condition:
- Human-authorized tagged release publication and artifact verification record (`HMAN-030`).

21. `COREIZATION-NEXT-001` `P1` `ROI=9/10` `DEP=003` Deep authority migration (core-first) for remaining TS heavy surfaces. `STATUS: COMPLETE`
- Scope:
- `client/runtime/lib/strategy_resolver.ts` -> `core/layer2/execution` authoritative path
- `client/runtime/lib/duality_seed.ts` -> `core/layer2/autonomy` authoritative path
- Exit criteria:
- TS files reduced to thin conduit wrappers only.
- Rust crate lanes carry source-of-truth behavior and pass parity tests.
- Completion evidence:
- `core/layer0/ops/src/strategy_resolver.rs`
- `core/layer0/ops/src/duality_seed.rs`
- `client/runtime/lib/strategy_resolver.ts`
- `client/runtime/lib/duality_seed.ts`
- `artifacts/coreization_wave1_static_audit_2026-03-10_coreization_next_001.json`

22. `V6-SEC-004` `P2` `ROI=7/10` `DEP=V6-SEC-001,V6-SEC-003` Independent security audit publication. `STATUS: IN_PROGRESS`
- Current state:
- Publication + remediation pack scaffolded in-repo:
  - `docs/client/security/INDEPENDENT_AUDIT_PUBLICATION_2026Q1.md`
  - `docs/client/security/INDEPENDENT_AUDIT_REMEDIATION_TRACKER.md`
- Remaining closure condition:
- External auditor-authored report publication (human/external dependency).

23. `V6-SEC-005` `P2` `ROI=7/10` `DEP=V6-SEC-002,V6-SEC-004` Formal verification expansion package. `STATUS: COMPLETE`
- Completion evidence:
  - `docs/client/security/FORMAL_VERIFICATION_EXPANSION_PACK.md`
  - `scripts/ci/formal_verification_expansion_report.mjs`
  - `artifacts/formal_verification_expansion_latest.json`

24. `V6-F100-025` `P2` `ROI=6/10` `DEP=human cadence` Weekly chaos evidence cadence contract. `STATUS: BLOCKED`
- Blocker:
- Requires sustained weekly operational cadence + human-owned evidence publication.

25. `V7-META-FOUNDATION` `P3` `ROI=8/10` `DEP=coreization-next` Metakernel foundation wave (`V7-META-001..015`). `STATUS: QUEUED`
- Notes:
- Keep queued until `COREIZATION-NEXT-001` is closed to avoid splitting authority lanes.

## Commands used in this tranche
- `npm run -s ops:client-layer:boundary > artifacts/client_layer_boundary_audit_2026-03-10_policy_enforcement.json`
- `npm run -s ops:repo-surface:audit > artifacts/repo_surface_policy_audit_2026-03-10_policy_enforcement.json`
- `npm run -s ops:public-platform:contract > artifacts/public_platform_contract_audit_2026-03-10_policy_enforcement.json`
- `node scripts/ci/client_legacy_debt_report.mjs --out=artifacts/client_legacy_debt_report_2026-03-10_policy_enforcement.json`
- `node scripts/ci/client_scope_inventory.mjs`
- `npm run -s ops:layer-placement:check`
- `node client/runtime/systems/tools/research.ts status`
- `node client/runtime/systems/autonomy/optimization_aperture_controller.ts verify`
- `node client/cognition/adaptive/sensory/eyes/collectors/google_trends.ts status`
- `node client/runtime/systems/compat/legacy_retired_lane.ts status --lane-id=RUNTIME-SYSTEMS-TOOLS-RESEARCH`
- `node apps/examples/dictionary-demo/run.js`
- `node apps/examples/personas-demo/run.js`
- `./verify.sh`
- `node scripts/ci/backlog_actionable_report.mjs > artifacts/backlog_actionable_report_2026-03-10_policy_enforcement.json`
