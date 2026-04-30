# Observability Domain

Owner: Assurance / Observability
Status: physical domain anchor

Observability owns live evidence: telemetry, health, traces, Sentinel evidence streams, runtime findings, evidence normalization, freshness, and source coverage.

Observability answers: "What is happening while the system runs?"

## Authority Boundary

Observability may define live evidence envelopes, health stream contracts, trace source maps, runtime finding schemas, Sentinel source contracts, freshness rules, and source coverage metadata.

Observability must not own controlled eval scoring, release scorecard verdicts, Kernel policy truth, Orchestration planning, Shell readiness inference, or direct code mutation.

Kernel Sentinel is a privileged resident of Observability. It synthesizes findings and issue candidates from evidence, but controlled eval definitions still live in Validation.

## Subdomains

- `telemetry/` live telemetry stream contracts and envelopes.
- `health/` health source contracts and status projection metadata.
- `traces/` runtime trace source maps and trace schemas.
- `sentinel/` Sentinel evidence stream contracts and resident observer metadata.
- `runtime_findings/` live finding schemas and issue-candidate source contracts.
- `evidence_normalization/` normalization rules from producers into Assurance envelopes.
- `freshness/` freshness budgets, stale-source policy, and timestamp contracts.
- `source_coverage/` required/optional source coverage definitions.

## Migration Rule

Existing Sentinel and telemetry code may keep compatibility locations while migration is active, but new live-evidence contracts should land here unless explicitly marked compatibility debt. Compatibility mirrors are declared in `observability/compatibility_mirrors.json`.
