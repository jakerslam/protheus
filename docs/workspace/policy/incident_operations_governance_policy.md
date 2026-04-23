# Incident Operations Governance Policy

## Purpose

Standardize incident response behavior across runtime, tooling, adapter, dashboard, and release lanes so severity handling and rollback decisions are auditable and deterministic.

## Hard Gates (Fail Closed)

These are release- and CI-enforced requirements:

1. Severity taxonomy must include `P0`, `P1`, `P2`, `P3`.
2. Ownership map must declare required incident roles and lane owners.
3. Owner roster must be real (no placeholder identities), role-complete, and active.
4. Escalation timing SLA must exist per severity with deterministic `ack`, `escalate`, and `update_cadence` values.
5. Rollback criteria must be explicit and machine-checkable.
6. Post-incident artifact requirements must be backed by schema contract validation.
7. Waivers must be explicit, approved, scoped to checks, and non-expired.

Authoritative config: [`client/runtime/config/incident_operations_governance_policy.json`](/Users/jay/.openclaw/workspace/client/runtime/config/incident_operations_governance_policy.json)

Supporting governance defaults:

- Owner roster: [`client/runtime/config/incident_owner_roster.json`](/Users/jay/.openclaw/workspace/client/runtime/config/incident_owner_roster.json)
- Artifact schema: [`client/runtime/config/post_incident_artifact_schema.json`](/Users/jay/.openclaw/workspace/client/runtime/config/post_incident_artifact_schema.json)
- Waiver register: [`client/runtime/config/incident_operations_governance_waivers.json`](/Users/jay/.openclaw/workspace/client/runtime/config/incident_operations_governance_waivers.json)

## Policy Layer (Required Practice)

Required non-gating contracts:

1. Incident communication templates.
2. Deployment/change checklist conventions.
3. Incident reporting format sections.
4. Script output field conventions for reliable troubleshooting handoffs.

## Process Layer (Required Documentation)

The process runbook must include all required lifecycle headings and remain synchronized with policy contracts.

- Process runbook: [`docs/workspace/process/incident_response_workflow.md`](/Users/jay/.openclaw/workspace/docs/workspace/process/incident_response_workflow.md)

## Suggestion Layer (Non-Blocking)

Recommended style conventions:

1. Keep updates concise and factual.
2. Keep user-facing messages plain-language.
3. Distinguish observed facts from assumptions.

## Enforcement

The policy is enforced by:

- Gate command: `npm run -s ops:incident-governance:gate`
- CI runner registration: `tests/tooling/config/tooling_gate_registry.json`
- Verify profile coverage: `tests/tooling/config/verify_profiles.json`
- Dedicated CI check: `incident-governance`
