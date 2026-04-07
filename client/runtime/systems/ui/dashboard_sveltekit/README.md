# Infring Dashboard SvelteKit Module

This folder is the SvelteKit module for the dashboard surface.

## Current state

- SvelteKit scaffold is available for incremental migration.
- Runtime API connectivity is validated via `/api/status`.
- Runtime API and websocket contracts remain owned by the gateway host.

## Local run

```bash
cd client/runtime/systems/ui/dashboard_sveltekit
npm install
npm run dev
```

## Incremental migration order

1. Port shell layout (`top bar`, `sidebar`, `chat canvas`) into Svelte components.
2. Port transport layer (`agents`, `messages`, `queue`) to typed Svelte stores.
3. Preserve existing API paths and websocket contracts from the runtime host.
4. Promote Svelte as default only after parity checks pass.
