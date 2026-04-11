# Mobile Wrapper Distribution Pack

This package now exposes a compatibility wrapper contract for the supported mobile targets.

Targets:

- `android_termux`
- `ios_tauri`

Supported verification commands:

```bash
node client/runtime/lib/ts_entrypoint.ts packages/protheus-edge/starter.ts --mode=status --target=android_termux
node client/runtime/lib/ts_entrypoint.ts packages/protheus-edge/starter.ts --mode=contract --max-mb=5 --max-ms=200
packages/protheus-edge/wrappers/android_termux/verify.sh
```

Current supported surfaces:

- wrapper directory presence and script integrity
- mobile adapter status
- mobile cockpit / mobile daemon status
- benchmark matrix status

Wrappers remain distributed from:

- `packages/protheus-edge/wrappers/android_termux`
- `packages/protheus-edge/wrappers/ios_tauri`

The old runtime build / rollback lane is no longer the active contract. Wrapper verification now routes through the package status/contract façade instead of the removed `mobile_wrapper_distribution_pack.ts` path.
