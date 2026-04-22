use super::*;

#[test]
fn adapted_tool_request_requires_explicit_tool_probe() {
    let mut runtime = OrchestrationSurfaceRuntime::new();
    let package = runtime.orchestrate(
        OrchestrationRequest {
            session_id: "sdk-missing-tool-probe".to_string(),
            intent: "opaque".to_string(),
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
                        "transport_available": true
                    }
                }
            }),
        },
        4_329,
    );

    assert_eq!(
        package.execution_state.plan_status,
        infring_orchestration_surface_v1::contracts::PlanStatus::Blocked
    );
    assert_eq!(
        package
            .execution_state
            .recovery
            .as_ref()
            .and_then(|row| row.reason.clone()),
        Some(infring_orchestration_surface_v1::contracts::RecoveryReason::PlannerContradiction)
    );
    assert!(package
        .selected_plan
        .blocked_on
        .contains(&infring_orchestration_surface_v1::contracts::Precondition::ToolAvailable));
    assert!(package.selected_plan.capability_probes.iter().any(|row| {
        row.capability.is_tool_family()
            && row.probe_sources.iter().any(|source| {
                source == "probe.required_for_typed_surface.web_search.tool_available"
            })
    }));
}

#[test]
fn adapted_tool_request_rejects_payload_tool_probe_shortcut() {
    let mut runtime = OrchestrationSurfaceRuntime::new();
    let package = runtime.orchestrate(
        OrchestrationRequest {
            session_id: "sdk-tool-shortcut-rejected".to_string(),
            intent: "opaque".to_string(),
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
                        "transport_available": true
                    }
                },
                "capability_probes": {
                    "web_search": {
                        "tool_available": true
                    }
                }
            }),
        },
        43_291,
    );

    assert!(package
        .selected_plan
        .blocked_on
        .contains(&infring_orchestration_surface_v1::contracts::Precondition::ToolAvailable));
    assert!(package.selected_plan.capability_probes.iter().any(|row| {
        row.capability.is_tool_family()
            && row.probe_sources.iter().any(|source| {
                source == "probe.required_for_typed_surface.web_search.tool_available"
            })
    }));
}

#[test]
fn adapted_workspace_request_requires_workspace_specific_probe_fields() {
    let mut runtime = OrchestrationSurfaceRuntime::new();
    let package = runtime.orchestrate(
        OrchestrationRequest {
            session_id: "sdk-workspace-missing-tool-probe".to_string(),
            intent: "opaque".to_string(),
            surface: RequestSurface::Sdk,
            payload: json!({
                "sdk": {
                    "operation_kind": "search",
                    "resource_kind": "workspace",
                    "request_kind": "direct",
                    "targets": [{ "kind": "workspace_path", "value": "README.md" }]
                }
            }),
        },
        43_292,
    );

    assert!(package
        .selected_plan
        .blocked_on
        .contains(&infring_orchestration_surface_v1::contracts::Precondition::ToolAvailable));
    assert!(package.selected_plan.capability_probes.iter().any(|row| {
        matches!(
            row.capability,
            infring_orchestration_surface_v1::contracts::Capability::WorkspaceRead
                | infring_orchestration_surface_v1::contracts::Capability::WorkspaceSearch
        ) && row.probe_sources.iter().any(|source| {
            source == "probe.required_for_typed_surface.workspace_search.tool_available"
                || source == "probe.required_for_typed_surface.workspace_read.tool_available"
        })
    }));
}

#[test]
fn adapted_assimilation_request_requires_explicit_policy_probe() {
    let mut runtime = OrchestrationSurfaceRuntime::new();
    let package = runtime.orchestrate(
        OrchestrationRequest {
            session_id: "sdk-missing-policy".to_string(),
            intent: "opaque".to_string(),
            surface: RequestSurface::Sdk,
            payload: json!({
                "sdk": {
                    "operation_kind": "assimilate",
                    "resource_kind": "workspace",
                    "targets": [{ "kind": "workspace_path", "value": "README.md" }]
                },
                "core_probe_envelope": {
                    "plan_assimilation": {
                        "target_supplied": true,
                        "target_syntactically_valid": true,
                        "target_exists": true
                    },
                    "mutate_task": {
                        "target_supplied": true,
                        "target_syntactically_valid": true,
                        "target_exists": true,
                        "authorization_valid": true,
                        "policy_allows": true
                    }
                }
            }),
        },
        4_330,
    );

    assert_eq!(
        package.execution_state.plan_status,
        infring_orchestration_surface_v1::contracts::PlanStatus::Blocked
    );
    assert_eq!(
        package
            .execution_state
            .recovery
            .as_ref()
            .and_then(|row| row.reason.clone()),
        Some(infring_orchestration_surface_v1::contracts::RecoveryReason::PolicyDenied)
    );
    assert!(package
        .selected_plan
        .blocked_on
        .contains(&infring_orchestration_surface_v1::contracts::Precondition::PolicyAllows));
    assert!(package.selected_plan.capability_probes.iter().any(|row| {
        row.capability == infring_orchestration_surface_v1::contracts::Capability::PlanAssimilation
            && row.probe_sources.iter().any(|source| {
                source == "probe.required_for_typed_surface.plan_assimilation.policy_allows"
            })
    }));
}

