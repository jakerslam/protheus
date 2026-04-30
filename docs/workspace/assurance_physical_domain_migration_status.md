# Assurance Physical Domain Migration Status

Owner: Kernel / Assurance
Status: canonical migration-status document
Updated: 2026-04-29
Depends on: `docs/workspace/assurance_plane_policy.md`

## Purpose

This document prevents a subtle but dangerous confusion: policy-complete Assurance is not the same thing as physical-complete Assurance.

The Assurance Plane policy defines the target ownership model. The physical-domain migration proves where the repo now stores the canonical definitions, which compatibility mirrors still exist, and which support paths remain harness-only.

## Status Rule

Policy-complete is not physical-complete.

A domain is `policy_complete` when ownership rules, authority boundaries, and consumer rules are defined.

A domain is `physically_migrated` when canonical machine-readable definitions live under the owning repo domain and active commands consume those definitions from that domain.

A domain is `compatibility_debt` when legacy paths, wrappers, or mirrors still exist to keep older commands and reports stable while callers migrate.

## Domain Status

| Domain | Policy status | Physical status | Canonical root | Notes |
|---|---|---|---|---|
| Validation | `policy_complete` | `physically_migrated_with_debt` | `validation/**` | Eval definitions, release gates, scorecard contracts, benchmark budgets, regression policies, conformance contracts, schemas, and controlled-proof definitions now have Validation-owned roots. |
| Observability | `policy_complete` | `physically_migrated_with_debt` | `observability/**` | Live evidence envelopes, source coverage, freshness, health, traces, runtime findings, and Sentinel resident-observer contracts now have Observability-owned roots. |
| Governance | `policy_complete` | `physical_domain_pending` | pending | The active Governance registry still lives under `tests/tooling/config/assurance_governance_registry.json` as a controlled harness/compatibility location. Scorecard derivation rules used by guards are already projected into `validation/scorecards/contracts/assurance_scorecard_derivation_contract.json`. |
| Harnesses | `support_only` | `not_an_owning_domain` | `tests/**` | Tooling scripts, CI launchers, fixtures, and gate registries may execute or verify Assurance work, but they are not owners of eval, release-gate, benchmark, telemetry, or Sentinel source definitions. |

## Canonical Definition Roots

Validation-owned definitions currently include:

- `validation/evals/**`
- `validation/release_gates/**`
- `validation/scorecards/**`
- `validation/benchmarks/**`
- `validation/regression/**`
- `validation/conformance/**`
- `validation/schemas/**`

Observability-owned definitions currently include:

- `observability/source_coverage/**`
- `observability/evidence_normalization/**`
- `observability/freshness/**`
- `observability/health/**`
- `observability/traces/**`
- `observability/runtime_findings/**`
- `observability/sentinel/**`

## Compatibility Mirrors

Compatibility mirrors are migration debt markers, not alternate owners.

The active mirror registries are:

- `validation/evals/compatibility_mirrors.json`
- `validation/release_gates/compatibility_mirrors.json`
- `validation/scorecards/compatibility_mirrors.json`
- `validation/benchmarks/compatibility_mirrors.json`
- `validation/regression/compatibility_mirrors.json`
- `validation/conformance/compatibility_mirrors.json`
- `observability/compatibility_mirrors.json`

Each mirror must point from an old consumer-visible path to a canonical Validation or Observability definition. Mirrors exist to preserve command compatibility while callers migrate inward.

## Physical-Domain Placement Guard

The physical-domain placement guard is:

```bash
npm run -s ops:assurance:physical-domain-placement:guard
```

The guard fails when new definition-shaped eval, scorecard, release-gate, benchmark, telemetry, or Sentinel source-registry files appear outside `validation/**` or `observability/**` without an explicit compatibility mirror or harness-only exemption.

The time-bounded exemption registry is:

- `validation/conformance/contracts/assurance_physical_domain_placement_exemptions.json`

Every exemption must carry owner, reason, and expiry. Expired exemptions are failures, not warnings.

## Consumer Status

Consumers may trigger, display, or route Assurance results, but they must not own Assurance definitions.

| Consumer | Allowed role | Forbidden role |
|---|---|---|
| Orchestration | Trigger controlled checks and consume Validation/Observability/Governance results for planning context. | Own eval definitions, release-gate truth, scorecard truth, or readiness verdicts. |
| Kernel | Provide deterministic facts, receipts, invariants, and safe hooks that Assurance inspects. | Absorb fuzzy eval interpretation or become a scorecard engine. |
| Shell | Display bounded Assurance projections and detail refs. | Infer health, readiness, release truth, or gate waivers. |
| Gateway | Expose bounded Assurance ingress/egress routes. | Decide Assurance verdicts or rewrite signal authority. |
| Tests/tooling | Execute harnesses and guards. | Become the canonical owner of Assurance definitions unless explicitly marked harness-only. |

## Closure Criteria

The physical migration can be considered complete only when:

1. Validation and Observability required directories and manifests exist.
2. Canonical registries and schemas live under `validation/**` or `observability/**`.
3. Active package scripts and tooling registry rows point at canonical definition paths.
4. Compatibility mirrors point inward to canonical definitions.
5. Consumers no longer own definitions they only consume.
6. Old scattered locations have explicit burn-down rows or time-bounded exemptions.
7. The physical-domain placement guard and aggregate Assurance governance pass.

## One-Line Rule

Assurance policy says who owns confidence. Physical migration proves the repo actually stores that confidence in the owning domains.
