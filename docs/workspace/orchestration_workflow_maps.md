# Orchestration Control-Plane Workflow Maps

This file is a readability map for the control plane.

Scope: request decomposition, coordination, sequencing, recovery, and result packaging.

Current chat workflow rule: the workflow interface is not allowed to help the LLM pick tools. It may only present a multiple-choice menu, present a text-input payload field, execute the submitted payload, record telemetry, or hand final response authorship back to the LLM.

## 1) Default Turn Flow

```mermaid
flowchart TD
    A[User turn] --> B["Gate 1: Need tool access? (T/F)"]
    B --> C{"LLM submits F or T"}
    C -->|F| D[LLM answers directly]
    C -->|T| E[Present numbered tool family menu]
    E --> F[Numbered tool menu]
    F --> G[Present selected tool request-format field]
    G --> H[Execute submitted tool payload]
    H --> I{"Post-tool menu: 1 finish / 2 another tool"}
    I -->|Another tool| E
    I -->|Finish| J[LLM-authored final answer]
```

## 2) Direct Conversation Flow

Direct conversation is the `F` branch at Gate 1. There is no separate bypass workflow and no automatic bypass classifier.

```mermaid
flowchart TD
    A[User turn] --> B["Gate 1: Need tool access? (T/F)"]
    B -->|F| C[LLM answers directly]
    B -->|T| D[Tool menu interface]
```

## 3) Visibility + No-Injection Rule

```mermaid
flowchart TD
    A[Workflow event] --> B{Visible chat?}
    B -->|No| C[Telemetry stream]
    B -->|Yes| D{Authored by LLM?}
    D -->|Yes| E[Chat bubble]
    D -->|No| F[Withhold from chat]
```

## 4) Tool Loop

```mermaid
flowchart TD
    A[Tool result received] --> B[Present post-tool menu]
    B --> C{"LLM submits option"}
    C -->|1 finish| D[LLM synthesizes final answer]
    C -->|2 another tool| E[Return to family menu]
```

## 5) Ownership Reminder

- Kernel: truth, policy, admission, enforcement.
- Orchestration control plane: what should happen next (decompose/coordinate/sequence/recover/package).
- Shell: presentation and input only.

See also: `docs/workspace/orchestration_ownership_policy.md`.
Workflow format policy: `docs/workspace/workflow_json_format_policy.md`.

## 6) Trace Streams + Exports

The workflow now emits separate streams so the UI harness can render each channel differently:

- `workflow_state` (machine-readable stage transitions)
- `ui_status` (short user-facing status lines like "Searching the web")
- `decision_summary` (LLM submissions and guard telemetry only; no tool recommendations)
- `tool_execution` (tool/audit timeline)

Export formats (same turn, same trace id):

- JSON object in `response_workflow` (live UI payload)
- JSONL append history (`<state_root>/chat_ui/workflow_trace_history.jsonl`)
- Timeline text snapshot (`<state_root>/chat_ui/workflow_trace_latest.timeline.txt`)
