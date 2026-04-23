# Orchestration Control-Plane Workflow Maps

This file is a readability map for the control plane.

Scope: request decomposition, coordination, sequencing, recovery, and result packaging.

## 1) Default Turn Flow

```mermaid
flowchart TD
    A[User turn] --> B["Gate 1: Need tool access? (T/F)"]
    B --> C{Tools required?}
    C -->|No| D[Direct answer draft]
    C -->|Yes| E[Numbered tool family menu]
    E --> F[Numbered tool menu]
    F --> G[Execute selected tool]
    G --> H{"Post-tool gate: finish or another tool?"}
    H -->|Another tool| E
    H -->|Finish| I[Collect receipts + events]
    D --> G
    I --> J[Final synthesis]
    J --> K{Valid visible response?}
    K -->|Yes| L[Return answer + receipts]
    K -->|No| M[Recovery: repair or direct fallback]
    M --> L
```

## 2) Conversation Bypass Flow (`conversation_bypass_v1`)

```mermaid
flowchart TD
    A[User turn] --> B{Bypass override phrase or sticky bypass active?}
    B -->|No| C[Run default workflow]
    B -->|Yes| D{Safety gate passes?}
    D -->|No: tooling/high-risk needed| C
    D -->|Yes| E[Select conversation_bypass_v1]
    E --> F[Direct response continuity]
    F --> G[Persist bypass state with remaining TTL]
    G --> H[Return response]
```

## 3) Recovery + Loop Guard Flow

```mermaid
flowchart TD
    A[Final response candidate] --> B{Empty, low-signal, or boilerplate loop?}
    B -->|No| C[Keep candidate]
    B -->|Yes| D[Generate workflow unexpected-state fallback]
    D --> E[Non-repeat sanitizer]
    E --> F{Still invalid?}
    F -->|No| G[Return repaired direct answer]
    F -->|Yes| H[Last-resort direct variant]
    H --> G
```

## 4) Ownership Reminder

- Kernel: truth, policy, admission, enforcement.
- Orchestration control plane: what should happen next (decompose/coordinate/sequence/recover/package).
- Shell: presentation and input only.

See also: `docs/workspace/orchestration_ownership_policy.md`.

## 5) Trace Streams + Exports

The workflow now emits separate streams so the UI harness can render each channel differently:

- `workflow_state` (machine-readable stage transitions)
- `ui_status` (short user-facing status lines like "Searching the web")
- `decision_summary` (concise rationale snapshots)
- `tool_execution` (tool/audit timeline)

Export formats (same turn, same trace id):

- JSON object in `response_workflow` (live UI payload)
- JSONL append history (`<state_root>/chat_ui/workflow_trace_history.jsonl`)
- Timeline text snapshot (`<state_root>/chat_ui/workflow_trace_latest.timeline.txt`)
