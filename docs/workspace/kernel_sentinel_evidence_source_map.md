# Kernel Sentinel Evidence Source Map

Owner: core/layer0/kernel_sentinel
Status: in-progress source inventory
Updated: 2026-04-26

## Purpose

Kernel Sentinel does not treat repo churn as evidence by itself. Existing telemetry must be bridged into normalized evidence streams under `local/state/kernel_sentinel/evidence/*.jsonl` before Sentinel can study failures, produce issue candidates, or support RSI readiness.

This map identifies current telemetry producers, their authority class, and the Sentinel stream they should feed.

## Evidence stream targets

| Sentinel stream | Authority class | Purpose |
|---|---|---|
| `kernel_receipts.jsonl` | deterministic_kernel_authority | Kernel receipt integrity, forged receipt rejection, mutation receipt coverage. |
| `runtime_observations.jsonl` | deterministic_kernel_authority | Runtime failures, workflow phases, finalization failures, state-machine correctness. |
| `state_mutations.jsonl` | deterministic_kernel_authority | Mutations that need receipt binding and legal transition checks. |
| `scheduler_admission.jsonl` | deterministic_kernel_authority | Admission, capability, scheduler, and fail-closed enforcement signals. |
| `live_recovery.jsonl` | deterministic_kernel_authority | Recovery loops, rollback, retry, auto-heal, and unresolved degraded states. |
| `boundedness_observations.jsonl` | deterministic_kernel_authority | RSS, queue depth, stale surfaces, recovery times, and resource ceilings. |
| `release_proof_packs.jsonl` | deterministic_kernel_authority | Proof-pack completeness, required-missing counts, release verdicts. |
| `release_repairs.jsonl` | deterministic_kernel_authority | Repair/fallback/rehearsal evidence for release-critical paths. |
| `gateway_health.jsonl` | deterministic_kernel_authority | Gateway health, support level, fail-closed behavior, and capability limits. |
| `gateway_quarantine.jsonl` | deterministic_kernel_authority | Quarantine events, repeated flapping, breaker state, and isolation outcomes. |
| `gateway_recovery.jsonl` | deterministic_kernel_authority | Gateway recovery attempts, route-around decisions, and recovery bounds. |
| `gateway_isolation.jsonl` | deterministic_kernel_authority | Sandbox/isolation/resource-limit evidence for external boundaries. |
| `queue_backpressure.jsonl` | deterministic_kernel_authority | Shed/defer/quarantine behavior, queue ceilings, and pressure policy decisions. |
| `control_plane_eval.jsonl` | advisory_workflow_quality | Control-plane eval findings, live monitor drift, issue candidates, synthesis quality failures. |
| `shell_telemetry.jsonl` | presentation_telemetry_only | Presentation symptoms such as stale UI phases, missing thinking/status display, and operator-visible shell drift; observation-only and cannot open Sentinel findings by itself. |

## Current producers to bridge

