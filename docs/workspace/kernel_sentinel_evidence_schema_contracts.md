# Kernel Sentinel Evidence Schema Contracts

Owner: core/layer0/kernel_sentinel
Status: in-progress schema contract
Updated: 2026-04-26

## Purpose

Kernel Sentinel evidence is line-delimited JSON under `local/state/kernel_sentinel/evidence/*.jsonl`. Each row is normalized by the Sentinel into a canonical record and may open a finding when it reports failure, critical status, explicit severity, stale data, or hard-fail invariant evidence.

## Common row fields

| Field | Required | Type | Meaning |
|---|---:|---|---|
| `id` | recommended | string | Stable row ID from the producer. Falls back to source and line number. |
| `ok` | optional | boolean | `false` opens a finding. |
| `status` | optional | string | `failed`, `blocked`, `invalid`, or `critical` opens a finding. |
| `severity` | optional | string | `critical`, `high`, `medium`, or `low`. Presence opens a finding. |
| `category` | optional | string | Sentinel category. PascalCase and snake_case are accepted. |
| `fingerprint` | recommended | string | Stable dedupe key for repeated failures. |
| `subject` | recommended | string | Operation, artifact, lane, gateway, receipt, or task being observed. |
| `kind` | recommended | string | Observation subtype, such as `receipt_check`, `workflow_trace`, or `proof_pack`. |
| `summary` | recommended for findings | string | Operator-readable failure summary. |
| `recommended_action` | recommended for findings | string | Repair action. |
| `evidence` | required for high-quality findings | string array | Source artifacts, receipts, traces, or local paths. |
| `details` | optional | object | Stream-specific data. Nested `details.details` is also tolerated for compatibility. |

## Common freshness fields

At least one freshness field should be present when the producer can provide it.

| Field | Type | Meaning |
|---|---|---|
| `details.generated_at` | ISO timestamp | Producer-generated timestamp. |
| `details.generated_at_epoch_seconds` | integer | Unix timestamp used by Sentinel freshness checks. |
| `details.freshness_age_seconds` | integer | Age at collection time. |
| `details.age_seconds` | integer | Compatibility alias for age. |
| `details.source_artifact_mtime_epoch_seconds` | integer | Filesystem mtime of source artifact if no embedded timestamp exists. |

## Common authority fields

Authority is primarily derived from the stream file name, but producers should include these fields when possible for auditability.

| Field | Type | Meaning |
|---|---|---|
| `details.authority_class` | string | `deterministic_kernel_authority`, `advisory_workflow_quality`, or `presentation_telemetry_only`. |
| `details.source_artifact` | string | Local source artifact path or URI. |
| `details.dedupe_key` | string | Producer-side dedupe key, if distinct from `fingerprint`. |
| `details.safe_to_auto_file_issue` | boolean | Issue filing policy. |
| `details.safe_to_auto_apply_patch` | boolean | Must be false for eval/advisory issue candidates. |

## Stream contracts

