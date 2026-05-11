# Kernel Sentinel Deterministic Evidence Refresh Policy

Kernel Sentinel problem discovery is only useful when it can distinguish live failure from stale historical churn.

## Required fresh evidence classes

A Sentinel run that gates RSI-readiness or release health should include fresh deterministic evidence for:

- Kernel receipt integrity.
- Sentinel final report budget and quality filter.
- Cadence/dream-state maintenance status.
- Observability source coverage.
- Governance/release verdict freshness.
- Open critical finding recurrence.

## Freshness rule

Evidence without a parseable timestamp, generated-at field, observed-at field, run id, or receipt id is advisory only. It must not be the sole basis for a release blocker.

## Stale evidence rule

Historical failures may remain useful for trend analysis, but they must be labeled as `stale_historical_evidence_failure` or equivalent before they can affect final reports.

## Current implementation note

The Sentinel evidence extractor now has stale-evidence handling for generated/observed/timestamp records. The next live confirmation requires a clean targeted Sentinel run; the current workspace contains unrelated compile/conflict blockers outside this policy lane, so this policy records the required refresh contract without claiming a new clean live run.
