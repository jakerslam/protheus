# Shell Authority Detachment Inventory

Status: Working inventory
Owner: Jay
Scope: Desktop UI shell 1.0, CLI replacement shells, OpenClaw UI attachment, Gateway contracts, Kernel/Orchestration ownership
Created: 2026-05-01

## Purpose

This document lists browser Shell responsibilities that are authority-shaped or system-critical enough that Infring must not depend on the desktop UI shell to provide them.

Use this as the pruning map for clean Shell detachment. If the current browser Shell were deleted, replaced by a CLI Shell, or replaced by the OpenClaw UI, every item in the "must exist outside Shell" column needs a non-browser owner and a stable Gateway/Nexus interface.

This inventory complements:

- `docs/workspace/shell_independent_operation_policy.md`
- `docs/workspace/shell_ui_projection_policy.md`
- `docs/workspace/gateway_ingress_egress_policy.md`
- `docs/workspace/interface_payload_budget_policy.md`
- `docs/workspace/shell_ui_message_detail_contract.md`

## Authority Definition

In this document, "authority" means any responsibility that would break the system, alter truth, alter permissions, mutate runtime state, or hide operational state if the browser Shell disappeared.

The Shell may own display and input. It must not own truth, admission, lifecycle, routing, retry, session persistence, workflow coordination, security decisions, or runtime payload shaping.

## Detachment Rule

For every row below:

```text
Shell should collect intent or render projection.
Gateway should validate, bound, audit, and expose the route.
Kernel should own truth and durable state.
Orchestration should own workflow progress and sequencing.
Assurance should own eval, issue triage, audit, and validation.
```

The Shell-side target is the Shell Socket Contract defined in
`docs/workspace/shell_ui_projection_policy.md`. Browser UI, terminal UI, OpenClaw
UI, and future shells should become peer Shell plugs that implement that contract
and call only Gateway routes.

The migration posture is the Parallel Socket Strategy from
`docs/workspace/shell_independent_operation_policy.md`: build and prove the socket
beside the legacy dashboard, treat `desktop UI shell 1.0` as tolerated
compatibility, and avoid touching it except for critical fixes or proven shims.
Execution plan: `docs/workspace/shell_socket_parallel_execution_plan.md`.

## Authority Inventory