| Stream | Required details | Finding triggers |
|---|---|---|
| `kernel_receipts.jsonl` | `receipt_hash`, `mutation_id`, `receipt_state`, `source_artifact` | `ok=false`, forged receipt fields, missing receipt, invalid hash. |
| `runtime_observations.jsonl` | `phase` or `phases`, `source_artifact`, `workflow_id` or `operation_id` | failed phase, finalization failure, runtime correctness failure, stale observation. |
| `state_mutations.jsonl` | `from_state`, `to_state`, `mutation_id`, `receipt_hash` | illegal transition, missing receipt, rollback gap. |
| `scheduler_admission.jsonl` | `capability`, `admission_status`, `policy_scope`, `probe_key` | capability bypass, raw payload shortcut, missing probe, unauthorized admission. |
| `live_recovery.jsonl` | `recovery_action`, `attempt_count`, `resolved`, `rollback_receipt` | unresolved recovery, retry storm, missing rollback receipt. |
| `boundedness_observations.jsonl` | `rss_mb`, `queue_depth_max`, `queue_depth_p95`, `recovery_time_ms` | ceiling regression, stale surface incident, recovery SLO breach. |
| `release_proof_packs.jsonl` | `required_missing`, `release_gate_pass`, `artifact_count`, `source_artifact` | any required-missing count, failed release gate, malformed proof-pack. |
| `release_repairs.jsonl` | `repair_status`, `fallback_used`, `source_artifact`, `closeout_receipt` | failed repair, unsafe fallback, missing closeout receipt. |
| `gateway_health.jsonl` | `gateway_id`, `support_level`, `health_status`, `source_artifact` | unhealthy graduated gateway, missing health check, unsafe degradation. |
| `gateway_quarantine.jsonl` | `gateway_id`, `quarantine_state`, `breaker_state`, `failure_count` | repeated flapping, quarantine failure, breaker bypass. |
| `gateway_recovery.jsonl` | `gateway_id`, `recovery_action`, `recovered`, `recovery_time_ms` | recovery failure, timeout, unsafe route-around. |
| `gateway_isolation.jsonl` | `gateway_id`, `sandbox`, `timeout_ms`, `memory_limit_mb` | missing isolation, limit breach, unsafe gateway mutation. |
| `queue_backpressure.jsonl` | `queue_id`, `depth`, `depth_p95`, `action`, `threshold` | missing shed/defer/quarantine action, threshold breach. |
| `control_plane_eval.jsonl` | `eval_id`, `finding_type`, `source_reference`, `safe_to_auto_apply_patch=false` | advisory authority claim, missing source reference, repeated stable failure. |
| `shell_telemetry.jsonl` | `surface_id`, `symptom`, `source_artifact`, `presentation_status` | observation-only; rows normalize for context but must not open Sentinel findings, write verdicts, waive findings, or block release by themselves. |

## Example deterministic runtime row

```json
{"id":"runtime-obs-1","ok":false,"subject":"turn-7","kind":"operation_trace","category":"RuntimeCorrectness","severity":"High","summary":"tool result phase failed","recommended_action":"replay the ordered trajectory and repair tool result handling","evidence":["local/state/ops/system_health_audit/latest.json"],"details":{"phases":[{"name":"input","status":"ok"},{"name":"tool_result","status":"failed"}],"source_artifact":"local/state/ops/system_health_audit/latest.json"}}
```

## Example advisory eval row

```json
{"id":"eval-1","ok":false,"subject":"workspace_tool_route","kind":"eval_issue_candidate","category":"RuntimeCorrectness","severity":"Medium","summary":"workspace request routed to web search","recommended_action":"tighten probe-driven routing before issue closure","evidence":["local/state/ops/eval_agent_feedback/issues.jsonl"],"details":{"authority_class":"advisory_workflow_quality","source":"control_plane_eval","source_reference":"local/state/ops/eval_agent_feedback/issues.jsonl","safe_to_auto_file_issue":true,"safe_to_auto_apply_patch":false}}
```

## Malformed evidence policy

Malformed rows must not disappear. Sentinel reports them separately from missing streams.

| State | Meaning | Operator action |
|---|---|---|
| `data_starved` | No normalized records were ingested. | Wire collectors before trusting health or RSI readiness. |
| `partial_evidence` | Some records exist, but one or more expected streams are missing. | Complete collector coverage for missing streams. |
| `malformed_evidence` | One or more rows failed JSON/schema parsing. | Fix producer serialization or bridge normalization. |
| `stale_evidence` | One or more normalized rows exceed the configured freshness window. | Refresh the producer before trusting Sentinel health or RSI readiness. |
| `healthy_observation` | Evidence exists and expected streams are covered. | Continue monitoring and trend comparison. |

Required coverage is based on deterministic Kernel evidence and advisory control-plane evidence. Presentation-only streams such as `shell_telemetry.jsonl` are reported as optional coverage so the operator can see missing user-visible symptom context, but they do not keep required evidence in `partial_evidence` once all required streams are present.

Freshness reporting is explicit: `freshness_observed_record_count` counts rows that carried age metadata, `stale_record_count` counts rows over the threshold, `stale_evidence_seconds` records the active threshold, and `max_evidence_age_seconds` exposes the worst observed producer lag.

Malformed evidence reports must include `malformed_by_source`, `malformed_by_path`, and `malformed_by_file_name` so operators can repair the exact broken producer instead of chasing a global count.

Freshness reports include `stale_record_count`, `stale_evidence_seconds`, and `max_evidence_age_seconds`. Operators can tighten the threshold with `--stale-evidence-seconds=<seconds>` for release or RSI gates.
