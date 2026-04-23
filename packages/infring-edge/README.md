# `@infring/edge`

Workspace compatibility facade for the supported mobile/edge subset:

- mobile adapter status -> `client/runtime/systems/hybrid/mobile/infring_mobile_adapter.ts status --json`
- mobile cockpit status -> `infring-ops persist-plane mobile-cockpit --op=status`
- mobile daemon status -> `infring-ops persist-plane mobile-daemon --op=status`
- benchmark matrix status -> `infring-ops benchmark-matrix status`
- wrapper verification -> static contract inspection over `packages/infring-edge/wrappers/*`

Deprecated compatibility exports:

- `edgeSwarm()` remains as an explicit compatibility notice because the old swarm bridge path no longer exists.
- `edgeRuntime('start')` is no longer supported; this package is status/contract only.

Quick start:

```bash
node client/runtime/lib/ts_entrypoint.ts packages/infring-edge/starter.ts --mode=status
```

Contract check:

```bash
node client/runtime/lib/ts_entrypoint.ts packages/infring-edge/starter.ts --mode=contract --max-mb=5 --max-ms=200
```

Wrapper directories:

- `packages/infring-edge/wrappers/android_termux`
- `packages/infring-edge/wrappers/ios_tauri`
