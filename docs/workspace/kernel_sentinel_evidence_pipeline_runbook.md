# Kernel Sentinel Evidence Pipeline Runbook

Owner: core/layer0/kernel_sentinel
Status: in-progress operator runbook
Updated: 2026-04-26

## What the pipeline does

Kernel Sentinel is the Kernel-side self-study mechanism. It should watch deterministic runtime evidence first, use control-plane eval as advisory input second, and produce issue/suggestion/automation candidates only after evidence is normalized.

The pipeline has four steps:

1. Existing producers write local telemetry.
2. `kernel-sentinel collect` bridges that telemetry into `local/state/kernel_sentinel/evidence/*.jsonl`.
3. `kernel-sentinel run` or `kernel-sentinel auto` ingests those streams and writes Sentinel reports.
4. Self-study outputs turn open findings into feedback inbox rows, top holes, daily report, and RSI readiness state.

## Important paths

| Path | Purpose |
|---|---|
| `docs/workspace/kernel_sentinel_evidence_source_map.md` | Source-to-stream inventory. |
| `docs/workspace/kernel_sentinel_evidence_schema_contracts.md` | Evidence row contracts and stream requirements. |
| `local/state/kernel_sentinel/evidence/*.jsonl` | Canonical Sentinel evidence inputs. |
| `core/local/artifacts/kernel_sentinel_collector_current.json` | Collector run report. |
| `local/state/kernel_sentinel/kernel_sentinel_report_current.json` | Main Sentinel report. |
| `local/state/kernel_sentinel/kernel_sentinel_verdict.json` | Sentinel verdict. |
| `local/state/kernel_sentinel/rsi_readiness_summary_current.json` | RSI readiness summary. |
| `local/state/kernel_sentinel/feedback_inbox.jsonl` | Deduped Sentinel feedback items. |
| `local/state/kernel_sentinel/top_system_holes_current.json` | Highest-priority Sentinel holes and issue candidates. |

## Operator commands

Collect evidence:

```bash
infring-ops kernel-sentinel collect
```

Run Sentinel once:

```bash
infring-ops kernel-sentinel run --strict=1
```

Run automatic Sentinel maintenance:

```bash
infring-ops kernel-sentinel auto --cadence=maintenance
```

Check current verdict:

```bash
infring-ops kernel-sentinel status --strict=1
```

## Reading evidence state

| State | Meaning | First repair |
|---|---|---|
| `data_starved` | No normalized evidence records were ingested. | Run/fix `kernel-sentinel collect`; add bridges for missing producers. |
| `partial_evidence` | Some streams exist, but expected streams are missing. | Check collector source reports and wire missing stream families. |
| `malformed_evidence` | At least one evidence row failed parsing or schema normalization. | Fix the producer or bridge row shape before trusting readiness. |
| `stale_evidence` | At least one normalized row is older than the configured freshness threshold. | Refresh the stale producer and rerun collection before trusting readiness. |
| `healthy_observation` | Evidence exists and expected streams are covered. | Review findings, trends, and readiness blockers. |

`missing_required_source_count` is the count that matters for required observation coverage. `missing_optional_source_count` is still useful, but it is reserved for presentation-only context such as Shell telemetry and should not block required evidence health by itself.

When the state is `malformed_evidence`, start with `evidence_ingestion.malformed_by_file_name`, then inspect `evidence_ingestion.malformed_by_path`. Do not treat a malformed stream as missing data; it means a producer exists but is writing bad rows.

When the state is `stale_evidence`, compare `evidence_ingestion.freshness_observed_record_count`, `evidence_ingestion.stale_record_count`, `evidence_ingestion.stale_evidence_seconds`, and `evidence_ingestion.max_evidence_age_seconds` before patching the producer. This separates genuinely stale evidence from records that simply do not publish age metadata yet.

When the state is `stale_evidence`, inspect `evidence_ingestion.max_evidence_age_seconds` and `evidence_ingestion.stale_record_count`, then refresh the producer that wrote the old source artifact.

## Authority rules

Deterministic Kernel evidence can open release-blocking findings. This includes receipt, runtime, state mutation, scheduler admission, recovery, boundedness, release proof, gateway, and queue evidence.

Control-plane eval evidence is advisory. It can open findings and issues, but it cannot write Sentinel verdicts, waive findings, or directly block release as Kernel authority.

Shell telemetry must remain presentation-only. It can help explain what users saw, but it cannot become canonical truth.

Shell telemetry rows are useful when Sentinel needs to explain what an operator saw, such as stale thinking text, missing workflow visibility, or dashboard drift. They should be bridged into `shell_telemetry.jsonl`, but their authority class is `presentation_telemetry_only`; they cannot open Sentinel findings unless deterministic Kernel or control-plane evidence corroborates the failure.

## RSI readiness requirements

Autonomous RSI readiness must remain blocked until all conditions are true:

1. Sentinel has nonzero evidence records.
2. Sentinel has at least one deterministic Kernel evidence record.
3. Evidence is not malformed.
4. Trend history has enough automatic runs.
5. Release gate passes.
6. Active regressions have operator review or issue/waiver routing.

## If Sentinel says healthy but no issues appear

Check these in order:

1. `local/state/kernel_sentinel/kernel_sentinel_report_current.json`
2. `evidence_ingestion.normalized_record_count`
3. `evidence_ingestion.observation_state`
4. `evidence_ingestion.sources[*].present`
5. `core/local/artifacts/kernel_sentinel_collector_current.json`
6. `records_read`, `records_written`, and `malformed_record_count`
7. `local/state/kernel_sentinel/feedback_inbox.jsonl`
8. `local/state/kernel_sentinel/top_system_holes_current.json`

If `normalized_record_count` is zero, the problem is collector wiring or missing producer data.

If records exist but findings are empty, the Sentinel is observing but did not see failure-triggering fields such as `ok=false`, `status=failed`, explicit `severity`, failed trajectory phases, stale freshness, or hard-fail invariant details.

## What to patch next

Patch missing streams in this order:

1. Verity receipts to `kernel_receipts.jsonl`.
2. Runtime/system health to `runtime_observations.jsonl`.
3. Eval feedback to `control_plane_eval.jsonl`.
4. Release proof packs to `release_proof_packs.jsonl`.
5. Gateway quarantine/recovery/isolation streams.
6. Queue and boundedness streams.
7. Shell telemetry as observation-only context.
