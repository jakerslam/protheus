# Shell-Independent Operation Policy

Status: Canonical architecture policy
Owner: Jay
Scope: Core, Orchestration Surface, CLI, Gateways, Shell UI assets, and Shell deletion/amputation tests
Effective: April 2026

## Purpose

The system must operate without the browser Shell.

The Shell is useful, but it is not part of runtime authority. If the dashboard,
browser bundle, Svelte islands, CSS, images, and other presentation assets vanish,
Core, Orchestration Surface, CLI command paths, Gateway status, and authoritative
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
- Orchestration Surface build and contract checks.
- CLI command registry and basic headless commands.
- Gateway status/health contracts.
- Nexus-Conduit-Checkpoint policy guards.
- Shell projection/Gateway/payload policy guards that do not require rendering the browser UI.

The next enforcement guard for this policy is the Shell amputation regression guard.
That guard must prove the above surfaces in a no-browser-Shell fixture.

## Prohibited Dependencies

Core, Orchestration Surface, CLI, and Gateway authority paths must not require:

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

## Dashboard Compatibility Rule

Names such as `dashboard_compat`, `dashboard_api`, or `chat_ui` may exist as
compatibility debt, but the implementation must be inspected by role:

- If it serves browser presentation, it is Shell.
- If it exposes a bounded Gateway/API projection, it is Gateway or adapter glue.
- If it decides truth, admission, policy, or receipts, it is misplaced and must
move to Core.
- If it coordinates workflow flow, it belongs in Orchestration Surface.

Compatibility naming does not grant permission for browser Shell assets to become
runtime dependencies.

## Deletion Fixture Rule

Shell independence must be tested by deletion, not assumption.

The guard must create or use a disposable workspace fixture, remove browser Shell
asset paths, and prove that Core, Orchestration Surface, CLI, and Gateway status
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
asset paths, prove Core, Orchestration Surface, CLI command registry, and Gateway
status/health smoke paths still run, and fail if non-UI runtime paths import,
embed, execute, or read browser Shell assets.
