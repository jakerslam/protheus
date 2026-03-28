# Dashboard Authority Parity Checklist

Owner: `core/layer0/ops::dashboard_ui`  
Generated: `2026-03-28`  
Source baseline: `client/runtime/systems/ui/infring_dashboard.js` at `HEAD` (legacy pre-wrapper)

## Baseline

- Legacy dashboard authority surface: `488` named JS functions.
- Current Rust dashboard authority surface: `52` functions in `core/layer0/ops/src/dashboard_ui.rs`.
- Current wrapper posture:
  - `client/runtime/systems/ui/infring_dashboard.js` = thin delegator
  - `client/runtime/systems/ui/infring_dashboard.ts` = thin entrypoint

## Domain Parity Matrix

Status legend:
- `MIGRATED`: authority now in Rust core
- `PARTIAL`: some behavior migrated, parity gaps remain
- `PENDING`: authority still not in Rust core

1. Dashboard boot/serve/snapshot/runtime-sync
- Status: `MIGRATED`
- Rust path: `core/layer0/ops/src/dashboard_ui.rs`
- Notes: snapshot hot path now uses cached/runtime state reads to avoid lane fan-out hangs.

2. Dashboard action bridge (`/api/dashboard/action`)
- Status: `PARTIAL`
- Migrated actions:
  - `dashboard.ui.toggleControls`
  - `dashboard.ui.toggleSection`
  - `dashboard.ui.switchControlsTab`
  - `app.switchProvider`
  - `app.chat`
  - `collab.launchRole`
  - `skills.run`
  - `dashboard.assimilate`
  - `dashboard.benchmark`
  - `dashboard.runtime.executeSwarmRecommendation`
  - `dashboard.runtime.applyTelemetryRemediations`
- Gap: legacy action surface outside these handlers.

3. Agent visibility merge (runtime + persisted profile state)
- Status: `PARTIAL`
- Migrated:
  - profile merge into collab snapshot
  - archived-agent exclusion from profile merge
- Gap: full legacy compatibility layer (`compatAgentsFromSnapshot`, contract formatting, fallback identity synthesis) not fully ported.

4. Agent contracts/lifespan/autotermination/revival
- Status: `PARTIAL`
- Legacy family: `contract*`, `terminateAgentForContract`, `enforceAgentContracts`, archival/revival helpers.
- Migrated:
  - Core module: `core/layer0/ops/src/dashboard_agent_state_registry.rs`
  - Action endpoints:
    - `dashboard.agent.upsertContract`
    - `dashboard.agent.enforceContracts`
  - Snapshot-time enforcement hook via `dashboard_agent_state::enforce_expired_contracts`.
- Gap: full legacy lifecycle edge-cases (revival + advanced idle/quarantine policies) remain.

5. Agent session persistence and per-agent memory KV
- Status: `MIGRATED`
- Legacy family: `loadAgentSession`, `saveAgentSession`, `appendAgentConversation`, `read/write/deleteAgentMemoryKv`.
- Migrated:
  - Core module: `core/layer0/ops/src/dashboard_agent_state_sessions.rs`
  - Core controls module: `core/layer0/ops/src/dashboard_agent_state_controls.rs`
  - Action endpoints:
    - `dashboard.agent.session.get`
    - `dashboard.agent.session.appendTurn`
    - `dashboard.agent.session.create`
    - `dashboard.agent.session.switch`
    - `dashboard.agent.session.delete`
    - `dashboard.agent.memoryKv.set`
    - `dashboard.agent.memoryKv.get`
    - `dashboard.agent.memoryKv.delete`
  - Snapshot summaries exposed at `agents.session_summaries`.
- Notes: session switching/removal and explicit memory-kv CRUD now routed through Rust authority actions.

6. Prompt suggestion generation + quality gates
- Status: `PARTIAL`
- Legacy family: `generatePromptSuggestions`, quality filters, dedupe, anti-parrot constraints.
- Migrated:
  - Rust suggestion engine in `dashboard_agent_state_sessions.rs`.
  - Action endpoint `dashboard.agent.suggestions` with:
    - dedupe
    - anti-parrot
    - quote stripping
    - max 3 suggestions.