| Area | Current Shell seam | Why it is authority-shaped | Must exist outside Shell | Proposed owner |
| --- | --- | --- | --- | --- |
| Agent roster and active session selection | `client/runtime/systems/ui/infring_static/js/app_agent_refresh_helpers.ts` fetches `/api/agents?view=sidebar&authority=runtime&compact=1`; `client/runtime/systems/ui/infring_static/js/app.ts` boot-selects agents, creates agents, archives agents, and revives agents. | The Shell currently buffers empty rosters, selects an active agent, creates/archives/revives agents, and preserves rows during transient backend failures. Those are UX-safe only if the authoritative roster and lifecycle state are elsewhere. | Canonical agent roster, lifecycle status, active-session intent endpoint, create/archive/revive commands, and stable sidebar projection. Shell-local active selection can remain a preference only. | Gateway projection over Kernel agent/session authority; Orchestration only when lifecycle requires coordination. |
| Chat session hydration and message windows | `client/runtime/systems/ui/infring_static/js/pages/chat.ts.parts/140-session-and-ws.part01.ts` loads `/api/agents/:id/session?limit=80`, older windows, session lists, session creation, and session switching. | Shell manages hydration timing, scroll determinism, session switching side effects, and message array replacement. A CLI/OpenClaw UI needs the same capabilities without browser render timing. | Bounded message-window endpoint, session list endpoint, switch/create session commands, cursor semantics, and detail refs. Shell must not be the only place that knows how to reconstruct a session. | Gateway message-window/detail contracts backed by Kernel session store. |
| Conversation cache and previews | `client/runtime/systems/ui/infring_static/js/pages/chat.ts.parts/050-conversation-cache.ts` persists `conversationCache`; `client/runtime/systems/ui/infring_static/js/pages/chat.ts.parts/192-slash-alias-and-alerts.ts` can route system messages into cached target rows. | Durable browser cache can become a second conversation store or preview authority. It can also hide bugs because chats appear from local cache even if the runtime route is broken. | Server-owned preview cache or projection endpoint with bounded message previews, unread state, tool summary counts, and detail refs. Browser cache may be a disposable performance cache only. | Kernel/Gateway projection. |
| Message send pipeline and transport fallback | `client/runtime/systems/ui/infring_static/js/pages/chat.ts.parts/200-send-pipeline.part01.ts` chooses WebSocket vs HTTP, creates thinking rows, runs auto-route preflight, tracks in-flight payloads, and formats response metadata. | The Shell decides fallback behavior and fabricates transient message rows. That is acceptable for display, but not as the command contract. Other shells need a single send-intent contract with progress/output projection. | Send-message command, accepted receipt, stream id, progress events, final assistant-message projection, failure semantics, and retryable error contract. | Gateway ingress plus Orchestration progress egress, backed by Kernel message persistence. |
| Terminal command execution | `client/runtime/systems/ui/infring_static/js/pages/chat.ts.parts/199-system-terminal-helpers.ts` creates terminal sessions and queues commands through `/api/terminal/sessions` and `/api/terminal/queue`; `client/runtime/systems/ui/infring_static/js/pages/chat.ts.parts/200-send-pipeline.part01.ts` sends agent terminal commands. | Terminal execution is a powerful mutation path. The browser should never be the authority deciding permission, cwd truth, command translation, or execution receipt. | Terminal session lifecycle, command queue, permission gate, executed command receipt, stdout/stderr projection, cwd tracking, and audit record. | Kernel command authority and Gateway capability boundary; Orchestration if command is part of a workflow. |
| Workflow and thinking telemetry | Chat thinking rows and status fallback live in `client/runtime/systems/ui/infring_static/js/pages/chat.ts.parts/140-session-and-ws.part01.ts` and related chat status helpers. | The Shell currently fills gaps with display strings such as waiting/analyzing. If workflow stage and inner-dialog telemetry are only reconstructed in the browser, other shells cannot present transparent progress. | Workflow stage projection, current step label, progress detail ref, optional inner-dialog display projection, timestamp, and source receipt. | Orchestration owns progress; Gateway exposes bounded egress projection. |
| Message retry, reply, quote, and fork | `client/runtime/systems/ui/infring_static/js/pages/chat.ts.parts/215-rendering-and-metadata-upgrades.ts` still has Shell metadata controls, including retry/reply placeholders and clone/fork behavior through `/api/agents/:id/clone`. | Retry/reply semantics require authoritative replay, quote-by-reference, context window choice, and deletion/retry receipts. Browser-only logic would fork history incorrectly or silently diverge. | Backend replay contract, quote-by-reference contract, fork/clone command, allowed actions projection, and audit receipts. | Kernel session authority plus Orchestration for replay/fork sequencing; Gateway for allowed actions. |
| Model/provider registry and active model selection | `client/runtime/systems/ui/infring_static/js/pages/settings.ts` loads `/api/providers`, `/api/models`, writes custom models, provider URLs, provider keys, and config; chat init writes `/api/agents/:id/model`. | Provider configuration and active model selection are runtime authority, not UI. Shell may choose from options, but cannot be the only place enforcing model validity, context windows, keys, or provider reachability. | Provider/model registry, model capability projection, active model command, custom model CRUD, provider key management, provider URL validation, and receipts. | Kernel config/secret authority through Gateway. |
| Agent initialization contract | `client/runtime/systems/ui/infring_static/js/pages/chat.ts.parts/130-render-window-and-session.part01.ts` builds role, identity, system prompt, archetype, permission manifest, lifecycle/expiry, and config payloads for `/api/agents/:id/config`. | This is one of the strongest authority smells. The Shell is assembling identity, prompts, permissions, expiry, and lifecycle choices. Those are agent contract inputs and need policy validation outside the browser. | Agent initialization command with schema, template refs, permission manifest validation, lifespan policy, prompt/identity receipt, and launch result projection. | Kernel owns agent contract truth; Orchestration may own launch sequencing; Gateway validates ingress. |
| Git tree/workspace context selection | `client/runtime/systems/ui/infring_static/js/pages/chat.ts.parts/020-init-roles-and-vibes.part01.ts` fetches `/api/agents/:id/git-trees` and posts `/api/agents/:id/git-tree/switch`. | Workspace branch/tree switching changes agent execution context. The Shell can display options, but branch validation, collision policy, and worktree ownership must live outside it. | Workspace tree list projection, switch intent, require-new semantics, conflict response, receipt, and refreshed agent context. | Kernel workspace authority through Gateway. |
| Approvals and capability decisions | `client/runtime/systems/ui/infring_static/js/pages/approvals.ts` loads `/api/approvals` and posts `/api/approvals/:id/approve|reject`. | Approval decisions grant or deny sensitive actions. The Shell may present buttons, but the approval queue, capability scope, expiry, and decision receipt must remain authoritative elsewhere. | Approval queue projection, approve/reject commands, capability metadata, expiration, actor identity, and immutable receipt. | Kernel/Governance authority with Gateway decision ingress. |
| Comms topology, messages, and tasks | `client/runtime/systems/ui/infring_static/js/pages/comms.ts` consumes `/api/comms/topology`, `/api/comms/events`, `/api/comms/events/stream`, and posts `/api/comms/send` plus `/api/comms/task`. | Inter-agent messaging and task posting are system operations. Shell topology display is UI, but message routing, task creation, event history, and stream ordering are not. | Topology projection, bounded event stream, send-message command, post-task command, event receipts, and replay cursors. | Orchestration/Comms authority exposed through Gateway egress/ingress. |
| Runtime status, connectivity, restart/update/shutdown | `client/runtime/systems/ui/infring_static/js/app_status_helpers.ts` reads `/api/status` and `/api/version`; `client/runtime/systems/ui/infring_static/js/app.ts` also posts dashboard actions and legacy `/api/system/restart|shutdown|update`. | Status and system controls must not be inferred from browser connectivity alone. Restart/update/shutdown are high-impact control-plane actions. | Health/status projection, connectivity state, release/update state, restart/shutdown/update command contracts, actor authorization, and receipts. | Gateway status/control endpoints backed by Kernel/Orchestration. |
| Auth/session and API key handling | `client/runtime/systems/ui/infring_static/js/api.ts` stores auth token in memory, removes `infring-api-key` on 401, and shows auth UI; `client/runtime/systems/ui/infring_static/js/app_auth_helpers.ts` calls `/api/auth/check`, `/api/auth/login`, `/api/auth/logout`, `/api/config`, and `/api/tools`. | Auth truth cannot live in the browser. Browser token UX is fine, but authorization, session validity, key storage, and tool visibility must be authoritative outside Shell. | Login/logout/check routes, session/token lifecycle, tool visibility projection, auth failure semantics, and secret storage. | Gateway auth boundary plus Kernel secret/session authority. |
| Suggestions and prompt assistance | Chat suggestion helpers call `/api/agents/:id/suggestions` and maintain Shell toggles for prompt suggestions. | Suggestions are not core authority, but if they consume context or trigger background LLM work, generation should be disabled/enabled server-side, not only hidden in UI. | Suggestion enablement preference, bounded suggestion projection, generation suppression flag, and context budget. | Gateway preference/command plus Orchestration suggestion route. |
| Search and history lookup | `client/runtime/systems/ui/infring_static/js/app_chat_sidebar_search_helpers.ts` queries `/api/search/conversations?q=...&limit=80`; Shell also has local filters and chat sidebar sort/topology state. | Search should not require browser-held full histories. Sidebar order can be presentation, but query results and relevance should be backend projection. | Bounded conversation search endpoint, result previews, refs, cursors, and search scope policy. | Gateway search projection over Kernel index/store. |
| Telemetry, eval feedback, and report issue | Chat code calls `/api/telemetry/alerts` and `/api/agents/:id/eval-feedback/report-issue`. | Eval/reporting should be Assurance-owned. The Shell should not decide issue context, severity, or evaluation target beyond sending a scoped report intent. | Eval-report command with context window rule, issue context projection, Assurance receipt, and optional GitHub submission policy. | Assurance through Gateway ingress. |
| File/folder reads and upload attachments | Chat uses `/api/agents/:id/upload`, `/api/agents/:id/file/read`, and `/api/agents/:id/folder/export`; large paste conversion currently happens in Shell. | Filesystem/file content access is sensitive. Browser may collect attachments, but read/export authority, size limits, redaction, and receipts must be outside Shell. | Upload route, file read/export route, payload limits, capability checks, sanitized projections, and detail refs. | Gateway file boundary plus Kernel capability authority. |
| Slash command aliases and command discovery | `client/runtime/systems/ui/infring_static/js/pages/chat.ts.parts/192-slash-alias-and-alerts.ts` stores slash aliases in localStorage and resolves aliases before command handling. | User-defined aliases are UI convenience. They become authority-shaped if command availability, permissions, or expansion semantics are only in the Shell. | Command registry projection, command execution endpoint, optional user alias preference, and server-side permission validation after expansion. | Gateway command registry plus Kernel/Orchestration command handlers. |

