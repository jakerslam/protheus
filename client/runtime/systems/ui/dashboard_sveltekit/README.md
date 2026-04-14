# Infring Dashboard SvelteKit Module

This module is retired as a live dashboard surface.

## Current state

- The authoritative dashboard is `client/runtime/systems/ui/infring_static`.
- `/dashboard` and `/dashboard/<page>` are served from the real classic dashboard host.
- `/dashboard-classic` and `/dashboard-shell` are compatibility aliases that redirect back to `/dashboard`.
- This folder remains only as cleanup debt until the remaining Svelte dashboard source is fully removed from the repo.
