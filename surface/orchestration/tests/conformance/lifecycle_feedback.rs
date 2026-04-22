use super::*;

#[test]
fn runtime_execution_observation_channel_projects_into_execution_state() {
    let mut runtime = OrchestrationSurfaceRuntime::new();
    runtime.apply_execution_observation_update(
        infring_orchestration_surface_v1::contracts::OrchestrationExecutionObservationUpdate {
            session_id: "observation-channel".to_string(),
            observation: infring_orchestration_surface_v1::contracts::CoreExecutionObservation {
                plan_status: Some(infring_orchestration_surface_v1::contracts::PlanStatus::Running),
                receipt_ids: vec!["receipt-channel-1".to_string()],
                outcome_refs: Vec::new(),
                step_statuses: vec![
                    infring_orchestration_surface_v1::contracts::CoreExecutionStepObservation {
                        step_id: "step_tool_broker_request".to_string(),
                        status: infring_orchestration_surface_v1::contracts::StepStatus::Running,
                    },
                ],
            },
        },
    );

    let package = runtime.orchestrate(
        OrchestrationRequest {
            session_id: "observation-channel".to_string(),
            intent: "search release notes".to_string(),
            surface: RequestSurface::Sdk,
            payload: json!({
                "sdk": {
                    "operation_kind": "search",
                    "resource_kind": "web",
                    "request_kind": "direct",
                    "targets": [{ "kind": "url", "value": "https://example.com/releases" }]
                },
                "core_probe_envelope": {
                    "web_search": {
                        "tool_available": true,
                        "transport_available": true
                    }
                }
            }),
        },
        4_800,
    );

    assert_eq!(
        package.execution_state.plan_status,
        infring_orchestration_surface_v1::contracts::PlanStatus::Running
    );
    assert_eq!(
        package
            .execution_state
            .correlation
            .observed_core_receipt_ids,
        vec!["receipt-channel-1".to_string()]
    );

    runtime.clear_execution_observation("observation-channel");
    let package_after_clear = runtime.orchestrate(
        OrchestrationRequest {
            session_id: "observation-channel".to_string(),
            intent: "search release notes".to_string(),
            surface: RequestSurface::Sdk,
            payload: json!({
                "sdk": {
                    "operation_kind": "search",
                    "resource_kind": "web",
                    "request_kind": "direct",
                    "targets": [{ "kind": "url", "value": "https://example.com/releases" }]
                },
                "core_probe_envelope": {
                    "web_search": {
                        "tool_available": true,
                        "transport_available": true
                    }
                }
            }),
        },
        4_801,
    );

    assert!(package_after_clear
        .execution_state
        .correlation
        .observed_core_receipt_ids
        .is_empty());
}

#[test]
fn control_plane_result_includes_template_lifecycle_and_owner_contract() {
    let mut runtime = OrchestrationSurfaceRuntime::new();
    let package = runtime.orchestrate(
        OrchestrationRequest {
            session_id: "lifecycle-template-contract".to_string(),
            intent: "compare workspace and web evidence".to_string(),
            surface: RequestSurface::Sdk,
            payload: json!({
                "sdk": {
                    "operation_kind": "compare",
                    "resource_kind": "mixed",
                    "targets": [
                        { "kind": "workspace_path", "value": "README.md" },
                        { "kind": "url", "value": "https://example.com/docs" }
                    ]
                },
                "core_probe_envelope": {
                    "tool_route": {
                        "tool_available": true,
                        "transport_available": true
                    },
                    "verify_claim": {
                        "transport_available": true
                    }
                }
            }),
        },
        4_975,
    );

    assert_eq!(
        package.control_plane_lifecycle.owner,
        "surface_orchestration_control_plane"
    );
    assert_eq!(
        package.control_plane_lifecycle.template,
        package.workflow_template
    );
    assert!(package
        .control_plane_lifecycle
        .stages
        .iter()
        .any(|row| row.stage == WorkflowStage::DecompositionPlanning));
    assert!(package
        .control_plane_lifecycle
        .stages
        .iter()
        .any(|row| row.stage == WorkflowStage::VerificationClosure));
    assert!(package
        .control_plane_lifecycle
        .handoff_chain
        .iter()
        .any(|row| row.handoff_id == "handoff_user_request_to_decomposition"));
    assert!(package
        .control_plane_lifecycle
        .handoff_chain
        .iter()
        .any(|row| row.handoff_id == "handoff_verification_to_memory_packaging"));
}

