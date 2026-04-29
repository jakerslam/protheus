# Shell UI Projection Policy

Status: Canonical architecture policy
Owner: Jay
Scope: Shell UI, Shell SDK/UI-facing gateways, Orchestration Surface output packaging, and Core-facing presentation contracts
Effective: April 2026

## Purpose

The Shell is a lens, not a mirror.

The Shell may render system state, collect operator input, and hold bounded presentation-local state. It must not retain or recreate the runtime authority graph inside the browser, desktop webview, SDK wrapper, or dashboard state store.

This policy exists because long-chat stress exposed a structural failure mode: if the Shell receives and stores full runtime objects, raw tool payloads, traces, workflow state, and cache mirrors, it becomes a second state owner. That violates the Shell contract and can turn one large session into multi-GB browser memory use.

## Core Axiom

Core decides what is true and allowed.

Orchestration decides what should happen next.

Gateways expose bounded ingress, egress, and detail-fetch routes.

Shell shows, collects, and requests details by reference.

The Shell default data shape is a projection, not a runtime object.

Shell-independent operation is canonicalized in `docs/workspace/shell_independent_operation_policy.md`; Core, Orchestration Surface, CLI, and Gateway status must build and operate without browser Shell assets.

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

The companion guard for this policy must fail on:

- Shell hot paths retaining raw/root store references;
- default UI payloads containing raw tool input/output bodies;
- default message/session rows containing full traces, plan graphs, eval payloads, or execution observations;
- unbounded Shell-owned arrays where a bounded projection/window is required;
- localStorage/sessionStorage caches that mirror full runtime state;
- Shell event payloads that retain raw/root store references.
