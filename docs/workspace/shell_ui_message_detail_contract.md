# Shell UI Message Detail Contract

Status: Canonical architecture policy
Owner: Jay
Scope: Shell-facing chat, session, workflow-status, tool, artifact, and trace display payloads
Effective: April 2026

## Purpose

The Shell renders message projections by default and fetches heavy details only when an operator asks for them.

This contract sharpens the Shell UI Projection Policy into concrete payload shapes. It exists to prevent the browser Shell from receiving full runtime envelopes, tool results, traces, workflow graphs, or artifact bodies as part of ordinary chat/session hydration.

## Contract Axiom

Default Shell payloads are for display.

Detail payloads are for explicit expansion.

Runtime truth remains behind Core, Orchestration Surface, Gateway, Nexus, and Conduit checkpoints.

## Default Message Projection

A default Shell chat message row may include:

- `id`
- `conversation_id`
- `origin_kind`
- `origin_display_name`
- `origin_agent_id`
- `timestamp`
- `status`
- `content_preview`
- `content_window`
- `line_count`
- `tool_summary_count`
- `artifact_summary_count`
- `workflow_stage_label`
- `workflow_thought_preview`
- `detail_ref`
- `allowed_display_actions`

The projection should be shallow. Nested objects must be short summaries or refs, not full runtime payloads.

Default endpoint-level byte, array, depth, string, cursor, detail-ref, audit, and Nexus budgets are canonicalized in `docs/workspace/interface_payload_budget_policy.md`.

## Default Session Projection

A default Shell session/conversation row may include:

- `id`
- `title`
- `active_agent_id`
- `active_agent_name`
- `status`
- `last_message_preview`
- `last_message_at`
- `unread_count`
- `message_count`
- `pinned`
- `detail_ref`

It must not include the full message array or normalized message bodies.

## Prohibited Default Fields

Default Shell rows must not include fields named or shaped like:

- `raw`
- `root`
- `full_message`
- `raw_payload`
- `tool_input`
- `tool_result`
- `tool_result_body`
- `trace_body`
- `decision_trace`
- `plan_graph`
- `execution_observation`
- `runtime_quality`
- `eval_payload`
- `workflow_graph`
- `artifact_body`
- `file_output`
- `folder_tree`
- `policy_decision`
- `authorization_state`

If a display needs one of these, the default row must carry a stable detail reference instead.

## Required Detail Routes

Heavy data must be fetched through explicit detail routes:

- `message_detail` by `message_id`
- `tool_result_detail` by `tool_result_id`
- `artifact_detail` by `artifact_id`
- `trace_detail` by `trace_id`
- `workflow_detail` by `workflow_id`

Each route must be bounded, capability scoped, auditable, and checkpointed through Nexus/Conduit. Detail routes may return richer payloads, but they must still enforce size limits and receipt context.

Gateway route classes for detail fetch, request ingress, event/output egress, health/status, and bounded search/query are canonicalized in `docs/workspace/gateway_ingress_egress_policy.md`.

Default payload budget enforcement is canonicalized in `docs/workspace/interface_payload_budget_policy.md`.

## Lazy Fetch Rule

These classes are lazy-only:

- raw tool inputs and outputs;
- trace bodies;
- plan graphs;
- execution observations;
- eval payloads;
- workflow graphs;
- full artifact bodies;
- full file or folder outputs.

The default projection may expose summary counts, labels, status, and refs for those classes, never the bodies.

## Preview Budget Rule

Default text previews must be bounded. The canonical preview budget is intentionally conservative:

- message preview/window text: max `12000` chars;
- tool summary rows: max `20`;
- artifact summary rows: max `20`;
- display actions: max `12`;
- long-history hydration must use cursor/window fields rather than complete arrays.

These are policy ceilings, not targets.

## Search And Cache Relationship

Shell search and Shell caches operate on default projections unless the operator explicitly requests detail expansion or deep search.

Search hits should return IDs, snippets, counts, and refs. Caches should retain previews and refs. Neither path should copy heavy detail bodies into Shell-owned durable state.

## Enforcement

Enforcement command: `npm run -s ops:shell:ui-message-contract:guard`.

The guard validates the structured contract in `client/runtime/config/shell_ui_message_detail_contract.json`, verifies this policy document contains the canonical contract language, and fails closed if default projections admit prohibited heavy fields or if lazy-only data classes lack explicit detail routes.
