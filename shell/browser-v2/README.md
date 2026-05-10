# Browser Shell V2

Status: initial clean browser Shell plug foundation

## Purpose

`shell/browser-v2/**` is the home for the clean browser Shell plug.

Browser Shell V2 is a replaceable presentation/input plug. It implements the
Shell Socket contract by calling Gateway routes through
`ShellSocketGatewayClient`; it must not import the legacy dashboard, own runtime
truth, mirror full conversations, or bypass Gateway.

## Current Scope

- Svelte component source for the first browser surface shell.
- Static artifact build that produces a standalone browser artifact without
  importing legacy dashboard assets.
- Independent static server for Browser Shell V2 local launch/smoke testing.
- Socket-only controller for runtime status, agent roster, session list,
  bounded message window, and input submission.
- Bounded agent/session selectors that reload Gateway projections by ID rather
  than storing canonical runtime state in the browser.
- Lazy message detail expansion by `detail_ref`, keeping default message rows
  projection-only.
- Bounded event projection refresh through the Shell Socket event route.
- Bounded Gateway search projection; the browser does not search full local
  conversation trees.
- Issue/eval submission through Gateway with a bounded context window and
  receipt display only.
- Approval decision requests through Gateway with receipt display only; Browser
  V2 collects intent but does not own approval authority.
- Model and git tree selection requests through Gateway with receipt display
  only; Browser V2 does not own model or repository authority.
- Gateway audit receipt ledger for recent Shell Socket calls, displayed as
  bounded refs instead of retained route payloads.
- Memory-surface guard that fails if Browser V2 starts retaining legacy caches,
  raw runtime/tool payloads, full conversation trees, or unbounded UI rows.
- Amputation guard that fails if Browser V2 regains a dependency on the legacy
  dashboard, Alpine bridge, or `4173` dashboard host.
- Accessibility guard that enforces baseline landmarks, labels, typed buttons,
  and disabled-state wiring on the clean Svelte plug.
- Visual parity guard that enforces the familiar skin/new substrate rule with
  clean V2 classes, shared tokens, responsive layout, and glass-style surfaces.
- Deterministic fixture smoke proving the V2 plug can hydrate and submit input
  through the Shell Socket shape without legacy dashboard assets.
- Contract guard proving the V2 plug is separate from `client/**`, Alpine, and
  legacy dashboard runtime state.

## Not In Scope Yet

- Full visual parity with Shell 1.0.
- Browser launch routing from `infring gateway`.
- Live event streaming.
- Rich lazy detail drawers beyond the current bounded summary panel.
- Model/git-tree menus.

## Local Build

```text
npm run -s ops:browser-shell-v2:build
```

The build writes a static artifact to
`core/local/artifacts/browser_shell_v2_app/`. The generated browser runtime
defaults to the canonical local Shell Socket Gateway target at
`http://127.0.0.1:5173` and also accepts `?gateway=<url>` for local testing.

## Local Serve

```text
npm run -s ops:browser-shell-v2:serve
```

The V2 server defaults to `http://127.0.0.1:5273/` and serves only the clean
Browser Shell V2 artifact. It does not proxy, import, or depend on the legacy
dashboard at `4173`; the browser runtime talks to Gateway through the Shell
Socket contract.

```text
npm run -s ops:browser-shell-v2:serve-smoke
```

## Presentation Contract

Browser Shell V2 may own:

- layout/theme selection
- input buffer
- selected agent/session IDs
- expanded IDs
- visible bounded message rows
- cursors and detail refs

Browser Shell V2 must not own:

- canonical truth
- workflow/planner/policy truth
- raw tool payloads
- raw traces
- full conversation trees
- Gateway policy or authorization truth

## Visual Direction

The first V2 component borrows the legacy visual language through clean CSS
tokens and component classes. It does not import legacy scripts, Alpine stores,
legacy chat caches, or dashboard hydration code.

The goal is a new substrate with a familiar skin: Shell V2 displays bounded
projections and collects input; Gateway and downstream owners keep authority.
