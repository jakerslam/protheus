# Orchestration Ownership Policy

## Purpose

Define a hard operating split between `core/`, `orchestration/`, and the shell path `client/` so placement decisions are predictable and enforceable.

## Transition Status

Documentation now defines `orchestration/` as the first-class Orchestration subsystem and its Control Plane.
Internal naming and placement cleanup is an incremental transition: existing compatibility module names remain valid bridge surfaces until the internal migration closes.
Readable control-plane flow maps live in `docs/workspace/orchestration_workflow_maps.md`.

## Canonical Naming Decision

The canonical concept name is **Orchestration Control Plane**.

`Tower` is not a canonical InfRing architecture term. It may appear only as an informal metaphor in discussion or historical notes, not as an owning layer, source path, subsystem name, gate name, release artifact, or generated map label.

`Cognition Control Plane` and `Cognition Plane (Orchestration Control Plane)` are historical transition phrases. New architecture docs, maps, gates, and TODO/SRS rows must use `Orchestration Control Plane` for the coordination subsystem and reserve `Shell` for presentation behavior.

## Boundary Axiom

Kernel decides what is true and allowed.  
Orchestration decides what should happen next.  
Assurance proves, observes, scores, gates, and explains the work.
Shell decides how it is shown and collected.

## Canonical Names vs Compatibility Paths

| Canonical concept | Compatibility/path wording | Rule |
|---|---|---|
| Kernel | `core/**`, historical `Core` | `core/**` is the implementation path for Kernel authority. `Core` must not be treated as a second owner. |
| Orchestration Control Plane | `orchestration/**`, rejected `Tower` metaphor | Orchestration owns coordination only; it recommends, sequences, recovers, and packages without becoming canonical truth. |
| Shell | `client/**`, historical `Client` | `client/**` is the Shell implementation path. `Client` must not be treated as a conceptual authority or state owner. |
| Gateways | `adapters/**`, historical `Adapters` | Gateways own the external membrane and bounded ingress/egress; `adapters/**` is the path compatibility surface. |

Canonical Nexus-Conduit-Checkpoint policy lives in `docs/workspace/nexus_conduit_checkpoint_policy.md`.

Canonical Layered Nexus Federation Resolution policy lives in `docs/workspace/layered_nexus_federation_resolution_policy.md`.

Canonical Cross-Domain Nexus Route Inventory lives in `docs/workspace/cross_domain_nexus_route_inventory.md`.

Canonical Conduit/Scrambler Posture policy lives in `docs/workspace/conduit_scrambler_posture_policy.md`.

Canonical Gateway Ingress/Egress Interface policy lives in `docs/workspace/gateway_ingress_egress_policy.md`.

Canonical Interface Payload Budget policy lives in `docs/workspace/interface_payload_budget_policy.md`.

Canonical Shell-Independent Operation policy lives in `docs/workspace/shell_independent_operation_policy.md`.

Canonical Shell UI Projection policy lives in `docs/workspace/shell_ui_projection_policy.md`.

Canonical Shell UI Message Detail contract lives in `docs/workspace/shell_ui_message_detail_contract.md`.

Canonical Assurance Plane policy lives in `docs/workspace/assurance_plane_policy.md`.

Canonical Assurance physical-domain migration status lives in `docs/workspace/assurance_physical_domain_migration_status.md`.

Every cross-module or cross-domain route must enter and exit through explicit Nexus checkpoint surfaces, travel over Conduit, declare Conduit/Scrambler security posture, and carry lease/capability, lifecycle, policy, and receipt context. Direct code-file-to-code-file cross-module paths are migration debt unless they are explicitly exempted with owner, expiry, and a replacement Nexus checkpoint plan.

## Kernel

### Mission

Own canonical truth, permission, and enforcement even if orchestration and shell disappear.

### Kernel Owns

- Canonical state and invariants.
- Policy evaluation and hard safety gates.
- Execution admission and fail-closed transitions.
- Canonical scheduling and resource enforcement.
- Deterministic receipt authority.

### Kernel Must Not Own

- UX rendering or shell behavior.
- Presentation formatting.
- Non-canonical workflow choreography.

### Placement Test

If orchestration vanished and a typed request hit conduit directly, would this still be required for correctness and safety?

## Control Plane (Orchestration)

### Mission

Coordinate workflow decomposition and execution flow without becoming authority on truth.

### Control Plane Owns

- Request/task decomposition.
- Workflow coordination.
- Workflow sequencing.
- Recovery orchestration (including clarification, retry, escalation, and fallback handling).
- Lane/adaptor selection recommendations.
- Result shaping and packaging for downstream consumers.
- Default workflow template selection (`clarify_then_coordinate`, `research_synthesize_verify`, `plan_execute_review`, `diagnose_retry_escalate`).
- Lifecycle state projection across control-plane stages (intake, decomposition, sequencing, recovery, packaging, verification closure).
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

The Shell is a projection lens, not a runtime mirror. Default Shell-facing payloads must be bounded presentation projections with explicit byte, depth, array, string, cursor, detail-ref, audit, and Nexus budgets; details such as traces, raw tool results, workflow internals, and execution observations are fetched lazily by reference through the proper gateway/conduit path. Core, Orchestration, CLI, and Gateway status must build and operate without browser Shell assets.

