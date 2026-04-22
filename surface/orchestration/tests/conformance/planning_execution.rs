use super::*;

#[test]
fn non_legacy_surface_fixture_fallback_rate_stays_below_threshold() {
    let fixtures = vec![
        OrchestrationRequest {
            session_id: "sdk-metric".to_string(),
            intent: "opaque".to_string(),
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
        OrchestrationRequest {
            session_id: "gateway-metric".to_string(),
            intent: "opaque".to_string(),
            surface: RequestSurface::Gateway,
            payload: json!({
                "gateway": {
                    "route": "compare.resource",
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
        OrchestrationRequest {
            session_id: "cli-metric".to_string(),
            intent: "opaque".to_string(),
            surface: RequestSurface::Cli,
            payload: json!({
                "cli": {
                    "command": "read",
                    "resource_kind": "workspace",
                    "targets": [{ "kind": "workspace_path", "value": "README.md" }]
                }
            }),
        },
        OrchestrationRequest {
            session_id: "dashboard-metric-fallback".to_string(),
            intent: "".to_string(),
            surface: RequestSurface::Dashboard,
            payload: json!({
                "dashboard": {
                    "selection_mode": "panel"
                }
            }),
        },
    ];
    let mut runtime = OrchestrationSurfaceRuntime::new();
    let packages = fixtures
        .into_iter()
        .enumerate()
        .map(|(idx, request)| runtime.orchestrate(request, 4_600 + idx as u64))
        .collect::<Vec<_>>();
    let fallback_count = packages
        .iter()
        .filter(|row| row.classification.surface_adapter_fallback)
        .count();
    let fallback_rate = fallback_count as f32 / packages.len() as f32;

    assert!(
        fallback_rate <= 0.25,
        "fallback rate should stay below threshold"
    );
    assert_eq!(fallback_count, 1);
}

#[test]
fn direct_tool_request_plan_variants_are_structurally_distinct() {
    let mut runtime = OrchestrationSurfaceRuntime::new();
    let package = runtime.orchestrate(
        OrchestrationRequest {
            session_id: "sdk-tool-variants".to_string(),
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
        4_399,
    );

    let mut signatures = std::iter::once(&package.selected_plan)
        .chain(package.alternative_plans.iter())
        .map(|plan| {
            plan.steps
                .iter()
                .map(|row| row.step_id.clone())
                .collect::<Vec<_>>()
                .join("->")
        })
        .collect::<Vec<_>>();
    signatures.sort();
    signatures.dedup();
    assert!(
        signatures.len() >= 2,
        "direct tool plans should preserve structurally distinct variants"
    );
    assert!(std::iter::once(&package.selected_plan)
        .chain(package.alternative_plans.iter())
        .any(|plan| plan
            .steps
            .iter()
            .any(|row| row.step_id.ends_with("_memory_fallback"))));
}

#[test]
fn comparative_request_exposes_verifier_and_alternative_plan_provenance() {
    let mut runtime = OrchestrationSurfaceRuntime::new();
    let package = runtime.orchestrate(
        OrchestrationRequest {
            session_id: "cmp-alt".to_string(),
            intent: "compare".to_string(),
            surface: RequestSurface::Sdk,
            payload: json!({
                "sdk": {
                    "operation_kind": "compare",
                    "resource_kind": "mixed",
                    "targets": [
                        { "kind": "workspace_path", "value": "README.md" },
                        { "kind": "url", "value": "https://example.com" }
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
        4_400,
    );

    assert!(package
        .core_contract_calls
        .contains(&CoreContractCall::VerifierRequest));
    assert!(!package.alternative_plans.is_empty());
    assert!(package.alternative_plans.iter().any(|row| row.variant
        == infring_orchestration_surface_v1::contracts::PlanVariant::ClarificationFirst));
    let mut signatures = std::iter::once(&package.selected_plan)
        .chain(package.alternative_plans.iter())
        .map(|plan| {
            plan.steps
                .iter()
                .map(|row| row.step_id.clone())
                .collect::<Vec<_>>()
                .join("->")
        })
        .collect::<Vec<_>>();
    signatures.sort();
    signatures.dedup();
    assert!(
        signatures.len() >= 2,
        "plan variants should preserve structurally distinct step sequences"
    );
    let merged_memory_step = package
        .selected_plan
        .steps
        .iter()
        .find(|row| row.target_contract == CoreContractCall::ContextTopologyMaterialize)
        .expect("shared memory read step");
    assert!(merged_memory_step
        .merged_capabilities
        .contains(&infring_orchestration_surface_v1::contracts::Capability::ReadMemory));
    assert!(merged_memory_step
        .merged_capabilities
        .contains(&infring_orchestration_surface_v1::contracts::Capability::VerifyClaim));
    assert!(
        merged_memory_step.expected_contract_refs.len() >= 2,
        "merged shared step should preserve multiple expected contract refs"
    );
}

#[test]
fn observed_core_execution_is_projected_into_execution_state_correlation() {
    let mut runtime = OrchestrationSurfaceRuntime::new();
    runtime.record_execution_observation(
        "core-running",
        infring_orchestration_surface_v1::contracts::CoreExecutionObservation {
            plan_status: Some(infring_orchestration_surface_v1::contracts::PlanStatus::Completed),
            receipt_ids: vec!["receipt-1".to_string(), "receipt-2".to_string()],
            outcome_refs: vec!["outcome-1".to_string()],
            step_statuses: vec![
                infring_orchestration_surface_v1::contracts::CoreExecutionStepObservation {
                    step_id: "step_tool_route_capability_probe".to_string(),
                    status: infring_orchestration_surface_v1::contracts::StepStatus::Succeeded,
                },
                infring_orchestration_surface_v1::contracts::CoreExecutionStepObservation {
                    step_id: "step_claim_verifier_request".to_string(),
                    status: infring_orchestration_surface_v1::contracts::StepStatus::Failed,
                },
            ],
        },
    );
    let package = runtime.orchestrate(
        OrchestrationRequest {
            session_id: "core-running".to_string(),
            intent: "compare this workspace state to the web".to_string(),
            surface: RequestSurface::Legacy,
            payload: json!({
                "path": "README.md",
                "url": "https://example.com"
            }),
        },
        4_500,
    );

    assert_eq!(
        package.execution_state.plan_status,
        infring_orchestration_surface_v1::contracts::PlanStatus::Completed
    );
    assert!(!package.execution_state.steps.is_empty());
    assert!(package.execution_state.steps.iter().any(|row| {
        row.step_id == "step_claim_verifier_request"
            && row.status == infring_orchestration_surface_v1::contracts::StepStatus::Failed
    }));
    assert_eq!(
        package
            .execution_state
            .correlation
            .observed_core_receipt_ids,
        vec!["receipt-1".to_string(), "receipt-2".to_string()]
    );
    assert_eq!(
        package
            .execution_state
            .correlation
            .observed_core_outcome_refs,
        vec!["outcome-1".to_string()]
    );
    assert!(!package
        .execution_state
        .correlation
        .expected_core_contract_ids
        .is_empty());
}

#[test]
fn typed_execution_observation_derives_plan_status_from_step_outcomes() {
    let mut runtime = OrchestrationSurfaceRuntime::new();
    runtime.record_execution_observation(
        "typed-observation-derived-status",
        infring_orchestration_surface_v1::contracts::CoreExecutionObservation {
            plan_status: None,
            receipt_ids: vec!["receipt-typed-1".to_string()],
            outcome_refs: Vec::new(),
            step_statuses: vec![
                infring_orchestration_surface_v1::contracts::CoreExecutionStepObservation {
                    step_id: "step_tool_capability_probe".to_string(),
                    status: infring_orchestration_surface_v1::contracts::StepStatus::Succeeded,
                },
                infring_orchestration_surface_v1::contracts::CoreExecutionStepObservation {
                    step_id: "step_tool_broker_request".to_string(),
                    status: infring_orchestration_surface_v1::contracts::StepStatus::Failed,
                },
            ],
        },
    );
    let package = runtime.orchestrate(
        OrchestrationRequest {
            session_id: "typed-observation-derived-status".to_string(),
            intent: "search web for release notes".to_string(),
            surface: RequestSurface::Sdk,
            payload: json!({
                "sdk": {
                    "operation_kind": "search",
                    "resource_kind": "web",
                    "request_kind": "direct",
                    "tool_hints": ["web_search"],
                    "targets": [{ "kind": "url", "value": "https://example.com/releases" }]
                },
                "core_probe_envelope": {
                    "web_search": {
                        "tool_available": true,
                        "transport_available": true
                    }
                },
                "core_execution_observation": {
                    "receipt_ids": ["request-payload-should-be-ignored"],
                    "status": "running"
                }
            }),
        },
        4_526,
    );

    assert_eq!(
        package.execution_state.plan_status,
        infring_orchestration_surface_v1::contracts::PlanStatus::Failed
    );
    assert_eq!(
        package
            .execution_state
            .correlation
            .observed_core_receipt_ids,
        vec!["receipt-typed-1".to_string()]
    );
}

#[test]
fn direct_tool_request_keeps_structurally_distinct_plan_variants() {
    let mut runtime = OrchestrationSurfaceRuntime::new();
    let package = runtime.orchestrate(
        OrchestrationRequest {
            session_id: "direct-variant-distinct".to_string(),
            intent: "search web for release notes".to_string(),
            surface: RequestSurface::Legacy,
            payload: json!({}),
        },
        4_525,
    );

    let mut signatures = std::iter::once(&package.selected_plan)
        .chain(package.alternative_plans.iter())
        .map(|plan| {
            plan.steps
                .iter()
                .map(|row| row.step_id.clone())
                .collect::<Vec<_>>()
                .join("->")
        })
        .collect::<Vec<_>>();
    signatures.sort();
    signatures.dedup();
    assert!(
        signatures.len() >= 2,
        "direct tool-call plans should preserve structurally distinct variants"
    );
}

#[test]
fn invalid_target_is_reported_separately_from_missing_target() {
    let mut runtime = OrchestrationSurfaceRuntime::new();
    let package = runtime.orchestrate(
        OrchestrationRequest {
            session_id: "invalid-target".to_string(),
            intent: "implement the requested mutation".to_string(),
            surface: RequestSurface::Legacy,
            payload: json!({
                "target": "???",
                "target_syntactically_valid": false
            }),
        },
        4_550,
    );

    assert_eq!(
        package
            .execution_state
            .recovery
            .as_ref()
            .and_then(|row| row.reason.clone()),
        Some(infring_orchestration_surface_v1::contracts::RecoveryReason::TargetInvalid)
    );
    assert_eq!(package.classification.request_class, RequestClass::Mutation);
    assert_eq!(
        package.selected_plan.blocked_on,
        vec![infring_orchestration_surface_v1::contracts::Precondition::TargetSyntacticallyValid]
    );
}

#[test]
fn missing_existing_target_is_reported_as_not_found() {
    let mut runtime = OrchestrationSurfaceRuntime::new();
    let package = runtime.orchestrate(
        OrchestrationRequest {
            session_id: "missing-target".to_string(),
            intent: "implement the requested mutation".to_string(),
            surface: RequestSurface::Legacy,
            payload: json!({
                "target": "task_fabric",
                "target_exists": false
            }),
        },
        4_560,
    );

    assert_eq!(
        package
            .execution_state
            .recovery
            .as_ref()
            .and_then(|row| row.reason.clone()),
        Some(infring_orchestration_surface_v1::contracts::RecoveryReason::TargetNotFound)
    );
    assert!(package
        .selected_plan
        .blocked_on
        .contains(&infring_orchestration_surface_v1::contracts::Precondition::TargetExists));
}

#[test]
fn degraded_comparative_request_preserves_multiple_probe_failures() {
    let mut runtime = OrchestrationSurfaceRuntime::new();
    let package = runtime.orchestrate(
        OrchestrationRequest {
            session_id: "cmp-multi-degrade".to_string(),
            intent: "compare this workspace state to the web".to_string(),
            surface: RequestSurface::Legacy,
            payload: json!({
                "path": "README.md",
                "url": "https://example.com",
                "tool_available": false,
                "transport_available": false
            }),
        },
        4_600,
    );

    let degradation = package
        .execution_state
        .degradation
        .as_ref()
        .expect("degradation state");
    assert!(degradation.reasons.contains(
        &infring_orchestration_surface_v1::contracts::DegradationReason::ToolUnavailable
    ));
    assert!(degradation.reasons.contains(
        &infring_orchestration_surface_v1::contracts::DegradationReason::TransportFailure
    ));
}

#[test]
fn adapted_mutation_request_requires_explicit_target_probe_envelope_fields() {
    let mut runtime = OrchestrationSurfaceRuntime::new();
    let package = runtime.orchestrate(
        OrchestrationRequest {
            session_id: "sdk-missing-target-probe".to_string(),
            intent: "opaque".to_string(),
            surface: RequestSurface::Sdk,
            payload: json!({
                "sdk": {
                    "operation_kind": "mutate",
                    "resource_kind": "task_graph",
                    "request_kind": "direct",
                    "mutability": "mutation",
                    "targets": [{ "kind": "task_id", "value": "task-99" }]
                },
                "core_probe_envelope": {
                    "mutate_task": {
                        "authorization_valid": true,
                        "policy_allows": true
                    }
                }
            }),
        },
        4_650,
    );

    assert!(package
        .selected_plan
        .blocked_on
        .contains(&infring_orchestration_surface_v1::contracts::Precondition::TargetSupplied));
    assert!(package.selected_plan.capability_probes.iter().any(|row| {
        row.capability == infring_orchestration_surface_v1::contracts::Capability::MutateTask
            && row.probe_sources.iter().any(|source| {
                source == "probe.required_for_typed_surface.mutate_task.target_supplied"
            })
    }));
}
