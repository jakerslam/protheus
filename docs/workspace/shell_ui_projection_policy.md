# Shell UI Projection Policy

Status: Canonical architecture policy
Owner: Jay
Scope: Shell UI, Shell SDK/UI-facing gateways, Orchestration output packaging, and Kernel-facing presentation contracts
Effective: April 2026

## Active Shell Instance

The current operator-facing implementation is `desktop UI shell 1.0`.

Use `desktop UI shell 1.0` when referring to the active dashboard/desktop UI
implementation currently in use. Use `Shell` when referring to the architecture
boundary that can be implemented by multiple shells.

## Purpose

The Shell is a lens, not a mirror.

The Shell may render system state, collect operator input, and hold bounded presentation-local state. It must not retain or recreate the runtime authority graph inside the browser, desktop webview, SDK wrapper, or dashboard state store.

This policy exists because long-chat stress exposed a structural failure mode: if the Shell receives and stores full runtime objects, raw tool payloads, traces, workflow state, and cache mirrors, it becomes a second state owner. That violates the Shell contract and can turn one large session into multi-GB browser memory use.

## Core Axiom

Kernel decides what is true and allowed (`core/**` is the implementation path only).

Orchestration decides what should happen next.

Gateways expose bounded ingress, egress, and detail-fetch routes.

Shell shows, collects, and requests details by reference.

The Shell default data shape is a projection, not a runtime object.

Shell-independent operation is canonicalized in `docs/workspace/shell_independent_operation_policy.md`; Core, Orchestration, CLI, and Gateway status must build and operate without browser Shell assets.

## Shell Socket Contract

The Shell Socket is the stable presentation/input contract that every concrete
Shell plug must implement. It is a contract, not a persistent middleware layer,
not a stateful runtime component, and not a new authority boundary.

The canonical socket home is `shell/socket/**`. That directory is the
clean Shell 2.0 substrate: contracts under `shell/socket/contract/**`,
thin transport clients under `shell/socket/client/**`, and headless
proof plugs under `shell/socket/probe/**`. Do not place canonical Shell
Socket infrastructure under legacy `client/**` or Gateway/integration
`adapters/**`; those paths are compatibility surfaces, not the clean socket
owner.
Do not place Shell Socket or Orchestration Control Plane artifacts under a
top-level `surface/**` path; that root is retired and must not become a new
architecture bucket.

Concrete Shell plugs include browser/dashboard UI, terminal/CLI UI, desktop/Tauri
UI, mobile UI, embedded UI, and future operator-facing presentation surfaces.
Each plug implements the same Shell Socket Contract and talks only through
Gateway routes.

The corrected shape is:

```text
Browser / Terminal / Desktop / Mobile / Embedded Shell Plug
        implements
Shell Socket Contract
        calls only
Gateway Routes
        then reaches
Kernel / Orchestration / Assurance
```

The rejected shape is:

```text
Kernel / Orchestration
        -> Gateway
        -> stateful Shell Socket runtime
        -> Shell plugs
```

That rejected shape risks turning the socket into another runtime authority or
state mirror. The socket must stay an interface that constrains shell behavior.

### Parallel Implementation Rule

The Shell Socket Contract should be implemented in parallel to the current
`desktop UI shell 1.0`, not by first rewiring the current dashboard.

Execution plan: `docs/workspace/shell_socket_parallel_execution_plan.md`.

The current dashboard is legacy compatibility. It can continue using legacy
compatibility routes while the clean socket is proven by a CLI/headless plug or
another clean Shell plug. A legacy dashboard seam may move to the socket only
after the replacement Gateway route has parity evidence and a rollback/fallback
path.

This rule exists to prevent the socket project from becoming another broad
dashboard refactor. Building the socket must reduce dependency on the fragile
browser Shell; it must not require editing the browser Shell as the critical path.

### Required Socket Capabilities

Every Shell plug should be able to express these capabilities through bounded
Gateway contracts:

