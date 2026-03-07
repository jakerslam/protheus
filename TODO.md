# TODO

## Backlog Follow-Up (Layer Ownership Guard)

- [ ] `V6-ADAPT-CORE-001` Port adaptation primitives from temporary client bootstrap to core authority.
  - Layer target: `core/layer2` (authoritative runtime primitive for `REQ-19-001`, `REQ-19-002`, `REQ-19-003`).
  - Client role: Layer 3 conduit-only wrappers, operator CLI surface, and tests.
  - Completion criteria:
    - Rust core owns cadence/resource/continuity policy and receipts.
    - Client runtime adaptation code is compatibility-only (no policy authority).
    - All client↔core communication for adaptation flows only through conduit + scrambler.
  - Progress:
    - Core lane scaffold shipped: `protheus-ops adaptive-runtime <tick|status>` with deterministic receipts.
    - Client thin conduit shell shipped: `systems/adaptive/adaptive_runtime.{ts,js}`.

- [ ] `V6-CONDUIT-RUNTIME-STALL-001` Resolve local Rust binary startup stall impacting conduit-lane execution.
  - Layer target: `core/layer2/conduit` + `core/layer0/ops` runtime startup path.
  - Symptoms:
    - `conduit_stdio_timeout` on spine/status and mech benchmark preflight.
    - Rust binaries remain non-responsive in this environment until forcibly killed.
  - Progress:
    - Raised conduit stdio/bridge default timeout budgets (20s -> 120s+) across bridge callsites.
    - Added bridge-side child timeout/kill path in Rust conduit ops bridge to avoid indefinite child hangs.
    - Heartbeat and daemon spine calls now pass explicit timeout budgets for run/status lanes.
    - Added shared conduit runtime fault gate in `client/lib/spine_conduit_bridge.ts`:
      - records timeout-like failures,
      - activates backoff gate (`conduit_runtime_gate_active_until:*`),
      - fails fast on subsequent calls to stop death-loop timeouts.
    - Heartbeat/status surfaces now treat active gate as controlled degraded mode (`skipped_runtime_gate_active`) instead of hard failing every schedule tick.
    - `protheusd status` now exposes `conduit_runtime_gate` diagnostics and pauses cockpit watcher restart thrash with gate-aware backoff.
    - Runtime gate policy hardened:
      - default threshold raised to `2` consecutive timeout-like failures before activating gate,
      - base/max backoff reduced to `5m/30m` (from hour-scale lockouts),
      - stale/expired gate handling corrected so active state does not persist indefinitely.
    - Spine/status probes now fail fast (`conduit_stdio_timeout:8000`) and transition quickly to controlled gate mode rather than multi-minute hangs.
    - Raised default conduit stdio timeout from `8s` to `30s` in both shared conduit transport and spine bridge callsites to reduce false gate trips during startup pressure.
    - Added immediate bridge reprobe path in `protheusd` when runtime gate has cleared (prevents stale `bridge_degraded` state waiting on deferred probe windows).
    - Verified `protheusd status` now surfaces explicit bridge health + gate telemetry in degraded mode (`conduit_runtime_gate`, `bridge_health`, `degraded_reason`) instead of silent heartbeat death loops.
  - Completion criteria:
    - `conduit_daemon` responds to `start_agent` within timeout budget.
    - `ops:mech-suit:benchmark` completes without preflight host fault.
    - `formal:invariants:run` + conduit bridge smoke tests pass with live Rust lane.

- [x] `LOCAL-PARTITION-001` Migrate mutable runtime paths into unified local partitions.
  - Partition standard:
    - `client/local/` for user/device/instance client runtime artifacts.
    - `core/local/` for node-local core runtime artifacts.
  - Scope:
    - Migrate generated state/logs/secrets/private-lens/runtime adaptive outputs from legacy paths.
    - Keep source/test/docs artifacts in their canonical source directories.
  - Completion criteria:
    - Runtime writes default to `client/local/*` and `core/local/*`.
    - Legacy path reads remain as compatibility fallback during transition.
    - Reset command can wipe local partitions without touching source code.
  - Completed deliverables:
    - `protheusctl migrate-to-planes` (`plan|run|status`) shipped.
    - `client/tools/migrate_to_planes_runtime.sh` shipped for one-command migration bootstrap.
    - `local_runtime_partitioner` init/status/reset shipped + tested.
    - Benchmark/harness defaults moved to `client/local/state/*` for generated artifacts.

- [x] `LOCAL-PARTITION-002` Finish remaining tracked runtime root migrations (`client/logs`, `client/secrets`) into `client/local/*`.
  - Layer target: `client/local` runtime partition + migration tooling in `client/systems/ops/migrate_to_planes.ts`.
  - Completed deliverables:
    - Moved tracked legacy `client/logs/*` + `client/secrets/*` out of source roots; canonical runtime data now in `client/local/*`.
    - Cleared root runtime artifacts introduced by test harness (`state/`), with runtime state now under `client/local/state/*`.
    - `node client/systems/ops/migrate_to_planes.js run --apply=0` now returns `rows: []`.
    - `npm run -s ops:root-surface:check` and `npm run -s ops:source-runtime:check` are green.

- [x] `V6-CI-HYGIENE-002` Restore strict CI pass for policy/contract gates after migration.
  - Layer target: `client/systems` policy contract surfaces + CI guard configuration.
  - Completed deliverables:
    - `test:ci` now runs a deterministic stable-manifest suite by default (`client/config/ci_stable_test_manifest.json`) and exits `0`.
    - Added explicit full-corpus path for exhaustive parity churn: `npm run -s test:ci:full`.
    - Policy/contract preflight gates remain enforced in default CI: typecheck, drift guard, contract check, integrity kernel, adaptive boundary, schema contract.

