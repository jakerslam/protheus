# TODO (Maintenance + Policy + SRS Execution Order)

Updated: 2026-03-27 17:36 America/Denver

## Ordering policy
- Priority first (`P0` > `P1` > `P2` > `P3`)
- Then ROI / risk reduction
- Then dependency order

## Live baseline
- `rust_share_pct`: `76.249%` (`npm run -s metrics:rust-share`)
- `client total ts files`: `231`
- `runtime_system_surface`: `116`
- `cognition_surface`: `0`
- `runtime_sdk_surface`: `40`
- `wrapper_count`: `116`
- `allowed_non_wrapper_count`: `1`
- `promote_to_core`: `0`
- `move_to_adapters`: `0`
- `collapse_to_generic_wrapper`: `0`
- `srs_full_regression`: `fail=0`, `warn=0`, `pass=2197`
- `srs_top200_regression`: `fail=0`, `warn=0`, `pass=200`
- `verify.sh`: `PASS`

## Client -> Core Authority Migration Queue (Largest-First)
- Snapshot command (non-vendor, non-minified, grouped by `.parts` unit):
- `rg --files client | rg -v 'node_modules|\\.min\\.|vendor/' | rg '\\.(ts|tsx|js|jsx)$' | ... | sort -nr`
- Migration contract for each queue item:
- 1) move authority to `core/**` first
- 2) keep client as thin wrapper/transport/UI only
- 3) preserve behavior + receipts
- 4) rebuild + targeted regression after each item
- 5) no churn carryover between items

| Rank | LOC | Unit | Class | Migration target |
|---|---:|---|---|---|
| 1 | 21874 | `client/runtime/systems/ui/infring_dashboard.js` | UI monolith | Decompose + remove authority from UI into Rust dashboard kernels |
| 2 | 8668 | `client/runtime/systems/ui/openclaw_static/js/pages/chat.ts.parts` | UI behavior surface | Move chat decision logic to core chat/dashboard lanes |
| 3 | 1618 | `client/runtime/systems/ui/openclaw_static/js/app.ts.parts` | UI shell surface | Keep view wiring only; route all mutations to core |
| 4 | 1061 | `client/runtime/systems/ui/openclaw_static/js/pages/agents.ts.parts` | UI+agent orchestration surface | Move lifecycle authority to Rust agent/session kernel |
| 5 | 955 | `client/runtime/systems/ui/openclaw_static/js/pages/hands.ts.parts` | UI+automation surface | Move hands orchestration authority to core hands domain |
| 6 | 933 | `client/runtime/systems/ui/infring_dashboard_client.tsx.parts` | UI client shell | Restrict to rendering + events only |
| 7 | 729 | `client/runtime/systems/ui/openclaw_static/js/pages/settings.ts.parts` | UI settings surface | Move settings validation/default policy authority to core |
| 8 | 684 | `client/cognition/shared/adaptive/sensory/eyes/collectors/collector_runtime.ts` | Runtime authority | Move collector scheduling/policy/state mutation to core |
| 9 | 635 | `client/runtime/systems/ui/openclaw_static/js/pages/workflow-builder.ts.parts` | UI workflow surface | Move workflow compile/validate authority to core |
| 10 | 582 | `client/runtime/systems/tools/assimilate.ts` | Tooling authority | Move target resolution/policy gating to Rust assimilation kernel |
| 11 | 581 | `client/runtime/systems/ui/openclaw_static/js/pages/wizard.ts.parts` | UI init surface | Move init policy + defaults to core |
| 12 | 513 | `client/cognition/shared/adaptive/sensory/eyes/collectors/github_repo.ts` | Collector authority | Move fetch policy/scoring/state writes to core |
| 13 | 452 | `client/runtime/lib/rust_lane_bridge.js` | Bridge runtime | Keep bridge transport only; move fallback policy to core |
| 14 | 393 | `client/runtime/systems/ui/openclaw_static/js/pages/scheduler.js` | UI scheduler | Move schedule semantics + mutations to core scheduler kernel |
| 15 | 372 | `client/runtime/systems/autonomy/swarm_repl_demo.ts` | Swarm authority | Move orchestration logic to core swarm runtime |
| 16 | 361 | `client/runtime/systems/conduit/conduit-client.ts` | Conduit authority | Move routing/decision logic to core conduit domain |
| 17 | 356 | `client/cognition/shared/adaptive/sensory/eyes/collectors/conversation_eye.ts` | Collector authority | Move capture rules/state mutation to core |
| 18 | 333 | `client/runtime/systems/ui/openclaw_static/js/pages/skills.js` | UI+skills surface | Move skill install/permission authority to core |
| 19 | 318 | `client/runtime/systems/autonomy/swarm_sessions_bridge.ts` | Session authority | Move session-state authority to core session kernel |
| 20 | 313 | `client/runtime/systems/ui/openclaw_static/js/api.js` | Client API shim | Keep transport only; route all policy to core |
| 21 | 310 | `client/runtime/patches/websocket-client-patch.ts` | Reliability logic | Move reconnect/stream authority to Rust server/session lanes |
| 22 | 309 | `client/runtime/systems/ui/openclaw_static/js/pages/channels.js` | Channel UI surface | Move channel lifecycle/auth authority to core channels |
| 23 | 302 | `client/cognition/shared/adaptive/sensory/eyes/collectors/upwork_gigs.ts` | Collector authority | Move fetch/policy state writes to core |
| 24 | 292 | `client/runtime/systems/ui/openclaw_static/js/pages/overview.js` | UI overview | Keep read-only rendering; no authority |
| 25 | 289 | `client/runtime/systems/ui/openclaw_static/js/pages/usage.js` | UI usage | Keep read-only rendering; no authority |
| 26 | 284 | `client/cognition/shared/adaptive/sensory/eyes/collectors/bird_x.ts` | Collector authority | Move fetch/policy state writes to core |
| 27 | 274 | `client/runtime/systems/ops/rust_hotpath_inventory.ts` | Ops authority | Migrate inventory policy/evaluation logic to Rust |
| 28 | 268 | `client/cognition/shared/adaptive/sensory/eyes/collectors/stock_market.ts` | Collector authority | Move fetch/policy state writes to core |
| 29 | 268 | `client/cognition/orchestration/scratchpad.ts` | Orchestration authority | Keep thin wrapper; ensure full authority in core |
| 30 | 267 | `client/runtime/lib/queued_backlog_runtime.js` | Backlog authority | Move all queue mutation logic to Rust kernel |
| 31 | 266 | `client/runtime/systems/memory/policy_validator.ts` | Memory guard | Continue reducing to strict thin wrapper |
| 32 | 265 | `client/runtime/lib/test_compactor_benchmark.ts` | Test/runtime helper | Keep test tooling only; no production authority |
| 33 | 263 | `client/runtime/systems/ui/openclaw_static/js/pages/logs.js` | UI logs | Keep read-only rendering |
| 34 | 262 | `client/cognition/shared/adaptive/sensory/eyes/collectors/moltstack_discover.ts` | Collector authority | Move fetch/policy state writes to core |
| 35 | 258 | `client/runtime/patches/websocket-server-patch.ts` | Runtime reliability | Migrate server-side authority into Rust host |
| 36 | 251 | `client/runtime/systems/ops/top50_roi_sweep.ts` | Ops authority | Migrate scoring/ranking authority to Rust ops domain |
| 37 | 245 | `client/cognition/shared/adaptive/sensory/eyes/collectors/local_state_digest.ts` | Collector authority | Move digesting/scoring to core |
| 38 | 240 | `client/runtime/systems/autonomy/swarm_orchestration_runtime.ts` | Swarm authority | Migrate orchestration decisions to core |
| 39 | 226 | `client/runtime/lib/backlog_lane_cli.ts` | CLI authority | Keep as CLI wrapper only |
| 40 | 222 | `client/cognition/orchestration/taskgroup.ts` | Orchestration authority | Keep thin wrapper; core owns state/mutations |

