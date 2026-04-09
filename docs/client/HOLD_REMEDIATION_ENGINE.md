# Hold Remediation Engine

`V5-HOLD-001` through `V5-HOLD-005` are implemented by `client/runtime/systems/autonomy/hold_remediation_engine.ts`.

## Commands

```bash
node client/runtime/systems/autonomy/hold_remediation_engine.ts admit --proposal-json='{"id":"p1","title":"A","kind":"generic","confidence":0.67}' --apply=1 --strict=1
node client/runtime/systems/autonomy/hold_remediation_engine.ts rehydrate --apply=1 --strict=1
node client/runtime/systems/autonomy/hold_remediation_engine.ts simulate --days=30 --apply=1 --strict=1
node client/runtime/systems/autonomy/hold_remediation_engine.ts status
```

## Coverage

- `V5-HOLD-001`: semantic unchanged-state gate with freshness-window dedupe.
- `V5-HOLD-002`: confidence routing calibration plus canary execute band.
- `V5-HOLD-003`: cap-aware parked queue with deterministic rehydrate.
- `V5-HOLD-004`: routeability preflight lint and manual queue routing.
- `V5-HOLD-005`: token burst smoothing and budget-pressure deferral/autopause signaling.
