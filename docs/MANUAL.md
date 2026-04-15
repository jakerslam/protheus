# Infring Manual

## Table of Contents
1. [What Infring Is](#what-infring-is)
2. [Installation + Startup](#installation--startup)
3. [CLI Reference](#cli-reference)
4. [Dashboard UI Guide](#dashboard-ui-guide)
5. [Tools, Receipts, and Evidence](#tools-receipts-and-evidence)
6. [Sessions, Memory, and Branching](#sessions-memory-and-branching)
7. [Safety and Governance Model](#safety-and-governance-model)
8. [Troubleshooting](#troubleshooting)
9. [Reporting Issues](#reporting-issues)
10. [Glossary](#glossary)

## What Infring Is
Infring is a governed agent runtime with both CLI and dashboard interfaces. It is designed around observable, auditable execution where tool calls, runtime state, and outcomes can be inspected instead of treated as opaque black-box behavior.

## Installation + Startup
### Windows
1. Run installer with `-Repair -Full` when command shims or runtime artifacts drift.
2. Confirm command resolution with `Get-Command infring`.
3. Start with `infring gateway` and validate with `infring gateway status`.

### POSIX shells
1. Confirm command resolution with `which infring`.
2. Start gateway/runtime using `infring gateway`.
3. Validate runtime health with `infring gateway status`.

### First dashboard run
1. Open dashboard.
2. Select or create an agent.
3. Send a first prompt.
4. Verify tool outputs and response rendering.

## CLI Reference
- `infring gateway`
Purpose: start/control gateway runtime behavior.

- `infring gateway status`
Purpose: fetch runtime health/readiness.

- Installer repair/full mode
Purpose: recover stale binaries, wrappers, or PATH-facing command shims.

## Dashboard UI Guide
### Taskbar
- Runtime controls (restart/update/shutdown lanes where enabled).
- Notifications and utility access.
- Help menu (Manual + Report an issue).

### Chat Sidebar
- Agent list and session switching.
- Conversation previews and activity hints.
- Drag/snap behavior for placement and wall-aware overlay positioning.

### Chat Map
- Jump-to-message navigation in long threads.
- Day-group visualization and compact timeline traversal.
- Hover metadata via popup previews.

### Chat Surface
- Prompt entry and output stream.
- Tool cards with status and result payload summaries.
- Runtime feedback such as thinking/progress states.

## Tools, Receipts, and Evidence
Infring favors evidence-backed behavior:
- tool invocation metadata,
- outcome/result snapshots,
- runtime receipts where available.

When troubleshooting agent quality, inspect tool outcomes before assuming model-only failure.

## Sessions, Memory, and Branching
- Session: active conversational timeline for an agent.
- Memory: persisted context associated with runtime flows.
- Branching: allows diverging work tracks while preserving continuity.

Best practice: keep one problem domain per session to reduce context bleed.

## Safety and Governance Model
Infring is intended to fail closed on high-risk paths. Practical implications:
- explicit policy checks,
- bounded mutation authority,
- runtime-state observability and clearer audit trails.

## Troubleshooting
- If UI appears stalled:
  - check runtime status first,
  - refresh runtime/dashboard state,
  - retry from a clean prompt.
- If tools fail repeatedly:
  - verify policy/config/provider state,
  - confirm adapters and runtime health.
- If install/launch fails on Windows:
  - run installer in repair/full mode,
  - verify `Get-Command infring`,
  - retry gateway status check.

## Reporting Issues
Use `Help -> Report an issue` and include:
- expected behavior,
- actual behavior,
- reproduction steps,
- relevant logs/screenshots,
- platform details (OS + shell + install mode).

## Glossary
- Agent: runtime worker handling prompts and tool orchestration.
- Session: scoped conversation timeline.
- Tool call: structured capability execution request.
- Receipt/evidence: runtime-output artifacts supporting traceability.