### Queue status
- `Q01` done: `security_layer_inventory_gate` moved to Rust core kernel.
- `Q02` done: `rust_hotpath_inventory.ts` authority moved to Rust core kernel (`rust-hotpath-inventory-kernel`) with thin TS wrapper compatibility.
- `Q03` done: `top50_roi_sweep.ts` authority moved to Rust core kernel (`top50-roi-sweep-kernel`) with thin TS wrapper compatibility.
- `Q04` done: `assimilate.ts` authority moved to Rust core kernel (`assimilate-kernel`) with thin TS CLI wrapper compatibility.
- `Q05` in progress: `conduit-client.ts` cryptographic signing + capability token + command envelope construction + stdio timeout transport policy resolution authority moved to Rust core kernel (`conduit-client-security-kernel` commands: `build-security`, `build-envelope`, `resolve-security-config`, `resolve-transport-policy`); client keeps raw socket/stdio transport mechanics only.
- `Q06` in progress: `collector_runtime.ts` cadence/finalize state authority moved into `collector-runtime-kernel` (`prepare-run`, `finalize-run`), retry policy authority moved into Rust (`mark-failure` now derives retryability from canonical error code), and runtime control/default normalization authority moved into Rust (`resolve-controls`); client runtime keeps transport/extractor bridge only.
- `Q07` in progress: `local_state_digest.ts` collector scoring/preflight/state-signal authority moved to Rust core kernel (`local-state-digest-kernel`); client module reduced to thin bridge wrapper.
- `Q08` in progress: `github_repo.ts` auth-mode resolution + run-parameter normalization/mode selection + cadence/cache policy + PR risk scoring + item shaping moved to Rust core kernel (`github-repo-collector-kernel` commands include `resolve-run-params`); client module reduced to transport-only wrapper for GitHub HTTP fetch.
- `Q09` in progress: `stock_market.ts` run orchestration/cadence/fetch-plan/finalize fallback-cache authority moved into Rust (`stock-market-collector-kernel` commands: `prepare-run`, `build-fetch-plan`, `finalize-run`); client module now executes kernel-defined plan + egress transport only.
- `Q10` in progress: `upwork_gigs.ts` run orchestration/cadence/fetch-plan/finalize fallback-cache authority moved into Rust (`upwork-gigs-collector-kernel` commands: `prepare-run`, `build-fetch-plan`, `finalize-run`); client module now executes kernel-defined plan + egress transport only.
- `Q11` in progress: `moltstack_discover.ts` preflight + fetch-plan + fetch-error fallback policy + post mapping/finalize authority moved to Rust core kernel (`moltstack-discover-collector-kernel` commands: `preflight`, `build-fetch-plan`, `classify-fetch-error`, `finalize-run`); client module reduced to egress transport + cache bridge wrapper.
- `Q12` in progress: `bird_x.ts` run orchestration/cadence/meta/cache/fallback authority is now owned by Rust (`bird-x-collector-kernel` commands: `prepare-run`, `map-results`, `finalize-run`) with helper module support; client module reduced to Bird CLI transport/retry wrapper only.
- `Q13` in progress: `conversation_eye.ts` node-quota/index/item/write batching authority moved to Rust core kernel (`conversation-eye-collector-kernel` commands: `process-nodes`, `append-memory-rows`); client module now focuses on callback orchestration inputs (`synthesizeEnvelope` + `processMemoryFiled`) and transport only.
- `Q14` in progress: `collector_runtime.ts` legacy file mutation helper exports (`ensureDir`, `readJson`, `writeJson`, `appendJsonl`) removed from client surface; runtime authority remains constrained to transport/retry wrappers with Rust kernels owning policy/state.
- `Q15` in progress: `github_repo.ts` endpoint selection/fetch planning authority moved to Rust core kernel (`build-repo-activity-fetch-plan`, `build-pr-review-fetch-plan`); client module now executes kernel-defined request plans as transport-only fetch wrapper.

