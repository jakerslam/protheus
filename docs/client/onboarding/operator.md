# Operator Onboarding Track

1. Run one command from repo root (canonical sequence: install -> setup -> gateway):
`./tests/tooling/scripts/onboarding/protheus_onboarding_bootstrap.sh --role=operator --install=1 --setup=1 --install-mode=full --gateway=1`.
2. Validate runtime health with `infring gateway status`.
3. Verify setup completion with `infring setup status --json`.
4. Capture first verified onboarding receipts:
   - `local/state/ops/onboarding_portal/bootstrap_operator.json`
   - `local/state/ops/onboarding_portal/bootstrap_operator.txt`
   - `local/state/ops/onboarding_portal/bootstrap_operator_setup_status.json`
5. Confirm outcomes: `binary_outcome=ready`, `setup_outcome=completed`, `setup_status_check=completed`, `gateway_outcome=started`, `status=success`.
