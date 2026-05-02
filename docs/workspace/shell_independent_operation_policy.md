# Shell-Independent Operation Policy

Status: Canonical architecture policy
Owner: Jay
Scope: Core, Orchestration, CLI, Gateways, Shell UI assets, and Shell deletion/amputation tests
Effective: April 2026

## Active Shell Instance

The active browser/webview presentation implementation is `desktop UI shell 1.0`.
This policy still governs the broader `Shell` architecture boundary so additional
shells can be added without making any one UI implementation authoritative.

## Purpose

The system must operate without the browser Shell.

The Shell is useful, but it is not part of runtime authority. If the dashboard,
browser bundle, Svelte islands, CSS, images, and other presentation assets vanish,
Core, Orchestration, CLI command paths, Gateway status, and authoritative
runtime contracts must still build and operate.

This policy exists because long-chat stress exposed that the Shell can accidentally
become a state mirror or hidden runtime dependency. Shell independence keeps the
system honest: the UI can be replaced, deleted, rebuilt, or moved into another host
without breaking the actual agentic framework.

## Core Axiom

Core owns truth.

Orchestration owns coordination.

Gateways own bounded boundary transport.

CLI owns operator/headless entry.

Shell owns presentation.

Deleting browser Shell assets must remove only presentation, not authority,
coordination, CLI operation, Gateway operation, or runtime truth.

## Shell Socket And Plug Model

The replaceable part of the Shell architecture is a plug. The stable part is the
Shell Socket Contract defined in `docs/workspace/shell_ui_projection_policy.md`.

The canonical implementation home for the socket itself is
`shell/socket/**`. Legacy browser assets under `client/**` may consume
the socket later as a compatibility plug, and Gateway code may implement
`/api/shell-socket/**` route backing, but neither `client/**` nor `adapters/**`
owns the canonical socket substrate.
The retired top-level `surface/**` path must not be used as a catch-all
placement bucket for Shell, Gateway, or Orchestration work.

The Shell Socket Contract is an interface, not a persistent stateful runtime
layer. It must not become a new middleware authority between Gateway and concrete
shells.

Each operator-facing medium is a Shell plug:

- browser/dashboard UI plug;
- terminal/CLI UI plug;
- desktop/Tauri UI plug;
- mobile UI plug;
- embedded UI plug;
- future presentation/input plugs.

Every Shell plug must implement the Shell Socket Contract and call only Gateway
routes for system interaction. A plug may render differently, but it must consume
the same bounded projections, detail refs, ingress acknowledgements, receipts,
and status/event streams.

Non-UI external integrations are not Shell plugs. SDK clients, CI bots, issue
submitters, third-party automations, and machine-to-machine integrations are
Gateway adapter plugs. They still cross the Gateway boundary, but they do not
implement Shell rendering/input behavior.

## Parallel Socket Strategy

The Shell Socket Contract must be created and proven independently of the current
browser/dashboard Shell.

Execution plan: `docs/workspace/shell_socket_parallel_execution_plan.md`.

The current `desktop UI shell 1.0` is legacy compatibility. It may remain alive
while the new socket path is built, but it must not be the first implementation
target for the clean socket. Broad surgery on the live dashboard has already
shown high regression risk.

The safe migration posture is:

```text
Kernel / Orchestration / Assurance
        ^
Gateway Routes
        ^
Shell Socket Contract
        ^
New CLI plug / OpenClaw plug / future clean UI plug

Legacy desktop UI shell 1.0
        ^
old compatibility routes until cutover
```

The legacy dashboard must be treated as a tolerated compatibility plug, not as
the canonical Shell architecture. It should receive only critical blocker fixes
and small compatibility shims unless a replacement socket path already passes
parity.

Do not refactor, purge, or rewire the legacy dashboard to create the socket.
Create the socket beside it, prove it through a CLI/headless plug first, then
decide whether to adapt the dashboard or delete it in favor of a clean plug.

Parallel socket acceptance requires:

- a Shell Socket Contract definition independent of browser assets;
- Gateway routes that satisfy the socket without reading browser state;
- a CLI/headless plug proving agent list, session load, message send, progress
  stream, detail fetch, status, approval, and eval/report issue paths;
- no dependency on Alpine, Svelte bundles, DOM APIs, localStorage, browser event
  buses, or dashboard hydration;
- parity evidence before any legacy dashboard seam is switched to the new route;
- deletion/amputation proof that removing browser Shell assets does not break
  Kernel, Orchestration, Gateway, CLI, or Assurance surfaces.

## Browser Shell Assets

For this policy, browser Shell assets are presentation files and browser-only
runtime files such as:

- dashboard HTML/CSS;
- browser dashboard assembly;
- Svelte web-component sources and generated bundles;
- static UI images, icons, wallpaper, fonts, and vendor browser libraries;
- browser-only chat/map/sidebar/taskbar/dock rendering code;
- local display settings that affect only presentation.

