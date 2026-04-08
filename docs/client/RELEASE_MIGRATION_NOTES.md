# Release Migration Notes

## Legacy Bridge To Infring-Ops

- Status: Deprecated
- Window: 120 days
- Action:
  1. Replace legacy bridge command usage with `infring-ops` domain commands.
  2. Validate with `infring doctor --json` and `infring verify-install --json`.
  3. Confirm dashboard health via `infring verify-gateway`.

- Verification checklist:
  - Runtime contract checks pass.
  - No `runtime_assets_missing` entries remain in doctor output.
  - Gateway health endpoint responds at `/healthz`.
