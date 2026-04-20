# Orchestration Ownership Policy

## Purpose

Define a hard operating split between `core/`, `surface/orchestration/`, and the shell path `client/` so placement decisions are predictable and enforceable.

## Transition Status

Documentation now defines `surface/orchestration/` as the Cognition Control Plane.
Internal naming and placement cleanup is an incremental transition: existing `orchestration` path/module names remain valid compatibility surfaces until the internal migration closes.

## Boundary Axiom

Core decides what is true and allowed.  
Orchestration decides what should happen next.  
Shell decides how it is shown and collected.

## Core

### Mission

Own canonical truth, permission, and enforcement even if orchestration and shell disappear.

### Core Owns

- Canonical state and invariants.
- Policy evaluation and hard safety gates.
- Execution admission and fail-closed transitions.
- Canonical scheduling and resource enforcement.
- Deterministic receipt authority.

### Core Must Not Own

- UX rendering or shell behavior.
- Presentation formatting.
- Non-canonical workflow choreography.

### Placement Test

If orchestration vanished and a typed request hit conduit directly, would this still be required for correctness and safety?

## Control Plane (Orchestration Surface)

### Mission

Coordinate workflow decomposition and execution flow without becoming authority on truth.

### Control Plane Owns

- Request/task decomposition.
- Workflow coordination.
- Workflow sequencing.
- Recovery orchestration (including clarification, retry, escalation, and fallback handling).
- Lane/adaptor selection recommendations.
- Result shaping and packaging for downstream consumers.
- Among other things in non-canonical control-plane coordination.

### Control Plane Must Not Own

- Canonical state truth.
- Policy truth and hard safety enforcement.
- Final execution admission or receipt authority.

### Placement Test

Is this deciding control-plane flow (what should run next) rather than deciding truth or permission?

## Shell (compat alias: Client)

### Mission

Render outputs, collect input, and manage presentation-local UX state.

### Shell Owns

- Rendering and interaction flows.
- Input capture and UX shells.
- Presentation-local state and caches.

### Shell Must Not Own

- Policy authority.
- Authoritative health/readiness inference.
- Workflow decomposition and retry authority.
- Queue truth, lane truth, or adapter truth.

### Placement Test

If this UI were replaced with another shell, would this logic still be needed?

## Gateways (compat alias: Adapters)

### Mission

Enforce controlled external-system boundaries without becoming authority on truth.

### Gateways Own

- External protocol/runtime bridging (SDK/API/tool/provider boundaries).
- Contract-normalized request/response envelopes for external systems.
- Fail-closed boundary behavior for unavailable/invalid external dependencies.
- Replaceable integration adapters behind stable gateway contracts.

### Gateways Must Not Own

- Canonical policy truth.
- Canonical queue/scheduler/execution admission truth.
- Authoritative receipt decision logic.

### Placement Test

If this code were removed, would core safety/truth still be intact while only external connectivity is reduced?

## Move Guidance

Move logic into `surface/orchestration/` when it does non-canonical coordination:

- Decomposition.
- Coordination.
- Sequencing.
- Recovery.
- Result shaping/packaging.
- Dependency graph workflow management.
- Non-authoritative result shaping/packaging.

Keep logic in `core/` when it is authoritative kernel logic:

- Scheduling authority.
- Queue and priority truth.
- Execution admission.
- Policy evaluation and enforcement.
- Deterministic receipt binding.

## Review Rubric

For each function/file:

1. Is it authoritative truth or enforcement? -> `core/`
2. Is it workflow coordination? -> `surface/orchestration/`
3. Is it presentation/input UX? -> shell path `client/`
4. Is it external boundary integration/bridge logic? -> `adapters/` (Gateway layer)

If code appears to satisfy multiple categories, split responsibilities.
