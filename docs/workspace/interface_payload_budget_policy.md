# Interface Payload Budget Policy

Status: Canonical architecture policy
Owner: Jay
Scope: Shell-facing, CLI-facing, SDK-facing, and adapter-facing default Gateway/API payloads
Effective: April 2026

## Purpose

Default Shell-facing endpoints must be budgeted before they can be used.

The system boundary should send the Shell what it needs to display, not everything
the runtime knows. This policy turns that rule into explicit payload ceilings so
large conversations, tool outputs, traces, and workflow state cannot silently move
back into browser-owned memory through default API responses.

## Core Axiom

A payload budget is a ceiling, not a target.

Default endpoint responses are bounded projections. Heavy data escapes the default path only through detail refs.

Default endpoint routes must also appear in the Cross-Domain Nexus Route Inventory when they cross Shell, Gateway, Core, Orchestration, app, package, or external-agent/plugin boundaries.

## Default Endpoint Rule

Any endpoint that hydrates ordinary Shell, CLI, SDK, or adapter display state must
declare:

- route class;
- endpoint pattern;
- allowed top-level fields;
- maximum response bytes;
- maximum array items;
- maximum object depth;
- maximum string characters;
- maximum nested collection items;
- cursor/window behavior;
- detail-reference behavior;
- capability or lease requirement;
- audit receipt requirement;
- Nexus checkpoint requirement.

If an endpoint cannot fit inside those limits, it is not a default endpoint. It must
be split into a smaller projection route plus explicit detail-fetch routes.

## Default Payload Ceilings

Default endpoint payload ceilings are intentionally conservative:

- response body: max `65536` bytes;
- array fields: max `100` items;
- object depth: max `4`;
- string field: max `12000` chars;
- nested collection: max `20` items;
- top-level field count: max `32`.

These are upper ceilings. Individual endpoints should usually be smaller.

## Forbidden Default Payloads

Default endpoint payloads must not include fields named or shaped like:

- `raw`
- `root`
- `full_state`
- `all_messages`
- `conversation_tree`
- `raw_payload`
- `tool_input`
- `tool_result`
- `trace_body`
- `decision_trace`
- `plan_graph`
- `workflow_graph`
- `execution_observation`
- `runtime_quality`
- `eval_payload`
- `artifact_body`
- `file_output`
- `folder_tree`
- `policy_decision`
- `authorization_state`

If the display needs one of those classes, the default payload carries a detail ref
or count instead.

## Window And Cursor Rule

Default endpoints that return collections must use cursor or window fields. They
must not return complete unbounded histories, full conversation trees, full agent
inventories with nested sessions, or full tool/artifact bodies.

Allowed collection responses return IDs, previews, snippets, counts, labels, refs,
and cursors.

## Detail Ref Rule

Default payloads may include stable refs for:

- message detail;
- tool result detail;
- artifact detail;
- trace detail;
- workflow detail.

Those detail routes must follow the Shell UI Message Detail contract and Gateway
Ingress/Egress Interface policy. Detail responses can be richer than default rows,
but still need their own size/window budget.

## Cache And Search Relationship

Shell caches and default search routes inherit these budgets.

Search returns hit IDs, snippets, labels, counts, cursors, and detail refs. Deep
payload search belongs behind the Gateway owner and returns bounded projections.

Caches store previews and refs. They do not store raw traces, full tool outputs, or
complete runtime state.

## Legacy Debt Treatment

Existing endpoints that exceed these rules are migration debt, not accepted design.

New default endpoints must comply immediately. Temporary exceptions must name:

- owner;
- expiry date;
- endpoint pattern;
- exact budget exceeded;
- replacement projection/detail-fetch plan.

## Enforcement

Enforcement command: `npm run -s ops:interface:payload-budget:guard`.

Route inventory command: `npm run -s ops:nexus:route-inventory:guard`.

Architecture policy governance command: `npm run -s ops:policy-refinement:governance`.

The guard validates `client/runtime/config/interface_payload_budget_contract.json`,
cross-checks the Shell message/detail and Gateway contracts, and fails closed if
default Shell-facing endpoints admit oversized responses, excessive nesting,
unbounded arrays, raw traces, full tool outputs, full histories, or missing
cursor/detail/audit/Nexus constraints.
