# ChromeOS/Fuchsia Distribution OTA Adapter

`V3-RACE-273`

## Purpose

Provide a governed distribution and OTA verification lane for ChromeOS and Fuchsia package channels with deterministic rollback controls.

## Commands

```bash
node client/runtime/systems/ops/chromeos_fuchsia_distribution_ota_adapter.ts run --channel=chromeos-stable --strict=1
node client/runtime/systems/ops/chromeos_fuchsia_distribution_ota_adapter.ts freeze-channel --channel=chromeos-stable --reason=integrity_drift
node client/runtime/systems/ops/chromeos_fuchsia_distribution_ota_adapter.ts restore-channel --channel=chromeos-stable
node client/runtime/systems/ops/chromeos_fuchsia_distribution_ota_adapter.ts status
```

## Verified Contracts

- Package signature integrity for `chromeos` and `fuchsia` targets
- Build revision parity across channels
- OTA staged rollout plan completeness (`5/25/50/100`)
- Rollback window minimums and freeze/restore controls

## Policy

- `client/runtime/config/chromeos_fuchsia_distribution_ota_adapter_policy.json`
