# Infring Dashboard SvelteKit Module

This folder is the active migration target for moving the dashboard UI from the current runtime shell into SvelteKit modules.

## Current state

- SvelteKit scaffold is in place.
- Runtime API connectivity is validated via `/api/status`.
- Existing gateway host remains the source of truth while porting components.

## Local run

```bash
cd client/runtime/systems/ui/dashboard_sveltekit
npm install
npm run dev
```

## Migration order

1. Port shell layout (`top bar`, `sidebar`, `chat canvas`) into Svelte components.
2. Port transport layer (`agents`, `messages`, `queue`) to typed Svelte stores.
3. Preserve all existing API paths and websocket contracts from the runtime host.
4. Cut over gateway host once parity tests pass.

