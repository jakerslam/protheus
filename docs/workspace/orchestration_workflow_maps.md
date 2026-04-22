# Orchestration Control-Plane Workflow Maps

This file is a readability map for the control plane.

Scope: request decomposition, coordination, sequencing, recovery, and result packaging.

## 1) Default Turn Flow

```mermaid
flowchart TD
    A[User turn] --> B[Workflow gate classification]
    B --> C{Tools required?}
    C -->|No| D[Direct answer draft]
    C -->|Yes| E[Minimal tool family selection]
    E --> F[Execute tools]
    F --> G[Collect receipts + events]
    D --> G
    G --> H[Final synthesis]
    H --> I{Valid visible response?}
    I -->|Yes| J[Return answer + receipts]
    I -->|No| K[Recovery: repair or direct fallback]
    K --> J
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
