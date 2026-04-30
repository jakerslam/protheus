# Assurance Governance Registry

Owner: Kernel / Assurance / Governance
Status: initial seed
Config: `tests/tooling/config/assurance_governance_registry.json`
Schema: `tests/tooling/schemas/assurance_governance_registry.schema.json`
Covers: `ASSURANCE-017` through `ASSURANCE-020`

## Purpose

Governance turns Validation and Observability evidence into confidence, scorecards, release verdicts, issue candidates, and retirement candidates.

Scorecard derivation rules used by active Validation guards are also projected into `validation/scorecards/contracts/assurance_scorecard_derivation_contract.json` so scorecard checks have a physical Validation-domain owner while the broader Governance registry awaits a physical Governance domain migration.

This means Governance is policy-complete but not yet physical-domain-complete. The active registry path under `tests/tooling/**` is a controlled compatibility location, not proof that tooling owns Governance. The migration status and debt markers are tracked in `docs/workspace/assurance_physical_domain_migration_status.md`.

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