## Layer Relocation Map

This section groups the same items by where the non-Shell responsibility should live. Some rows intentionally appear in more than one layer because a clean contract usually has one truth owner plus one projection/transport owner.

### Kernel

Kernel owns truth, durable state, admission, permission, and receipts. These items should land primarily in Kernel authority, with Gateway routes exposing only bounded projections or commands:

| Item | Kernel responsibility | Supporting layers | Shell after detachment |
| --- | --- | --- | --- |
| Agent roster and lifecycle | Canonical agent records, lifecycle state, create/archive/revive truth, active session references, immutable lifecycle receipts. | Gateway exposes roster/lifecycle projections and lifecycle commands; Orchestration coordinates launches when needed. | Render roster projection; send create/archive/revive intent. |
| Chat sessions and message persistence | Canonical session records, message records, cursor/window truth, message IDs, detail refs, and durable transcript receipts. | Gateway exposes bounded windows/detail fetches; Orchestration emits progress/final output. | Render message windows; request details by ref. |
| Conversation previews | Server-owned preview rows, unread state, latest-message summaries, tool/artifact counts, and preview invalidation. | Gateway exposes preview projections. | Optional disposable cache only. |
| Terminal execution authority | Terminal session truth, cwd truth, command admission, permission gates, executed command receipts, output records. | Gateway enforces ingress/capability limits; Orchestration can sequence workflow-bound commands. | Collect command text; render output projection. |
| Model/provider/config registry | Provider registry, model registry, custom model records, provider URL/key state, config mutation receipts, secret storage. | Gateway validates config/provider ingress and exposes safe projections. | Render settings forms; send config intents. |
| Agent initialization contract | Agent identity/prompt/template/lifespan/permission-manifest validation, launch contract, and init receipt. | Orchestration can coordinate launch; Gateway validates init request shape. | Send template/ref/intent, not final authority payload. |
| Git tree/workspace context | Worktree/branch records, collision policy, branch switch truth, workspace ownership receipts. | Gateway exposes git-tree list/switch commands. | Render choices; send switch intent. |
| Approval decisions | Approval queue truth, capability scope, expiry, actor identity, approve/reject receipts. | Gateway exposes approval projection and decision ingress; Assurance observes/audits. | Render approve/reject controls. |
| Runtime/system controls | Restart/shutdown/update command authority, health truth, release/update state, actor authorization. | Gateway exposes status/control routes; Orchestration may sequence update/restart operations. | Render status; send control intent. |
| Auth/session/secrets | Auth session truth, token lifecycle, authorization, API/provider key storage, tool visibility by capability. | Gateway owns ingress enforcement and auth challenge/response. | Collect credentials/tokens only through Gateway flow. |
| File/folder/upload access | File read/export authority, upload storage refs, redaction, payload limits, capability checks, receipts. | Gateway file boundary; Assurance can inspect suspicious/report flows. | Collect file selection; render sanitized projections. |
| Command registry execution | Canonical command definitions, allowed command scopes, server-side alias expansion if aliases become operator prefs, command execution receipts. | Gateway exposes command registry and command ingress; Orchestration handles workflow commands. | Render command help; optionally expand local aliases as display convenience. |

