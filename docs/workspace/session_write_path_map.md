# Session Write Path Map

Status: active working map

## Current Session Sink

Legacy dashboard sessions are persisted under:

```text
client/runtime/local/state/ui/infring_dashboard/agent_sessions/{agent_id}.json
```

This file is still monolithic during the transition, but it is no longer allowed to be the default sink for raw runtime internals.

## Primary Writers

- `save_session_state(...)` in `core/layer0/ops/src/dashboard_compat_api_parts/set_config_payload_parts/020-part.rs`
- `append_turn(...)` in dashboard agent state session parts
- `append_turn_message(...)` and `persist_last_assistant_turn_metadata(...)` in compat API turn persistence
- Session reset/compact routes in compat API route blocks
- Context keyframe append helpers in compat API session utilities

## Highest-Risk Bloat Source

The largest bloat source is assistant-turn metadata added after message finalization:

```text
tools
response_workflow
response_finalization
process_summary
workflow_visibility
turn_transaction
terminal_transcript
```

Before this boundary, those fields could be embedded directly into the assistant chat message. That made the chat session file behave like a runtime mirror.

## New Boundary

New assistant-turn metadata now passes through `session_safe_turn_metadata(...)`.

The full metadata is routed into:

```text
client/runtime/local/state/ui/infring_dashboard/session_artifacts/{agent_id}/
```

The chat message keeps only:

- Compact tool previews.
- Compact workflow visibility.
- Compact terminal previews.
- `detail_refs` for full artifacts.
- `session_projection_contract`.

`save_session_state(...)` also applies `bound_session_state_for_persistence(...)` as a final safety net before writing any session file.

## Remaining Work

- Add lazy detail fetch routes for `session_artifact:*` refs.
- Convert monolithic session JSON to indexed `messages.jsonl` windows.
- Add a guard test that fails if forbidden fields survive in persisted messages.
- Route workflow telemetry to the proper observability store instead of using session artifacts as the transitional detail store.
