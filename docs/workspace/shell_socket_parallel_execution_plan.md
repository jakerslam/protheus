# Shell Socket Parallel Execution Plan

Status: Canonical migration plan
Owner: Jay
Scope: Shell Socket Contract, Gateway routes, CLI/headless Shell plug, OpenClaw UI readiness, legacy dashboard isolation
Created: 2026-05-01

## Purpose

This plan defines how to create the Shell Socket path without hard-breaking the
current browser dashboard.

The active dashboard, `desktop UI shell 1.0`, is legacy compatibility. It should
remain operable, but it is not the target implementation for the new Shell Socket
work. The new socket is built beside it, proven independently, and only later used
to replace or delete legacy dashboard seams.

## Core Decision

Build the Shell Socket independently of the current Shell.

```text
1. Define independent Shell Socket Contract.
2. Hook the contract to Gateway routes only.
3. Ensure Gateway routes bridge to Kernel, Orchestration, and Assurance.
4. Build a minimal CLI/headless Shell plug against the socket.
5. Use CLI parity tests to prove the socket works.
6. Only later decide whether to adapt or replace the legacy dashboard.
```

The Shell Socket must never connect directly to Orchestration.

Correct path:

```text
Shell Plug
        -> Shell Socket Contract
        -> Gateway Routes
        -> Kernel / Orchestration / Assurance
```

Rejected path:

```text
Shell Plug
        -> Orchestration directly
```

Direct Shell-to-Orchestration routes bypass the Gateway boundary and recreate
hidden shell authority.

## Migration Posture

Use the Parallel Socket Strategy:

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

The current browser dashboard is treated as a tolerated compatibility plug. It
may receive critical blocker fixes and tiny proven compatibility shims, but it is
not refactored, purged, or rewired as the first socket implementation.

## Execution Waves

### Wave 1: Contract Freeze

Deliverables:

- Finalize the Shell Socket capability list in `docs/workspace/shell_ui_projection_policy.md`.
- Create explicit request/response shapes for:
  - `submit_input`;
  - `subscribe_events`;
  - `get_message_window`;
  - `get_message_detail`;
  - `get_runtime_status`;
  - `search`;
  - `submit_issue`;
  - `list_agents`;
  - `list_sessions`;
  - `submit_approval_decision`;
  - `set_model`;
  - `set_git_tree`;
  - `submit_terminal_command`.
- Define payload budgets and forbidden fields for each default projection.
- Define detail refs for heavy payloads.

Acceptance:

- Contract can be implemented by browser, CLI, OpenClaw UI, or future shells.
- Contract does not mention Alpine, Svelte, DOM, localStorage, dashboard state, or browser globals.
- Contract calls Gateway route classes only.

### Wave 2: Gateway Route Mapping

Deliverables:

- Map every Shell Socket capability to a Gateway route class:
  - request ingress;
  - event/output egress;
  - health/status;
  - detail fetch;
  - bounded search/query;
  - approval/control ingress.
- Identify existing routes that can satisfy the contract.
- Identify missing routes that need Kernel, Orchestration, or Assurance backing.
- Add or update Gateway route docs before implementation.

Acceptance:

- Every socket capability has a Gateway route or explicit missing-route task.
- No socket capability depends on browser/dashboard state.
- Gateway remains the only external ambiguity boundary.

### Wave 3: Authority Backing

Deliverables:

- Kernel backs truth and durable state:
  - agent/session/message records;
  - model/provider/config/secrets;
  - terminal/file/git authority;
  - approvals and receipts.
- Orchestration backs coordination:
  - message execution;
  - workflow progress;
  - retry/replay/fork sequencing;
  - suggestions and update/restart workflows.
- Assurance backs proof/eval:
  - report issue;
  - telemetry alerts;
  - governance and amputation evidence.

Acceptance:

- Gateway routes return bounded projections from the correct authority owner.
- Gateway routes do not manufacture truth.
- Shell Socket does not import or call Kernel/Orchestration/Assurance directly.

### Wave 4: CLI/Headless Shell Plug

Deliverables:

- Build a minimal CLI/headless Shell plug against the Shell Socket Contract.
- It should prove:
  - list agents;
  - list sessions;
  - load a message window;
  - send a message;
  - stream workflow progress;
  - fetch message/detail refs;
  - approve/reject;
  - show runtime status;
  - submit eval/report issue;
  - run capability-gated terminal command where allowed.

Acceptance:

- CLI plug works without browser assets.
- CLI plug does not import dashboard code.
- CLI plug does not depend on localStorage, DOM, Svelte, Alpine, or browser event buses.
- CLI plug can be used as the first parity proof for new socket routes.

### Wave 5: Parity And Amputation Proof

Deliverables:

- Add parity checks comparing:
  - Gateway projection output;
  - CLI plug rendering/output;
  - legacy dashboard behavior where relevant.
