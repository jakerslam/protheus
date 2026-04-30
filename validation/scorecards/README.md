# Validation scorecards

This subdomain owns evidence-derived scorecard definitions and derivation contracts.

Canonical scorecard contracts now live here:

- `contracts/release_scorecard_contract.json` defines the release scorecard as a derived Validation artifact, lists required source domains, and forbids scorecards from introducing independent truth.
- `contracts/assurance_scorecard_derivation_contract.json` defines evidence-backed scorecard derivation rules used by active Assurance guards. Broader Governance verdict, promotion, and issue-candidate contracts live under `validation/governance/**`.

Generated scorecard outputs may still be written to runtime-local state for compatibility, but the derivation contract belongs to Validation.
