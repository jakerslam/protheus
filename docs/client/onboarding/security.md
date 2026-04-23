# Security Onboarding Track

1. Bootstrap using the canonical install -> setup sequence:
`./tests/tooling/scripts/onboarding/infring_onboarding_bootstrap.sh --role=security --install=1 --setup=1 --install-mode=full`.
2. Run `infring-ops enterprise-hardening run --strict=1`.
3. Run `infring-ops supply-chain-provenance-v2 run --strict=1` against release bundle fixtures.
4. Verify setup status with `infring setup status --json`.
5. Verify receipts:
   - `local/state/ops/onboarding_portal/bootstrap_security.json`
   - `local/state/ops/onboarding_portal/bootstrap_security.txt`
   - `local/state/ops/onboarding_portal/bootstrap_security_setup_status.json`
6. Confirm outcomes: `binary_outcome=ready`, `setup_outcome=completed`, `setup_status_check=completed`, `gateway_outcome=not_requested`, `status=success`.
