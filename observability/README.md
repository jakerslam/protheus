# Observability Domain

Owner: Assurance / Observability
Status: physical domain anchor

Observability owns live evidence: telemetry, health, traces, Sentinel evidence streams, runtime findings, evidence normalization, freshness, and source coverage.

Observability also owns the universal trace substrate. Fragmented observability, where each subsystem emits isolated local traces without a shared causal envelope, is an architecture smell and a system-understanding failure.

The trace substrate has a hard identity rule: one `trace_id` is minted at the initial user request and flows unchanged through every workflow, orchestration decision, tool call, Gateway/Conduit boundary, Kernel receipt, Validation span, Shell projection, Sentinel observation, and final response. Child work receives new `span_id` values, not new root trace IDs. No component may remint, replace, drop, or fork the trace ID.

Citation: `Machine Learning Systems, Volume 1`, Chapter 5 `AI Workflow`, frames production AI systems around workflow feedback loops, distributed monitoring, degradation prevention, and system-level behavior. That is the external reference for treating fragmented observability as a negative architecture state: https://mlsysbook.ai/vol1/assets/downloads/Machine-Learning-Systems-Vol1.pdf

Observability answers: "What is happening while the system runs?"

## Authority Boundary

Observability may define live evidence envelopes, health stream contracts, trace envelopes, trace source maps, runtime finding schemas, Sentinel source contracts, freshness rules, and source coverage metadata.

Observability must not own controlled eval scoring, release scorecard verdicts, Kernel policy truth, Orchestration planning, Shell readiness inference, or direct code mutation.

Kernel Sentinel is a privileged resident of Observability. It synthesizes findings and issue candidates from evidence, but controlled eval definitions still live in Validation.

## Subdomains

- `contracts/` live observability plane contracts and migrated observability SRS contract records.
- `telemetry/` live telemetry stream contracts and envelopes.
- `health/` health source contracts and status projection metadata.
- `traces/` runtime trace source maps and trace schemas.
- `sentinel/` Sentinel evidence stream contracts and resident observer metadata.
- `runtime_findings/` live finding schemas and issue-candidate source contracts.
- `evidence_normalization/` normalization rules from producers into Assurance envelopes.
- `freshness/` freshness budgets, stale-source policy, and timestamp contracts.
- `source_coverage/` required/optional source coverage definitions.

- `benchmarks/` observability benchmark outputs and runtime measurement reports.

- `dashboards/` dashboard-facing metrics specifications and display contracts.

- `reports/` durable observability reports and audits.

- `research/` observability research notes, findings, and archived investigations.

- `runbooks/` incident command, postmortem, and deployment-safety runbooks.

- `deploy/` Observability stack deployment defaults and compose fixtures.

## Migration Rule

Legacy Observability compatibility locations are retired. New live-evidence contracts, reports, runbooks, dashboard specs, and observability research should land under `observability/**`; compatibility mirrors should be declared only if new migration debt is explicitly introduced and given a burn-down path.
