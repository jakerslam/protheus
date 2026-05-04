# Assurance Plane Policy

Owner: Kernel / Assurance
Status: canonical architecture policy
Applies to: tests, evals, benchmarks, conformance guards, regression suites, release gates, scorecards, Sentinel observations, runtime findings, proof packs, readiness verdicts

## Purpose

InfRing needs a first-class place for system confidence.

The Assurance Plane is that place. It does not execute the product workflow, plan the next user action, or render the Shell. It proves, observes, scores, gates, and explains whether the system is behaving correctly.

This policy prevents tests, evals, benchmarks, conformance checks, release gates, scorecards, and Sentinel outputs from becoming scattered support artifacts with unclear ownership.

## Boundary Axiom

Kernel decides what is true and allowed.

Orchestration decides what should happen next.

Gateway protects external boundaries.

Conduit carries internal cross-domain delivery.

Shell shows and collects.

Assurance proves, observes, scores, gates, and explains the work.

## Plane Shape

Assurance is an umbrella with three sibling domains.

The policy is complete when these ownership boundaries are defined. The physical repo migration is tracked separately in `docs/workspace/assurance_physical_domain_migration_status.md`, because a domain can be policy-complete while compatibility mirrors and harness-only support paths still exist.

### Validation

Validation answers: "Does this system behave correctly under controlled checks?"

Validation owns:

- test definitions and controlled fixtures;
- eval definitions and scoring rubrics;
- benchmark definitions and performance budgets;
- conformance guards;
- regression suites;
- controlled replay scenarios;
- proof artifacts produced by controlled runs.

Validation must not:

- own runtime planning;
- mutate production state as part of ordinary scoring;
- replace Kernel policy;
- hide evidence behind hand-written summaries.

### Observability

Observability answers: "What is happening while the system runs?"

Observability owns:

- telemetry envelopes;
- health signals;
- runtime traces;
- the universal trace substrate and cross-domain correlation fabric;
- runtime findings;
- trend samples;
- Sentinel evidence streams;
- source freshness and coverage state.

Fragmented observability is prohibited as a target architecture. It is the negative state where local subsystem traces exist but cannot be joined into a cross-domain causal graph. The canonical policy is `docs/workspace/universal_trace_substrate_policy.md`, with the machine-readable root envelope under `observability/traces/trace_envelope.schema.json`.

Every causal story starts with exactly one `trace_id` minted at the initial user request. That `trace_id` must flow unchanged through workflow, Orchestration, tool, Gateway/Conduit, Kernel, Validation, Shell, Sentinel, and final-response spans. Domains may create local spans and typed extension payloads, but they must not create alternate root trace identities, drop the ID, fork it, or replace it for the same request. No exceptions.

Kernel Sentinel is a privileged resident of Observability. Sentinel watches runtime, Kernel, Gateway, Orchestration, and Shell evidence, then synthesizes findings, architectural incidents, issue candidates, self-understanding artifacts, and RSI readiness blockers.

Sentinel is not the whole eval system. Controlled evals live in Validation. Sentinel may consume eval evidence as advisory or corroborating input according to authority class.

### Governance

Governance answers: "Given the evidence, what confidence, verdict, or next action should exist?"

Governance owns:

- release gates;
- scorecards derived from evidence;
- readiness verdicts;
- issue-candidate thresholds and dedupe;
- hard/advisory/diagnostic signal classification;
- lifecycle state for checks and gates;
- trend deltas and regression posture.

Governance must derive scorecards and verdicts from evidence artifacts. Scorecards are summaries, not sources of truth.

## Signal Classes

Every Assurance output must declare one of these classes:

| Class | Meaning | Example |
|---|---|---|
| `hard_gate` | Blocks release, promotion, or unsafe operation until resolved or waived through policy. | Required proof artifact missing. |
| `advisory` | Useful signal that should influence planning or review but does not block by itself. | Eval quality degradation without deterministic corroboration. |
| `diagnostic` | Requests or records more evidence before judgment. | Sentinel asks for a bounded topology probe. |

Hard gates require deterministic evidence or an explicit policy rule. Advisory signals may become hard only after recurrence, corroboration, or configured promotion.

## Lifecycle For Checks

Checks, evals, benchmarks, guards, and gates should have a lifecycle:

```text
experimental -> advisory -> release_gate -> retirement_candidate -> retired
```

Temporary scaffolding checks must include retirement criteria. A check may become a retirement candidate when the underlying system mechanism proves stable over a declared sample budget.

## Authority Rules

Assurance can:

- observe live behavior;
- run controlled validation;
- emit verdicts and scorecards;
- block release through hard gates;
- file or propose issue candidates;
- request bounded diagnostics;
- inform Orchestration planning and recovery.

Assurance must not:

- silently apply patches;
- directly mutate production state;
- become a second Kernel policy engine;
- become a second Orchestration planner;
- let Shell telemetry become canonical truth;
- treat one-off warnings as release blockers without policy.

Future self-modification must pass through a separate propose -> validate -> apply -> monitor -> rollback pipeline. Assurance supplies evidence and verdicts to that pipeline; it does not bypass it.

## Placement Rules

Authoritative Assurance runtime logic belongs in Rust under `core/**`.

Controlled harnesses, fixture runners, and CI wrappers may live under `tests/**`.

Orchestration may trigger or consume Assurance results, but must not own eval definitions, release gates, or scorecard truth.

Shell may display Assurance summaries, detail refs, and operator controls, but must not infer health, readiness, or release truth.

Gateway may expose bounded Assurance ingress/egress routes, but must not decide Assurance verdicts.

## Evidence Envelope Requirements

Validation and Observability outputs should converge on compatible evidence envelopes with:

- stable `type`;
- `generated_at`;
- `source`;
- `authority_class`;
- `subject`;
- `ok` or verdict field;
- severity or signal class;
- evidence references;
- receipt or artifact hash where applicable;
- freshness metadata;
- dedupe or fingerprint key when the output can become an issue candidate.

Governance must be able to compare controlled validation evidence and live observability evidence without custom one-off glue for each source.

## Relationship To System Understanding

Assurance is the evidence spine for system understanding.

For external assimilation, Validation and Observability gather the target's behavior, pressure response, conformance shape, and failure model.

For internal RSI, Assurance gathers InfRing's live behavior, controlled proofs, architectural incidents, trends, and readiness blockers.

Both directions should feed the System Understanding Dossier before implementation begins.

## One-Line Rule

Assurance does not do the work. Assurance proves, observes, scores, gates, and explains the work.

## Canonical Evidence Contract

The shared Assurance evidence envelope is defined in `docs/workspace/assurance_evidence_envelope_contract.md` and machine-readable schema `observability/evidence_normalization/assurance_evidence_envelope.schema.json`.

All new Validation, Observability, and Governance artifacts should either emit that envelope directly or provide a deterministic projection into it.

## Validation Registry

The Validation registry lives at `validation/conformance/contracts/assurance_validation_registry.json`, with human-readable guidance in `docs/workspace/assurance_validation_registry.md`.

The registry distinguishes self-enforcing controlled proof from harness-only execution plumbing, assigns lifecycle state, and requires retirement criteria for scaffolding or temporary monitors.

## Observability Registry

The Observability registry lives at `observability/source_coverage/assurance_observability_registry.json`, with human-readable guidance in `docs/workspace/assurance_observability_registry.md`.

The registry defines live source classes, authority classes, default signal classes, source freshness requirements, source coverage requirements, Sentinel's privileged Observability role, and the rule that Shell telemetry remains presentation-only unless corroborated.

The universal trace substrate lives under `observability/traces/**`. Domains may provide typed extension payloads, but Observability owns the root trace envelope, trace IDs, span IDs, parent-span relationships, authority class, subject identity, and correlation rules.

## Governance Registry

The Governance registry lives at `validation/governance/contracts/assurance_governance_registry.json`, with schema `validation/schemas/assurance_governance_registry.schema.json` and human-readable guidance in `docs/workspace/assurance_governance_registry.md`.

The registry defines verdict inputs, verdict outputs, scorecard derivation rules, advisory-to-hard-gate promotion rules, and repeated-failure issue-candidate routing. Governance may block release or produce issue candidates from evidence, but it may not auto-apply patches.

Governance is physically owned under `validation/governance/**`. Tooling remains harness-only: it may run Governance checks, but it must not own Governance verdict, promotion, or issue-candidate definitions.

## Consumer Boundary Contract

The consumer boundary contract lives at `validation/conformance/contracts/assurance_consumer_boundary_contract.json`, with human-readable guidance in `docs/workspace/assurance_consumer_boundary_contract.md`.

The physical-domain placement guard is `ops:assurance:physical-domain-placement:guard`. It fails when definition-shaped eval, scorecard, release-gate, benchmark, telemetry, or Sentinel source-registry files appear outside `validation/**` or `observability/**` without an explicit compatibility mirror or harness-only exemption.

The contract defines how Orchestration, Shell, Gateway, and Kernel consume Assurance without owning Assurance. Consumers may use bounded Assurance projections and detail refs, but they must not recompute verdicts, waive gates, or auto-apply patches.

The migration-debt marker for compatibility wrappers is an explicit `compatibility_mirrors.json` file under the owning `validation/**` or `observability/**` subdomain whenever active mirror debt exists, plus the time-bounded exemption registry at `validation/conformance/contracts/assurance_physical_domain_placement_exemptions.json`.
