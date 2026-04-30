# Shell Purge Authority Leak Ledger

Generated: 2026-04-30
Scope: temporary working ledger for shell/client authority leaks found during the shell purge.
Status legend: `queued`, `in_progress`, `blocked`, `done`.

This is not a canonical architecture spec. It is a burn-down ledger for moving authority out of the shell while preserving behavior.

## Boundary Rule

- Core decides what is true and allowed.
- Orchestration decides what should happen next: decomposition, coordination, sequencing, recovery, packaging.
- Shell decides how it is shown and collected.

A row is not `done` until the authority has moved to the correct layer, the shell is a thin projection or input wrapper, and targeted validation passes.

## Wave Plan

- Wave 1: chat-path authority leaks that can alter visible conversation behavior.
- Wave 2: workflow, terminal, agents, and hands coordination leaks.
- Wave 3: settings, security, status, and telemetry authority leaks.

## Ledger

### SHP-AUTH-001 - Chat send path owns recovery and failover decisions

Status: in_progress
Priority: P0
Current files:
- `client/runtime/systems/ui/infring_static/js/pages/chat_send_payload_helpers.ts`
- `client/runtime/systems/ui/infring_static/js/pages/chat_ws_response_event_helpers.ts`
- `client/runtime/systems/ui/infring_static/js/pages/chat_ws_error_event_helpers.ts`
- `client/runtime/systems/ui/infring_static/js/pages/chat_model_failover_helpers.ts`

Leak:
The shell classifies recoverable backend failures, chooses failover candidates, switches models, retries messages, clears pending request state, and emits recovery notices. That is recovery coordination and provider routing, not presentation.

Target owner:
- Orchestration: recovery plan, retry/escalation recommendation, provider/model failover candidate ordering.
- Core/runtime: admissibility and authoritative model/provider state.
- Shell: render recovery telemetry and submit explicit user/LLM requests only.

Action:
Create a Rust orchestration recovery coordinator that accepts a structured failure receipt and returns a recovery plan. Change shell helpers to render plan telemetry and call a backend endpoint rather than deciding failover locally.

Acceptance:
- Shell no longer contains recoverable-backend marker tables or automatic failover candidate selection.
- Visible chat text is not system-injected during recovery.
- Regression covers backend failure -> orchestration recovery plan -> shell telemetry projection.

### SHP-AUTH-002 - Shell slash commands directly route tool and system actions

Status: queued
Priority: P0
Current files:
- `client/runtime/systems/ui/infring_static/js/pages/chat_slash_command_helpers.ts`
- `client/runtime/systems/ui/infring_static/js/pages/chat_slash_alias_helpers.ts`
- `client/runtime/systems/ui/infring_static/js/pages/chat_slash_telemetry_helpers.ts`
- `client/runtime/systems/ui/infring_static/js/pages/chat_slash_apikey_helpers.ts`
- `client/runtime/systems/ui/infring_static/js/pages/chat_memprobe_helpers.ts`

Leak:
The shell interprets slash commands, routes file/folder reads, status, budget, peers, A2A, model switching, memory hygiene, optimization, and compaction. Several command handlers also inject system role messages into the chat transcript.

Target owner:
- Orchestration: command interpretation, command-to-workflow/tool-family mapping, non-canonical coordination.
- Core/runtime: actual permission/admission and authoritative command execution.
- Shell: command palette UI, user input capture, result projection.

Action:
Replace direct command execution with a single command-dispatch request to orchestration/runtime. Move command registry metadata out of shell except view labels. Convert visible command output to telemetry cards or LLM-visible tool results, not system chat text.

Acceptance:
- No direct `/file`, `/folder`, `/budget`, `/peers`, `/a2a`, `/compact`, or `/model` execution logic remains in shell.
- Slash commands that produce content return structured receipts.
- Shell renders receipts without adding assistant/system prose.

### SHP-AUTH-003 - Prompt queue truth is owned by the shell

Status: queued
Priority: P0
Current files:
- `client/runtime/systems/ui/infring_static/js/pages/chat_prompt_queue_helpers.ts`
- `client/runtime/systems/ui/infring_static/js/pages/chat_queue_processing_helpers.ts`
- `client/runtime/systems/ui/infring_static/js/pages/chat_send_message_helpers.ts`
- `client/runtime/systems/ui/infring_static/js/pages/chat_send_payload_helpers.ts`