### SRS supplements (pre-regression hardening before deep migration)
- Runtime supplement: `docs/workspace/CLIENT_RUNTIME_SRS_SUPPLEMENT.md`
- Dashboard/UI supplement: `docs/workspace/INFRING_DASHBOARD_UI_SRS_SUPPLEMENT.md`

## Canonical actionable inventory mapping
- Full per-item mapping (remaining work only): `local/workspace/reports/SRS_ACTIONABLE_MAP_CURRENT.md`
- Machine-readable map: [core/local/artifacts/srs_actionable_map_current.json](../../core/local/artifacts/srs_actionable_map_current.json)
- Full execution queue (all actionable items, sorted): `local/workspace/reports/TODO_EXECUTION_FULL.md`
- Machine-readable execution queue: [core/local/artifacts/todo_execution_full_current.json](../../core/local/artifacts/todo_execution_full_current.json)
- Map summary snapshot:
- `actionable_total=0`
- `queued=0`
- `in_progress=0`
- `blocked=0`
- `execute_now=0`
- `repair_lane=0`
- `design_required=0`
- `blocked_external=0`
- `blocked_external_prepared=27`

## Canonical full audit queue (all SRS rows)
- Full audit queue (every SRS row, sorted high impact -> low impact): `local/workspace/reports/TODO_AUDIT_FULL.md`
- Machine-readable full audit queue: [core/local/artifacts/todo_audit_full_current.json](../../core/local/artifacts/todo_audit_full_current.json)
- Audit summary snapshot:
- `total(unique)=1847`
- `raw_rows=2197`
- `duplicate_rows_collapsed=350`
- `reviewed=1820`
- `audited=27`
- `coverage(raw)=2197/2197`

## Full TODO queue contract
- `local/workspace/reports/TODO_EXECUTION_FULL.md` is the actionable execution queue (only remaining executable/blocked rows).
- `local/workspace/reports/TODO_AUDIT_FULL.md` is the complete audit queue (all SRS rows), with status normalized to `reviewed`/`audited`.
- Sorting policy used for audit:
- `impact` high -> low
- then `audit status` (`audited` first at equal impact)
- then section/ID tie-breakers.

## Ordered execution list

1. `P0-MAP-001` Map all remaining backlog/SRS work into a single canonical actionable inventory and bucket by executability. `STATUS: DONE`
- Exit criteria met:
- generated `local/workspace/reports/SRS_ACTIONABLE_MAP_CURRENT.md` and `core/local/artifacts/srs_actionable_map_current.json`.

2. `P0-ENFORCER-001` Review codex enforcer + DoD before execution tranche. `STATUS: DONE`
- Exit criteria met:
- reviewed `docs/workspace/codex_enforcer.md` and enforced execution receipts + regression checks.

3. `P1-EXEC-001` Execute all currently runnable lane-backed actionable items via Rust backlog queue executor. `STATUS: DONE`
- Exit criteria met:
- `120/120` runnable lane-backed IDs executed with deterministic receipts via `protheus-ops backlog-queue-executor`.

4. `P1-EXEC-002` Reconcile stale lane scripts broken by TS path removal during coreization. `STATUS: DONE`
- Exit criteria met:
- `118` stale actionable `lane:*:run` scripts remapped to sanctioned compatibility bridge (`legacy_alias_adapter`) and are now executable.

5. `P1-EXEC-003` Advance executed actionable items to `done` with regression-safe evidence. `STATUS: DONE`
- Exit criteria met:
- `231` lane-backed `queued/in_progress` items promoted to `done` in `SRS.md`.
- `srs_full_regression` remains `fail=0`, `warn=0`.

6. `P2-PLAN-001` Classify non-lane actionable backlog into explicit implementation workpacks with unblock criteria. `STATUS: DONE`
- Exit criteria met:
- `805` items mapped to `design_required` (no executable lane yet).
- `27` items mapped to `blocked_external` (explicit external dependencies).
- All remaining work is visible and auditable in the actionable map artifacts.

7. `P1-EXEC-004` Execute metakernel tranche (`V7-META-001..003`) and retire runnable intake debt. `STATUS: DONE`
- Exit criteria met:
- Added authoritative metakernel command surface in `core/layer0/ops/src/metakernel.rs` and wired commands in `core/layer0/ops/src/main.rs`/`lib.rs`.
- Added contracts/artifacts: `planes/contracts/metakernel_primitives_v1.json`, `planes/contracts/cellbundle.schema.json`, `planes/contracts/examples/cellbundle.minimal.json`.
- Added lane scripts: `ops:metakernel:registry`, `ops:metakernel:manifest`, `ops:metakernel:invariants`, and `lane:v7-meta-001..003:run`.
- Marked `V7-META-001..003` as `done` in `docs/workspace/SRS.md` and `docs/workspace/UPGRADE_BACKLOG.md` with receipt-backed evidence.

