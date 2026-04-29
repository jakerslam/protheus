# Assurance Plane Execution Plan

Owner: Kernel / Assurance
Status: execution plan
Depends on: `docs/workspace/assurance_plane_policy.md`

## Goal

Create a first-class Assurance Plane that unifies controlled validation, live observability, and governance verdicts without polluting Kernel authority, Orchestration planning, or Shell presentation.

## Execution Waves

### Wave 1: Policy And Inventory

Outcome: existing tests, evals, benchmarks, guards, gates, scorecards, Sentinel outputs, and proof artifacts are inventoried and assigned to Validation, Observability, or Governance.

Tasks:

- `ASSURANCE-001` Inventory current controlled checks under tests/tooling, Rust tests, eval scripts, benchmarks, release gates, and scorecards.
- `ASSURANCE-002` Inventory current live observation producers, including Sentinel streams, telemetry, health, traces, proof packs, and runtime findings.
- `ASSURANCE-003` Classify every inventoried item as `validation`, `observability`, `governance`, or `harness_only`.
- `ASSURANCE-004` Flag misplaced ownership where Orchestration owns eval definitions, Kernel owns fuzzy eval interpretation, or Shell infers assurance truth.

### Wave 2: Evidence Contract

Outcome: Validation and Observability emit compatible evidence envelopes.

Tasks:

- `ASSURANCE-005` Define the shared Assurance evidence envelope schema for controlled and live evidence.
- `ASSURANCE-006` Map current Sentinel evidence rows to the Assurance envelope without losing existing authority classes.
- `ASSURANCE-007` Map current eval/report/proof-pack outputs to the Assurance envelope.
- `ASSURANCE-008` Add signal-class fields: `hard_gate`, `advisory`, and `diagnostic`.

### Wave 3: Validation Domain

Outcome: controlled proof has a clear owner and lifecycle.

Tasks:

- `ASSURANCE-009` Create the Validation registry for tests, evals, benchmarks, conformance guards, regression suites, and release-proof checks.
- `ASSURANCE-010` Add lifecycle state for checks: `experimental`, `advisory`, `release_gate`, `retirement_candidate`, `retired`.
- `ASSURANCE-011` Attach retirement criteria to scaffolding or temporary checks.
- `ASSURANCE-012` Separate harness-only tests from self-enforcing validation proofs.

### Wave 4: Observability Domain

Outcome: live system understanding is explicit and Sentinel is correctly positioned.

Tasks:

- `ASSURANCE-013` Define Observability source classes for telemetry, health, traces, runtime findings, and Sentinel evidence.
- `ASSURANCE-014` Reframe Kernel Sentinel as an Observability resident that synthesizes findings and issue candidates from evidence.
- `ASSURANCE-015` Add freshness and source-coverage reporting for every required Observability source.
- `ASSURANCE-016` Ensure Shell telemetry remains presentation-only unless corroborated by deterministic or control-plane evidence.

### Wave 5: Governance Domain

Outcome: confidence, scorecards, release gates, and issue candidates derive from evidence.

Tasks:

- `ASSURANCE-017` Define Governance verdict inputs and outputs.
- `ASSURANCE-018` Build scorecards from evidence references, not manually curated summaries.
- `ASSURANCE-019` Define promotion rules from advisory signal to hard gate by recurrence, corroboration, or policy.
- `ASSURANCE-020` Route repeated stable failure signatures into issue candidates with dedupe and thresholding.

### Wave 6: Consumers And Boundaries

Outcome: Orchestration, Shell, Gateway, and Kernel consume Assurance without owning it.

Tasks:

- `ASSURANCE-021` Let Orchestration consume Assurance results for planning and recovery without owning eval or gate definitions.
- `ASSURANCE-022` Let Shell display Assurance summaries and detail refs without inferring readiness or health.
- `ASSURANCE-023` Let Gateway expose bounded Assurance routes without deciding verdicts.
- `ASSURANCE-024` Keep Kernel deterministic by exposing facts, receipts, invariants, and hooks without absorbing fuzzy scoring.

### Wave 7: Enforcement

Outcome: the split becomes self-enforcing.

Tasks:

- `ASSURANCE-025` Add a placement guard that flags eval/gate/scorecard ownership outside the Assurance Plane unless marked harness-only.
- `ASSURANCE-026` Add an envelope guard that rejects Assurance artifacts missing authority class, signal class, evidence refs, or freshness metadata.
- `ASSURANCE-027` Add a scorecard derivation guard proving scorecards point back to evidence artifacts.
- `ASSURANCE-028` Add a Shell truth-leak guard for Assurance state.

## Initial Success Criteria

The first implementation milestone is complete when:

- the repo has an Assurance inventory;
- all controlled checks and live observation streams have a domain classification;
- Sentinel is documented and wired as Observability, not the whole eval system;
- scorecards and release gates point to evidence artifacts;
- Shell displays Assurance state only by projection/detail ref;
- Orchestration consumes Assurance results without owning eval definitions or gates.

## Non-Goals

This plan does not authorize automatic code mutation.

This plan does not move all tests out of `tests/**`; harnesses may stay there.

This plan does not make Sentinel the owner of all validation.

This plan does not make scorecards canonical truth independent of evidence.