Leak:
The shell creates queue IDs, owns queue ordering, injects steer prompts into active workflows, chooses websocket versus HTTP fallback, and requeues failed steering. Queue truth and workflow steering are not presentation concerns.

Target owner:
- Core/runtime: authoritative queue state and execution admission.
- Orchestration: sequencing and steer coordination.
- Shell: local drag/drop projection and request submission.

Action:
Move queue mutation and steer injection behind backend queue APIs. Keep local drag/drop as optimistic UX only, reconciled against backend queue snapshots.

Acceptance:
- Shell queue rows carry backend IDs only.
- Shell cannot execute queued prompt/steer work directly.
- Regression covers queue reorder, steer injection, and failure requeue through backend receipts.

### SHP-AUTH-004 - Terminal execution path performs coordination and recovery hints in shell

Status: queued
Priority: P0
Current files:
- `client/runtime/systems/ui/infring_static/js/pages/chat_terminal_session_helpers.ts`
- `client/runtime/systems/ui/infring_static/js/pages/chat_terminal_compose_helpers.ts`
- `client/runtime/systems/ui/infring_static/js/pages/chat_ws_terminal_event_helpers.ts`
- `client/runtime/systems/ui/infring_static/js/pages/chat_message_source_run_helpers.ts`

Leak:
The shell creates terminal sessions, retries missing sessions, chooses agent/system terminal path, handles permission gate payloads, translates command metadata, summarizes tool execution, and renders deterministic recovery hints.

Target owner:
- Core/runtime: command admission, permission gate, terminal session truth, execution receipt.
- Orchestration: terminal workflow coordination and recovery suggestion packaging.
- Shell: terminal input widget and receipt projection.

Action:
Replace shell terminal send/retry logic with a backend terminal-intent endpoint. Normalize terminal receipts upstream so the shell only renders prompt, output, status, and telemetry fields.

Acceptance:
- Shell no longer retries terminal sessions or interprets permission gates.
- Terminal errors do not generate system-authored chat text.
- Regression covers user command, agent command, blocked command, and low-signal recovery telemetry.

### SHP-AUTH-005 - Tool completion and result truth are inferred in shell

Status: queued
Priority: P1
Current files:
- `client/runtime/systems/ui/infring_static/js/pages/chat_assistant_text_signal_helpers.ts`
- `client/runtime/systems/ui/infring_static/js/pages/chat_response_tool_payload_helpers.ts`
- `client/runtime/systems/ui/infring_static/js/pages/chat_ws_tool_event_helpers.ts`
- `client/runtime/systems/ui/infring_static/js/pages/chat_tool_summary_helpers.ts`
- `client/runtime/systems/ui/infring_static/js/pages/chat_tool_card_helpers.ts`

Leak:
The shell determines whether a response has authoritative tool completion, builds tool rows, normalizes tool summaries, and can influence whether an empty/failure response is treated as usable.

Target owner:
- Core/runtime: canonical tool receipts and success/error/blocked status.
- Orchestration: result packaging for downstream consumption.
- Shell: render already-normalized tool cards.

Action:
Introduce a normalized tool transcript/receipt envelope from backend. Delete shell-side authoritative tool-completion inference and keep only display formatting.

Acceptance:
- Shell does not contain `authoritative` completion heuristics.
- Empty assistant text with tool receipts is represented by structured receipt state, not shell inference.
- Regression covers success, error, blocked, low-signal, and no-output tool receipts.

### SHP-AUTH-006 - System-authored chat text still exists in shell helpers

Status: queued
Priority: P0
Current files:
- `client/runtime/systems/ui/infring_static/js/pages/chat_notice_message_helpers.ts`
- `client/runtime/systems/ui/infring_static/js/pages/chat_ws_error_event_helpers.ts`
- `client/runtime/systems/ui/infring_static/js/pages/chat_ws_phase_event_helpers.ts`
- `client/runtime/systems/ui/infring_static/js/pages/chat_model_usage_notice_helpers.ts`
- `client/runtime/systems/ui/infring_static/js/pages/chat_proactive_telemetry_helpers.ts`
- `client/runtime/systems/ui/infring_static/js/pages/chat_agent_lifecycle_helpers.ts`
- `client/runtime/systems/ui/infring_static/js/pages/chat_slash_command_helpers.ts`

Leak:
The shell can push `role: system` rows into the visible chat transcript for notices, errors, slash output, lifecycle events, context warnings, model usage, and telemetry alerts. This risks violating the final-response policy: visible assistant/chat text must come from LLM finalization or user-authored content, while diagnostics belong in telemetry/attention surfaces.