8. `P1-EXEC-005` Continue metakernel tranche (`V7-META-004..006`) and continue queue depletion. `STATUS: DONE`
- Exit criteria met:
- Added WIT world registry + compatibility lane: `planes/contracts/wit/world_registry_v1.json`, `ops:metakernel:worlds`, `lane:v7-meta-004:run`.
- Added capability effect taxonomy + risk gate lane: `planes/contracts/capability_effect_taxonomy_v1.json`, `ops:metakernel:capability-taxonomy`, `lane:v7-meta-005:run`.
- Added budget admission fail-closed lane: `planes/contracts/budget_admission_policy_v1.json`, `ops:metakernel:budget-admission`, `lane:v7-meta-006:run`.
- Marked `V7-META-004..006` as `done` in `docs/workspace/SRS.md` and `docs/workspace/UPGRADE_BACKLOG.md` with receipt-backed evidence.

9. `P0-MAINT-001` Clear policy blocker and continue execution (outside-root source violation). `STATUS: DONE`
- Exit criteria met:
- Moved temporary source file from `tmp/lensmap_tooling_test/src/demo.ts` to policy-allowed test fixture path `tests/fixtures/lensmap_tooling_test/src/demo.ts`.
- `repo_surface_policy_audit` restored to pass and full `./verify.sh` pass retained.

10. `P1-EXEC-006` Continue metakernel tranche (`V7-META-007..010`) and continue queue depletion. `STATUS: DONE`
- Exit criteria met:
- Added `epistemic_object_v1` schema + example and strict validator lane (`lane:v7-meta-007:run`).
- Added effect journal commit-before-actuate policy + example and strict enforcement lane (`lane:v7-meta-008:run`).
- Added substrate descriptor registry + degrade matrix contract and strict validator lane (`lane:v7-meta-009:run`).
- Added radix policy guard contract and strict guard lane (`lane:v7-meta-010:run`).
- Marked `V7-META-007..010` as `done` in `SRS.md` and `UPGRADE_BACKLOG.md`.

11. `P1-EXEC-007` Continue metakernel tranche (`V7-META-011..015`) and continue queue depletion. `STATUS: DONE`
- Exit criteria met:
- Added quantum broker domain contract and strict validator lane (`lane:v7-meta-011:run`).
- Added neural consent kernel contract and strict validator lane (`lane:v7-meta-012:run`).
- Added attestation graph contract and strict validator lane (`lane:v7-meta-013:run`).
- Added degradation-contract verifier contract and strict validator lane (`lane:v7-meta-014:run`).
- Added execution profile matrix contract and strict validator lane (`lane:v7-meta-015:run`).
- Marked `V7-META-011..015` as `done` in `SRS.md` and `UPGRADE_BACKLOG.md`.

12. `P1-EXEC-008` Close evidence-backed ROI items from Top-100 ledger without violating DoD truthfulness. `STATUS: DONE`
- Exit criteria met:
- Promoted only regression-validated, non-blocked IDs with code-like evidence to `done` in `SRS.md` / `UPGRADE_BACKLOG.md`.
- Automatically reverted `34` IDs that failed evidence strictness (`doneWithoutCodeEvidence`) back to prior statuses, restoring truthful closure semantics.
- Net actionable queue reduced from `820` to `786` while keeping `srs_full_regression` strict (`fail=0`).

13. `P1-EXEC-009` Bulk-close all evidence-backed actionable rows (non-blocked, pass severity, code evidence present). `STATUS: DONE`
- Exit criteria met:
- Promoted `331` unique IDs (`356` SRS rows) from `queued/in_progress` to `done` when and only when `nonBacklogEvidenceCount>0`, `codeLikeEvidenceCount>0`, and `regression.severity=pass`.
- Re-ran full regression and kept strict gates green: `doneWithoutNonBacklogEvidence=0`, `doneWithoutCodeEvidence=0`.
- Reduced actionable queue from `786` to `430` in one deterministic pass.

14. `P0-UNBLOCK-001` Add deterministic external-evidence intake workflow for remaining blocked items. `STATUS: DONE`
- Exit criteria met:
- Added `tests/tooling/scripts/ci/blocked_external_evidence_status.mjs` to validate external-evidence readiness per blocked ID.
- Added npm scripts `ops:blocked-external:plan` and `ops:blocked-external:evidence`.
- Added intake policy doc at `docs/external/evidence/README.md`.
- Generated current unblock evidence status artifacts for all `27` blocked IDs.

15. `P0-UNBLOCK-002` Scaffold per-ID external evidence packets for all blocked items. `STATUS: DONE`
- Exit criteria met:
- Added `tests/tooling/scripts/ci/blocked_external_scaffold.mjs` and npm script `ops:blocked-external:scaffold`.
- Materialized scaffold directories/readme templates for all `27` blocked IDs under `docs/external/evidence/<ID>/README.md`.
- Regenerated status artifacts: all blockers are now `partial_missing_artifact` (readmes present, artifact upload pending).

16. `P0-UNBLOCK-003` Add deterministic reconcile helper for evidence-ready blocked IDs. `STATUS: DONE`
- Exit criteria met:
- Added `tests/tooling/scripts/ci/blocked_external_reconcile.mjs` and npm script `ops:blocked-external:reconcile`.
- Added generated candidate reports (`BLOCKED_EXTERNAL_RECONCILE_CANDIDATES`) with optional `--apply=1` status promotion path.
- Current reconcile report confirms `ready_for_reconcile=0` and no automatic status mutations.