### Orchestration Control Plane

Orchestration owns sequencing, workflow progress, recovery, routing, and "what happens next" when a request is more than a single Kernel mutation.

| Item | Orchestration responsibility | Supporting layers | Shell after detachment |
| --- | --- | --- | --- |
| Message send pipeline | Route user intent into agent/workflow execution, emit accepted/progress/final events, handle fallback/recovery sequencing. | Kernel persists messages and receipts; Gateway carries send ingress/progress egress. | Send one intent; render stream/progress. |
| Workflow and thinking telemetry | Own current stage, step labels, progress source, recovery state, and optional inner-dialog display projection. | Gateway exposes bounded telemetry stream; Assurance may audit. | Render shimmer/status text from projection. |
| Agent launch sequencing | Coordinate initialization, tool/bootstrap work, lifecycle transition from draft to running, and failure recovery. | Kernel validates/records agent contract; Gateway validates ingress. | Render launch state and errors. |
| Retry/replay/fork sequencing | Coordinate replay, context selection, quote-by-reference, fork/clone flow, and deletion/retry ordering. | Kernel owns transcript truth and receipts; Gateway exposes allowed actions. | Render allowed controls; send action intent. |
| Comms tasks and inter-agent messages | Route messages/tasks between agents, maintain event ordering, task claim/completion transitions. | Kernel persists task/event truth; Gateway streams topology/events. | Render topology/events; send message/task intent. |
| Prompt suggestions | Generate suggestions, respect enabled/disabled preference, and account for context budget. | Gateway exposes preference and bounded suggestion projection; Kernel may store preference. | Render/hide suggestions. |
| Terminal commands inside workflows | Sequence command execution as part of agent/workflow plans and recover from blocked/failed commands. | Kernel owns permission and command receipts. | Render workflow-bound terminal events. |
| System update/restart workflows | Coordinate shutdown/update/restart plans and user-visible progress. | Kernel owns command authority; Gateway exposes operator control route. | Render progress and final state. |

