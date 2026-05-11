# Browser Shell V2

Status: initial clean browser Shell plug foundation

## Purpose

`shell/browser-v2/**` is the home for the clean browser Shell plug.

Browser Shell V2 is a replaceable presentation/input plug. It implements the
Shell Socket contract by calling Gateway routes through
`ShellSocketGatewayClient`; it must not own runtime truth, mirror full
conversations, or bypass Gateway.

The surface contract is strict: Browser Shell V2 must look like Shell 1.0 on the
outside unless a deliberate product/design migration is approved separately. V2
may inherit legacy CSS and legacy DOM class names as a visual skin, but it must
not import legacy scripts, Alpine stores, legacy caches, or dashboard authority.

## Current Scope

- Svelte component source for the first browser surface shell.
- Static artifact build that produces a standalone browser artifact with the
  legacy dashboard visual skin and V2-only socket/runtime code.
- Independent static server for Browser Shell V2 local launch/smoke testing.
- Socket-only controller for runtime status, agent roster, session list,
  bounded message window, and input submission.
- Bounded agent/session selectors that reload Gateway projections by ID rather
  than storing canonical runtime state in the browser.
- Lazy message detail expansion by `detail_ref`, keeping default message rows
  projection-only.
- Rich bounded lazy detail drawers with kind, title, summary, up to 12
  projection rows, refs, cursor, and receipt refs.
- Bounded event projection refresh through the Shell Socket event route.
- Live bounded event projection polling through the Shell Socket event route,
  with an in-flight guard and a 20-row retained tail.
- Bounded Gateway search projection; the browser does not search full local
  conversation trees.
- Issue/eval submission through Gateway with a bounded context window and
  receipt display only.
- Approval decision requests through Gateway with receipt display only; Browser
  V2 collects intent but does not own approval authority.
- Model and git tree selection requests through Gateway with receipt display
  only; Browser V2 does not own model or repository authority.
- Bounded model and git-tree selector rows from Gateway runtime projections,
  with local fallbacks treated only as request presets.
- Gateway audit receipt ledger for recent Shell Socket calls, displayed as
  bounded refs instead of retained route payloads.
- Memory-surface guard that fails if Browser V2 starts retaining legacy caches,
  raw runtime/tool payloads, full conversation trees, or unbounded UI rows.
- Amputation guard that fails if Browser V2 regains a dependency on the legacy
  dashboard, Alpine bridge, or `4173` dashboard host.
- Accessibility guard that enforces baseline landmarks, labels, typed buttons,
  and disabled-state wiring on the clean Svelte plug.
- Visual parity guard that enforces the familiar skin/new substrate rule:
  legacy dashboard CSS/class skeleton on the surface, V2-only socket/runtime
  code underneath.
- Gateway launch routing through `infring gateway --shell=ui-v2`, which starts
  the independent Browser Shell V2 static server on port `5273` and points it at
  the canonical Shell Socket Gateway target.
- Deterministic fixture smoke proving the V2 plug can hydrate and submit input
  through the Shell Socket shape without legacy dashboard assets.
- Contract guard proving the V2 plug is separate from `client/**`, Alpine, and
  legacy dashboard runtime state.

## Not In Scope Yet

- Pixel-level parity is still being filled in feature-by-feature, but arbitrary
  new dashboard styles, invented page chrome, or replacement visual objects are
  not allowed.

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

The V2 server defaults to `http://127.0.0.1:5273/` and serves only the Browser
Shell V2 artifact. It does not proxy or depend on the legacy dashboard host at
`4173`; the browser runtime talks to Gateway through the Shell Socket contract.
The artifact may bundle legacy dashboard CSS as a visual-only skin.

```text
npm run -s ops:browser-shell-v2:serve-smoke
```

## Gateway Launch

```text
infring gateway --shell=ui-v2
```

`--shell=ui-v2` launches the clean Browser Shell V2 server separately from the
legacy dashboard host. The launched page defaults to the Gateway Shell Socket
target at `http://127.0.0.1:5173` and may be overridden with:

- `INFRING_BROWSER_SHELL_V2_HOST`
- `INFRING_BROWSER_SHELL_V2_PORT`
- `INFRING_SHELL_SOCKET_URL`

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

The first V2 surface must reuse the legacy dashboard skin directly. That means
legacy CSS variables, legacy layout classes, and legacy visible object names such
as `app-layout`, `global-taskbar`, `sidebar`, `chat-wrapper`, `messages`,
`message-bubble`, `chat-map`, and `input-area`.

V2 must not invent a replacement dashboard chrome, alternate page structure, or
new visual language while it is intended to replace Shell 1.0. It does not
import legacy scripts, Alpine stores, legacy chat caches, or dashboard hydration
code.

The goal is a new substrate with a familiar skin: the legacy dashboard skin on
the surface, Shell Socket/Gateway-only data flow underneath. Shell V2 displays
bounded projections and collects input; Gateway and downstream owners keep
authority.