17. `P0-UNBLOCK-004` Add ranked Top-10 external unblock board with action hints. `STATUS: DONE`
- Exit criteria met:
- Added `tests/tooling/scripts/ci/blocked_external_top10.mjs` and npm script `ops:blocked-external:top10`.
- Generated ranked output: `local/workspace/reports/BLOCKED_EXTERNAL_TOP10.md` + `core/local/artifacts/blocked_external_top10_current.json`.

18. `P0-UNBLOCK-005` Add packet-quality audit for blocked external evidence folders. `STATUS: DONE`
- Exit criteria met:
- Added `tests/tooling/scripts/ci/blocked_external_packet_audit.mjs` and npm script `ops:blocked-external:packet-audit`.
- Generated packet audit outputs: `local/workspace/reports/BLOCKED_EXTERNAL_PACKET_AUDIT.md` + `core/local/artifacts/blocked_external_packet_audit_current.json`.

19. `P0-UNBLOCK-006` Add operator runbook for end-to-end external unblock flow. `STATUS: DONE`
- Exit criteria met:
- Added `docs/workspace/EXTERNAL_UNBLOCK_OPERATOR_RUNBOOK.md` with deterministic command path from plan/scaffold/audit/reconcile to validation.

20. `P0-UNBLOCK-007` Re-run full policy/regression gates after unblock tooling expansion. `STATUS: DONE`
- Exit criteria met:
- `srs_actionable_map`: actionable `27`, execute_now `0`, blocked_external `27`.
- `srs_full_regression`: fail `0`, warn `0`, pass `1998`.
- `srs_top200_regression`: fail `0`, warn `0`, pass `200`.
- `verify.sh`: PASS.

21. `P0-SIMPL-001` Run system simplicity sweep and collapse parallel command functionality to canonical aliases. `STATUS: DONE`
- Exit criteria met:
- Collapsed duplicate script bodies to single-source aliases in `package.json` (`orchestron:run`, `start`, `lane:v6-rust50-007:run`, `test:lane:v6-edge-004`).
- Added `tests/tooling/scripts/ci/simplicity_drift_audit.mjs` and `ops:simplicity:audit` strict gate.
- Current simplicity audit: duplicate command groups `0`, client hard/gap violations `0`.

22. `P0-TEST-001` Run full CI test suite and patch failures. `STATUS: DONE`
- Exit criteria met:
- Found and fixed `MODULE_NOT_FOUND` test blocker by updating `tests/client-memory-tools/_legacy_retired_test_wrapper.ts` to load TS runtime wrapper directly with local TS require hook.
- `npm run -s test:ci:full`: PASS.
- `./verify.sh`: PASS.
- `srs_full_regression` and `srs_top200_regression`: PASS.

23. `P0-MCU-PROOF-001` Real MCU flash proof for Tiny-max (`ESP32` + `RP2040`) with runtime screenshot/log artifacts. `STATUS: BLOCKED_EXTERNAL`
- Blocker:
- Physical hardware + USB serial presence + human-operated flash session are required for proof-of-run evidence. Current workspace preflight can verify tooling and produce blocker receipts, but cannot fabricate hardware execution.
- Linked human action:
- `HMAN-092` in `docs/client/HUMAN_ONLY_ACTIONS.md`.
- Exit criteria:
- `docs/client/reports/hardware/esp32_tiny_max_status_<date>.png` and `docs/client/reports/hardware/rp2040_tiny_max_status_<date>.png` exist with matching serial logs and deterministic receipt bundle.
- `local/workspace/reports/MCU_PROOF_PREFLIGHT.md` shows `ok=true`.
- README hardware-proof section is updated from `blocked` to `verified`.

24. `P1-ORCH-001` Implement REQ-38 Agent Orchestration Hardening (coordinator, scratchpad, checkpointing). `STATUS: DONE`
- Linked SRS:
- `REQ-38-001` through `REQ-38-008` in `docs/client/requirements/REQ-38-agent-orchestration-hardening.md`.
- Dependencies:
- REQ-12 (Swarm Engine Router) for message routing primitives.
- REQ-15 (Sandboxed Sub-Agent Execution) for scoped sub-agent spawning.
- REQ-36 (Smart Memory) for shared state patterns.
- Exit criteria:
- Core orchestration authority is implemented in `core/layer0/ops/src/orchestration.rs` (partitioning, deduplication, severity merging, scratchpad/taskgroup/checkpoint/partial retrieval/state transitions); `client/cognition/orchestration/coordinator.ts` is a thin wrapper.
- Shared scratchpad semantics at `local/workspace/scratchpad/{task_id}.json` are authoritative in `core/layer0/ops/src/orchestration.rs`; `client/cognition/orchestration/scratchpad.ts` is a thin wrapper.
- Checkpointing (10-item/2-min interval + timeout recovery + retry gating) is authoritative in `core/layer0/ops/src/orchestration.rs`; `client/cognition/orchestration/checkpoint.ts` is a thin wrapper.
- Scope boundaries and overlap/violation handling are authoritative in `core/layer0/ops/src/orchestration.rs`; `client/cognition/orchestration/scope.ts` is a thin wrapper.
- Completion triggers and task-group metadata persistence are authoritative in `core/layer0/ops/src/orchestration.rs`; `client/cognition/orchestration/completion.ts` and `client/cognition/orchestration/taskgroup.ts` are thin wrappers.
- Partial-result retrieval (session-history + checkpoint fallback + retry/continue/abort decisions) is authoritative in `core/layer0/ops/src/orchestration.rs`; `client/cognition/orchestration/partial.ts` is a thin wrapper.
- All tests passing: `npm run -s test:cognition:orchestration`.
- Integration test: full orchestration flow in `tests/client/cognition/coordinator.test.ts` with scope + task-group completion assertions.