- Add deletion/amputation checks proving browser Shell assets can be absent while:
  - Kernel builds/runs required checks;
  - Orchestration builds/runs required checks;
  - Gateway status routes work;
  - CLI plug can perform the core socket smoke.

Acceptance:

- Shell independence is proven by deletion, not assumption.
- Browser dashboard removal breaks only browser presentation.
- CLI plug provides the system-operable proof.

### Wave 6: Optional Legacy Dashboard Cutover

Deliverables:

- Only after CLI parity passes, choose one legacy dashboard seam at a time.
- Add a tiny shim from the legacy dashboard to the proven Gateway route.
- Keep old route fallback until parity holds.
- Remove old dashboard authority only after verified replacement.

Acceptance:

- No broad dashboard refactors.
- No multi-seam rewires in one batch.
- Every cutover has rollback/fallback.
- Every removed legacy path has a proven socket/Gateway replacement.

## Non-Goals

- Do not migrate the current dashboard to the socket as the first implementation.
- Do not use Orchestration as a direct shell backend.
- Do not create a stateful Shell Socket runtime service.
- Do not move authority into `client/**`.
- Do not use browser caches as runtime truth.
- Do not require OpenClaw UI to inherit legacy dashboard code.

## First Practical Build Target

The first implementation target should be:

```text
Gateway-backed Shell Socket contract
+ minimal CLI/headless Shell plug
+ parity smoke tests
```

This gives the project a clean proof harness before any further browser dashboard
work.

## Socket-Only Build Plan

The next build target is the Shell Socket infrastructure itself, not a UI.

The socket is the permanent Shell-facing infrastructure contract. It defines what
any Shell plug can ask for, what it can submit, what it can subscribe to, and
what it can fetch lazily. It does not render, cache full runtime state, own
workflow truth, or decide execution policy.

### Planned Artifacts

Create the socket path as independent artifacts:

- `shell/socket/README.md`: canonical directory contract explaining that
  Shell Socket 2.0 lives under `shell/socket/**`, not legacy `client/**`
  or Gateway/integration `adapters/**`.
- `shell/socket/contract/shell_socket_contract.json`: canonical capability list,
  request shapes, response shapes, forbidden fields, payload budgets, and detail
  ref rules for Shell plugs.
- `validation/conformance/contracts/shell_socket_gateway_contract.json`: mapping
  from every socket capability to Gateway route class, owner of truth, capability
  requirement, audit requirement, Nexus checkpoint, and payload budget.
- `tests/tooling/scripts/ci/shell_socket_contract_guard.ts`: fail-closed guard
  proving the socket contract stays projection-only, Gateway-only, and free of
  full-state/raw payload fields.
- `tests/tooling/scripts/ci/shell_socket_gateway_route_guard.ts`: fail-closed
  guard proving every socket capability has a declared Gateway route mapping and
  no direct Shell-to-Kernel or Shell-to-Orchestration path.
- `shell/socket/client/shell_socket_gateway_client.ts`: typed Gateway client for
  shell plugs. This is transport/client glue only; it must not hold canonical
  state or import dashboard code.
- `shell/socket/probe/shell_socket_headless_probe.ts`: minimal headless/CLI proof
  that exercises the socket without browser, DOM, Alpine, Svelte, localStorage,
  or the legacy dashboard.

The current browser dashboard must not be modified as part of the socket build
except for later critical compatibility shims after the headless socket path has
passing evidence.

### Socket Capability Contract

The first socket contract should include these capabilities:

| Capability | Route class | Owner of truth | Default response |
| --- | --- | --- | --- |
| `get_runtime_status` | health/status | Gateway shaped from owner status | bounded runtime status projection |
| `list_agents` | event/output egress | Kernel/Gateway projection | agent roster window |
| `list_sessions` | event/output egress | Kernel/Gateway projection | session window |
| `get_message_window` | event/output egress | Kernel/Gateway projection | message rows, cursors, detail refs |
| `get_message_detail` | detail fetch | Kernel/Orchestration/Assurance by detail kind | bounded detail projection |
| `submit_input` | request ingress | Orchestration through Gateway | ingress ack, receipt, event cursor |
| `subscribe_events` | event/output egress | Orchestration/Gateway projection | bounded event stream |
| `search` | bounded search/query | owner-backed Gateway search | hit projections and detail refs |
| `submit_issue` | request ingress | Assurance through Gateway | receipt and eval/report ref |
| `submit_approval_decision` | request ingress | Kernel/Orchestration through Gateway | receipt |
| `set_model` | request ingress | Kernel config through Gateway | receipt and active model projection ref |
| `set_git_tree` | request ingress | Kernel/git authority through Gateway | receipt and tree projection ref |
| `submit_terminal_command` | request ingress | Kernel capability gate through Gateway | receipt or stream ref |