### Shell Owns

- Rendering and interaction flows.
- Input capture and UX shells.
- Presentation-local state and bounded preview caches.

### Shell Must Not Own

- Policy authority.
- Authoritative health/readiness inference.
- Workflow decomposition and retry authority.
- Queue truth, lane truth, or adapter truth.
- Full runtime objects, raw tool payloads, traces, workflow truth, execution observations, or durable full-state conversation caches.

### Placement Test

If this UI were replaced with another shell, would this logic still be needed?

Projection test: is this field required to display the current view, or can Shell keep an ID/preview and fetch details on demand?

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

## Assurance Plane

### Mission

Produce confidence about the system without becoming the system's actor, planner, or presentation layer.

### Assurance Owns

- Controlled validation: tests, evals, benchmarks, conformance guards, regression suites, and replay proof.
- Live observability: telemetry, health, traces, runtime findings, Sentinel evidence, and freshness coverage.
- Governance: release gates, scorecards, readiness verdicts, signal classification, issue-candidate thresholds, and trend deltas.

### Assurance Must Not Own

- Canonical policy truth or execution permission.
- Workflow planning or adaptive action selection.
- Shell rendering or operator-input UX.
- External boundary protocol ownership.
- Silent code mutation or self-modification.

### Placement Test

Is this judging, observing, scoring, gating, or explaining behavior rather than doing the behavior?

## Move Guidance

Move logic into `orchestration/` when it does non-canonical coordination:

- Decomposition.
- Coordination.
- Sequencing.
- Recovery.
- Result shaping/packaging.
- Dependency graph workflow management.
- Non-authoritative result shaping/packaging.

Control-plane wrapper lock (transition phase):

- `client/cognition/orchestration/{cli_shared,core_bridge,coordinator,coordinator_cli,checkpoint,completion,partial,schema_runtime,scope,scratchpad,taskgroup,taskgroup_cli}.ts` are compatibility bridges only.
- Those files must delegate to `orchestration/scripts/cognition/**` and must not embed coordination logic.
- `orchestration/scripts/cognition/**` are shell-compatible shims only and must delegate to `adapters/runtime/orchestration_cognition_impl/**`.
- Control-plane authority and coordination decisions must be implemented in Rust (`orchestration/src/**`), not TypeScript.
- `adapters/runtime/orchestration_cognition_impl/**` must remain Rust-facing transport glue (`orchestration invoke` op bridges), not a second coordination authority.
- Delegate target should match file identity (`foo.ts` bridge delegates to `surface/.../foo.ts`) to avoid wrapper drift.
- Shell and orchestration cognition bridge trees are parity-governed: relative file paths must match, and mirrored schema assets (for example `schemas/*.json`) must stay byte-identical.
- New decomposition/coordination/sequencing/recovery/packaging logic in shell/client wrappers is prohibited; implement under `orchestration/**` and bridge from shell.

Keep logic in `core/` when it is authoritative kernel logic:

- Scheduling authority.
- Queue and priority truth.
- Execution admission.
- Policy evaluation and enforcement.
- Deterministic receipt binding.

Place logic in the Assurance Plane when it is controlled proof, live observation, scoring, release gating, trend analysis, issue-candidate governance, or confidence reporting. Harnesses may remain in `tests/**`, but the source of assurance truth must be explicit and evidence-backed.

For Assurance definitions, physical placement matters: controlled definitions should live under `validation/**`, live evidence/source contracts should live under `observability/**`, and old tooling or shell paths must be compatibility mirrors, harness-only wrappers, or time-bounded exemptions. The placement guard `ops:assurance:physical-domain-placement:guard` enforces that split for definition-shaped files.

## Review Rubric

For each function/file:

1. Is it authoritative truth or enforcement? -> `core/`
2. Is it workflow coordination? -> `orchestration/`
3. Is it validation, live observation, scoring, gating, or confidence reporting? -> Assurance Plane
4. Is it presentation/input UX? -> shell path `client/`
5. Is it external boundary integration/bridge logic? -> `adapters/` (Gateway layer)

If code appears to satisfy multiple categories, split responsibilities.

## Nexus Coupling Enforcement

Core coupling governance is enforced by policy + CI:

- Policy: `tests/tooling/config/kernel_nexus_coupling_policy.json`
- Guard: `tests/tooling/scripts/ci/kernel_nexus_coupling_guard.ts`
- Canonical policy: `docs/workspace/nexus_conduit_checkpoint_policy.md`
- Command: `npm run -s ops:nexus:kernel-coupling:guard`
- CI workflow: `.github/workflows/core-nexus-coupling.yml`

Rule intent:

- Non-nexus core modules must not directly couple to other non-nexus core modules in enforced scope.
- Cross-module connectivity should route through nexus contracts.
- Cross-boundary routes should be represented by explicit Nexus checkpoint surfaces rather than private implementation imports.
- Temporary exemptions are explicit, dated, and fail-closed on expiry.
