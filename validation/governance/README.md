# Validation governance

This subdomain owns Assurance Governance contracts: evidence-derived verdict inputs/outputs, advisory-to-hard-gate promotion rules, issue-candidate routing, retirement-candidate routing, and no-auto-mutation constraints.

Governance consumes Validation and Observability evidence. It may derive verdicts, scorecards, issue candidates, and retirement candidates, but it must not execute product work or apply patches.

Canonical contracts now live here:

- `contracts/assurance_governance_registry.json` defines verdict inputs, verdict outputs, scorecard derivation source rules, promotion rules, and issue-candidate routing.

The schema for this contract lives at `validation/schemas/assurance_governance_registry.schema.json`.
