# Organization Onboarding

This guide defines the canonical first-run sequence and role handoff for operator and contributor onboarding.

## Canonical First-Run Sequence (All Roles)

1. Install full runtime:
   - `curl -fsSL https://raw.githubusercontent.com/protheuslabs/InfRing/main/install.sh | sh -s -- --full`
2. Complete setup:
   - `infring setup --yes --defaults`
   - `infring setup status --json`
3. Start and verify gateway:
   - `infring gateway`
   - `infring gateway status`

Windows equivalent (PowerShell) follows the same install -> setup -> gateway sequence.

## Wrapper Contract

- Canonical wrappers: `infring`, `infringctl`, `infringd`
- Legacy aliases (`protheus`, `protheusctl`, `protheusd`) are deprecated compatibility shims only.

## Role Handoff

1. Validate branch protection and provenance policies.
2. Run onboarding bootstrap by role:
   - `./tests/tooling/scripts/onboarding/protheus_onboarding_bootstrap.sh --role=<operator|backend|infra|security> --install=1 --setup=1 --install-mode=full`
3. Verify onboarding receipts:
   - `local/state/ops/onboarding_portal/bootstrap_<role>.json`
   - `local/state/ops/onboarding_portal/bootstrap_<role>.txt`
   - `local/state/ops/onboarding_portal/bootstrap_<role>_setup_status.json`
4. Verify setup status contract:
   - `infring setup status --json`
5. Record release/governance evidence artifacts for handoff.

Expected bootstrap outcomes (all roles):

- `binary_outcome=ready`
- `setup_outcome=completed`
- `setup_status_check=completed`
- `gateway_outcome=started` for `operator`; `gateway_outcome=not_requested` for `backend|infra|security`
- `status=success`

## Release-Mode Runtime Surface Matrix

Authoritative install/runtime manifest:
- `client/runtime/config/install_runtime_manifest_v1.txt`

| Surface | Required in all modes | `full` | `minimal` | `pure` / `tiny-max` |
| --- | --- | --- | --- | --- |
| Wrappers (`infring`, `infringctl`, `infringd`) | Yes | Yes | Yes | Yes |
| Setup lane (`infring setup`, `infring setup status --json`) | Yes | Yes | Yes | Yes |
| Gateway status (`infring gateway status`) | Yes | Yes | Yes | Yes |
| Rich gateway launch (`infring gateway`) | Optional | Available | Available with explicit setup if needed | Limited/optional by design |
