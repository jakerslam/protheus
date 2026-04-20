# External Machine Verification Matrix (V11-TODO-003)

Purpose: deterministic weekly verification across clean external machines so install/runtime behavior remains reproducible.

Cadence: weekly (suggested every Monday).

## Required Test Matrix

| Platform | Arch | Profile | Install | Gateway Start | Doctor Smoke | Result |
| --- | --- | --- | --- | --- | --- | --- |
| macOS 14+ | arm64 | rich | required | required | required | pending |
| macOS 14+ | arm64 | pure | required | required | required | pending |
| macOS 14+ | x64 (CI/VM) | rich | required | required | required | pending |
| macOS 14+ | x64 (CI/VM) | pure | required | required | required | pending |
| Ubuntu 22.04+ | x64 | rich | required | required | required | pending |
| Ubuntu 22.04+ | x64 | pure | required | required | required | pending |
| Ubuntu 22.04+ | arm64 | rich | required | required | required | pending |
| Ubuntu 22.04+ | arm64 | pure | required | required | required | pending |

## Procedure (per matrix row)

1. Use a clean profile/home directory.
2. Run install command from README.
3. Run gateway start/restart and confirm daemon status is healthy.
4. Run doctor/verify-install command.
5. Capture:
   - command outputs
   - platform/arch
   - release/ref
   - pass/fail reason code

## Machine-Readable Report Schema

```json
{
  "run_date": "YYYY-MM-DD",
  "platform": "macos|linux",
  "arch": "arm64|x64",
  "profile": "rich|pure",
  "install_ok": true,
  "gateway_ok": true,
  "doctor_ok": true,
  "error_codes": [],
  "notes": ""
}
```

## Exit Rule

A weekly run is complete only when every required row has a report entry and no unexplained failures remain.
