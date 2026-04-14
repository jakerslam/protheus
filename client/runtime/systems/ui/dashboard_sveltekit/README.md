# Infring Dashboard SvelteKit Module

This folder is the SvelteKit module for the dashboard surface.

## Current state

- SvelteKit is the primary dashboard shell when a local `build/` exists.
- `/dashboard-classic` remains a top-level compatibility host while the remaining classic asset corpus is retired.
- Runtime API connectivity is validated via `/api/status`.
- Runtime API and websocket contracts remain owned by the gateway host.

## Local run

```bash
cd client/runtime/systems/ui/dashboard_sveltekit
npm install
npm run dev
npm run build
```

## Incremental migration order

1. Keep the SvelteKit shell authoritative for navigation, page framing, and migration status.
2. Port remaining views natively instead of adding new embedded classic fallback routes.
3. Burn down the remaining `infring_static` asset corpus until `/dashboard-classic` can be retired entirely.
4. Preserve existing API paths and websocket contracts from the runtime host.