```text
submit_input(input) -> ingress_ack
subscribe_events(session_id, cursor) -> shell_event_projection stream
get_message_window(session_id, cursor, limit) -> message_window_projection
get_message_detail(detail_ref) -> message_detail_projection
get_runtime_status() -> runtime_status_projection
search(query) -> bounded_search_results
submit_issue(issue_projection) -> ingress_ack
list_agents(cursor, limit) -> agent_roster_projection
list_sessions(agent_id, cursor, limit) -> session_list_projection
submit_approval_decision(approval_id, decision) -> receipt
set_model(agent_id, model_ref) -> receipt
set_git_tree(agent_id, tree_ref) -> receipt
submit_terminal_command(target_ref, command) -> receipt_or_stream_ref
```

The exact transport may differ by plug. A browser plug may use HTTP/SSE/WebSocket,
a terminal plug may use CLI-friendly JSON or streaming text, and an embedded plug
may use another adapter. The contract and payload boundaries must remain the same.

### Plug Ownership

A Shell plug may own:

- layout, theme, and presentation preferences;
- input buffers, focus, hover, selection, and expansion state;
- scroll position and visible-window placement;
- local keybindings and command entry affordances;
- bounded visible rows, IDs, cursors, and refs;
- small disposable preview caches.

A Shell plug must not own:

- canonical truth;
- workflow/planner state;
- policy or authorization truth;
- Gateway policy;
- Kernel state;
- raw tool outputs, raw traces, full plan graphs, or eval payloads;
- full conversation trees or unbounded runtime mirrors;
- mutation semantics for retry, replay, fork, approval, config, model, git, or terminal execution.

### Shell Plug Versus Gateway Adapter Plug

Only presentation/input media are Shell plugs.

External systems that are not presentation/input media are Gateway adapter plugs,
not Shell plugs. Examples include SDK consumers, CI bots, external automation,
issue submitters, third-party integrations, and machine-to-machine clients.

Both Shell plugs and Gateway adapter plugs use Gateway routes, but only Shell
plugs implement Shell rendering/input behavior.

## Dynamic Long-Chat Memory Regression

Static Shell projection checks are necessary but not sufficient. The Shell must also prove that a synthetic long chat stays bounded across the interaction phases that historically expose projection leaks:

- open a long thread;
- scroll to a non-tail window;
- search within the long thread;
- expand a bounded tool/detail reference;
- switch to another session;
- cleanup/unload the session projection.

The canonical dynamic regression is `npm run -s ops:shell:long-chat-ram:guard`. It records per-phase estimated JS heap, DOM node count, custom-element count, storage bytes, projected row count, and cleanup state. A passing Shell cannot rely on visual virtualization alone; it must keep heap, DOM, storage, detail expansion, session switching, and cleanup inside release-blocking budgets.

## What The Shell May Own

The Shell may own:

- visual layout state;
- transient input state;
- hover, focus, selection, expansion, and drag state;
- bounded visible-window projections;
- IDs, cursors, page tokens, and detail references;
- short previews, labels, counts, timestamps, and status text;
- presentation caches with explicit size and time bounds;
- user-selected display settings that do not decide system truth.

## What The Shell Must Not Own

The Shell must not own, mirror, or infer:

- canonical state truth;
- policy decisions or authorization truth;
- execution admission, retry, lane, route, or queue truth;
- workflow decomposition or planner truth;
- authoritative health or readiness truth;
- raw Core, Orchestration, Gateway, or tool runtime objects;
- raw tool inputs, raw tool outputs, trace bodies, plan graphs, or eval payloads in default message/session rows;
- full conversation trees as durable browser-owned state;
- full-state localStorage/sessionStorage caches;
- raw/root app-store references in emitted UI events;
- hidden fallback decisions that affect execution rather than display.

## Default Projection Rule

Every Shell-facing default payload must answer only:

1. What should be displayed in the current view?
2. What stable ID can fetch more detail if the user asks?
3. What lightweight status or count helps the user understand the display?

If a field is not needed for the current visible projection, it must be omitted from the default payload and fetched lazily by ID.

## Minimal Message Projection Shape

Canonical contract: `docs/workspace/shell_ui_message_detail_contract.md`.

A default chat/session message projection should be shallow and bounded. It may include fields shaped like:

```text
id
conversation_id
origin_kind
origin_display_name
timestamp
status
content_window_or_preview
line_count
tool_summary_count
artifact_summary_count
detail_ref
allowed_display_actions
```

