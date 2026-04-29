# Gateway Ingress/Egress Interface Policy

Status: Canonical architecture policy
Owner: Jay
Scope: Shell-facing gateways, CLI-facing gateways, SDK-facing gateways, Core/Orchestration boundary adapters, and Gateway route contracts
Effective: April 2026

## Purpose

Gateways are boundary surfaces, not authority mirrors.

This policy separates Gateway traffic into explicit route classes so the Shell cannot become a full-state mirror, an authority relay, or a hidden runtime owner. The Gateway may accept bounded requests, emit bounded projections, report health/status, and fetch explicit details by ID. It must not collapse those responsibilities into one full-state endpoint.

## Core Axiom

Ingress asks for work.

Egress reports bounded display output.

Health/status reports bounded readiness truth from the owner.

Detail fetch expands one referenced object.

No Gateway route may return the whole system state just because one consumer might need a subset.

Default route payloads must also satisfy the Interface Payload Budget policy in `docs/workspace/interface_payload_budget_policy.md`.

Conduit route posture must satisfy the Conduit/Scrambler Posture policy in `docs/workspace/conduit_scrambler_posture_policy.md`.

Cross-domain route inventory must satisfy `docs/workspace/cross_domain_nexus_route_inventory.md`.

## Required Route Classes

### Request Ingress

Request ingress routes carry operator, Shell, CLI, SDK, or adapter requests into the authoritative system.

Request ingress may carry:

- request id;
- actor/session id;
- command or intent;
- selected refs;
- typed input fields;
- capability/lease context;
- idempotency key;
- trace/correlation id.

Request ingress should return an acknowledgement, receipt, accepted/rejected status, and follow-up refs. It must not return full conversation history, full tool results, full traces, full workflow graphs, or raw runtime state.

### Event Output Egress

Event/output egress routes carry bounded system output projections back to the Shell or another presentation surface.

Event/output egress may carry:

- event id;
- event kind;
- display projection;
- status label;
- cursor/window refs;
- detail refs;
- receipt refs.

Event/output egress must not carry raw Core, Orchestration, planner, tool, trace, artifact, or eval envelopes by default.

### Health And Status

Health/status routes expose bounded status projections from the owner of truth.

Health/status may carry:

- state enum;
- label;
- source;
- source sequence;
- age seconds;
- stale flag;
- degraded reason;
- next retry hint.

Health/status must not require the Shell to infer readiness, reconstruct runtime truth, or query full internal state to decide what is healthy.

### Detail Fetch

Detail-fetch routes expand one referenced object by ID.

Detail fetch may carry:

- stable detail id;
- requested view;
- capability scope;
- size/window bounds;
- receipt/audit context.

Detail fetch may return richer data than default projections, but it must be bounded, capability-scoped, audited, revocable, and Nexus-checkpointed.

### Bounded Search/Query

Search/query routes return bounded result projections, not raw corpus mirrors.

Search/query may carry:

- query text or structured query;
- scope refs;
- cursor/window bounds;
- result limit;
- capability context.

Search/query must return hit ids, snippets, labels, counts, and detail refs. Deep payload search belongs behind the Gateway owner, not inside Shell memory.

## Prohibited Mixed Routes

Gateway routes must not combine unrelated responsibilities into a single full-state route.

Disallowed route shapes include:

- `full_state`
- `all_state`
- `mirror_state`
- `raw_runtime_state`
- `request_plus_full_result`
- `health_plus_state_dump`
- `search_plus_raw_payloads`
- `event_plus_trace_body`

If a route needs more than one class, split it into separate ingress, egress, status, detail, or query routes with shared correlation ids.

## Direction And Ownership Rule

Each route must declare:

- source domain;
- target domain;
- route class;
- owner of truth;
- payload class;
- capability or lease requirement;
- Nexus checkpoint requirement;
- audit receipt requirement;
- bounded response requirement.
- Conduit/Scrambler security posture.

Shell-facing route ownership is presentation-boundary ownership only. The route may collect input or display output, but truth and admission remain behind Core/Orchestration owners.

Gateway routes that carry sensitive detail fetches, authority-bearing request ingress, external agent/plugin ingress, recovery actions, policy/permission data, trace bodies, tool results, or execution observations must declare `strong_scrambler` posture rather than defaulting to standard transport.

## Gateway And Shell Relationship

The Shell may call Gateway routes. The Shell must not own Gateway policy.

The Shell may keep refs, cursors, previews, and display-local state. It must not cache Gateway full responses as durable runtime state, infer missing health, or make hidden retry/admission decisions from Gateway transport behavior.

Default Shell-facing Gateway responses must declare byte, array, depth, string, cursor, detail-ref, audit, and Nexus budgets before they are accepted as default routes.

## Enforcement

Enforcement command: `npm run -s ops:gateway:interface:guard`.

Payload budget command: `npm run -s ops:interface:payload-budget:guard`.

Route inventory command: `npm run -s ops:nexus:route-inventory:guard`.

The guard validates `client/runtime/config/gateway_ingress_egress_contract.json`, verifies this policy document contains the canonical route-class language, and fails closed if required route classes are missing, if a route class allows full-state/default raw payloads, or if detail/search/status classes lack bounded/capability/audit/Nexus constraints.
