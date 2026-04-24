use super::*;

#[test]
fn ambiguous_legacy_intent_returns_machine_readable_clarification_reason() {
    let mut runtime = OrchestrationSurfaceRuntime::new();
    let package = runtime.orchestrate(
        OrchestrationRequest {
            session_id: "s6".to_string(),
            intent: "maybe do something".to_string(),
            surface: RequestSurface::Legacy,
            payload: json!({}),
        },
        2_500,
    );
    assert!(package.classification.needs_clarification);
    assert!(package
        .classification
        .clarification_reasons
        .contains(&ClarificationReason::AmbiguousOperation));
    assert!(package.summary.contains("clarification"));
}

#[test]
fn mutation_without_target_requires_typed_scope_clarification() {
    let mut runtime = OrchestrationSurfaceRuntime::new();
    let package = runtime.orchestrate(
        OrchestrationRequest {
            session_id: "s7".to_string(),
            intent: "update workflow".to_string(),
            surface: RequestSurface::Legacy,
            payload: json!({}),
        },
        3_000,
    );
    assert!(package.classification.needs_clarification);
    assert!(package
        .classification
        .clarification_reasons
        .contains(&ClarificationReason::MutationScopeRequired));
}

#[test]
fn comparative_request_changes_plan_when_transport_is_unavailable() {
    let mut runtime = OrchestrationSurfaceRuntime::new();
    let ready = runtime.orchestrate(
        OrchestrationRequest {
            session_id: "cmp-ready".to_string(),
            intent: "compare this workspace state to the web".to_string(),
            surface: RequestSurface::Legacy,
            payload: json!({
                "path": "README.md",
                "url": "https://example.com"
            }),
        },
        3_500,
    );
    let degraded = runtime.orchestrate(
        OrchestrationRequest {
            session_id: "cmp-degraded".to_string(),
            intent: "compare this workspace state to the web".to_string(),
            surface: RequestSurface::Legacy,
            payload: json!({
                "path": "README.md",
                "url": "https://example.com",
                "transport_available": false
            }),
        },
        3_600,
    );

    assert!(
        ready
            .core_contract_calls
            .contains(&CoreContractCall::ToolBrokerRequest)
            || ready
                .core_contract_calls
                .contains(&CoreContractCall::VerifierRequest)
    );
    assert!(degraded
        .core_contract_calls
        .contains(&CoreContractCall::ContextTopologyMaterialize));
    assert!(degraded
        .core_contract_calls
        .contains(&CoreContractCall::UnifiedMemoryRead));
    assert!(matches!(
        degraded.execution_state.plan_status,
        infring_orchestration_surface_v1::contracts::PlanStatus::Degraded
    ));
}

#[test]
fn execution_state_is_typed_for_blocked_mutation_requests() {
    let mut runtime = OrchestrationSurfaceRuntime::new();
    let package = runtime.orchestrate(
        OrchestrationRequest {
            session_id: "blocked-mutation".to_string(),
            intent: "implement the requested mutation".to_string(),
            surface: RequestSurface::Legacy,
            payload: json!({
                "target": "task_fabric",
                "authorization_valid": false
            }),
        },
        3_700,
    );

    assert!(package.recovery_applied);
    assert!(matches!(
        package.execution_state.plan_status,
        infring_orchestration_surface_v1::contracts::PlanStatus::Blocked
            | infring_orchestration_surface_v1::contracts::PlanStatus::ClarificationRequired
    ));
    assert_eq!(
        package
            .execution_state
            .recovery
            .as_ref()
            .and_then(|row| row.reason.clone()),
        Some(infring_orchestration_surface_v1::contracts::RecoveryReason::AuthorizationFailure)
    );
}