- Gap: full model-assisted suggestion planner parity remains.

7. Model/provider registry, local model discovery, routing heuristics
- Status: `PARTIAL`
- Legacy family: `loadProviderRegistry`, `buildDashboardModels`, `rustRouteDecision`, local bootstrap reminder logic.
- Migrated:
  - Rust `/api/providers` compatibility endpoint now reads core-owned provider registry state.
  - Rust `/api/models` compatibility endpoint now emits normalized model catalog metadata.
  - Rust `/api/route/decision` (GET/POST) now computes route decisions from registry metadata and request intent.
  - Rust action endpoints:
    - `dashboard.models.catalog`
    - `dashboard.model.routeDecision`
- Gap: richer bootstrap reminder orchestration + deeper context-window heuristics parity remains.

8. Terminal session orchestration and command queueing
- Status: `PARTIAL`
- Legacy family: terminal process/session queue functions.
- Migrated:
  - Core module: `core/layer0/ops/src/dashboard_terminal_broker.rs`
  - Rust compatibility endpoints:
    - `GET /api/terminal/sessions`
    - `POST /api/terminal/sessions`
    - `POST /api/terminal/queue`
    - `DELETE /api/terminal/sessions/:id`
  - Rust action endpoints:
    - `dashboard.terminal.session.create`
    - `dashboard.terminal.exec`
    - `dashboard.terminal.session.close`
- Gap: asynchronous long-running terminal queue workers + stream multiplexing parity remains.

9. Attention/queue/runtime reliability orchestration glue
- Status: `PARTIAL`
- Migrated: runtime-sync summary/pressure/conduit recommendation model.
- Gap: full legacy runtime remediation orchestration (`maybeHealCoarseSignal`, richer stall recovery policy wiring).

10. Release check + self-update orchestration
- Status: `PARTIAL`
- Legacy family: GitHub release check/update state and updater workflow.
- Migrated:
  - Core module: `core/layer0/ops/src/dashboard_release_update.rs`
  - Rust compatibility endpoints:
    - `GET /api/update/check`
    - `POST /api/update/apply`
  - Rust action endpoints:
    - `dashboard.update.check`
    - `dashboard.update.apply`
- Gap: restart orchestration handshake after apply + background scheduled polling UX hooks.

11. Channel registry / QR session management
- Status: `PARTIAL`
- Legacy family: channel registry + QR state readers/writers.
- Migrated:
  - Rust `/api/channels` compatibility endpoint.
  - Rust channel configure/test/delete handlers:
    - `POST /api/channels/:name/configure`
    - `POST /api/channels/:name/test`
    - `DELETE /api/channels/:name/configure`
  - Rust WhatsApp QR flow handlers:
    - `POST /api/channels/whatsapp/qr/start`
    - `GET /api/channels/whatsapp/qr/status`
- Gap: full external bridge parity (real connector backends beyond dashboard state) still pending.

13. Legacy Compat API Surface (`/api/*` dashboard endpoints)
- Status: `PARTIAL`
- Migrated:
  - Rust compatibility endpoints for health/usage/providers/channels/skills/mcp/audit/version/network/security/tools/commands/budget/a2a/approvals/sessions/workflows/cron/triggers/schedules/comms/hands/profiles/templates.
- Gap: endpoint-level parity verification against all historical consumers still pending.

12. Skills marketplace/ClawHub integration glue
- Status: `PENDING`
- Legacy family: cache/http wrappers + skills install metadata management.
- Required Rust target: skills marketplace authority lane in core.

## Immediate Migration Queue (next batches)

1. Close remaining agent lifecycle parity (revival/idle/quarantine edge behavior).
2. Port provider/model route selection authority to Rust (`buildDashboardModels`, `rustRouteDecision` parity).
3. Expand channel bridge beyond local dashboard state into fully wired runtime connectors.
4. Add post-update restart handshake for gateway/dashboard lifecycle.

## Guardrails

- No new authority in `client/**`.
- Dashboard client remains UX + wrapper only.
- Each parity closure must include:
  - one regression test in Rust
  - one runtime evidence command
  - checklist status update in this file.