Target owner:
- Orchestration/runtime: diagnostic event stream and workflow telemetry.
- Shell: separate notice rail, thinking bubble, tool trace, or non-chat diagnostic panel.

Action:
Split visible chat messages from non-chat system notices. Make `pushSystemMessage` write to telemetry/notice state only unless a backend LLM-authored final message is explicitly marked as chat-visible.

Acceptance:
- No shell helper can add user-visible chat prose with `role: system` as an assistant substitute.
- Existing notices still appear in UI outside the chat transcript.
- Regression covers fallback/finalization edge with no injected visible text.

Progress:
- 2026-04-30: Committed ledger, then started the shell-side hardening pass.
- `pushSystemMessage` now records diagnostics to `systemTelemetry` only and no longer supports visible chat injection.
- First direct visible system-message sites were converted to notice/telemetry projection in chat recovery, slash, prompt queue, context warning, session load, fresh-init, lifecycle, and voice-transcription helpers.
- Added Rust orchestration packaging primitives in `surface/orchestration/src/control_plane/result_shaping_packaging.rs`: runtime diagnostics package as telemetry-only by default, and only final LLM output packages as chat-visible.

### SHP-AUTH-007 - Workflow builder compiles and persists executable workflow semantics in shell

Status: queued
Priority: P1
Current files:
- `client/runtime/systems/ui/infring_static/js/pages/workflow-builder.ts`
- `client/runtime/systems/ui/infring_static/js/pages/workflow_builder_persist_trace_helpers.ts`
- `client/runtime/systems/ui/infring_static/js/pages/workflow_builder_canvas_helpers.ts`
- `client/runtime/systems/ui/infring_static/js/pages/workflows.ts`

Leak:
The shell translates graph nodes into workflow steps, emits TOML, assigns modes like fan-out/conditional/loop, chooses next edges, and persists executable workflow shape. That is workflow compilation and control-plane semantics.

Target owner:
- Orchestration: workflow JSON schema, validation, graph compilation, sequencing semantics.
- Shell: graph editor and trace viewer.

Action:
Move graph-to-workflow compilation into orchestration. Shell submits graph JSON, receives validation diagnostics and compiled workflow preview.

Acceptance:
- Shell no longer generates executable workflow steps or TOML.
- Invalid graph returns structured validation errors from orchestration.
- Regression covers graph compile for sequential, conditional, fan-out, loop, and collect nodes.

### SHP-AUTH-008 - Agent lifecycle/archive operations are coordinated in shell

Status: queued
Priority: P1
Current files:
- `client/runtime/systems/ui/infring_static/js/pages/agents_lifecycle_archive_helpers.ts`
- `client/runtime/systems/ui/infring_static/js/pages/agents_detail_control_helpers.ts`
- `client/runtime/systems/ui/infring_static/js/pages/chat_agent_lifecycle_helpers.ts`
- `client/runtime/systems/ui/infring_static/js/pages/chat_agent_selection_helpers.ts`

Leak:
The shell infers active/archived/terminated lifecycle state, performs archive/delete batches, revives records, activates system threads, and decides when expired agents should be rebound.

Target owner:
- Core/runtime: authoritative agent lifecycle state.
- Orchestration: batch lifecycle coordination and rebind recommendations.
- Shell: lifecycle display and action requests.

Action:
Move lifecycle batch operations and rebind planning to backend APIs that return receipts. Keep shell action buttons and local selection reconciliation only.

Acceptance:
- Shell cannot decide agent lifecycle truth or perform multi-agent state transitions locally.
- Rebind behavior comes from backend receipt/plan.
- Regression covers missing agent, archived agent, revive, and batch archive flows.

### SHP-AUTH-009 - Model/provider configuration and guidance are partly shell-owned

Status: queued
Priority: P1
Current files:
- `client/runtime/systems/ui/infring_static/js/pages/chat_model_guidance_helpers.ts`
- `client/runtime/systems/ui/infring_static/js/pages/chat_model_failover_helpers.ts`
- `client/runtime/systems/ui/infring_static/js/pages/settings_view_provider_helpers.ts`
- `client/runtime/systems/ui/infring_static/js/pages/wizard.ts`

Leak:
The shell generates no-models guidance, resolves model catalog options, writes provider keys and URLs, polls OAuth, tests providers, and can advise recovery. Provider/model authority and credential mutation should not live in presentation logic.

