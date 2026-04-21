# External Validation Evidence Cadence

Status: active  
Date: 2026-04-21

## Purpose

Keep public-facing validation evidence on a deterministic cadence so external trust signals do not drift behind runtime reality.

## Canonical Inputs

- Cadence plan: `client/runtime/config/external_validation_evidence_cadence.json`
- Workflow: `.github/workflows/external-validation-evidence-cadence.yml`
- Guard command: `npm run -s ops:external-validation:evidence:cadence`

## Required Outputs

- `core/local/artifacts/external_validation_evidence_cadence_current.json`
- `local/workspace/reports/EXTERNAL_VALIDATION_EVIDENCE_CADENCE_CURRENT.md`
- `core/local/artifacts/benchmark_public_audit_current.json` (from benchmark-public-audit lane)

## Operating Rule

The cadence workflow must stay scheduled weekly and publish the cadence artifact bundle even when no claim deltas are observed.