#[test]
fn sdk_surface_adapter_bypasses_legacy_intent_shim() {
    let mut runtime = OrchestrationSurfaceRuntime::new();
    let package = runtime.orchestrate(
        OrchestrationRequest {
            session_id: "sdk-s1".to_string(),
            intent: "maybe do something".to_string(),
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
                }
            }),
        },
        4_000,
    );
    assert_eq!(package.classification.request_class, RequestClass::ToolCall);
    assert!(package.classification.surface_adapter_used);
    assert!(!package.classification.surface_adapter_fallback);
    assert!(!package
        .classification
        .reasons
        .iter()
        .any(|row| row == "legacy_intent_compatibility_shim"));
    assert!(package
        .classification
        .reasons
        .iter()
        .any(|row| row == "surface_adapter:sdk"));
}

#[test]
fn sdk_and_gateway_adapters_converge_on_same_tool_plan() {
    let mut runtime = OrchestrationSurfaceRuntime::new();
    let sdk = runtime.orchestrate(
        OrchestrationRequest {
            session_id: "sdk-compare".to_string(),
            intent: "compare things".to_string(),
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
        4_100,
    );
    let gateway = runtime.orchestrate(
        OrchestrationRequest {
            session_id: "gateway-compare".to_string(),
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
        4_200,
    );

    assert_eq!(
        sdk.classification.request_class,
        gateway.classification.request_class
    );
    assert_eq!(sdk.core_contract_calls, gateway.core_contract_calls);
    assert!(sdk
        .core_contract_calls
        .contains(&CoreContractCall::ContextTopologyMaterialize));
}

#[test]
fn typed_read_request_avoids_context_append_by_default() {
    let mut runtime = OrchestrationSurfaceRuntime::new();
    let package = runtime.orchestrate(
        OrchestrationRequest {
            session_id: "typed-read-no-append".to_string(),
            intent: "opaque".to_string(),
            surface: RequestSurface::Sdk,
            payload: json!({
                "sdk": {
                    "operation_kind": "read",
                    "resource_kind": "workspace",
                    "request_kind": "direct",
                    "targets": [{ "kind": "workspace_path", "value": "README.md" }]
                }
            }),
        },
        4_250,
    );

    assert!(!package
        .selected_plan
        .steps
        .iter()
        .any(|row| row.target_contract == CoreContractCall::ContextAtomAppend));
    assert!(package.selected_plan.steps.iter().any(|row| {
        row.target_contract == CoreContractCall::ContextTopologyInspect
            || row.target_contract == CoreContractCall::ContextTopologyMaterialize
            || row.target_contract == CoreContractCall::ToolBrokerRequest
            || row.target_contract == CoreContractCall::UnifiedMemoryRead
    }));
}

#[test]
fn comparative_variants_expose_structurally_distinct_capability_graphs() {
    let mut runtime = OrchestrationSurfaceRuntime::new();
    let package = runtime.orchestrate(
        OrchestrationRequest {
            session_id: "comparative-capability-graphs".to_string(),
            intent: "compare this workspace state to web references".to_string(),
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
        4_255,
    );

    let all_capability_graphs = std::iter::once(&package.selected_plan)
        .chain(package.alternative_plans.iter())
        .map(|plan| {
            let mut names = plan
                .capabilities
                .iter()
                .map(|row| format!("{row:?}"))
                .collect::<Vec<_>>();
            names.sort();
            names.join(",")
        })
        .collect::<Vec<_>>();

    let contains_tool_family = |graph: &String| {
        [
            "ExecuteTool",
            "WebSearch",
            "WebFetch",
            "WorkspaceSearch",
            "WorkspaceRead",
            "ToolRoute",
        ]
        .iter()
        .any(|needle| graph.contains(needle))
    };
    assert!(all_capability_graphs.iter().any(contains_tool_family));
    assert!(all_capability_graphs
        .iter()
        .any(|row| !contains_tool_family(row)));
}

#[test]
fn surface_adapter_fallback_requires_clarification() {
    let mut runtime = OrchestrationSurfaceRuntime::new();
    let package = runtime.orchestrate(
        OrchestrationRequest {
            session_id: "dashboard-fallback".to_string(),
            intent: "".to_string(),
            surface: RequestSurface::Dashboard,
            payload: json!({ "dashboard": {} }),
        },
        4_300,
    );
    assert!(package.classification.needs_clarification);
    assert!(!package.classification.surface_adapter_used);
    assert!(package.classification.surface_adapter_fallback);
    assert!(package
        .classification
        .reasons
        .iter()
        .any(|row| row == "surface_adapter_fallback:dashboard"));
}

#[test]
fn dashboard_selection_mode_defaults_to_read_memory_without_fallback() {
    let mut runtime = OrchestrationSurfaceRuntime::new();
    let package = runtime.orchestrate(
        OrchestrationRequest {
            session_id: "dashboard-selection-mode-default".to_string(),
            intent: "".to_string(),
            surface: RequestSurface::Dashboard,
            payload: json!({
                "dashboard": {
                    "selection_mode": "panel"
                }
            }),
        },
        4_302,
    );
    assert!(!package.classification.needs_clarification);
    assert!(package.classification.surface_adapter_used);
    assert!(!package.classification.surface_adapter_fallback);
    assert_eq!(
        package.classification.request_class,
        infring_orchestration_surface_v1::contracts::RequestClass::ReadOnly
    );
    assert!(package
        .classification
        .reasons
        .iter()
        .any(|row| row == "surface_adapter_dashboard_default:read_memory"));
}

#[test]
fn non_legacy_surface_adapter_fallback_requires_authoritative_tool_probe() {
    let mut runtime = OrchestrationSurfaceRuntime::new();
    let package = runtime.orchestrate(
        OrchestrationRequest {
            session_id: "dashboard-fallback-strict-probe".to_string(),
            intent: "search web for release notes".to_string(),
            surface: RequestSurface::Dashboard,
            payload: json!({ "dashboard": {} }),
        },
        4_305,
    );

    assert!(package.classification.needs_clarification);
    assert!(package.classification.surface_adapter_fallback);
    assert!(package.selected_plan.capability_probes.iter().any(|row| {
        row.capability.is_tool_family()
            && row
                .probe_sources
                .iter()
                .any(|source| source == "missing_probe: web_search")
    }));
    assert!(!package.selected_plan.capability_probes.iter().any(|row| {
        row.capability.is_tool_family()
            && row
                .probe_sources
                .iter()
                .any(|source| source == "heuristic.tool_hints_or_resource_kind")
    }));
}

#[test]
fn adapted_tool_request_requires_explicit_transport_probe() {
    let mut runtime = OrchestrationSurfaceRuntime::new();
    let package = runtime.orchestrate(
        OrchestrationRequest {
            session_id: "sdk-missing-transport".to_string(),
            intent: "opaque".to_string(),
            surface: RequestSurface::Sdk,
            payload: json!({
                "sdk": {
                    "operation_kind": "search",
                    "resource_kind": "web",
                    "request_kind": "direct",
                    "tool_hints": ["web_search"],
                    "targets": [{ "kind": "url", "value": "https://example.com/releases" }]
                }
            }),
        },
        4_320,
    );

    assert_eq!(
        package.execution_state.plan_status,
        infring_orchestration_surface_v1::contracts::PlanStatus::Degraded
    );
    assert_eq!(
        package
            .execution_state
            .recovery
            .as_ref()
            .and_then(|row| row.reason.clone()),
        Some(infring_orchestration_surface_v1::contracts::RecoveryReason::TransportFailure)
    );
    assert!(package
        .selected_plan
        .blocked_on
        .contains(&infring_orchestration_surface_v1::contracts::Precondition::TransportAvailable));
    assert!(package.selected_plan.capability_probes.iter().any(|row| {
        row.capability.is_tool_family()
            && row.probe_sources.iter().any(|source| {
                source == "probe.required_for_typed_surface.web_search.transport_available"
            })
    }));
}