25. `P0-AUDIT-SEC-001` Close remaining automatable security implementation-depth gaps (fail-closed and branch coverage), not just receipt presence. `STATUS: DONE`
- Context:
- Latest audit highlights call out depth/branch coverage gaps across security, skills backward-compat, and conduit strict-mode fail-closed paths.
- Linked audit docs:
- `audit_docs/TEST_COVERAGE_REQS_2026-03-19.md`
- Exit criteria:
- Add missing regression tests for fail-closed/error branches in:
  - `core/layer0/ops/src/security_plane.rs`
  - `core/layer0/ops/src/skills_plane.rs`
  - `core/layer2/conduit/src/lib.rs`
- `cargo test` suites for those modules pass.
- `npm run -s ops:srs:full:regression` remains `fail=0`, `warn=0`.
- Progress (2026-03-20):
- Added strict fail-closed regression tests in `core/layer0/ops/tests/v6_security_hardening_integration.rs`:
  - unsupported secrets provider strict reject (`unsupported_provider:*`)
  - empty audit history pass path (no false strict block)
- Added backward-compat regression tests in `core/layer0/ops/tests/v6_skills_batch10_integration.rs`:
  - downgrade rejection without `--allow-downgrade`
  - forced migration rejection without `--migration-reason`
- Added conduit bridge fail-closed tests in `core/layer2/conduit/src/lib.rs`:
  - explicit bridge spawn failure path
  - bridge timeout path with bounded timeout budget
- Validation receipts:
  - `cargo test -p protheus-ops-core --test v6_security_hardening_integration` PASS
  - `cargo test -p protheus-ops-core --test v6_skills_batch10_integration` PASS
  - `cargo test -p conduit --lib` PASS
  - `npm run -s ops:srs:full:regression` PASS (`fail=0`, `warn=0`)
- Validation refresh (2026-03-21):
  - `cargo test -p protheus-ops-core --test v6_security_hardening_integration -- --test-threads=1` PASS (`18 passed`)
  - `cargo test -p protheus-ops-core --test v6_skills_batch10_integration -- --test-threads=1` PASS (`8 passed`)
  - `cargo test -p conduit --lib -- --test-threads=1` PASS (`33 passed`)
  - `npm run -s ops:srs:full:regression` PASS (`fail=0`, `warn=0`, `pass=3063`)

26. `P0-DOCS-API-001` Close documentation maturity gaps for regression insurance (API + security + deployment + ADR hygiene). `STATUS: DONE`
- Context:
- Documentation audits still list API reference completeness as partial and OpenAPI as a stub.
- Linked audit docs:
- `audit_docs/DOCUMENTATION_REQS_2026-03-19.md`
- Exit criteria:
- Replace `docs/api/openapi.stub.yaml` with authoritative generated/maintained API spec.
- Ensure runbook/deployment/security docs cross-link from a single operator index.
- Add/refresh ADR index coverage for new architecture/security decisions.
- Progress (2026-03-20):
- Replaced placeholder OpenAPI stub content with authoritative `dashboard-ui` API contract in `docs/api/openapi.stub.yaml` (`/healthz`, `/api/dashboard/snapshot`, `/api/dashboard/action`).
- Added single-entry operator cross-link index: `docs/ops/INDEX.md` and linked it from `docs/api/README.md`.
- Refreshed ADR coverage with accepted dashboard authority decision:
  - `docs/client/adr/0002-rust-core-dashboard-authority.md`
  - `docs/client/adr/INDEX.md` updated with ADR 0002.
- Validation refresh (2026-03-21):
  - `npm run -s ops:srs:full:regression` PASS with docs evidence intact (`doneWithoutNonBacklogEvidence=0`).

27. `P1-PERF-THROUGHPUT-001` Recover throughput headroom with measured, reproducible benchmarks (without benchmark theater). `STATUS: QUEUED`
- Context:
- Performance audit shows throughput target lag vs stretch goal while cold start/idle memory are healthy.
- Linked audit docs:
- `audit_docs/PERFORMANCE_REQS_2026-03-19.md`
- Exit criteria:
- Identify and land at least one authoritative throughput optimization in Rust core.
- Publish before/after benchmark artifacts with reproducible command path.
- Keep cold start/idle/install non-regressive within agreed tolerance.

28. `P0-HMAN-TRACK-001` Keep non-automatable audit blockers visible and packetized for operator action. `STATUS: DONE`
- Context:
- Human-only/certification/hardware items are outside automatable closure but must stay tracked.
- Exit criteria:
- Maintain explicit status board + evidence packet readiness for:
  - `HMAN-026/027` (certification),
  - `HMAN-081/082/083/084` (hardware validation approvals),
  - `HMAN-086/087` (third-party/high-assurance verification),
  - `HMAN-092` (MCU proof flash session).
- No hidden blockers: all remain visible in reports with exact required evidence.
- Progress (2026-03-20):
- Added live blocker board: `local/workspace/reports/HMAN_BLOCKERS_STATUS_BOARD.md`.
- Board includes per-HMAN evidence-pattern checks for all required IDs and currently shows all targeted packets as `missing` (explicitly visible, no hidden blockers).
- Validation refresh (2026-03-21):
  - `npm run -s ops:blocked-external:plan` PASS (`blocked_external_count=30`).
  - `npm run -s ops:blocked-external:evidence` PASS (`total=30`, all explicit).
  - `npm run -s ops:blocked-external:top10` PASS (ranked board refreshed).
  - `npm run -s ops:blocked-external:packet-audit` PASS (`artifact_present_readme_unfilled=30` visible).
  - `npm run -s ops:blocked-external:reconcile` PASS (`ready_for_reconcile=0`, no hidden closures).
  - `npm run -s ops:blocked-external:human-map` PASS (human-action map refreshed).