The `client/` repository path is a compatibility shell path and is broader than
browser UI assets. CLI wrappers, SDK-facing wrappers, setup helpers, and thin
Gateway callers may still live under `client/` during transition, but they must not
depend on browser Shell assets for headless operation.

## Required Independent Surfaces

These surfaces must keep working when browser Shell assets are removed in a
disposable fixture:

- Core build and authority crates.
- Orchestration build and contract checks.
- CLI command registry and basic headless commands.
- Gateway status/health contracts.
- Nexus-Conduit-Checkpoint policy guards.
- Shell projection/Gateway/payload policy guards that do not require rendering the browser UI.

The next enforcement guard for this policy is the Shell amputation regression guard.
That guard must prove the above surfaces in a no-browser-Shell fixture.

## Prohibited Dependencies

Core, Orchestration, CLI, and Gateway authority paths must not require:

- `infring_static` browser assets;
- Svelte component bundles;
- dashboard CSS or HTML;
- browser global state;
- DOM APIs;
- localStorage/sessionStorage;
- browser event buses;
- UI cache hydration;
- chat/sidebar/map rendering modules.

If a non-UI path imports, shells out to, reflects over, embeds, or reads browser
Shell assets to decide runtime behavior, it violates this policy.

## Allowed Relationships

Browser Shell may depend on:

- Gateway contracts;
- bounded Shell-facing projections;
- lazy detail refs;
- CLI helpers where explicitly presentation-bound;
- local display configuration;
- static assets and generated presentation bundles.

Core, Orchestration, CLI, and Gateways may expose stable contracts consumed by the
Shell. They must not consume browser Shell implementation files.

## CLI Independence Rule

The CLI is a headless operator surface. It may present text, JSON, receipts,
diagnostics, and setup/status information without loading browser Shell assets.

CLI commands must call authoritative contracts through the proper Gateway,
Conduit, Nexus, or Kernel path. They must not depend on dashboard hydration, UI
stores, Svelte custom elements, browser event helpers, or browser caches.

The CLI should be treated as a first-class Shell plug over the Shell Socket
Contract, not as the parent or broker for other shells. Browser UI, terminal UI,
OpenClaw UI, and future shells are peers that implement the same contract.

## Dashboard Compatibility Rule

Names such as `dashboard_compat`, `dashboard_api`, or `chat_ui` may exist as
compatibility debt, but the implementation must be inspected by role:

- If it serves browser presentation, it is Shell.
- If it exposes a bounded Gateway/API projection, it is Gateway or adapter glue.
- If it decides truth, admission, policy, or receipts, it is misplaced and must
move to Core.
- If it coordinates workflow flow, it belongs in Orchestration.

Compatibility naming does not grant permission for browser Shell assets to become
runtime dependencies.

## Deletion Fixture Rule

Shell independence must be tested by deletion, not assumption.

The guard must create or use a disposable workspace fixture, remove browser Shell
asset paths, and prove that Core, Orchestration, CLI, and Gateway status
still build or smoke successfully. The fixture must not delete non-browser CLI/SDK
compatibility wrappers unless a later migration explicitly separates those paths.

The deletion fixture is allowed to make the browser dashboard unavailable. It is
not allowed to break headless runtime operation.

## Failure Semantics

A Shell-independent operation failure means one of these happened:

- browser assets were required for authority, coordination, CLI, or Gateway status;
- a non-UI path imported browser rendering code;
- a non-UI path read browser cache/state as truth;
- a command only worked because the dashboard had already hydrated state;
- deleting the Shell changed Core/Orchestration behavior.

Those failures are architecture violations, not UI bugs.

## Relationship To Other Policies

This policy depends on:

- `docs/workspace/nexus_conduit_checkpoint_policy.md`
- `docs/workspace/shell_ui_projection_policy.md`
- `docs/workspace/shell_ui_message_detail_contract.md`
- `docs/workspace/gateway_ingress_egress_policy.md`
- `docs/workspace/interface_payload_budget_policy.md`

Together they define the intended shape:

```text
Core/Orchestration/Gateway/CLI operate headlessly.
Shell consumes bounded projections.
Browser Shell assets are replaceable presentation.
```

## Enforcement

This policy is defined by `POLICY-REFINE-006`.

Executable enforcement is owned by `POLICY-REFINE-007`, the Shell amputation
regression guard: `npm run -s ops:shell:amputation:guard`.

The guard must create a disposable no-browser-Shell fixture, omit browser Shell
asset paths, prove Core, Orchestration, CLI command registry, and Gateway
status/health smoke paths still run, and fail if non-UI runtime paths import,
embed, execute, or read browser Shell assets.
