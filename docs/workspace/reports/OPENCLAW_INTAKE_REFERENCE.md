# OpenClaw Intake Reference

## Source
- Repo: `https://github.com/openclaw/openclaw`
- Local source mirror: `/Users/jay/.openclaw/workspace/projects/openclaw-upstream`
- Checked revision: `6133d24` (short SHA)

## Wave Scope
- This ledger tracks the current tooling-assimilation shortlist for OpenClaw.
- Format: `status<TAB>path<TAB>notes`

## Status Vocabulary
- `pending`: selected but not yet assimilated.
- `reviewed_no_import`: reviewed, no safe/runtime-worthy import for this lane.
- `reviewed_candidate`: reviewed, future candidate worth revisiting.
- `imported`: capability ported into this runtime lane.
- `skipped_non_runtime`: assets/docs/non-runtime file skipped.

## Current Imported Capability
- `OPENCLAW-TOOLING-CHAT-001`:
  - Imported provider-scoped model resolution and fallback dedupe patterns from:
    - `ui/src/ui/chat-model-select-state.ts`
    - `ui/src/ui/app-chat.ts`
  - Landed in:
    - `client/runtime/systems/ui/infring_static/js/pages/chat.ts.parts/040-models-and-cache.part02.ts`
    - `client/runtime/systems/ui/infring_static/js/pages/chat.ts.parts/190-drawer-and-queue.ts`
- `OPENCLAW-TOOLING-CHAT-002`:
  - Imported reconnect-aware pending-run sync patterns from:
    - `ui/src/ui/app-gateway.ts`
    - `ui/src/ui/app-chat.ts`
  - Landed in:
    - `client/runtime/systems/ui/infring_static/js/pages/chat.ts.parts/140-session-and-ws.part01.ts`
- `OPENCLAW-TOOLING-CHAT-003`:
  - Imported tool-stream fallback/preview shaping patterns from:
    - `ui/src/ui/app-tool-stream.ts`
    - `ui/src/ui/chat/tool-cards.ts`
  - Landed in:
    - `client/runtime/systems/ui/infring_static/js/pages/chat.ts.parts/140-session-and-ws.part02.ts`
    - `client/runtime/systems/ui/infring_static/js/pages/chat.ts.parts/160-runtime-events-and-render.part01.ts`
- `OPENCLAW-TOOLING-CHAT-004`:
  - Imported qualified model-ref normalization and display shaping from:
    - `ui/src/ui/chat-model-ref.ts`
  - Landed in:
    - `client/runtime/systems/ui/infring_static/js/pages/chat.ts.parts/030-init-selection-helpers.ts`
    - `client/runtime/systems/ui/infring_static/js/pages/chat.ts.parts/120-slash-and-agent-select.ts`
- `OPENCLAW-TOOLING-CHAT-005`:
  - Imported session-cache hygiene and stale pending-run recovery patterns from:
    - `ui/src/ui/chat/session-cache.ts`
    - `ui/src/ui/controllers/chat.ts`
  - Landed in:
    - `client/runtime/systems/ui/infring_static/js/pages/chat.ts.parts/040-models-and-cache.part01.ts`
    - `client/runtime/systems/ui/infring_static/js/pages/chat.ts.parts/110-failover-and-health.part01.ts`
- `OPENCLAW-TOOLING-CHAT-006`:
  - Imported slash-command metadata, alias expansion, and model-command execution patterns from:
    - `ui/src/ui/chat/slash-commands.ts`
    - `ui/src/ui/chat/slash-command-executor.ts`
  - Landed in:
    - `client/runtime/systems/ui/infring_static/js/pages/chat.ts.parts/120-slash-and-agent-select.ts`
    - `client/runtime/systems/ui/infring_static/js/pages/chat.ts.parts/192-slash-alias-and-alerts.ts`

## Current Intake State
- Imported rows in this wave: `10`
- Pending rows in this wave: `0`