#[test]
fn failed_execution_observation_triggers_feedback_reroute_contract() {
    let mut runtime = OrchestrationSurfaceRuntime::new();
    runtime.record_execution_observation(
        "feedback-reroute",
        infring_orchestration_surface_v1::contracts::CoreExecutionObservation {
            plan_status: Some(infring_orchestration_surface_v1::contracts::PlanStatus::Failed),
            receipt_ids: vec!["receipt-feedback-1".to_string()],
            outcome_refs: vec!["outcome-feedback-1".to_string()],
            step_statuses: vec![
                infring_orchestration_surface_v1::contracts::CoreExecutionStepObservation {
                    step_id: "step_tool_broker_request".to_string(),
                    status: infring_orchestration_surface_v1::contracts::StepStatus::Failed,
                },
            ],
        },
    );

    let package = runtime.orchestrate(
        OrchestrationRequest {
            session_id: "feedback-reroute".to_string(),
            intent: "search release notes".to_string(),
            surface: RequestSurface::Sdk,
            payload: json!({
                "sdk": {
                    "operation_kind": "search",
                    "resource_kind": "web",
                    "request_kind": "direct",
                    "targets": [{ "kind": "url", "value": "https://example.com/releases" }]
                },
                "core_probe_envelope": {
                    "web_search": {
                        "tool_available": true,
                        "transport_available": true
                    }
                }
            }),
        },
        4_976,
    );

    assert!(package.recovery_applied);
    assert!(package
        .classification
        .reasons
        .iter()
        .any(|row| row.starts_with("feedback_reroute:")));
    assert!(package
        .control_plane_lifecycle
        .next_actions
        .iter()
        .any(|row| row.contains("retry") || row.contains("escalate")));
}

#[test]
fn failed_execution_without_viable_alternative_emits_terminal_feedback_contract() {
    let mut runtime = OrchestrationSurfaceRuntime::new();
    runtime.record_execution_observation(
        "feedback-terminal-no-reroute",
        infring_orchestration_surface_v1::contracts::CoreExecutionObservation {
            plan_status: Some(infring_orchestration_surface_v1::contracts::PlanStatus::Failed),
            receipt_ids: vec!["receipt-terminal-feedback-1".to_string()],
            outcome_refs: vec!["outcome-terminal-feedback-1".to_string()],
            step_statuses: vec![
                infring_orchestration_surface_v1::contracts::CoreExecutionStepObservation {
                    step_id: "step_task_fabric_proposal".to_string(),
                    status: infring_orchestration_surface_v1::contracts::StepStatus::Failed,
                },
            ],
        },
    );

    let package = runtime.orchestrate(
        OrchestrationRequest {
            session_id: "feedback-terminal-no-reroute".to_string(),
            intent: "apply task mutation".to_string(),
            surface: RequestSurface::Sdk,
            payload: json!({
                "sdk": {
                    "operation_kind": "mutate",
                    "resource_kind": "task_graph",
                    "request_kind": "direct",
                    "mutability": "mutation",
                    "targets": [{ "kind": "task_id", "value": "task-42" }]
                },
                "core_probe_envelope": {
                    "mutate_task": {
                        "target_supplied": true,
                        "target_syntactically_valid": true,
                        "target_exists": true,
                        "authorization_valid": false,
                        "policy_allows": true
                    }
                }
            }),
        },
        4_977,
    );

    assert!(package
        .classification
        .reasons
        .iter()
        .any(|row| row == "feedback_no_viable_reroute"));
    assert!(package
        .control_plane_lifecycle
        .next_actions
        .iter()
        .any(|row| row.contains("clarification") || row.contains("escalate")));
    assert!(package
        .execution_state
        .recovery
        .as_ref()
        .is_some_and(|row| !matches!(
            row.decision,
            infring_orchestration_surface_v1::contracts::RecoveryDecision::None
        )));
}

#[test]
fn degraded_or_fallback_paths_emit_self_maintenance_recommendations() {
    let mut runtime = OrchestrationSurfaceRuntime::new();
    let package = runtime.orchestrate(
        OrchestrationRequest {
            session_id: "self-maintenance-fallback".to_string(),
            intent: "search release notes".to_string(),
            surface: RequestSurface::Dashboard,
            payload: json!({
                "dashboard": {
                    "selection_mode": "panel"
                }
            }),
        },
        4_978,
    );

    assert!(package
        .fallback_actions
        .iter()
        .any(|row| row.kind == "self_maintenance_review"));
    assert!(package
        .control_plane_lifecycle
        .next_actions
        .iter()
        .any(|row| row.starts_with("self_maintenance_")));
}
