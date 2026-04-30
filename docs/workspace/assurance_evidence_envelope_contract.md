# Assurance Evidence Envelope Contract

Owner: Kernel / Assurance
Status: canonical evidence contract
Schema: `observability/evidence_normalization/assurance_evidence_envelope.schema.json`
Applies to: Validation artifacts, Observability artifacts, Governance artifacts, Sentinel evidence, eval outputs, proof packs, release gates, scorecards, benchmarks, conformance guards

## Purpose

The Assurance Plane needs one evidence shape that can carry controlled proof and live observation without flattening authority.

This contract defines the shared envelope that Validation, Observability, and Governance outputs should converge toward. It does not replace existing artifact schemas immediately; it defines the canonical projection they must be able to provide.

## Required Envelope Fields

| Field | Type | Required | Meaning |
|---|---|---:|---|
| `type` | string | yes | Must be `assurance_evidence`. |
| `schema_version` | integer | yes | Current version is `1`. |
| `generated_at` | ISO timestamp | yes | Producer timestamp. |
| `domain` | enum | yes | `validation`, `observability`, or `governance`. |
| `source` | object | yes | Stable producer identity and optional artifact path. |
| `source_kind` | enum | yes | Producer family, such as `sentinel_stream`, `eval_output`, `proof_pack`, `release_gate`, `scorecard`, `benchmark`, `conformance_guard`, `regression_suite`, `telemetry`, `health`, or `trace`. |
| `authority_class` | enum | yes | Authority of the evidence, preserving existing Sentinel classes where applicable. |
| `signal_class` | enum | yes | `hard_gate`, `advisory`, or `diagnostic`. |
| `subject` | string | yes | Operation, artifact, check, lane, stream, route, subsystem, or verdict being judged or observed. |
| `status` | enum | yes | `pass`, `fail`, `blocked`, `degraded`, `observed`, `missing`, `malformed`, `stale`, `unknown`, or `not_applicable`. |
| `evidence` | array | yes | Source artifact paths, receipts, traces, issue refs, detail refs, or proof artifacts. |
| `freshness` | object | yes | Freshness state for the source. |

## Recommended Envelope Fields

| Field | Type | Meaning |
|---|---|---|
| `ok` | boolean | Compatibility success/failure value. |
| `verdict` | string | Governance verdict such as `allow`, `block`, `warn`, or `needs_more_evidence`. |
| `severity` | enum | `critical`, `high`, `medium`, `low`, or `info`. |
| `category` | string | Failure or proof category. |
| `summary` | string | Operator-readable summary. |
| `recommended_action` | string | Next repair, review, or diagnostic action. |
| `fingerprint` | string | Stable issue or trend dedupe key. |
| `receipt_hash` | string | Receipt or artifact hash when available. |
| `details` | object | Source-specific structured detail. |
| `lifecycle_state` | enum | `experimental`, `advisory`, `release_gate`, `retirement_candidate`, or `retired` for controlled checks. |

## Source Object

`source` must include at least `id`.

```json
{
  "id": "kernel_sentinel.runtime_observations",
  "owner": "core/layer0/kernel_sentinel",
  "artifact_path": "local/state/kernel_sentinel/evidence/runtime_observations.jsonl",
  "producer": "kernel-sentinel collect"
}
```

## Freshness Object

`freshness` must explicitly say whether freshness is observed.

```json
{
  "observed": true,
  "generated_at_epoch_seconds": 1777500000,
  "age_seconds": 42,
  "stale": false,
  "stale_after_seconds": 5400
}
```

If a producer cannot provide freshness yet, use:

```json
{
  "observed": false,
  "stale": false,
  "reason": "producer_has_no_timestamp_yet"
}
```

Missing freshness is a diagnostic gap; it must not be silently treated as fresh evidence.

## Authority Classes

| Authority class | Meaning |
|---|---|
| `deterministic_kernel_authority` | Kernel/Core/Gateway/runtime/proof evidence that can support hard gates. |
| `controlled_validation` | Controlled checks such as tests, evals, benchmarks, conformance guards, and regression suites. |
| `advisory_workflow_quality` | Advisory eval or workflow-quality evidence. |
| `presentation_telemetry_only` | Shell/operator-visible context that cannot open release-blocking findings by itself. |
| `governance_verdict` | Derived verdict, scorecard, release gate, readiness state, or issue-candidate governance output. |
| `harness_only` | Runner or wrapper evidence that proves execution happened but is not itself behavioral truth. |

Existing Kernel Sentinel authority classes must be preserved. Do not collapse `advisory_workflow_quality` or `presentation_telemetry_only` into Kernel authority when projecting into this envelope.

## Signal Classes

| Signal class | Required behavior |
|---|---|
| `hard_gate` | May block release, promotion, or unsafe operation. Requires deterministic evidence, controlled validation configured as release-gating, or Governance policy. |
| `advisory` | May recommend review, planning change, issue candidate, or follow-up. Does not block by itself. |
| `diagnostic` | Requests or records bounded evidence gathering before judgment. |

## ASSURANCE-006: Sentinel Stream Mapping