Every capability must declare:

- stable request type;
- stable response type;
- maximum response bytes;
- maximum array size;
- maximum string size;
- cursor/window behavior;
- detail refs;
- forbidden fields;
- capability or lease requirement;
- audit receipt requirement;
- Nexus checkpoint requirement;
- Conduit/Scrambler posture.

### Minimal Projection Types

The socket must define a small set of projection families:

- `RuntimeStatusProjection`: readiness label, health enum, degraded reason, age,
  source, and retry hint.
- `AgentRosterProjection`: agent ids, names, status labels, avatar refs, unread
  counts, current session ids, and cursors.
- `SessionListProjection`: session ids, title previews, timestamps, status, model
  label, message counts, and cursors.
- `MessageWindowProjection`: bounded message rows, line counts, previews or
  content windows, origin labels, timestamps, status, action refs, and detail refs.
- `MessageDetailProjection`: bounded expansion for one message/tool/artifact/trace
  ref, capability-scoped and audited.
- `ShellEventProjection`: event id, kind, session id, display label, content delta
  or status projection, detail refs, cursor, and receipt refs.
- `BoundedSearchResults`: hit ids, snippets, labels, counts, cursors, and detail
  refs.
- `IngressAck`: accepted/rejected state, receipt id, correlation id, event cursor,
  and human-readable status label.

Forbidden default fields include `raw`, `root`, `full_state`, `all_messages`,
`conversation_tree`, `tool_input`, `tool_result`, `trace_body`,
`decision_trace`, `workflow_graph`, `execution_observation`, `runtime_quality`,
`eval_payload`, `artifact_body`, `file_output`, `folder_tree`,
`policy_decision`, and `authorization_state`.

### Build Waves For The Socket Itself

#### Socket Wave 1: Contract Artifacts

Deliver:

- `shell_socket_contract.json`;
- `shell_socket_gateway_contract.json`;
- policy references back to Gateway, Shell projection, message detail, and payload
  budget policies.

Acceptance:

- no UI code;
- no dashboard imports;
- every capability has a declared route class;
- every default response has a payload budget and forbidden-field list.

#### Socket Wave 2: Contract Guards

Deliver:

- `ops:shell:socket-contract:guard`;
- `ops:shell:socket-gateway-route:guard`;
- positive and controlled-negative fixtures.

Acceptance:

- guard fails if a socket capability has no Gateway route mapping;
- guard fails if a socket route allows direct Shell-to-Orchestration or
  Shell-to-Kernel calls;
- guard fails if default responses allow raw/full-state fields;
- guard fails if cursor, detail ref, audit, or Nexus requirements are missing.

#### Socket Wave 3: Typed Gateway Client

Deliver:

- transport-neutral TypeScript client in `adapters/runtime/**`;
- generated or declared TypeScript types from the contract;
- timeout, abort, and bounded error projection behavior.

Acceptance:

- client imports no dashboard/Svelte/Alpine/browser-global code;
- client keeps no canonical state;
- client exposes only contract methods;
- client routes only to Gateway endpoints declared in the contract.

#### Socket Wave 4: Headless Probe

Deliver:

- headless socket probe that can run without browser assets;
- smoke path:
  - runtime status;
  - agent list;
  - session list;
  - message window;
  - submit input;
  - event stream read;
  - message detail fetch;
  - internal issue/eval submission.

Acceptance:

- proves the system can display output and collect input through Gateway without
  the legacy browser dashboard;
- produces a JSON artifact and markdown report;
- no DOM, localStorage, Svelte, Alpine, or dashboard globals.

#### Socket Wave 5: Legacy Quarantine Hook

Deliver:

- `client/runtime/systems/ui/legacy_shell_manifest.json`;
- guard that blocks new legacy Shell features unless marked as critical fix,
  parity bridge, or retirement support.

Acceptance:

- Shell 1.0 is formally `LegacyBrowserShellPlug`;
- Shell 2.0 socket work can progress without editing Shell 1.0;
- deletion/parity conditions are explicit.

### First Code Wave Recommendation

Start with Socket Wave 1 and Wave 2 only.

Do not build BrowserShellV2 yet. Do not wire the old dashboard. Do not port UI
behavior. The first proof should be boring by design: a contract, a route map,
guards, and controlled-negative failures. Once those pass, build the typed client
and headless probe as the first real consumer.

## Cutover Rule

The legacy dashboard may be deleted or replaced only when the socket path proves
these minimum capabilities without browser assets:

- list agents;
- list sessions;
- load message window;
- send message;
- stream workflow progress;
- fetch detail refs;
- show status;
- approve/reject;
- submit eval/report issue.

Until then, keep the dashboard alive as compatibility and avoid surgery.
