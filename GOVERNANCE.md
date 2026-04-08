# Governance

This document defines the repository-level governance model for InfRing.

## Decision Process

- Major behavior changes require an RFC or equivalent design note in `docs/`.
- Security-impacting changes require explicit security review before merge.
- SRS contract changes must be accompanied by deterministic receipt-producing runtime lanes.
- Changes that alter operator/public command surfaces must include compatibility evidence.

## Code Review Policy

- All changes merge through pull requests.
- CI, contract, and receipt gates must pass before merge.
- Security plane and authority-lane changes require at least one security-focused review.
- Breaking changes require migration notes and operator-facing release notes.

## Release Controls

- Releases follow semantic versioning.
- Every release includes changelog entries and signed artifacts.
- Supply-chain evidence (SBOM/signing/provenance) must be attached to release outputs.
- Failed security or contract gates block release publication.

## Security Response

- Security findings are triaged by severity with fail-closed defaults for critical paths.
- Incident receipts and response actions are written to deterministic state artifacts.
- Patch timelines and remediation status are tracked in repository runbooks/reports.
- Emergency stop and containment controls remain available throughout incident handling.

## Compatibility & Deprecation

- Interface and skill-runtime compatibility must be explicitly validated.
- Major-version skill/runtime upgrades require migration evidence.
- Deprecation behavior must be documented with enforcement policy and operator guidance.

## Auditability

- Governance actions should produce machine-verifiable receipts where applicable.
- Historical evidence must remain queryable from `local/state/ops/*` paths.
- Contract/runtime drift is treated as a release blocker.