It must not include raw tool result bodies, full decision traces, full execution observations, full workflow graphs, or authority-bearing policy fields.

## Detail Fetch Rule

Heavy information is loaded by explicit detail routes, not default rows.

Examples:

- `get_message_detail(message_id)`
- `get_tool_result(tool_result_id)`
- `get_trace_detail(trace_id)`
- `get_artifact_detail(artifact_id)`
- `get_workflow_detail(workflow_id)`

Detail fetches must remain bounded, capability scoped, auditable, and revocable through the same Nexus-Conduit-Checkpoint rules as any other cross-boundary route.

## Windowing Rule

Unbounded lists must be windowed before they enter Shell-owned reactive state.

The Shell may maintain a lightweight map, count, cursor, and scroll position for long histories. It must not require every historical row to be represented as a full rendered or full data object in the active UI store.

## Cache Rule

Shell caches are preview caches, not state mirrors.

Allowed cache entries contain bounded display previews and refs.

Disallowed cache entries contain full runtime envelopes, raw tool payloads, trace bodies, plan graphs, workflow observations, or complete conversation arrays without a strict bounded retention policy.

## Search Rule

Default Shell search may search display text, labels, summaries, and previews.

It must not stringify raw tool results, trace bodies, full folder trees, full file outputs, or large nested payloads inside the Shell. Deep search belongs behind a backend or orchestration detail/search route that returns bounded projection hits.

## Event Rule

Shell event payloads must be minimal and serializable. Events must not carry references named or shaped like `raw`, `root`, full stores, full component instances, or full runtime state trees.

If an event payload could keep the whole app state alive through a listener or devtool reference, it violates this policy.

## Nexus And Gateway Relationship

Shell projection traffic must cross boundaries through explicit Nexus checkpoint surfaces and Conduit-backed routes.

Gateway route classes are canonicalized in `docs/workspace/gateway_ingress_egress_policy.md`.

Shell-facing gateway routes should be separated by responsibility:

- request ingress for user actions;
- event/output egress for display projections;
- health/status projections;
- detail fetch routes for explicit expansion;
- search/query routes for bounded result projections.

The Shell must not merge these into one full-state endpoint that returns the system internals.

Default Shell-facing endpoint payload budgets are canonicalized in `docs/workspace/interface_payload_budget_policy.md`. A route that cannot satisfy the default byte, depth, array, string, cursor, detail-ref, audit, and Nexus ceilings must stop being a default route and move heavy data behind detail refs.

## Legacy Debt Treatment

Existing Shell paths that violate this policy are migration debt, not accepted architecture.

New Shell code must comply immediately unless an explicit temporary exception names:

- owner;
- expiry date;
- affected route or file;
- reason the projection cannot be used yet;
- replacement projection/detail-fetch plan.

## Practical Placement Test

Before adding any Shell field, state, cache, event, or endpoint response, ask:

1. Is this required to render or collect input in the current view?
2. Can the Shell hold an ID or preview instead?
3. Would this field let the Shell decide truth, permission, routing, retry, admission, or health?
4. Would retaining this field across 10,000 messages create linear browser memory growth?
5. If the Shell vanished, would Core, Orchestration, and CLI still operate correctly?

If any answer points to authority or unbounded retention, the field belongs outside the Shell projection.

## Enforcement Intent

Enforcement command: `npm run -s ops:shell:projection:guard`.

Message/detail contract command: `npm run -s ops:shell:ui-message-contract:guard`.

Interface payload budget command: `npm run -s ops:interface:payload-budget:guard`.

Architecture policy governance command: `npm run -s ops:policy-refinement:governance`.

That aggregate governance command must include `npm run -s ops:shell:long-chat-ram:guard`, making long-chat heap, DOM, storage, detail-expansion, session-switch, and cleanup regressions release-blocking Shell projection failures.

The companion guard for this policy must fail on:

- Shell hot paths retaining raw/root store references;
- default UI payloads containing raw tool input/output bodies;
- default message/session rows containing full traces, plan graphs, eval payloads, or execution observations;
- unbounded Shell-owned arrays where a bounded projection/window is required;
- localStorage/sessionStorage caches that mirror full runtime state;
- Shell event payloads that retain raw/root store references.