| Producer path | Current signal | Sentinel target | Authority class | Freshness source | Collector action |
|---|---|---|---|---|---|
| `local/state/ops/verity/**` | Receipt and verification artifacts. | `kernel_receipts.jsonl` | deterministic_kernel_authority | artifact timestamp or row timestamp | Normalize receipt ok/status, subject, fingerprint, evidence, and receipt hash. |
| `local/state/ops/system_health_audit/**` | System health audit summaries and JSONL rows. | `runtime_observations.jsonl` | deterministic_kernel_authority | generated_at or filesystem mtime | Normalize health state, degraded/critical status, source artifact, and recommended action. |
| `local/state/ops/eval_agent_feedback/**` | Eval-generated findings and issue-style feedback. | `control_plane_eval.jsonl` | advisory_workflow_quality | generated_at | Preserve advisory authority class, dedupe key, severity, evidence, and no-auto-apply posture. |
| `local/state/ops/eval_learning_loop/**` | Reviewer outcome, learning-loop, and eval quality records. | `control_plane_eval.jsonl` | advisory_workflow_quality | generated_at | Normalize repeated stable signatures and reviewer outcomes as advisory feedback. |
| `local/state/ops/synthetic_user_chat_harness/**` | Chat/workflow/tool-routing harness outcomes. | `runtime_observations.jsonl` and `control_plane_eval.jsonl` | mixed | run timestamp | Deterministic route failures go to runtime; grader-only judgments go to control-plane eval. |
| `core/local/artifacts/*proof*` | Proof-pack, release gate, and release-governance artifacts. | `release_proof_packs.jsonl` | deterministic_kernel_authority | generated_at | Normalize required_missing, release gate pass, blocker counts, and artifact list. |
| `core/local/artifacts/*repair*` | Repair, installer, fallback, or rehearsal artifacts. | `release_repairs.jsonl` | deterministic_kernel_authority | generated_at | Normalize repair status, fallback reason, source artifact, and required follow-up. |
| `core/local/artifacts/*stateful*`, `*rollback*`, and `*upgrade*` | Stateful upgrade, rollback, and mutation transition gates. | `state_mutations.jsonl` | deterministic_kernel_authority | generated_at | Normalize transition/mutation evidence, rollback receipts, and illegal-transition status. |
| `core/local/artifacts/*scheduler*`, `*schedule*`, `*admission*`, `agent_surface_status_guard*`, and `layer3_contract_guard*` | Scheduler, admission, and execution-surface boundary gates. | `scheduler_admission.jsonl` | deterministic_kernel_authority | generated_at | Normalize scheduler/admission status, capability-boundary state, and fail-closed admission checks. |
| `core/local/artifacts/*recovery*`, `*auto_heal*`, `*rollback*`, and `*retry*` | Live recovery, retry, rollback, and auto-heal evidence. | `live_recovery.jsonl` | deterministic_kernel_authority | generated_at | Normalize recovery action, resolved state, retry/bounds evidence, and rollback receipt status. |
| `core/local/artifacts/*gateway*` | Gateway manifest, status, chaos, and support artifacts. | `gateway_health.jsonl`, `gateway_quarantine.jsonl`, `gateway_recovery.jsonl`, `gateway_isolation.jsonl` | deterministic_kernel_authority | generated_at | Route by event kind: health, quarantine, recovery, isolation. |
| `core/local/artifacts/*queue*` and `core/local/artifacts/*backpressure*` | Queue/backpressure policy and boundedness outputs. | `queue_backpressure.jsonl` | deterministic_kernel_authority | generated_at | Normalize queue depth, p95/max, action taken, and policy status. |
| `core/local/artifacts/*boundedness*` | Resource ceiling and soak outputs. | `boundedness_observations.jsonl` | deterministic_kernel_authority | generated_at | Normalize RSS, disk, queue, stale incidents, recovery time, and regression status. |
| `local/state/ops/shell_telemetry/**`, `local/state/ops/runtime_telemetry/**`, and `core/local/artifacts/*shell*/*dashboard*` | Shell-facing telemetry, dashboard presentation symptoms, chat phase display drift, and thinking/status UI evidence. | `shell_telemetry.jsonl` | presentation_telemetry_only | generated_at or filesystem mtime | Normalize as observation-only context; preserve source artifact and symptom fields, but never allow Shell telemetry to write verdicts, waive findings, or open release-blocking findings. |
| `local/state/kernel_sentinel/**` | Sentinel reports, trend history, feedback inbox, and issues. | not a primary input stream | deterministic_kernel_authority | generated_at | Use only for trend/readiness self-study; do not recursively turn Sentinel outputs into primary evidence. |

## Bridge rules

1. The collector must write only to `local/state/kernel_sentinel/evidence/*.jsonl`.
2. Every normalized row must preserve a `source_artifact` or evidence reference.
3. Control-plane eval data remains advisory and cannot write Sentinel verdicts, waive findings, or block release directly.
4. Shell telemetry is presentation-only context and cannot open Sentinel findings by itself.
5. Kernel/runtime/proof/gateway/queue evidence may open release-blocking findings.
6. Missing producers must be reported as skipped sources in the collector artifact, not silently ignored.
7. Malformed rows must be counted separately from missing streams.
8. Sentinel must report `data_starved` when no normalized records exist.
9. Sentinel must report `partial_evidence` when some streams are present and others are missing.
10. RSI readiness requires fresh deterministic evidence, not merely advisory eval feedback.

## Next bridge order

| Order | Bridge | Reason |
|---:|---|---|
| 1 | `local/state/ops/verity/**` -> `kernel_receipts.jsonl` | Deterministic Kernel receipts are the highest-value Sentinel input. |
| 2 | `local/state/ops/system_health_audit/**` -> `runtime_observations.jsonl` | Turns health/churn into runtime correctness evidence. |
| 3 | `local/state/ops/eval_agent_feedback/**` -> `control_plane_eval.jsonl` | Makes current eval feedback visible without giving it Kernel authority. |
| 4 | proof-pack artifacts -> `release_proof_packs.jsonl` | Catches release-governance regressions. |
| 5 | gateway/queue/boundedness artifacts -> dedicated streams | Completes runtime closure coverage. |
| 6 | Shell telemetry -> `shell_telemetry.jsonl` | Gives Sentinel user-visible symptom context without granting Shell authority. |
