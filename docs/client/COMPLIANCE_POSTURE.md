# Compliance Posture

## Purpose

`client/runtime/systems/ops/compliance_posture.js` provides a single operational score for deployment + controls readiness by aggregating:
- SOC2 evidence/readiness (`client/runtime/systems/ops/compliance_reports.js`)
- Integrity kernel state (`client/runtime/systems/security/integrity_kernel.js`)
- Startup attestation freshness (`client/runtime/systems/security/startup_attestation.js`)
- Deployment hardening gate (`client/runtime/systems/ops/deployment_packaging.js`)
- Contract surface stability (`client/runtime/systems/spine/contract_check.js`)

For framework-depth reporting (SOC2/ISO/NIST) and control inventory completeness, use:
`node client/runtime/systems/ops/compliance_reports.js framework-readiness --framework=all`
`node client/runtime/systems/ops/compliance_reports.js control-inventory`

## Commands

Run (non-blocking posture snapshot):

```bash
node client/runtime/systems/ops/compliance_posture.js run --days=30 --profile=prod --strict=0
```

Run strict gate (non-zero unless verdict is `pass`):

```bash
node client/runtime/systems/ops/compliance_posture.js run --days=30 --profile=prod --strict=1
```

Status:

```bash
node client/runtime/systems/ops/compliance_posture.js status latest
```

## Output

Artifacts are written to:
- `state/ops/compliance_posture/YYYY-MM-DD.json`
- `state/ops/compliance_posture/latest.json`
- `state/ops/compliance_posture/history.jsonl`

## Scoring

Score is weighted via `client/runtime/config/compliance_posture_policy.json`.

Default thresholds:
- `pass`: score >= 0.80
- `warn`: score >= 0.65 and < 0.80
- `fail`: score < 0.65

This is a posture signal, not legal certification. Use it to drive operational remediation before external audits.
