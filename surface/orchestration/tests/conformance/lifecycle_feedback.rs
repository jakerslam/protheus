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
fn workflow_phase_trace_projects_orchestration_lifecycle_for_eval_consumers() {
    let mut runtime = OrchestrationSurfaceRuntime::new();
    let package = runtime.orchestrate(
        OrchestrationRequest {
            session_id: "phase-trace-contract".to_string(),
            intent: "read workspace files and summarize findings".to_string(),
            surface: RequestSurface::Sdk,
            payload: json!({
                "sdk": {
                    "operation_kind": "read",
                    "resource_kind": "workspace",
                    "request_kind": "direct",
                    "targets": [{ "kind": "workspace_path", "value": "README.md" }]
                },
                "core_probe_envelope": {
                    "workspace_read": {
                        "tool_available": true,
                        "transport_available": true
                    }
                }
            }),
        },
        5_001,
    );

    let trace =
        infring_orchestration_surface_v1::telemetry::build_workflow_phase_trace(&package, 5_001);

    assert_eq!(
        trace.trace_type,
        infring_orchestration_surface_v1::telemetry::WORKFLOW_PHASE_TRACE_TYPE
    );
    assert_eq!(trace.owner, "surface_orchestration_control_plane");
    assert_eq!(trace.workflow_template, package.workflow_template);
    assert_eq!(
        trace.active_stage,
        package.control_plane_lifecycle.active_stage
    );
    assert!(trace.phases.iter().any(|row| {
        row.phase == WorkflowStage::IntakeNormalization && row.eval_visible
    }));
    assert!(trace
        .collectors
        .iter()
        .any(|row| row.collector_id == "dashboard_troubleshooting_snapshot"));
    assert_eq!(
        trace.expected_kernel_contract_ids,
        package
            .execution_state
            .correlation
            .expected_core_contract_ids
    );
    assert!(!trace.receipt_hash.is_empty());
    let serialized = serde_json::to_value(&trace).expect("trace serializes");
    assert_eq!(
        serialized.get("type").and_then(|value| value.as_str()),
        Some("orchestration_workflow_phase_trace")
    );
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

#[test]
fn forgecode_assimilation_request_selects_forgecode_workflow_template_and_lane_actions() {
    let mut runtime = OrchestrationSurfaceRuntime::new();
    let package = runtime.orchestrate(
        OrchestrationRequest {
            session_id: "forgecode-template-selection".to_string(),
            intent: "assimilate forgecode workflow lanes and routing mechanics".to_string(),
            surface: RequestSurface::Sdk,
            payload: json!({
                "sdk": {
                    "operation_kind": "assimilate",
                    "resource_kind": "tooling",
                    "request_kind": "workflow",
                    "targets": [
                        { "kind": "url", "value": "https://github.com/antinomyhq/forgecode" },
                        { "kind": "workspace_path", "value": "local/workspace/assimilations/ForgeCode-Assimilation" }
                    ]
                },
                "core_probe_envelope": {
                    "plan_assimilation": {
                        "transport_available": true
                    },
                    "tool_route": {
                        "tool_available": true,
                        "transport_available": true
                    }
                }
            }),
            },
        4_979,
    );

    assert_eq!(
        package.workflow_template,
        infring_orchestration_surface_v1::contracts::WorkflowTemplate::ForgeCodeAgentComposition
    );
    assert!(package
        .control_plane_lifecycle
        .next_actions
        .iter()
        .any(|row| row == "require_tool_name_alias_normalization_before_route_probe"));
    assert!(package
        .control_plane_lifecycle
        .next_actions
        .iter()
        .any(|row| row == "require_retryable_error_backoff_contract_for_tool_calls"));
    assert!(package
        .control_plane_lifecycle
        .next_actions
        .iter()
        .any(|row| row == "require_mcp_transport_fallback_http_then_sse"));
}

#[test]
fn tool_failure_budget_recovery_projects_budget_quality_and_retry_guard_action() {
    let mut runtime = OrchestrationSurfaceRuntime::new();
    runtime.record_execution_observation(
        "forgecode-budget-quality",
        infring_orchestration_surface_v1::contracts::CoreExecutionObservation {
            plan_status: Some(infring_orchestration_surface_v1::contracts::PlanStatus::Failed),
            receipt_ids: vec!["receipt-budget-1".to_string()],
            outcome_refs: vec!["outcome-budget-1".to_string()],
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
            session_id: "forgecode-budget-quality".to_string(),
            intent: "assimilate forgecode retry policy and enforce budget".to_string(),
            surface: RequestSurface::Sdk,
            payload: json!({
                "max_tool_failure_per_turn": 1,
                "sdk": {
                    "operation_kind": "search",
                    "resource_kind": "web",
                    "request_kind": "direct",
                    "targets": [{ "kind": "url", "value": "https://github.com/antinomyhq/forgecode" }]
                },
                "core_probe_envelope": {
                    "web_search": {
                        "tool_available": true,
                        "transport_available": true
                    }
                }
            }),
        },
        4_980,
    );

    assert_eq!(
        package
            .execution_state
            .recovery
            .as_ref()
            .and_then(|row| row.reason.clone()),
        Some(
            infring_orchestration_surface_v1::contracts::RecoveryReason::ToolFailureBudgetExceeded
        )
    );
    assert!(package.runtime_quality.tool_failure_budget_exceeded);
    assert_eq!(package.runtime_quality.tool_failure_budget_limit, 1);
    assert!(
        package
            .runtime_quality
            .tool_failure_budget_failed_step_count
            >= 1
    );
    assert!(package
        .control_plane_lifecycle
        .next_actions
        .iter()
        .any(|row| row == "require_scope_narrowing_or_budget_override_before_retry"));
}
