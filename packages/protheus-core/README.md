# `@protheus/core`

Workspace compatibility facade for the live core kernel contracts:

- `spineStatus()` -> `infring-ops spine status`
- `reflexStatus()` -> `client/cognition/habits/scripts/reflex_habit_bridge.ts status`
- `gateStatus()` -> `infring-ops security-plane status`

This package intentionally stays lightweight and maps only to supported status surfaces.

Quick start:

```bash
node client/runtime/lib/ts_entrypoint.ts packages/protheus-core/starter.ts
```

Optional flags:

```bash
node client/runtime/lib/ts_entrypoint.ts packages/protheus-core/starter.ts --spine=1 --reflex=0 --gates=1
```

Cold-start contract:

```bash
node client/runtime/lib/ts_entrypoint.ts packages/protheus-core/starter.ts --mode=contract --max-mb=5 --max-ms=200
```
