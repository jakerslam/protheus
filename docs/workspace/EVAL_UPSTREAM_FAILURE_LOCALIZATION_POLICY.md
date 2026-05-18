# Eval Upstream Failure Localization Policy

## Purpose

This policy makes research-workflow improvement explicitly upstream-first.
When a run contains multiple failure signals, the system must localize the
**earliest broken layer** and treat that layer as the only authorized starting
point for fixes.

The goal is to prevent downstream tuning while an upstream layer is still
unstable.

## Canonical Layer Order

1. `run_stability`
2. `workflow_path`
3. `retrieval_mechanics`
4. `evidence_carrythrough`
5. `synthesis_quality`
6. `ux_smoke`
7. `none`

## Layer Meanings

### `run_stability`

Transport timeouts, missing agent/session state, request delivery failure,
dashboard/API availability issues, or recovery failures that prevent a usable
workflow payload from existing.

### `workflow_path`

Failures before or at tool execution:

- workflow gate 1-4 misses
- request-candidate creation failure
- candidate promotion failure
- tool execution never recorded

### `retrieval_mechanics`

Failures after execution began but before evidence is meaningfully usable:

- provider candidates absent
- provider surface degraded
- packaged result missing
- content-rich candidates missing
- claim extraction missing
- retrieval quality unusable / low relevance

### `evidence_carrythrough`

Retrieved evidence exists, but the final scored answer does not preserve or use
it correctly:

- citations/source refs lost
- evidence context not handed to synthesis
- recorded evidence not used

### `synthesis_quality`

Evidence and workflow are present, but the answer is still weak:

- synthesis contract miss
- bounded answer not produced
- scope/tradeoff/recommendation quality too weak

### `ux_smoke`

Non-authoritative “feels bad” lane. This layer exists to flag obviously
frustrating outputs for manual review, but it must not be treated as stronger
than the authoritative workflow/evidence contracts.

## Required Artifact Behavior

Each research eval case must emit:

- `upstream_failure_localization.earliest_failure_layer`
- `upstream_failure_localization.earliest_failure_boundary`
- `upstream_failure_localization.hardness`
- `upstream_failure_localization.authoritative_contract_failures`
- `upstream_failure_localization.soft_smoke_flags`

Each research eval summary must emit:

- `measurement_split.upstream_failure_localization.layer_counts`
- `measurement_split.upstream_failure_localization.boundary_counts`
- `measurement_split.upstream_failure_localization.top_layer`

## Operating Rule

When reviewing a run:

1. Group failures by `earliest_failure_layer`.
2. Select the highest upstream layer with recurring failures.
3. Fix only that layer until it is stable enough.
4. Re-run a focused subset first.
5. Re-run the full set only after the focused subset clears.

Do not optimize downstream layers while an upstream layer remains unstable.

## Notes

- `ux_smoke` is intentionally non-authoritative.
- Good metrics plus bad output means the measurement system is incomplete.
- Bad metrics plus good output means the measurement system may be overfitted or
  pointed at the wrong abstraction.