| Sentinel stream | Assurance domain | `source_kind` | `authority_class` | Default `signal_class` |
|---|---|---|---|---|
| `kernel_receipts.jsonl` | `observability` | `sentinel_stream` | `deterministic_kernel_authority` | `hard_gate` on failure, `diagnostic` when missing/stale. |
| `runtime_observations.jsonl` | `observability` | `sentinel_stream` | `deterministic_kernel_authority` | `hard_gate` on failed runtime correctness, `diagnostic` when incomplete. |
| `state_mutations.jsonl` | `observability` | `sentinel_stream` | `deterministic_kernel_authority` | `hard_gate` on illegal transition or missing receipt. |
| `scheduler_admission.jsonl` | `observability` | `sentinel_stream` | `deterministic_kernel_authority` | `hard_gate` on admission/capability bypass. |
| `live_recovery.jsonl` | `observability` | `sentinel_stream` | `deterministic_kernel_authority` | `hard_gate` on unresolved recovery loops, otherwise `diagnostic`. |
| `boundedness_observations.jsonl` | `observability` | `sentinel_stream` | `deterministic_kernel_authority` | `hard_gate` on budget regression, `advisory` for early trend drift. |
| `release_proof_packs.jsonl` | `observability` | `sentinel_stream` | `deterministic_kernel_authority` | `hard_gate` on missing required proof. |
| `release_repairs.jsonl` | `observability` | `sentinel_stream` | `deterministic_kernel_authority` | `hard_gate` on failed release repair or unsafe fallback. |
| `gateway_health.jsonl` | `observability` | `sentinel_stream` | `deterministic_kernel_authority` | `hard_gate` for graduated gateway failure, `advisory` for experimental gateway degradation. |
| `gateway_quarantine.jsonl` | `observability` | `sentinel_stream` | `deterministic_kernel_authority` | `hard_gate` on quarantine failure. |
| `gateway_recovery.jsonl` | `observability` | `sentinel_stream` | `deterministic_kernel_authority` | `hard_gate` on failed or unbounded recovery. |
| `gateway_isolation.jsonl` | `observability` | `sentinel_stream` | `deterministic_kernel_authority` | `hard_gate` on missing isolation for required gateway boundary. |
| `queue_backpressure.jsonl` | `observability` | `sentinel_stream` | `deterministic_kernel_authority` | `hard_gate` on pressure-policy breach, `advisory` on trend drift. |
| `control_plane_eval.jsonl` | `observability` | `sentinel_stream` | `advisory_workflow_quality` | `advisory` unless promoted by recurrence/corroboration through Governance. |
| `shell_telemetry.jsonl` | `observability` | `sentinel_stream` | `presentation_telemetry_only` | `advisory` or `diagnostic`; never a hard gate by itself. |

## ASSURANCE-007: Eval, Report, And Proof-Pack Mapping

| Current output family | Assurance domain | `source_kind` | `authority_class` | Default `signal_class` |
|---|---|---|---|---|
| Eval gold dataset checks | `validation` | `eval_output` | `controlled_validation` | `hard_gate` when configured as release-gating, otherwise `advisory`. |
| Eval monitor/chat feedback | `observability` | `eval_output` | `advisory_workflow_quality` | `advisory`; may become `diagnostic` when requesting more data. |
| Eval quality gates | `governance` | `release_gate` | `governance_verdict` | `hard_gate` when regression threshold is crossed. |
| Runtime proof gates | `validation` | `regression_suite` | `controlled_validation` | `hard_gate` when required proof fails. |
| Conformance guards | `validation` | `conformance_guard` | `controlled_validation` | `hard_gate` when the guard is in release-gate lifecycle, otherwise `advisory`. |
| Benchmarks | `validation` | `benchmark` | `controlled_validation` | `advisory` for trend drift, `hard_gate` for configured budget breach. |
| Release proof packs | `governance` | `proof_pack` | `governance_verdict` | `hard_gate` on missing required artifact or failed release verdict. |
| Release scorecards | `governance` | `scorecard` | `governance_verdict` | `hard_gate`, `advisory`, or `diagnostic` per derived evidence row. |
| Sentinel top holes | `governance` | `runtime_finding` | `governance_verdict` | `advisory` for issue candidate, `hard_gate` only if tied to release-blocking evidence. |
| Sentinel RSI readiness | `governance` | `readiness_verdict` | `governance_verdict` | `hard_gate` for autonomous RSI, often `diagnostic` for missing trend/data. |

## Minimal Example

```json
{
  "type": "assurance_evidence",
  "schema_version": 1,
  "generated_at": "2026-04-29T22:00:00Z",
  "domain": "observability",
  "source_kind": "sentinel_stream",
  "source": {
    "id": "kernel_sentinel.runtime_observations",
    "owner": "core/layer0/kernel_sentinel",
    "artifact_path": "local/state/kernel_sentinel/evidence/runtime_observations.jsonl"
  },
  "authority_class": "deterministic_kernel_authority",
  "signal_class": "hard_gate",
  "subject": "gateway_startup_lifecycle",
  "status": "fail",
  "ok": false,
  "severity": "high",
  "summary": "Gateway reported startup success while health endpoint stayed unavailable.",
  "recommended_action": "Run bounded lifecycle diagnostic and repair process ownership source of truth.",
  "evidence": [
    "local/state/kernel_sentinel/evidence/runtime_observations.jsonl",
    "local/state/kernel_sentinel/evidence/gateway_health.jsonl"
  ],
  "freshness": {
    "observed": true,
    "age_seconds": 12,
    "stale": false,
    "stale_after_seconds": 5400
  },
  "fingerprint": "gateway_startup_lifecycle::health_unavailable_after_success"
}
```

## Compatibility Rule

Existing producer artifacts do not need to rewrite themselves immediately. During migration, each producer must either emit this envelope directly or provide a deterministic projection into this envelope.
