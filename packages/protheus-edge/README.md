# `@protheus/edge`

Workspace compatibility facade for the supported mobile/edge subset:

- mobile adapter status -> `client/runtime/systems/hybrid/mobile/protheus_mobile_adapter.ts status --json`
- mobile cockpit status -> `infring-ops persist-plane mobile-cockpit --op=status`
- mobile daemon status -> `infring-ops persist-plane mobile-daemon --op=status`
- benchmark matrix status -> `infring-ops benchmark-matrix status`
- wrapper verification -> static contract inspection over `packages/protheus-edge/wrappers/*`

Deprecated compatibility exports:

- `edgeSwarm()` remains as an explicit compatibility notice because the old swarm bridge path no longer exists.
- `edgeRuntime('start')` is no longer supported; this package is status/contract only.

Quick start:

```bash
node client/runtime/lib/ts_entrypoint.ts packages/protheus-edge/starter.ts --mode=status
```

Contract check:

```bash
node client/runtime/lib/ts_entrypoint.ts packages/protheus-edge/starter.ts --mode=contract --max-mb=5 --max-ms=200
```

Wrapper directories:

- `packages/protheus-edge/wrappers/android_termux`
- `packages/protheus-edge/wrappers/ios_tauri`
