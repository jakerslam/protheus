# Control Plane API Contract

Status: active (2026-04-20)  
Scope: `surface/orchestration/**` control-plane boundaries

## Purpose

Define the only allowed kernel-facing message contracts for the control plane so coordination logic stays non-authoritative.

## Allowed Kernel Inputs (read-only snapshots)

- `core_probe_envelope`
- `typed_request_snapshot`
- `execution_observation_snapshot`
- `capability_probe_snapshot`
- `policy_scope_snapshot`

## Allowed Kernel Outputs (recommendation/projection only)

- `core_contract_call_envelope`
- `task_fabric_proposal_envelope`
- `tool_broker_request_envelope`
- `recovery_recommendation_envelope`
- `result_package_projection`

## Forbidden Authority Domains

- `canonical_policy_truth`
- `execution_admission_truth`
- `deterministic_receipt_authority`
- `scheduler_truth`
- `queue_truth`

## Message Boundary Invariants

- `control_plane_reads_kernel_snapshots_only`
- `control_plane_writes_recommendations_only`
- `control_plane_receipt_binding_forbidden`
- `kernel_is_final_authority`

## Subdomain Ownership Map

1. `intake_normalization`
2. `decomposition_planning`
3. `workflow_graph_dependency_tracking`
4. `recovery_escalation`
5. `result_shaping_packaging`

## Subdomain Module Bindings

1. Intake/Normalization: `ingress`, `ingress/parser`, `request_classifier`
2. Decomposition/Planning: `planner`, `planner/plan_candidates`, `planner/scoring`, `planner/preconditions`
3. Workflow Graph/Dependency Tracking: `sequencing`, `progress`, `transient_context`
4. Recovery/Escalation: `recovery`, `clarification`, `posture`
5. Result Shaping/Packaging: `result_packaging`, `progress`, `contracts`

## Default Workflow Templates (Control-Plane Authority)

Control-plane default templates are explicit, versioned behavior contracts (not ad-hoc prompt flow):

1. `clarify_then_coordinate`
2. `research_synthesize_verify`
3. `plan_execute_review`
4. `diagnose_retry_escalate`

Template selection is computed in Rust by `control_plane::lifecycle::select_workflow_template(...)` using typed request class/kind, live plan status, and recovery state.

## Lifecycle Loop (Required Stages)

Every orchestration result package must carry one lifecycle projection with these stage IDs:

1. `intake_normalization`
2. `decomposition_planning`
3. `coordination_sequencing`
4. `recovery_escalation`
5. `result_packaging`
6. `verification_closure`

The lifecycle projection is emitted by `control_plane::lifecycle::build_lifecycle_state(...)` and includes:

- single control-plane owner (`surface_orchestration_control_plane`)
- active stage
- stage statuses (`pending|ready|running|completed|blocked|skipped`)
- next actions
- closure status vectors (`verification`, `receipt_correlation`, `memory_packaging`)

## Feedback Loop Contract

Control-plane sequencing must actively attempt reroute/retry when selected plans are failed/blocked/degraded and an improved alternative candidate exists.

- Runtime path: `sequencing::apply_retry_reroute_feedback(...)`
- Behavioral invariant: orchestration does not stop at first-candidate packaging when retryable alternatives exist.

## Canonical Control-Plane Entrypoints (Compatibility Aliases Preserved)

- Clarification: `clarification::build_clarification_prompt` (compat alias: `clarification_prompt_for`)
- Posture: `posture::choose_execution_posture` (compat alias: `choose_posture`)
- Recovery: `recovery::coordinate_recovery_escalation` (compat alias: `apply_recovery_policy`)
- Progress shaping: `progress::build_progress_projection` (compat alias: `progress_message`)
- Result packaging: `result_packaging::shape_result_package` (compat alias: `package_result`)
- Lifecycle projection: `control_plane::lifecycle::{select_workflow_template, build_lifecycle_state}`
- Feedback reroute: `sequencing::apply_retry_reroute_feedback` (compat alias: `feedback_loop_reroute`)

## Enforcement

- Source contract implementation: `surface/orchestration/src/control_plane/**`
- Conformance tests: `surface/orchestration/tests/control_plane_subdomains.rs`
- Ownership policy reference: `docs/workspace/orchestration_ownership_policy.md`
