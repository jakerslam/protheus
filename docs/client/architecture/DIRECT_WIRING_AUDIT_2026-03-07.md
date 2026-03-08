# Direct Wiring Audit (2026-03-07)

## Removed in this sprint

- `client/core_memory_compat/` (deprecated memory compatibility shim)
- `client/core/memory/compat_bridge.ts` (duplicate shim entrypoint)
- `client/memory/tools/tests/core_memory_compat_bridge.test.js` (shim-only test)
- `client/runtime/state` symlink (legacy runtime alias)
- root `local/` runtime leakage (migrated into `client/runtime/local/state/*`)

## Canonical runtime roots

- Client runtime/user data: `client/runtime/local/*`
- Core runtime/node-local data: `core/local/*`

## Enforced blocked legacy surfaces

- `state`
- `client/runtime/state`
- `local`

Enforced by:

- `client/runtime/systems/ops/runtime_state_surface_guard.ts`
- `client/runtime/systems/ops/migrate_cleanup.ts`

## Remaining intentional exceptions

- Root identity/memory markdown files remain tracked for current bootstrap/test contracts:
  - `MEMORY.md`, `SOUL.md`, `HEARTBEAT.md`, `IDENTITY.md`, `TAGS_INDEX.md`, `LEARNINGS_INDEX.md`
- Follow-up item: `V6-ROOT-INTERNAL-003` in `TODO.md` to either migrate these to `client/runtime/local/internal/*` or formally keep them as root exceptions.

