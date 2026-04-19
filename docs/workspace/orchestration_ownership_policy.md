# Orchestration Ownership Policy

## Purpose

Define a hard operating split between `core/`, `surface/orchestration/`, and `client/` so placement decisions are predictable and enforceable.

## Boundary Axiom

Core decides what is true and allowed.  
Orchestration decides what should happen next.  
Client decides how it is shown and collected.

## Core

### Mission

Own canonical truth, permission, and enforcement even if orchestration and client disappear.

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

## Orchestration

### Mission

Coordinate workflow without becoming authority on truth.

### Orchestration Owns

- Request normalization and task decomposition.
- Candidate plan generation and dependency tracking.
- Sequencing, clarification, retry, and escalation coordination.
- Lane/adaptor selection recommendations.
- Progress tracking and result packaging.

### Orchestration Must Not Own

- Canonical state truth.
- Policy truth and hard safety enforcement.
- Final execution admission or receipt authority.

### Placement Test

Is this deciding workflow flow (what should run next) rather than deciding truth or permission?

## Client

### Mission

Render outputs, collect input, and manage presentation-local UX state.

### Client Owns

- Rendering and interaction flows.
- Input capture and UX shells.
- Presentation-local state and caches.

### Client Must Not Own

- Policy authority.
- Authoritative health/readiness inference.
- Workflow decomposition and retry authority.
- Queue truth, lane truth, or adapter truth.

### Placement Test

If this UI were replaced with another shell, would this logic still be needed?

## Move Guidance

Move logic into `surface/orchestration/` when it does non-canonical coordination:

- Task decomposition.
- Multi-step workflow planning.
- Clarification and retry/escalation decisions.
- Dependency graph workflow management.
- Non-authoritative result packaging.

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
3. Is it presentation/input UX? -> `client/`

If code appears to satisfy multiple categories, split responsibilities.