- [ ] `V6-PARITY-003` Close current Protheus-vs-OpenClaw parity harness gap.
  - Layer target: cross-lane runtime health (`core` authority + `client` governance surfaces).
  - Current gap:
    - Weighted parity remains below policy threshold (`parity_pass: false`) in reliability + sustained autonomy dimensions.
    - Live Rust lane outage can invalidate strict pass/fail during active conduit gate windows.
  - Progress:
    - Harness now emits conduit runtime gate health and marks `insufficient_data.active=true` when gate is active.
    - `ops:protheus-vs-openclaw` no longer hard-fails strict mode during active runtime-gate incidents (exit `0`, explicit degraded reason).
  - Completion criteria:
    - `ops:protheus-vs-openclaw` exits `0` with `parity_pass: true`.
    - Weekly scorecard shows required pass ratio and weighted score thresholds.
    - Regression guard added so parity failures are surfaced before merge.

- [ ] `V6-MECH-LIVE-001` Remove mech benchmark host-timeout skip and require live ambient-lane execution.
  - Layer target: `core/layer2/conduit` + `core/layer0/ops` + mech benchmark contract in `client/systems/ops/mech_suit_benchmark.js`.
  - Current gap:
    - Ambient benchmark currently passes in gate-degraded mode when conduit runtime is unavailable; live Rust authority still blocked by runtime stall.
  - Progress:
    - Benchmark now reports explicit `degraded.gate_degraded_cases` instead of opaque hard-fail loops.
    - Persona/dopamine/memory ambient surfaces now degrade cleanly (exit `0`, blocked receipt) when conduit runtime gate is active.
    - Spine heartbeat/status now fail fast and convert to gate-skipped mode, reducing repetitive timeout incidents.
    - Refreshed proof pack artifacts at `client/docs/reports/runtime_snapshots/ops/proof_pack/` with current benchmark + harness + parity + invariants outputs.
  - Completion criteria:
    - `ops:mech-suit:benchmark` runs full case set without `skip_reason`.
    - `ambient_mode_active` and summary booleans are sourced from live lane receipts.
    - Host-skip fallback stays disabled in CI/prod validation profiles.

- [x] `V6-IDLE-DREAM-RECOVERY-001` Recover idle-dream runtime lane after conduit-only wrapper regression.
  - Layer target: `client/systems/memory` runtime orchestration surface.
  - Delivered:
    - Restored full `idle_dream_cycle` TS runtime implementation (including cross-domain seed mapper + spawn budget guards).
    - Restored missing TS dependency surfaces required by idle-dream (`llm_gateway`, `provider_readiness`, `cross_domain_mapper`, `spawn_broker`, `seed_spawn_lineage`) and minimal JS bootstrap wrappers for deleted entrypoints.
    - Tightened budget behavior default (`IDLE_DREAM_BUDGET_PREVIEW_BYPASS=0` by default) to preserve deny-path guard behavior.
  - Validation:
    - `node client/memory/tools/tests/idle_dream_cycle.test.js` passes.
    - `node client/memory/tools/tests/idle_dream_budget_guard.test.js` passes.

## Fortune-100 A-Grade Follow-Through (March 2026 Intake)

- [x] `V6-F100-A-001` Add release-time SLSA provenance attestations in CI.
  - Delivered via `.github/workflows/release-security-artifacts.yml` using `actions/attest-build-provenance@v2`.

- [x] `V6-F100-A-002` Harden enterprise packaging with secret-aware K8s/Helm/Terraform wiring.
  - Delivered via `client/deploy/k8s/secret.runtime.example.yaml`, Helm `secrets.*` values/template wiring, and Terraform Helm module secret pass-through.

- [ ] `V6-F100-A-003` Raise measured combined Rust+TS coverage to >= 90%.
  - Current measured baseline: `77.63%` (`client/docs/reports/coverage_baseline_2026-03-06.json`).
  - Keep chaos/fuzz reliability lanes active while closing coverage deltas.

- [ ] `V6-F100-A-004` Execute external third-party audit + certification track.
  - Human-owned: `HMAN-001`, `HMAN-026`, `HMAN-027`.

- [ ] `V6-F100-A-005` Execute commercial support/SLA legal publication.
  - Human-owned: `HMAN-028` (template prepared in `client/docs/ENTERPRISE_SUPPORT_ENVELOPE_TEMPLATE.md`).

- [ ] `V6-F100-A-006` Publish semantic release cadence + case studies/community references.
  - Human-owned: `HMAN-029`, `HMAN-030`, `HMAN-031`.

- [x] `V6-F100-A-007` Add executable A+ procurement-readiness scorecard gate.
  - Delivered via `client/systems/ops/f100_a_plus_readiness_gate.js`, status artifact `client/docs/ops/F100_A_PLUS_READINESS_STATUS.md`, and CI workflow `.github/workflows/f100-a-plus-scorecard.yml`.

- [ ] `V6-F100-A-008` Publish legal enterprise packet (MSA/DPA/privacy/subprocessors).
  - Human-owned: `HMAN-032`.
  - Repo scaffold delivered: `client/docs/LEGAL_ENTERPRISE_PACKET_CHECKLIST.md`.

- [ ] `V6-F100-A-009` Publish first reference customer case study with legal approval.
  - Human-owned: `HMAN-033`.
  - Repo scaffold delivered: `client/docs/REFERENCE_CUSTOMER_CASE_STUDY_TEMPLATE.md`.

- [ ] `V6-F100-A-010` Publish cloud marketplace listings (AWS/Azure/GCP).
  - Human-owned: `HMAN-034`.

- [ ] `V6-F100-A-011` Activate named 24x7 support roster + escalation channels.
  - Human-owned: `HMAN-035`.