#[test]
fn adapted_assimilation_rejects_payload_policy_shortcut_without_probe_envelope() {
    let mut runtime = OrchestrationSurfaceRuntime::new();
    let package = runtime.orchestrate(
        OrchestrationRequest {
            session_id: "sdk-policy-shortcut-rejected".to_string(),
            intent: "opaque".to_string(),
            surface: RequestSurface::Sdk,
            payload: json!({
                "sdk": {
                    "operation_kind": "assimilate",
                    "resource_kind": "workspace",
                    "targets": [{ "kind": "workspace_path", "value": "README.md" }]
                },
                "capability_probes": {
                    "plan_assimilation": {
                        "policy_allows": true
                    }
                }
            }),
        },
        4_331,
    );

    assert!(package
        .selected_plan
        .blocked_on
        .contains(&infring_orchestration_surface_v1::contracts::Precondition::PolicyAllows));
    assert!(package.selected_plan.capability_probes.iter().any(|row| {
        row.capability == infring_orchestration_surface_v1::contracts::Capability::PlanAssimilation
            && row.probe_sources.iter().any(|source| {
                source == "probe.required_for_typed_surface.plan_assimilation.policy_allows"
            })
    }));
}

#[test]
fn adapted_mutation_request_requires_explicit_authorization_probe() {
    let mut runtime = OrchestrationSurfaceRuntime::new();
    let package = runtime.orchestrate(
        OrchestrationRequest {
            session_id: "sdk-missing-authorization-probe".to_string(),
            intent: "opaque".to_string(),
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
                        "policy_allows": true
                    }
                }
            }),
        },
        4_332,
    );

    assert_eq!(
        package.execution_state.plan_status,
        infring_orchestration_surface_v1::contracts::PlanStatus::Blocked
    );
    assert_eq!(
        package
            .execution_state
            .recovery
            .as_ref()
            .and_then(|row| row.reason.clone()),
        Some(infring_orchestration_surface_v1::contracts::RecoveryReason::AuthorizationFailure)
    );
    assert!(package
        .selected_plan
        .blocked_on
        .contains(&infring_orchestration_surface_v1::contracts::Precondition::AuthorizationValid));
    assert!(package.selected_plan.capability_probes.iter().any(|row| {
        row.capability == infring_orchestration_surface_v1::contracts::Capability::MutateTask
            && row.probe_sources.iter().any(|source| {
                source == "probe.required_for_typed_surface.mutate_task.authorization_valid"
            })
    }));
}

#[test]
fn adapted_mutation_rejects_payload_authorization_shortcut_without_probe_envelope() {
    let mut runtime = OrchestrationSurfaceRuntime::new();
    let package = runtime.orchestrate(
        OrchestrationRequest {
            session_id: "sdk-authorization-shortcut-rejected".to_string(),
            intent: "opaque".to_string(),
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
                        "policy_allows": true
                    }
                },
                "capability_probes": {
                    "mutate_task": {
                        "authorization_valid": true
                    }
                }
            }),
        },
        4_333,
    );

    assert!(package
        .selected_plan
        .blocked_on
        .contains(&infring_orchestration_surface_v1::contracts::Precondition::AuthorizationValid));
    assert!(package.selected_plan.capability_probes.iter().any(|row| {
        row.capability == infring_orchestration_surface_v1::contracts::Capability::MutateTask
            && row.probe_sources.iter().any(|source| {
                source == "probe.required_for_typed_surface.mutate_task.authorization_valid"
            })
    }));
}
