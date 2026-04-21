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

## Canonical Control-Plane Entrypoints (Compatibility Aliases Preserved)

- Clarification: `clarification::build_clarification_prompt` (compat alias: `clarification_prompt_for`)
- Posture: `posture::choose_execution_posture` (compat alias: `choose_posture`)
- Recovery: `recovery::coordinate_recovery_escalation` (compat alias: `apply_recovery_policy`)
- Progress shaping: `progress::build_progress_projection` (compat alias: `progress_message`)
- Result packaging: `result_packaging::shape_result_package` (compat alias: `package_result`)

## Enforcement

- Source contract implementation: `surface/orchestration/src/control_plane/**`
- Conformance tests: `surface/orchestration/tests/control_plane_subdomains.rs`
- Ownership policy reference: `docs/workspace/orchestration_ownership_policy.md`
