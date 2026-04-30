# Assurance Consumer Boundary Contract

Owner: Kernel / Assurance
Status: initial seed
Config: `validation/conformance/contracts/assurance_consumer_boundary_contract.json`
Schema: `validation/schemas/assurance_consumer_boundary_contract.schema.json`
Covers: `ASSURANCE-021` through `ASSURANCE-024`

## Purpose

This contract defines how other domains consume Assurance without becoming Assurance.

The older `tests/tooling/**` consumer-boundary paths are compatibility mirrors only. Active Assurance guards should read the Validation-owned contract.

The guiding rule is simple: consumers may use Assurance signals, but they must not recreate Assurance verdicts, own Assurance definitions, or mutate the system from Assurance output.

## Global Rules

- Assurance owns evidence truth and verdict derivation.
- Consumers may cache bounded projections only.
- Consumers must reference Assurance artifacts when acting on Assurance signals.
- Consumers must not recompute release, health, readiness, or scorecard verdicts.
- Consumers must not auto-apply patches from Assurance signals.
- Raw Assurance detail must be fetched by detail ref, not bundled by default.

## Consumer Rules

| Consumer | Allowed | Forbidden |
|---|---|---|
| Orchestration | Trigger checks, consume summaries, adapt plans, request bounded diagnostics. | Owning eval definitions, release gates, scorecard truth, waivers, evidence rewrites, auto-patches. |
| Shell | Display summaries, signal class, freshness, detail refs, review controls. | Inferring health/readiness, branching on raw payloads, caching full evidence, waiving gates. |
| Gateway | Expose bounded summary/detail routes and approved submissions. | Deciding verdicts, downgrading signal class, expanding raw evidence by default, mutating artifacts. |
| Kernel | Expose deterministic facts, receipts, invariants, and authorized hooks. | Owning fuzzy scoring, adaptive eval interpretation, replacing Governance verdicts, treating Shell telemetry as truth. |

## Orchestration Consumption

Orchestration can use Assurance to choose safer plans, request clarification, trigger diagnostics, or avoid known-bad routes.

It cannot own eval scoring, release gates, or scorecard truth. If Orchestration needs a new check, it requests or registers it through Validation/Governance rather than embedding the judge in planner logic.

## Shell Consumption

Shell displays Assurance state as projection.

Allowed Shell payloads are summaries, statuses, signal classes, freshness, and stable detail refs. Shell must not infer readiness or health from raw evidence and must not keep full Assurance artifacts in presentation caches.

## Gateway Consumption

Gateway exposes Assurance boundaries but does not decide Assurance truth.

Gateway must keep Assurance routes bounded, audited, detail-ref based, and compatible with interface payload budgets and Conduit/Scrambler posture rules.

## Kernel Consumption

Kernel remains deterministic.

Kernel exposes facts, receipts, invariants, and authorized diagnostic hooks that Assurance can observe. Kernel must not absorb fuzzy eval interpretation or adaptive scoring. Shell telemetry cannot become Kernel truth.