Target owner:
- Core/runtime: provider config truth, credentials, tests, and capability state.
- Orchestration: provider recovery guidance and model selection recommendations.
- Shell: forms and status display.

Action:
Move provider guidance and model resolution to backend endpoints. Keep shell form submission and result rendering only.

Acceptance:
- Shell no longer constructs provider recovery prose or model fallback recommendations.
- Provider writes return structured audit receipts.
- Regression covers no-models, provider key write/delete, OAuth poll, and provider test result projection.

### SHP-AUTH-010 - Settings security/network/migration page holds authority-shaped logic

Status: queued
Priority: P1
Current files:
- `client/runtime/systems/ui/infring_static/js/pages/settings_security_network_helpers.ts`
- `client/runtime/systems/ui/infring_static/js/pages/settings.ts`
- `client/runtime/systems/ui/infring_static/js/pages/runtime.ts`
- `client/runtime/systems/ui/infring_static/js/pages/overview.ts`

Leak:
The shell formats security posture, infers active protections, polls peers, verifies audit chains, scans migrations, and initiates migration. Some formatting is display-only, but active/security/migration truth should be backend-owned and rendered from receipts.

Target owner:
- Core/runtime: security posture, audit verification, network/peer truth, migration admission.
- Orchestration: migration workflow coordination and progress packaging.
- Shell: settings forms and receipt/progress display.

Action:
Normalize security/network/migration responses into display-ready backend projections. Move migration step state and peer polling policy out of shell.

Acceptance:
- Shell no longer infers protection active/default state when data is missing.
- Migration runs return receipts and progress events.
- Regression covers missing security data, audit verification, peer polling, migration scan, and dry run.

### SHP-AUTH-011 - Hands activation/install/dependency coordination is shell-owned

Status: queued
Priority: P2
Current files:
- `client/runtime/systems/ui/infring_static/js/pages/hands.ts`
- `client/runtime/systems/ui/infring_static/js/pages/hands_setup_wizard_helpers.ts`
- `client/runtime/systems/ui/infring_static/js/pages/hands_dashboard_viewer_helpers.ts`

Leak:
The shell checks dependencies, installs dependencies, activates/deactivates hands, launches/configures instances, and shapes runtime stats. That is runtime orchestration and execution coordination, not UI logic.

Target owner:
- Orchestration: activation workflow, dependency install coordination, setup sequencing.
- Core/runtime: execution/admission and instance truth.
- Shell: setup wizard, dashboard viewer, progress projection.

Action:
Move install/check/activate sequencing behind a backend orchestration job API with progress receipts. Shell renders job status and submits config.

Acceptance:
- Shell no longer sequences dependency check/install/activate.
- Activation produces durable receipt/job state.
- Regression covers dependency missing, install failure, activation success, and deactivate receipt.

### SHP-AUTH-012 - Status, phase, and context labels are inferred in shell

Status: queued
Priority: P2
Current files:
- `client/runtime/systems/ui/infring_static/js/pages/chat_ws_phase_event_helpers.ts`
- `client/runtime/systems/ui/infring_static/js/pages/chat_thinking_display_helpers.ts`
- `client/runtime/systems/ui/infring_static/js/pages/chat_context_helpers.ts`
- `client/runtime/systems/ui/infring_static/js/pages/chat_agent_live_status_helpers.ts`
- `client/runtime/systems/ui/infring_static/js/pages/chat_lifecycle_init_helpers.ts`

Leak:
The shell infers working/typing/idle labels, context warnings, phase text, and status transitions from partial events. These are acceptable as projections only when directly backed by backend event semantics; otherwise they become a second control plane.

Target owner:
- Orchestration: workflow phase/state events and progress packaging.
- Core/runtime: authoritative health/readiness and agent activity truth.
- Shell: render supplied labels and progress state.

Action:
Define a backend workflow/status event envelope with explicit display fields and source authority. Remove shell-side status inference except local transient UI state clearly marked as optimistic.

Acceptance:
- Shell labels identify source authority or optimistic state.
- Context warnings come from backend event payloads, not shell-authored chat rows.
- Regression covers phase events, thinking bubble updates, idle transition, and context warning projection.

## Execution Notes

- Start with P0 rows because they can directly alter chat output or decide work flow.
- Do not delete shell behavior before the backend/orchestration replacement exists.
- For each row, preserve UI affordances while moving decisions into Rust-first orchestration or core/runtime authority.
- Each completed row should add or update a guard so the same authority does not drift back into shell.
