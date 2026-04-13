# Infring Dashboard SvelteKit Module

This folder is the SvelteKit module for the dashboard surface.

## Current state

- SvelteKit is the primary dashboard shell when a local `build/` exists.
- `infring_static` remains the explicit fallback surface at `/dashboard-classic`.
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
2. Render unmigrated pages through `/dashboard-classic?embed=1&page=...` instead of duplicating old chrome.
3. Port native pages in churn order: `chat`, `agents`, `settings`, then supporting views.
4. Preserve existing API paths and websocket contracts from the runtime host.