### Gateways

Gateways own bounded ingress/egress between external shells and internal authority. They should not own truth, but they should enforce authentication, authorization, payload limits, projection shaping, rate limits, detail access, and audit receipts at the boundary.

| Item | Gateway responsibility | Backing authority | Shell after detachment |
| --- | --- | --- | --- |
| Agent roster projection | Return compact/sidebar/full roster projections with clear budgets, cursors, and stale/empty semantics. | Kernel agent/session store. | Render rows only. |
| Message window and detail routes | Return bounded message windows, cursors, line counts, allowed actions, detail refs, and lazy detail payloads. | Kernel transcript store; Orchestration progress stream. | Render window; fetch detail on expand. |
| Send-message ingress | Accept user input, attachments refs, mode flags, and return accepted receipt/stream id. | Orchestration and Kernel. | Submit input; subscribe to output. |
| Workflow progress egress | Stream stage label, current step, source, timestamps, and optional inner-dialog projection. | Orchestration. | Render status line. |
| Model/provider/config routes | Validate model/provider/config mutations, enforce payload/secret policy, return safe status projections. | Kernel config/secrets. | Render forms/options. |
| Git tree routes | List workspace tree options and accept switch intents with conflict/receipt semantics. | Kernel workspace authority. | Render menu/options. |
| Approval ingress/egress | Expose approval queue and accept approve/reject decisions with actor/capability receipts. | Kernel/Governance. | Render buttons. |
| Auth routes | Handle login/logout/check, token/session state, and challenge responses. | Kernel auth/secrets. | Collect auth input. |
| File/upload/detail routes | Enforce file read/upload/export budgets, redaction, capability scope, and detail refs. | Kernel file/capability authority. | Render sanitized file/detail output. |
| Search routes | Return bounded search hits, result refs, cursors, and relevance summaries without full transcript payloads. | Kernel index/store. | Render search results. |
| Runtime status/control routes | Return health/version/release projection and accept authorized control intents. | Kernel/Orchestration. | Render controls/status. |

