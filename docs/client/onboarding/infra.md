# Infra Onboarding Track

1. Bootstrap using the canonical install -> setup sequence:
`./tests/tooling/scripts/onboarding/protheus_onboarding_bootstrap.sh --role=infra --install=1 --setup=1 --install-mode=full`.
2. Run `protheus-ops benchmark-matrix run --refresh-runtime=1`.
3. Verify release-security workflow contract in `.github/workflows/release-security-artifacts.yml`.
4. Verify setup status with `infring setup status --json`.
5. Verify receipts:
   - `local/state/ops/onboarding_portal/bootstrap_infra.json`
   - `local/state/ops/onboarding_portal/bootstrap_infra.txt`
   - `local/state/ops/onboarding_portal/bootstrap_infra_setup_status.json`
6. Confirm outcomes: `binary_outcome=ready`, `setup_outcome=completed`, `setup_status_check=completed`, `gateway_outcome=not_requested`, `status=success`.