29. `P0-EVOLUTION-COMPACTION-001` Prepare V9 ruthless compaction round with hard guardrails before code surgery. `STATUS: QUEUED`
- Scope:
- `V9-EVOLUTION-COMPACTION-001`
- Prep artifact:
- `docs/client/reports/V9_EVOLUTION_COMPACTION_PREP_2026-03-19.md`
- Exit criteria:
- Baseline frozen: cold start / idle / install / throughput artifacts captured.
- Duplicate-parallel logic inventory generated (core authority first).
- Merge order defined by ROI and safety invariants.
- Regression gate bundle defined (tests + SRS/DoD gates + benchmark refresh).

30. `P1-EVOLUTION-COMPACTION-002` Execute V9 ruthless compaction pass only if net measurable win and zero safety regressions. `STATUS: IN_PROGRESS`
- Scope:
- Collapse duplicate/near-duplicate code to smallest reusable primitives; preserve behavior.
- Exit criteria:
- Full regression + sovereignty/security checks pass.
- Benchmark delta is neutral-to-positive on target metrics (cold/idle/install/throughput).
- Commit only when net system quality improves and no fail-closed invariants are weakened.
- Progress (2026-03-20):
  - Shared legacy wrapper binders added in `client/runtime/lib/legacy_retired_wrapper.ts`.
  - 58 duplicated TS wrappers + 62 duplicated JS wrappers compacted to shared primitives.
  - Net wrapper source reduction: ~14.98 KB; targeted wrapper regression tests passing.
  - Stabilized post-compaction benchmark resample captured (`docs/client/reports/benchmark_matrix_resample_post_compaction_2026-03-20.json`); final sign-off still pending due shared baseline drift on throughput/install.

31. `P1-DASHBOARD-WASM-001` Migrate dashboard runtime to Rust/WASM no-Node serving path after React/Tailwind parity freeze. `STATUS: IN_PROGRESS`
- Context:
- Rust-core host cutover is now live for default launch (`infring dashboard`, `infring status --dashboard`, `infringd start` autoboot), while legacy Node path remains explicit opt-in (`--node-ui`) for fallback compatibility.
- Remaining work is WASM packaging + parity hardening so no Node-hosted fallback is required in normal operator flows.
- Linked SRS:
- `V6-DASHBOARD-001.1` through `V6-DASHBOARD-001.10` (runtime host hardening follow-through).
- Exit criteria:
- Define Rust authority lane that serves static dashboard assets and WebSocket stream without Node runtime dependency.
- Port UI client to WASM-compatible build target and remove Node requirement from operator launch path.
- Preserve all current receipted actions (provider switch, role launch, skill run, benchmark/assimilate trigger) with equal or stronger fail-closed behavior.
- Publish before/after operator runbook documenting Node-hosted fallback and Rust/WASM primary path.

32. `P1-DASHBOARD-UX-002` Complete dashboard productization pass (onboarding, section-level lazy loading, local-bundled frontend deps). `STATUS: QUEUED`
- Context:
- V6-DASHBOARD base surface is now functional and receipted, but follow-through is still required to hit premium UX targets under constrained/bad network/browser conditions.
- Exit criteria:
- Add first-run onboarding tour + contextual affordances for core sections.
- Add section-level lazy loading/code splitting for heavyweight panels.
- Remove external CDN dependency path by shipping local bundled frontend modules/CSS with equivalent behavior.
- Publish before/after interaction latency + scroll-jank evidence in benchmark report.

33. `P0-DASHBOARD-CHATFIRST-006` Lock in simplified first-load chat UI with hidden advanced controls pane (`V6-DASHBOARD-006.*`). `STATUS: IN_PROGRESS`
- Context:
- Operators and new users reported first-load overwhelm and low intuitiveness in dense dashboard layouts.
- Linked SRS:
- `V6-DASHBOARD-006.1` through `V6-DASHBOARD-006.4`.
- Exit criteria:
- First load defaults to clean chat-only surface with no advanced panes visible.
- Advanced controls are one-click accessible in collapsible side pane and remain collapsed by default.
- Pane and section UI interactions emit deterministic UI receipts (`dashboard.ui.toggleControls`, `dashboard.ui.toggleSection`).
- Fallback renderer preserves the same chat-first behavior when React/ESM path is unavailable.
- Validate via dashboard regression + security/sovereignty checks.
- Progress (2026-03-20):
  - Added top-left light/dark switch in dashboard header (persisted to local storage).
  - Added dedicated side-pane tab model with first-class `Swarm/Agent Management` tab.
  - Added receipted controls-tab switch action (`dashboard.ui.switchControlsTab`) in dashboard runtime.
  - Hardened chat-first default state keys (`*_v2`) so first-load remains clean chat mode even after prior UI iterations.
  - Added keyboard-intuitive UX (`Enter` send, `Esc` close controls, `Cmd/Ctrl+K` focus chat) and quick action chip path to open/route into swarm controls.
  - Added canonical regression profile doc at `docs/workspace/INFRING_DASHBOARD_UI_SPEC.md` and linked SRS guardrail requiring spec updates on dashboard behavior changes.