### Assurance

Assurance owns proof, observation, evaluation, issue analysis, and governance. It should receive scoped context and return receipts/projections, not rely on browser state.

| Item | Assurance responsibility | Supporting layers | Shell after detachment |
| --- | --- | --- | --- |
| Eval/report issue | Evaluate a scoped context window, classify issue, produce diagnostic receipt, optionally prepare external issue submission under separate authorization. | Gateway accepts report intent; Kernel supplies transcript/detail refs. | Hazard/report button sends scoped intent. |
| Telemetry alerts | Own alert interpretation, severity, auditability, and bounded alert projections. | Gateway exposes alert projection. | Render alerts. |
| Long-chat memory/heap proof | Prove Shell projection boundaries, payload budgets, and deletion/amputation behavior. | Gateway/Kernel fixtures provide deterministic projections. | No runtime dependency; Shell only subject under test. |
| Approval/governance audit | Observe approval decisions, capability use, and policy violations. | Kernel/Gateway provide receipts. | Render audit state if requested. |

### CLI / Headless Shell

CLI should be a first-class shell, not an emergency workaround. It should consume the same Gateway/Nexus contracts as the browser Shell and prove the system can operate without browser assets.

| Capability | CLI responsibility | Backing layers |
| --- | --- | --- |
| Agent/session list | Display roster/session projections and accept explicit target IDs. | Gateway over Kernel. |
| Chat send/load | Load bounded windows, send messages, stream progress, fetch details. | Gateway over Kernel/Orchestration. |
| Approvals | List pending approvals and submit approve/reject decisions. | Gateway over Kernel/Governance. |
| Config/model/git controls | Display options and submit validated intents. | Gateway over Kernel. |
| Terminal | Queue commands and stream output through capability-gated route. | Gateway over Kernel/Orchestration. |
| Status/control | Show health/version/release state and trigger authorized controls. | Gateway over Kernel/Orchestration. |
| Eval/report issue | Submit scoped report issue requests and show Assurance receipt. | Gateway over Assurance. |

### Shell-Local Only

These should remain in Shell and do not need Kernel/Orchestration ownership unless they become cross-shell user preferences:

| Item | Why Shell-local is enough | If promoted later |
| --- | --- | --- |
| Wallpaper, glass, theme, visual styling | Pure presentation; deleting it cannot change runtime truth. | Store as display settings through Gateway only if shared across shells. |
| Dock/taskbar/chat-map/chat-bar placement | Pure layout preference. | Store as operator display profile if multi-device sync matters. |
| Hover popup placement and animation | Pure presentation behavior. | Keep per-shell. |
| Draft input and focus/menu state | Ephemeral UI state. | Do not promote unless draft sync becomes an explicit feature. |
| One-shell onboarding/tip dismissal | Presentation convenience. | Promote only as operator preference. |
| Local slash aliases | Safe only if validated after expansion by backend command registry. | Promote to Kernel operator preferences if aliases should follow the user across shells. |
| Disposable preview cache | Performance-only and delete-safe. | Replace with Gateway preview projection if it affects correctness. |

## Priority Split

The cleanest detachment path is not "move everything at once." It is:

1. Kernel first: agent/session/message truth, model/config/secrets, file/git/terminal authority, approvals.
2. Gateway second: bounded projection and command contracts for every shell-facing default view.
3. Orchestration third: send pipeline, workflow progress, retry/replay/fork sequencing, suggestions, update/restart flows.
4. Assurance fourth: report issue, telemetry alerts, governance proofs, heap/amputation guards.
5. CLI fifth: parity shell over the same Gateway contracts.
6. Browser Shell last: delete or simplify any code now replaced by those contracts.

## Safe Shell-Local State

These are safe to keep per-shell, because the system should still work if they vanish:

- wallpaper/background selection;
- glass/fog/warped visual style;
- dock/taskbar/chat-map/chat-bar position and tile order;
- hover popup placement;
- input focus, menu open/closed state, and draft text;
- display-only sort mode, as long as server projections remain independently queryable;
- dark/light theme preference;
- one-shell-only onboarding/tip dismissal;
- local slash aliases, if treated as a convenience layer and validated after expansion;
- disposable preview caches that can be deleted without losing runtime truth.

## High-Risk Existing Debt

These are the highest-risk places to inspect before Shell detachment:

1. `client/runtime/systems/ui/infring_static/js/pages/chat.ts` and `client/runtime/systems/ui/infring_static/js/pages/chat.ts.parts/**`
   - It still contains session hydration, send pipeline, cache, message normalization, terminal, suggestions, workflow display, retry/fork UI, file reads, git tree switching, agent init, and telemetry/report issue seams.
2. `client/runtime/systems/ui/infring_static/js/app.ts` and `client/runtime/systems/ui/infring_static/js/app_*_helpers.ts`
   - It still contains global app store, agent roster, boot selection, taskbar/dock, status, auth, system actions, and sidebar behavior.
3. `client/runtime/systems/ui/infring_static/js/pages/settings.ts`
   - It still exposes provider/model/config/key mutation controls that must remain Gateway/Kernel-owned.
4. `client/runtime/systems/ui/infring_static/js/api.ts`
   - It is the browser transport wrapper. It is allowed as Shell glue, but any replacement shell needs equivalent Gateway client behavior without DOM/Alpine assumptions.
5. `client/runtime/systems/ui/infring_static/js/shell/app_store_shell_services.ts`
   - It is a bridge between legacy app state and Svelte islands. It should stay projection-only and must not become a durable state owner.

## Detachment Acceptance Criteria

The system is cleanly detachable when a CLI Shell or OpenClaw UI can do all of the following without loading browser Shell assets, browser global state, localStorage, DOM APIs, Svelte bundles, or Alpine state:

1. List agents and sessions through bounded Gateway projections.
2. Select or address an agent/session by ID without relying on browser active-agent state.
3. Load a bounded message window plus cursors and detail refs.
4. Send a user message and receive accepted/progress/final projections.
5. Stream workflow stage and optional inner-dialog display projection.
6. Fetch message/tool/artifact/trace details lazily by ref.
7. Create/archive/revive agents with receipts.
8. Initialize an agent from a validated backend contract, not browser-built authority.
9. Select model/provider/git tree through Gateway commands with receipts.
10. Approve/reject capability requests through an authoritative approval queue.
11. Run or queue terminal commands through capability-gated routes.
12. Submit eval/report-issue requests to Assurance without GitHub submission side effects unless separately authorized.
13. Read health/status/version/release state without inferring truth from browser connectivity.
14. Authenticate/logout and manage provider keys without browser-local secret authority.
15. Search conversations without requiring full browser-held histories.

## Recommended Extraction Order

1. Freeze browser Shell behavior to blocker fixes only.
2. Write or verify a CLI parity smoke for agent list, session load, message send, progress stream, and details fetch.
3. Define one Gateway projection contract for each default Shell view: agent roster, chat window, session list, runtime status, approvals, model registry, git tree list.
4. Move agent initialization payload building into a backend contract: Shell sends template/ref/intent, not prompt/permissions/lifespan authority.
5. Move retry/reply/fork/replay semantics into backend commands with receipts.
6. Make browser conversation cache disposable preview-only and prove deletion does not change loaded chat truth.
7. Remove or quarantine browser-only localStorage mirrors that can masquerade as runtime state.
8. Add a Shell-deletion fixture that proves CLI/Gateway/Core/Orchestration still operate.

## Open Questions

- Which Gateway route should become the canonical "message window plus progress stream" contract for every shell?
- Should active agent/session selection be fully caller-supplied on every command, or should Kernel expose an operator-scoped active session preference?
- Should slash aliases remain per-shell display preferences, or become operator preferences exposed by Gateway?
- Should Shell-side system messages be eliminated entirely in favor of event projections and notification receipts?
- What subset of Settings belongs in CLI before a new clean Shell exists?
