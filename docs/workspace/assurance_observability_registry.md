# Assurance Observability Registry

Owner: Kernel / Assurance / Observability
Status: initial seed
Config: `observability/source_coverage/assurance_observability_registry.json`
Schema: `observability/source_coverage/assurance_observability_registry.schema.json`
Covers: `ASSURANCE-013` through `ASSURANCE-016`

## Purpose

The Observability registry is the Assurance-owned index of live system evidence.

It defines the live source classes that Sentinel and Governance may consume, their authority classes, freshness requirements, coverage requirements, and whether each source can open findings or block release.

## Sentinel Position

Kernel Sentinel is a privileged Observability resident.

Sentinel owns:

- evidence normalization from live producers;
- source freshness and coverage reporting;
- finding synthesis;
- architectural incident synthesis;
- issue-candidate evidence preparation;
- self-study outputs and RSI readiness blockers.

Sentinel does not own:

- controlled eval definitions;
- benchmark definitions;
- regression-suite definitions;
- scorecard truth independent of evidence;
- direct code mutation;
- recursive ingestion of its own self-study outputs as primary evidence.

## Source Classes

| Source class | Authority | Can open finding | Can block release | Notes |
|---|---|---:|---:|---|
| `kernel_receipt_stream` | `deterministic_kernel_authority` | yes | yes | Receipt integrity and mutation coverage. |
| `runtime_observation_stream` | `deterministic_kernel_authority` | yes | yes | Runtime correctness, workflow phase, and state-machine evidence. |
| `state_mutation_stream` | `deterministic_kernel_authority` | yes | yes | Legal transitions, rollback, and mutation receipts. |
| `scheduler_admission_stream` | `deterministic_kernel_authority` | yes | yes | Admission, capability, scheduler, and fail-closed checks. |
| `recovery_stream` | `deterministic_kernel_authority` | yes | yes | Recovery loops, retry storms, rollback gaps. |
| `boundedness_stream` | `deterministic_kernel_authority` | yes | yes | Resource ceilings and boundedness regressions. |
| `release_proof_stream` | `deterministic_kernel_authority` | yes | yes | Proof-pack and release repair evidence. |
| `gateway_boundary_stream` | `deterministic_kernel_authority` | yes | yes | Gateway health, quarantine, recovery, and isolation. |
| `queue_pressure_stream` | `deterministic_kernel_authority` | yes | yes | Queue/backpressure and pressure-policy behavior. |
| `advisory_eval_stream` | `advisory_workflow_quality` | yes | no | Eval feedback can open advisory findings but cannot block release directly. |
| `presentation_telemetry_stream` | `presentation_telemetry_only` | no | no | Shell/operator context only unless corroborated. |
| `sentinel_self_study_output` | `deterministic_kernel_authority` | no | no | Output for Governance/operator review, not recursive primary evidence. |

## Freshness And Coverage

Required Observability sources must publish freshness metadata. Missing freshness is a diagnostic gap.

Default thresholds:

```json
{
  "default_stale_after_seconds": 5400,
  "release_stale_after_seconds": 1800
}
```

Required sources that are stale in release context may become `hard_gate` through Governance. Optional sources, including Shell telemetry, remain advisory when stale or absent.

## Shell Telemetry Corroboration Rule

Shell telemetry is valuable because it records what the operator saw. It is not canonical truth.

`presentation_telemetry_stream` entries:

- cannot open findings by themselves;
- cannot block release by themselves;
- cannot waive findings;
- cannot write Sentinel verdicts;
- can corroborate deterministic or advisory evidence;
- can improve issue summaries by preserving user-visible symptoms.

## Required Coverage Seeds

The initial required source coverage is:

- Kernel receipts;
- runtime observations;
- state mutations;
- scheduler/admission;
- live recovery;
- boundedness;
- release proof packs;
- gateway boundaries;
- queue pressure;
- advisory control-plane eval.

Shell telemetry and Sentinel self-study outputs are intentionally not required primary sources.

## Physical Domain Migration

The Observability registry and schema now live in the physical Observability domain. The older `tests/tooling/**` copies are compatibility mirrors only and are declared in `observability/compatibility_mirrors.json`.

Related Observability-owned contracts:

- `observability/freshness/evidence_freshness_policy.json`
- `observability/health/health_stream_contract.json`
- `observability/traces/sentinel_trace_source_map.json`
- `observability/runtime_findings/runtime_finding.schema.json`
- `observability/evidence_normalization/assurance_evidence_envelope.schema.json`
- `observability/sentinel/sentinel_resident_observer_contract.json`