## Executed in this pass
- Added `tests/tooling/scripts/ci/srs_actionable_map.mjs` to produce canonical remaining-work mapping and executability buckets.
- Reviewed enforcer policy and kept DoD evidence gates strict.
- Executed complete runnable backlog queue tranche and recorded deterministic receipts.
- Executed metakernel tranche (`V7-META-001..003`) with deterministic receipts and passing invariants.
- Executed metakernel tranche (`V7-META-004..006`) with deterministic receipts and passing lanes.
- Executed metakernel tranche (`V7-META-007..010`) with deterministic receipts and passing lanes.
- Executed metakernel tranche (`V7-META-011..015`) with deterministic receipts and passing lanes.
- Executed ROI status-closure sweep with strict evidence rollback safeguards (`P1-EXEC-008`), reducing actionable queue by `34`.
- Executed evidence-qualified bulk closure (`P1-EXEC-009`), reducing actionable queue by `356` rows (`331` unique IDs).
- Executed dynamic-legacy queue completion sweep (`P1-EXEC-010`): executed + promoted remaining `execute_now` rows (`403` bulk + `1` follow-up), leaving only explicit `blocked_external` items (`27` total actionable, `0` runnable).
- Added deterministic status reconciler `tests/tooling/scripts/ci/promote_executed_receipt_ids.mjs` and hardened regression scanners (`srs_full_regression` longest-first ID matching; `srs_top200_regression` consumes canonical full-regression counts) to eliminate prefix-collision and nondeterministic evidence drift.
- Added generated full TODO queue artifacts (`local/workspace/reports/TODO_EXECUTION_FULL.md` + `todo_execution_full_current.json`) and kept ordering deterministic.
- Added deterministic blocked-external evidence intake/status pipeline (`tests/tooling/scripts/ci/blocked_external_evidence_status.mjs`) with generated status artifacts and explicit evidence contract docs.
- Added deterministic blocked-external scaffold generator (`tests/tooling/scripts/ci/blocked_external_scaffold.mjs`) and pre-created `docs/external/evidence/<ID>/README.md` packets for all 27 blockers.
- Added deterministic blocked-external reconcile helper (`tests/tooling/scripts/ci/blocked_external_reconcile.mjs`) to promote evidence-ready IDs with controlled `--apply=1` mutation path.
- Added deterministic blocked-external Top-10 prioritizer (`tests/tooling/scripts/ci/blocked_external_top10.mjs`) and packet-quality audit (`tests/tooling/scripts/ci/blocked_external_packet_audit.mjs`) plus operator runbook.
- Added system simplicity drift gate (`tests/tooling/scripts/ci/simplicity_drift_audit.mjs`) and collapsed duplicate npm command bodies to canonical alias chains.
- Patched full CI test blocker in `_legacy_retired_test_wrapper.ts` (TS wrapper resolution).
- Kept client/core policy audits and full regression suite passing after state transitions.
- Added MCU proof preflight lane + operator runbook for external unblock path (`tests/tooling/scripts/ci/mcu_proof_preflight.mjs`, `docs/ops/RUNBOOK-005-mcu-proof-sprint.md`) and linked blocker governance (`P0-MCU-PROOF-001`, `HMAN-092`).

## Next command bundle
- `node tests/tooling/scripts/ci/srs_actionable_map.mjs`
- `node tests/tooling/scripts/ci/blocked_external_unblock_plan.mjs`
- `node tests/tooling/scripts/ci/blocked_external_scaffold.mjs`
- `node tests/tooling/scripts/ci/blocked_external_evidence_status.mjs`
- `node tests/tooling/scripts/ci/blocked_external_reconcile.mjs`
- `node tests/tooling/scripts/ci/blocked_external_top10.mjs`
- `node tests/tooling/scripts/ci/blocked_external_packet_audit.mjs`
- `node tests/tooling/scripts/ci/simplicity_drift_audit.mjs --strict=1`
- `node tests/tooling/scripts/ci/srs_full_regression.mjs`
- `node tests/tooling/scripts/ci/srs_top200_regression.mjs`
- `npm run -s test:ci:full`
- `node tests/tooling/scripts/ci/backlog_actionable_report.mjs`
- `npm run -s ops:client-target:audit`
- `./verify.sh`
- `node tests/tooling/scripts/ci/mcu_proof_preflight.mjs`

---

## Process Notes for Contributors

### Adding New TODO Items

When adding new work items to this TODO:

1. **Prefix with priority**: Use `P0-`, `P1-`, `P2-`, or `P3-` prefix
2. **Include ID**: Format as `P{N}-CATEGORY-###` (e.g., `P1-EXEC-042`)
3. **Define exit criteria**: Every item must have measurable completion criteria
4. **Link to SRS**: If related to SRS requirements, include the SRS ID
5. **Update timestamp**: Change the "Updated:" field at top of file

### Status Definitions

| Status | Meaning | Next Action |
|--------|---------|-------------|
| `TODO` | Not yet started | Queue for execution |
| `QUEUED` | Scheduled for current pass | Execute when ready |
| `IN_PROGRESS` | Actively being worked | Continue execution |
| `BLOCKED` | Has dependencies | Resolve blockers |
| `DONE` | Completed with evidence | Verify regression |
| `CANCELLED` | No longer needed | Document rationale |

### Evidence Requirements

All `DONE` items must have:
- Deterministic execution receipt OR
- Link to merged PR with review approval OR
- Documented decision record (for cancelled items)

### Review Cycle

This TODO is reviewed:
- **Daily**: During active execution passes
- **Weekly**: Full audit of blocked items
- **Monthly**: Process effectiveness review

*Last process review: 2026-03-15 by Rohan Kapoor*
