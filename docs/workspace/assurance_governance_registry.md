# Assurance Governance Registry

Owner: Validation / Governance
Status: physical domain owner
Config: `validation/governance/contracts/assurance_governance_registry.json`
Schema: `validation/schemas/assurance_governance_registry.schema.json`
Covers: `ASSURANCE-017` through `ASSURANCE-020`

## Purpose

Governance turns Validation and Observability evidence into confidence, scorecards, release verdicts, issue candidates, and retirement candidates.

Scorecard derivation rules used by active Validation guards remain projected into `validation/scorecards/contracts/assurance_scorecard_derivation_contract.json`, while the full Governance registry now has a physical Validation-owned home under `validation/governance/**`.

This means Governance is no longer a `tests/tooling/**` compatibility definition. Tooling may execute the guards, but the canonical verdict, promotion, and issue-candidate routing contract belongs to Validation Governance.

Governance does not execute product work or apply patches. It derives verdicts from evidence and routes next actions.

## Verdict Inputs

Governance consumes:

- Validation release gates and proof-pack outputs;
- required Observability sources and Sentinel evidence streams;
- Sentinel self-study outputs;
- eval quality thresholds and regression guard outputs.

All inputs must carry evidence references. Missing, stale, malformed, or partial evidence becomes a diagnostic signal instead of disappearing.

## Verdict Outputs

Governance may emit:

- release verdicts;
- release scorecards;
- issue candidates;
- retirement candidates for temporary scaffolding.

No Governance output may auto-apply a patch.

## Scorecard Derivation Rule

Scorecards are summaries, not sources of truth.

Every scorecard row must reference source evidence. Scorecards must expose stale, missing, malformed, or partial evidence as diagnostic rows rather than hiding uncertainty behind an aggregate pass/fail.

## Promotion Rule

Advisory signals can become hard gates only through Governance.

Promotion requires at least one of:

- recurrence;
- deterministic corroboration;
- an explicit release policy rule.

Shell telemetry cannot promote itself. It must be corroborated by deterministic or advisory evidence before it can contribute to a hard gate.

## Issue Candidate Routing

Issue candidates require:

- stable fingerprint or dedupe key;
- evidence references;
- repeated or release-significant failure signature;
- `safe_to_auto_file_issue: true` when filing is allowed;
- `safe_to_auto_apply_patch: false`;
- `human_review_required: true`;
- `autonomous_mitigation_allowed: false`.

This keeps Assurance strong enough to surface high-quality failures without crossing into autonomous mutation.
