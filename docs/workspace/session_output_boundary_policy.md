# Session Output Boundary Policy

Status: active

## Purpose

Chat/session storage is a user-visible conversation record, not a dump of runtime internals. The session boundary must route each turn artifact to the authority store that owns it, then expose only bounded display projections and refs to shells.

## Contract

Session messages may contain:

- `id`, `role`, `text`, `ts`, and bounded display metadata.
- Compact tool rows with status, small previews, and `detail_ref`.
- Compact workflow visibility rows for current stage/status display.
- Compact terminal transcript previews with `detail_ref`.
- `detail_refs` pointing to routed artifacts.

Session messages must not contain:

- Raw tool input/output bodies.
- Raw traces, `trace_body`, or full decision traces.
- `workflow_graph`, full `response_workflow`, or full `response_finalization`.
- `execution_observation`, raw runtime state, `raw`, or `root`.
- Full context snapshots or unbounded telemetry blobs.

## Routing

Visible chat text goes to session storage. Heavy or internal turn artifacts route out before persistence:

- Tool inputs/results route to session artifact detail refs.
- Workflow and finalization state route to session artifact detail refs, with only compact `workflow_visibility` retained inline.
- Process summaries route to detail refs.
- Terminal transcripts retain small previews inline and full rows by ref.
- Context keyframes are bounded to small summaries.

## Enforcement

The legacy dashboard session save path now applies a persistence sanitizer before writing `agent_sessions/*.json`. New turn metadata uses `session_safe_turn_metadata(...)` so full metadata is persisted to `session_artifacts/{agent_id}/...` and only shell-safe projections are embedded in messages.

This is a transitional boundary. The next storage step is to replace monolithic `agent_sessions/{agent_id}.json` files with indexed message windows and lazy detail fetch routes.
