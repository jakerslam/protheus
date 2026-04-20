# Backend Onboarding Track

1. Bootstrap using the canonical install -> setup sequence:
`./tests/tooling/scripts/onboarding/protheus_onboarding_bootstrap.sh --role=backend --install=1 --setup=1 --install-mode=full`.
2. Run `cargo test -p protheus-ops-core` and `cargo clippy -p protheus-ops-core --all-targets -- -D warnings`.
3. Verify setup status with `infring setup status --json`.
4. Verify receipts:
   - `local/state/ops/onboarding_portal/bootstrap_backend.json`
   - `local/state/ops/onboarding_portal/bootstrap_backend.txt`
   - `local/state/ops/onboarding_portal/bootstrap_backend_setup_status.json`
5. Confirm outcomes: `binary_outcome=ready`, `setup_outcome=completed`, `setup_status_check=completed`, `gateway_outcome=not_requested`, `status=success`.
6. Commit one deterministic lane receipt update.
