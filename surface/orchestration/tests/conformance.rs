// Layer ownership: tests (regression proof for orchestration surface contracts).
use infring_orchestration_surface_v1::contracts::{
    Capability, CapabilityProbeSnapshot, ClarificationReason, CoreContractCall, CoreProbeEnvelope,
    Mutability, OperationKind, OrchestrationRequest, PolicyScope, Precondition, RequestClass,
    RequestKind, RequestSurface, ResourceKind, TargetDescriptor, TypedOrchestrationRequest,
};
use infring_orchestration_surface_v1::OrchestrationSurfaceRuntime;
use serde_json::json;

#[test]
fn orchestration_surface_cannot_bypass_tool_broker() {
    let mut runtime = OrchestrationSurfaceRuntime::new();
    let package = runtime.orchestrate(
        OrchestrationRequest {
            session_id: "s1".to_string(),
            intent: "search web for release notes".to_string(),
            surface: RequestSurface::Legacy,
            payload: json!({}),
        },
        1_000,
    );
    assert_eq!(
        package.core_contract_calls,
        vec![
            CoreContractCall::ToolCapabilityProbe,
            CoreContractCall::ToolBrokerRequest
        ]
    );
    assert_eq!(package.classification.request_class, RequestClass::ToolCall);
    assert!(package
        .fallback_actions
        .iter()
        .any(|row| row.kind == "inspect_tool_capabilities"));
}

#[test]
fn orchestration_surface_cannot_persist_private_durable_task_state() {
    let mut runtime = OrchestrationSurfaceRuntime::new();
    let _ = runtime.orchestrate(
        OrchestrationRequest {
            session_id: "s2".to_string(),
            intent: "plan tasks".to_string(),
            surface: RequestSurface::Legacy,
            payload: json!({}),
        },
        1_000,
    );
    assert_eq!(runtime.transient_entry_count(), 1);
    let swept = runtime.sweep_transient(31_500);
    assert_eq!(swept, 1);
    assert_eq!(runtime.transient_entry_count(), 0);
}

#[test]
fn orchestration_surface_cannot_canonize_truth() {
    let mut runtime = OrchestrationSurfaceRuntime::new();
    let package = runtime.orchestrate(
        OrchestrationRequest {
            session_id: "s3".to_string(),
            intent: "update workflow".to_string(),
            surface: RequestSurface::Legacy,
            payload: json!({"target":"task_fabric"}),
        },
        1_000,
    );
    assert!(package.requires_core_promotion);
    assert!(package
        .core_contract_calls
        .contains(&CoreContractCall::TaskFabricProposal));
}

#[test]
fn orchestration_transient_state_is_sweepable() {
    let mut runtime = OrchestrationSurfaceRuntime::new();
    let _ = runtime.orchestrate(
        OrchestrationRequest {
            session_id: "s4".to_string(),
            intent: "read status".to_string(),
            surface: RequestSurface::Legacy,
            payload: json!({}),
        },
        10_000,
    );
    assert_eq!(runtime.transient_entry_count(), 1);
    assert_eq!(runtime.sweep_transient(9_000), 0);
    assert_eq!(runtime.sweep_transient(40_001), 1);
}

#[test]
fn orchestration_transient_restart_requires_boot_sweep_before_resume() {
    let mut runtime = OrchestrationSurfaceRuntime::new();
    let _ = runtime.orchestrate(
        OrchestrationRequest {
            session_id: "s5".to_string(),
            intent: "hold short-term context".to_string(),
            surface: RequestSurface::Legacy,
            payload: json!({}),
        },
        10_000,
    );
    assert_eq!(runtime.transient_entry_count(), 1);
    assert_eq!(runtime.transient_ephemeral_count(), 1);

    runtime.begin_transient_restart();
    let blocked = runtime
        .resume_transient_after_restart()
        .expect_err("resume should block on stale transient payload");
    assert!(blocked.starts_with("transient_context_resume_blocked:"));

    let swept = runtime
        .sweep_transient_before_resume()
        .expect("boot sweep should succeed");
    assert_eq!(swept, 1);
    assert_eq!(runtime.transient_entry_count(), 0);
    assert_eq!(runtime.transient_ephemeral_count(), 0);
    runtime
        .resume_transient_after_restart()
        .expect("resume should succeed after boot sweep");
}

#[test]
fn orchestration_sleep_cycle_cleanup_wipes_transient_ephemeral_state() {
    let mut runtime = OrchestrationSurfaceRuntime::new();
    let _ = runtime.orchestrate(
        OrchestrationRequest {
            session_id: "sleep-cycle-1".to_string(),
            intent: "hold short-term context".to_string(),
            surface: RequestSurface::Legacy,
            payload: json!({}),
        },
        10_000,
    );
    let _ = runtime.orchestrate(
        OrchestrationRequest {
            session_id: "sleep-cycle-2".to_string(),
            intent: "hold short-term context".to_string(),
            surface: RequestSurface::Legacy,
            payload: json!({}),
        },
        10_200,
    );
    assert_eq!(runtime.transient_entry_count(), 2);
    assert_eq!(runtime.transient_ephemeral_count(), 2);

    let report = runtime
        .run_transient_sleep_cycle_cleanup("nightly")
        .expect("sleep cleanup should succeed");
    assert_eq!(report.cleaned_count, 2);
    assert_eq!(report.removed_session_count, 2);
    assert_eq!(runtime.transient_entry_count(), 0);
    assert_eq!(runtime.transient_ephemeral_count(), 0);
}

#[test]
fn orchestration_auto_sleep_cycle_cleanup_runs_on_idle_gap() {
    let mut runtime = OrchestrationSurfaceRuntime::new().with_sleep_cycle_idle_gap_ms(10_000);
    let _ = runtime.orchestrate(
        OrchestrationRequest {
            session_id: "auto-sleep-1".to_string(),
            intent: "hold short-term context".to_string(),
            surface: RequestSurface::Legacy,
            payload: json!({}),
        },
        10_000,
    );
    assert_eq!(runtime.transient_entry_count(), 1);
    assert_eq!(runtime.transient_ephemeral_count(), 1);

    let _ = runtime.orchestrate(
        OrchestrationRequest {
            session_id: "auto-sleep-2".to_string(),
            intent: "continue after idle period".to_string(),
            surface: RequestSurface::Legacy,
            payload: json!({}),
        },
        25_000,
    );
    assert_eq!(runtime.transient_entry_count(), 1);
    assert_eq!(runtime.transient_ephemeral_count(), 1);
}

#[test]
fn orchestration_legacy_intent_path_still_produces_typed_tool_plan() {
    let mut runtime = OrchestrationSurfaceRuntime::new();
    let package = runtime.orchestrate(
        OrchestrationRequest {
            session_id: "legacy-s1".to_string(),
            intent: "  Search web for release notes  ".to_string(),
            surface: RequestSurface::Legacy,
            payload: serde_json::Value::Null,
        },
        2_000,
    );
    assert_eq!(package.classification.request_class, RequestClass::ToolCall);
    assert!(!package.classification.needs_clarification);
    assert!(package
        .classification
        .reasons
        .iter()
        .any(|row| row == "legacy_intent_compatibility_shim"));
}

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
                    "execute_tool": {
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
                    "execute_tool": {
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
                    "execute_tool": {
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
                    "execute_tool": {
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

    assert!(all_capability_graphs
        .iter()
        .any(|row| row.contains("ExecuteTool")));
    assert!(all_capability_graphs
        .iter()
        .any(|row| !row.contains("ExecuteTool")));
}

#[test]
fn surface_adapter_fallback_requires_clarification() {
    let mut runtime = OrchestrationSurfaceRuntime::new();
    let package = runtime.orchestrate(
        OrchestrationRequest {
            session_id: "dashboard-fallback".to_string(),
            intent: "".to_string(),
            surface: RequestSurface::Dashboard,
            payload: json!({
                "dashboard": {
                    "selection_mode": "panel"
                }
            }),
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
fn non_legacy_surface_adapter_fallback_uses_heuristics_and_stays_clarification_first() {
    let mut runtime = OrchestrationSurfaceRuntime::new();
    let package = runtime.orchestrate(
        OrchestrationRequest {
            session_id: "dashboard-fallback-strict-probe".to_string(),
            intent: "search web for release notes".to_string(),
            surface: RequestSurface::Dashboard,
            payload: json!({
                "dashboard": {
                    "selection_mode": "panel"
                }
            }),
        },
        4_305,
    );

    assert!(package.classification.needs_clarification);
    assert!(package.classification.surface_adapter_fallback);
    assert!(package.selected_plan.capability_probes.iter().any(|row| {
        row.capability == infring_orchestration_surface_v1::contracts::Capability::ExecuteTool
            && row
                .probe_sources
                .iter()
                .any(|source| source == "heuristic.tool_hints_or_resource_kind")
    }));
    assert!(!package.selected_plan.capability_probes.iter().any(|row| {
        row.capability == infring_orchestration_surface_v1::contracts::Capability::ExecuteTool
            && row
                .probe_sources
                .iter()
                .any(|source| source.starts_with("probe.required_for_typed_surface"))
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
        row.capability == infring_orchestration_surface_v1::contracts::Capability::ExecuteTool
            && row.probe_sources.iter().any(|source| {
                source == "probe.required_for_typed_surface.execute_tool.transport_available"
            })
    }));
}

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
                    "execute_tool": {
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
        row.capability == infring_orchestration_surface_v1::contracts::Capability::ExecuteTool
            && row.probe_sources.iter().any(|source| {
                source == "probe.required_for_typed_surface.execute_tool.tool_available"
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
                    "execute_tool": {
                        "transport_available": true
                    }
                },
                "capability_probes": {
                    "execute_tool": {
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
        row.capability == infring_orchestration_surface_v1::contracts::Capability::ExecuteTool
            && row.probe_sources.iter().any(|source| {
                source == "probe.required_for_typed_surface.execute_tool.tool_available"
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
                    "execute_tool": {
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
                    "execute_tool": {
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
                    "execute_tool": {
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
            .any(|row| row.step_id == "step_memory_fallback")));
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
                    "execute_tool": {
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
                    step_id: "step_tool_capability_probe".to_string(),
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
    assert!(package.execution_state.steps.iter().any(|row| {
        row.step_id == "step_tool_capability_probe"
            && row.status == infring_orchestration_surface_v1::contracts::StepStatus::Succeeded
    }));
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
                    "execute_tool": {
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

#[test]
fn non_legacy_surface_fixture_quality_stays_within_surface_thresholds() {
    #[derive(Default, Clone, Copy)]
    struct SurfaceStats {
        total: usize,
        fallback: usize,
        low_confidence: usize,
    }

    let fixtures = vec![
        OrchestrationRequest {
            session_id: "sdk-quality-1".to_string(),
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
                    "execute_tool": {
                        "tool_available": true,
                        "transport_available": true
                    }
                }
            }),
        },
        OrchestrationRequest {
            session_id: "gateway-quality-1".to_string(),
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
                    "execute_tool": {
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
            session_id: "dashboard-quality-1".to_string(),
            intent: "opaque".to_string(),
            surface: RequestSurface::Dashboard,
            payload: json!({
                "dashboard": {
                    "operation_kind": "read",
                    "resource_kind": "workspace",
                    "targets": [{ "kind": "workspace_path", "value": "README.md" }]
                }
            }),
        },
        OrchestrationRequest {
            session_id: "dashboard-quality-fallback".to_string(),
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
    let mut sdk = SurfaceStats::default();
    let mut gateway = SurfaceStats::default();
    let mut dashboard = SurfaceStats::default();

    for (idx, request) in fixtures.into_iter().enumerate() {
        let surface = request.surface;
        let package = runtime.orchestrate(request, 4_700 + idx as u64);
        let low_confidence = package
            .classification
            .reasons
            .iter()
            .any(|reason| reason == "parse_confidence_below_threshold");
        let stats = match surface {
            RequestSurface::Sdk => &mut sdk,
            RequestSurface::Gateway => &mut gateway,
            RequestSurface::Dashboard => &mut dashboard,
            RequestSurface::Legacy | RequestSurface::Cli => continue,
        };
        stats.total += 1;
        if package.classification.surface_adapter_fallback {
            stats.fallback += 1;
        }
        if low_confidence {
            stats.low_confidence += 1;
        }
    }

    let sdk_fallback_rate = sdk.fallback as f32 / sdk.total as f32;
    let sdk_low_confidence_rate = sdk.low_confidence as f32 / sdk.total as f32;
    let gateway_fallback_rate = gateway.fallback as f32 / gateway.total as f32;
    let gateway_low_confidence_rate = gateway.low_confidence as f32 / gateway.total as f32;
    let dashboard_fallback_rate = dashboard.fallback as f32 / dashboard.total as f32;
    let dashboard_low_confidence_rate = dashboard.low_confidence as f32 / dashboard.total as f32;

    assert!(sdk_fallback_rate <= 0.05, "sdk fallback rate regression");
    assert!(
        sdk_low_confidence_rate <= 0.05,
        "sdk low-confidence rate regression"
    );
    assert!(
        gateway_fallback_rate <= 0.05,
        "gateway fallback rate regression"
    );
    assert!(
        gateway_low_confidence_rate <= 0.05,
        "gateway low-confidence rate regression"
    );
    assert!(
        dashboard_fallback_rate <= 0.50,
        "dashboard fallback rate regression"
    );
    assert!(
        dashboard_low_confidence_rate <= 0.50,
        "dashboard low-confidence rate regression"
    );

    println!(
        "surface_quality_metrics={}",
        json!({
            "sdk": {
                "total": sdk.total,
                "fallback_rate": sdk_fallback_rate,
                "low_confidence_rate": sdk_low_confidence_rate
            },
            "gateway": {
                "total": gateway.total,
                "fallback_rate": gateway_fallback_rate,
                "low_confidence_rate": gateway_low_confidence_rate
            },
            "dashboard": {
                "total": dashboard.total,
                "fallback_rate": dashboard_fallback_rate,
                "low_confidence_rate": dashboard_low_confidence_rate
            }
        })
    );
}

#[test]
fn planner_quality_fixture_metrics_stay_within_thresholds() {
    let fixtures = vec![
        OrchestrationRequest {
            session_id: "planner-quality-sdk".to_string(),
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
                    "execute_tool": {
                        "tool_available": true,
                        "transport_available": true
                    }
                }
            }),
        },
        OrchestrationRequest {
            session_id: "planner-quality-compare".to_string(),
            intent: "compare workspace and web".to_string(),
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
                    "execute_tool": {
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
            session_id: "planner-quality-legacy".to_string(),
            intent: "search the web for release notes".to_string(),
            surface: RequestSurface::Legacy,
            payload: json!({}),
        },
        OrchestrationRequest {
            session_id: "planner-quality-ambiguous".to_string(),
            intent: "maybe do something".to_string(),
            surface: RequestSurface::Legacy,
            payload: json!({}),
        },
        OrchestrationRequest {
            session_id: "planner-quality-mutation".to_string(),
            intent: "implement requested mutation".to_string(),
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
    ];
    let mut runtime = OrchestrationSurfaceRuntime::new();
    let mut candidate_counts = Vec::new();
    let mut clarification_first_selected = 0usize;
    let mut degraded_selected = 0usize;
    let mut heuristic_probe_selected = 0usize;
    let mut zero_executable_selected = 0usize;
    let mut all_candidates_clarification_selected = 0usize;
    let mut all_candidates_degraded_selected = 0usize;

    for (idx, request) in fixtures.into_iter().enumerate() {
        let package = runtime.orchestrate(request, 4_760 + idx as u64);
        let candidate_count = 1 + package.alternative_plans.len();
        candidate_counts.push(candidate_count);
        if package.selected_plan.variant
            == infring_orchestration_surface_v1::contracts::PlanVariant::ClarificationFirst
        {
            clarification_first_selected += 1;
        }
        if package.execution_state.plan_status
            == infring_orchestration_surface_v1::contracts::PlanStatus::Degraded
            || package.selected_plan.variant
                == infring_orchestration_surface_v1::contracts::PlanVariant::DegradedFallback
        {
            degraded_selected += 1;
        }
        if package.selected_plan.capability_probes.iter().any(|probe| {
            probe
                .probe_sources
                .iter()
                .any(|source| source.starts_with("heuristic."))
        }) {
            heuristic_probe_selected += 1;
        }
        if package.runtime_quality.zero_executable_candidates {
            zero_executable_selected += 1;
        }
        if package.runtime_quality.all_candidates_require_clarification {
            all_candidates_clarification_selected += 1;
        }
        if package.runtime_quality.all_candidates_degraded {
            all_candidates_degraded_selected += 1;
        }
    }

    let total = candidate_counts.len() as f32;
    let average_candidate_count = candidate_counts.iter().sum::<usize>() as f32 / total.max(1.0);
    let clarification_first_rate = clarification_first_selected as f32 / total.max(1.0);
    let degraded_rate = degraded_selected as f32 / total.max(1.0);
    let heuristic_probe_rate = heuristic_probe_selected as f32 / total.max(1.0);
    let zero_executable_candidate_rate = zero_executable_selected as f32 / total.max(1.0);
    let all_candidates_require_clarification_rate =
        all_candidates_clarification_selected as f32 / total.max(1.0);
    let all_candidates_degraded_rate = all_candidates_degraded_selected as f32 / total.max(1.0);

    assert!(
        candidate_counts.iter().all(|count| *count >= 2),
        "planner candidate diversity regression"
    );
    assert!(
        clarification_first_rate <= 0.50,
        "clarification-first selection rate regression"
    );
    assert!(degraded_rate <= 0.60, "degraded selection rate regression");
    assert!(
        heuristic_probe_rate <= 0.60,
        "heuristic probe dependence regression"
    );
    assert!(
        zero_executable_candidate_rate <= 0.60,
        "zero-executable candidate rate regression"
    );
    assert!(
        all_candidates_require_clarification_rate <= 0.60,
        "all-candidates-clarification rate regression"
    );
    assert!(
        all_candidates_degraded_rate <= 0.60,
        "all-candidates-degraded rate regression"
    );

    println!(
        "planner_quality_metrics={}",
        json!({
            "request_count": candidate_counts.len(),
            "average_candidate_count": average_candidate_count,
            "clarification_first_rate": clarification_first_rate,
            "degraded_rate": degraded_rate,
            "heuristic_probe_rate": heuristic_probe_rate,
            "zero_executable_candidate_rate": zero_executable_candidate_rate,
            "all_candidates_require_clarification_rate": all_candidates_require_clarification_rate,
            "all_candidates_degraded_rate": all_candidates_degraded_rate
        })
    );
}

#[test]
fn runtime_quality_telemetry_metrics_stay_within_thresholds() {
    let fixtures = vec![
        OrchestrationRequest {
            session_id: "runtime-quality-sdk".to_string(),
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
                    "execute_tool": {
                        "tool_available": true,
                        "transport_available": true
                    }
                }
            }),
        },
        OrchestrationRequest {
            session_id: "runtime-quality-gateway".to_string(),
            intent: "compare workspace and web".to_string(),
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
                    "execute_tool": {
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
            session_id: "runtime-quality-dashboard-fallback".to_string(),
            intent: "".to_string(),
            surface: RequestSurface::Dashboard,
            payload: json!({
                "dashboard": {
                    "selection_mode": "panel"
                }
            }),
        },
        OrchestrationRequest {
            session_id: "runtime-quality-legacy".to_string(),
            intent: "search web for release notes".to_string(),
            surface: RequestSurface::Legacy,
            payload: json!({}),
        },
    ];
    let fixture_count = fixtures.len().max(1);

    let mut runtime = OrchestrationSurfaceRuntime::new();
    let mut non_legacy_total = 0usize;
    let mut non_legacy_fallback = 0usize;
    let mut non_legacy_heuristic = 0usize;
    let mut non_legacy_clarification = 0usize;
    let mut non_legacy_zero_executable = 0usize;
    let mut non_legacy_all_candidates_degraded = 0usize;
    let mut candidate_total = 0usize;

    for (idx, request) in fixtures.into_iter().enumerate() {
        let package = runtime.orchestrate(request, 4_900 + idx as u64);
        candidate_total += package.runtime_quality.candidate_count as usize;
        if package.classification.request_class == RequestClass::ReadOnly {
            assert!(
                package.runtime_quality.candidate_count >= 1,
                "runtime telemetry candidate_count should always be populated"
            );
        }
        if !matches!(package.classification.request_class, RequestClass::ReadOnly)
            && package.runtime_quality.used_heuristic_probe
        {
            assert!(
                !package.selected_plan.capability_probes.is_empty(),
                "runtime heuristic probe signal must correspond to probe rows"
            );
        }
        if !matches!(package.classification.request_class, RequestClass::ReadOnly)
            && package.runtime_quality.selected_plan_requires_clarification
        {
            assert!(
                package.classification.needs_clarification
                    || package.selected_plan.requires_clarification,
                "runtime clarification signal must match selected plan/classification"
            );
        }
        if !matches!(package.classification.request_class, RequestClass::ReadOnly)
            && package.runtime_quality.selected_plan_degraded
        {
            assert!(
                !package.selected_plan.degradation.is_empty()
                    || package.execution_state.plan_status
                        == infring_orchestration_surface_v1::contracts::PlanStatus::Degraded,
                "runtime degraded signal must match plan degradation state"
            );
        }

        if package.classification.surface_adapter_used
            || package.classification.surface_adapter_fallback
        {
            non_legacy_total += 1;
            if package.runtime_quality.surface_adapter_fallback {
                non_legacy_fallback += 1;
            }
            if package.runtime_quality.used_heuristic_probe {
                non_legacy_heuristic += 1;
            }
            if package.runtime_quality.selected_plan_requires_clarification {
                non_legacy_clarification += 1;
            }
            if package.runtime_quality.zero_executable_candidates {
                non_legacy_zero_executable += 1;
            }
            if package.runtime_quality.all_candidates_degraded {
                non_legacy_all_candidates_degraded += 1;
            }
            if package.runtime_quality.zero_executable_candidates {
                assert_eq!(
                    package.runtime_quality.executable_candidate_count, 0,
                    "zero executable flag must align with executable count"
                );
            }
            if package.runtime_quality.all_candidates_degraded {
                assert_eq!(
                    package.runtime_quality.degraded_candidate_count,
                    package.runtime_quality.candidate_count,
                    "all-candidates-degraded flag must align with counts"
                );
            }
        }
    }

    let total = non_legacy_total.max(1) as f32;
    let fallback_rate = non_legacy_fallback as f32 / total;
    let heuristic_probe_rate = non_legacy_heuristic as f32 / total;
    let clarification_rate = non_legacy_clarification as f32 / total;
    let zero_executable_rate = non_legacy_zero_executable as f32 / total;
    let all_candidates_degraded_rate = non_legacy_all_candidates_degraded as f32 / total;
    let average_candidate_count = candidate_total as f32 / fixture_count as f32;

    assert!(fallback_rate <= 0.50, "runtime fallback rate regression");
    assert!(
        heuristic_probe_rate <= 0.75,
        "runtime heuristic probe rate regression"
    );
    assert!(
        clarification_rate <= 0.60,
        "runtime clarification rate regression"
    );
    assert!(
        zero_executable_rate <= 0.60,
        "runtime zero-executable rate regression"
    );
    assert!(
        all_candidates_degraded_rate <= 0.60,
        "runtime all-candidates-degraded rate regression"
    );
    assert!(
        average_candidate_count >= 1.5,
        "runtime candidate count regression"
    );

    println!(
        "runtime_quality_metrics={}",
        json!({
            "sample_size_non_legacy": non_legacy_total,
            "fallback_rate_non_legacy": fallback_rate,
            "heuristic_probe_rate_non_legacy": heuristic_probe_rate,
            "clarification_rate_non_legacy": clarification_rate,
            "zero_executable_rate_non_legacy": zero_executable_rate,
            "all_candidates_degraded_rate_non_legacy": all_candidates_degraded_rate,
            "average_candidate_count": average_candidate_count
        })
    );
}

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
                    "execute_tool": {
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
                    "execute_tool": {
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
fn adapted_probe_authority_matrix_executes_50_real_cases() {
    #[derive(Clone)]
    struct MatrixCase {
        capability: Capability,
        missing_field: &'static str,
        expected_precondition: Precondition,
    }

    fn capability_key(capability: Capability) -> &'static str {
        match capability {
            Capability::ReadMemory => "read_memory",
            Capability::MutateTask => "mutate_task",
            Capability::ExecuteTool => "execute_tool",
            Capability::PlanAssimilation => "plan_assimilation",
            Capability::VerifyClaim => "verify_claim",
        }
    }

    fn probe_snapshot_with_missing_field(
        capability: Capability,
        missing_field: &str,
    ) -> CapabilityProbeSnapshot {
        let mut snapshot = CapabilityProbeSnapshot {
            capability,
            tool_available: Some(true),
            target_supplied: Some(true),
            target_syntactically_valid: Some(true),
            target_exists: Some(true),
            authorization_valid: Some(true),
            policy_allows: Some(true),
            transport_available: Some(true),
        };
        match missing_field {
            "tool_available" => snapshot.tool_available = None,
            "target_supplied" => snapshot.target_supplied = None,
            "target_syntactically_valid" => snapshot.target_syntactically_valid = None,
            "target_exists" => snapshot.target_exists = None,
            "authorization_valid" => snapshot.authorization_valid = None,
            "policy_allows" => snapshot.policy_allows = None,
            "transport_available" => snapshot.transport_available = None,
            _ => {}
        }
        snapshot
    }

    fn adapted_typed_request(
        surface: RequestSurface,
        capability: Capability,
        missing_field: &str,
    ) -> TypedOrchestrationRequest {
        let (
            request_kind,
            operation_kind,
            resource_kind,
            mutability,
            policy_scope,
            targets,
            refs,
            tool_hints,
        ) = match capability {
            Capability::ExecuteTool => (
                RequestKind::Direct,
                OperationKind::Search,
                ResourceKind::Web,
                Mutability::ReadOnly,
                PolicyScope::WebOnly,
                vec![TargetDescriptor::Url {
                    value: "https://example.com/releases".to_string(),
                }],
                vec!["https://example.com/releases".to_string()],
                vec!["web_search".to_string()],
            ),
            Capability::VerifyClaim => (
                RequestKind::Comparative,
                OperationKind::Compare,
                ResourceKind::Mixed,
                Mutability::ReadOnly,
                PolicyScope::Default,
                vec![
                    TargetDescriptor::WorkspacePath {
                        value: "README.md".to_string(),
                    },
                    TargetDescriptor::Url {
                        value: "https://example.com/reference".to_string(),
                    },
                ],
                vec![
                    "README.md".to_string(),
                    "https://example.com/reference".to_string(),
                ],
                vec![],
            ),
            Capability::MutateTask => (
                RequestKind::Direct,
                OperationKind::Mutate,
                ResourceKind::TaskGraph,
                Mutability::Mutation,
                PolicyScope::CoreProposal,
                vec![TargetDescriptor::TaskId {
                    value: "task-42".to_string(),
                }],
                vec!["task-42".to_string()],
                vec![],
            ),
            Capability::PlanAssimilation => (
                RequestKind::Workflow,
                OperationKind::Assimilate,
                ResourceKind::Workspace,
                Mutability::Mutation,
                PolicyScope::CoreProposal,
                vec![TargetDescriptor::WorkspacePath {
                    value: "README.md".to_string(),
                }],
                vec!["README.md".to_string()],
                vec![],
            ),
            Capability::ReadMemory => (
                RequestKind::Direct,
                OperationKind::Read,
                ResourceKind::Memory,
                Mutability::ReadOnly,
                PolicyScope::Default,
                vec![TargetDescriptor::MemoryRef {
                    scope: "session".to_string(),
                    object_id: None,
                }],
                vec!["memory:session".to_string()],
                vec![],
            ),
        };

        TypedOrchestrationRequest {
            session_id: format!("matrix-{surface:?}-{capability:?}-{missing_field}")
                .to_lowercase()
                .replace(':', "_"),
            surface,
            legacy_intent: "synthetic".to_string(),
            adapted: true,
            payload: json!({}),
            request_kind,
            operation_kind,
            resource_kind,
            mutability,
            target_descriptors: targets,
            target_refs: refs,
            tool_hints,
            policy_scope,
            user_constraints: Vec::new(),
            core_probe_envelope: Some(CoreProbeEnvelope {
                probes: vec![probe_snapshot_with_missing_field(capability, missing_field)],
            }),
        }
    }

    fn legacy_tool_request_without_probe_envelope() -> TypedOrchestrationRequest {
        TypedOrchestrationRequest {
            session_id: "legacy-tool-no-envelope".to_string(),
            surface: RequestSurface::Legacy,
            legacy_intent: "search web".to_string(),
            adapted: false,
            payload: json!({}),
            request_kind: RequestKind::Direct,
            operation_kind: OperationKind::Search,
            resource_kind: ResourceKind::Web,
            mutability: Mutability::ReadOnly,
            target_descriptors: vec![TargetDescriptor::Url {
                value: "https://example.com/releases".to_string(),
            }],
            target_refs: vec!["https://example.com/releases".to_string()],
            tool_hints: Vec::new(),
            policy_scope: PolicyScope::WebOnly,
            user_constraints: Vec::new(),
            core_probe_envelope: None,
        }
    }

    fn legacy_assimilation_request_without_probe_envelope() -> TypedOrchestrationRequest {
        TypedOrchestrationRequest {
            session_id: "legacy-assimilation-no-envelope".to_string(),
            surface: RequestSurface::Legacy,
            legacy_intent: "assimilate workspace".to_string(),
            adapted: false,
            payload: json!({}),
            request_kind: RequestKind::Workflow,
            operation_kind: OperationKind::Assimilate,
            resource_kind: ResourceKind::Workspace,
            mutability: Mutability::Mutation,
            target_descriptors: vec![TargetDescriptor::WorkspacePath {
                value: "README.md".to_string(),
            }],
            target_refs: vec!["README.md".to_string()],
            tool_hints: Vec::new(),
            policy_scope: PolicyScope::CrossBoundary,
            user_constraints: Vec::new(),
            core_probe_envelope: None,
        }
    }

    let matrix = vec![
        MatrixCase {
            capability: Capability::ExecuteTool,
            missing_field: "tool_available",
            expected_precondition: Precondition::ToolAvailable,
        },
        MatrixCase {
            capability: Capability::ExecuteTool,
            missing_field: "transport_available",
            expected_precondition: Precondition::TransportAvailable,
        },
        MatrixCase {
            capability: Capability::VerifyClaim,
            missing_field: "transport_available",
            expected_precondition: Precondition::TransportAvailable,
        },
        MatrixCase {
            capability: Capability::MutateTask,
            missing_field: "target_supplied",
            expected_precondition: Precondition::TargetSupplied,
        },
        MatrixCase {
            capability: Capability::MutateTask,
            missing_field: "target_syntactically_valid",
            expected_precondition: Precondition::TargetSyntacticallyValid,
        },
        MatrixCase {
            capability: Capability::MutateTask,
            missing_field: "target_exists",
            expected_precondition: Precondition::TargetExists,
        },
        MatrixCase {
            capability: Capability::MutateTask,
            missing_field: "authorization_valid",
            expected_precondition: Precondition::AuthorizationValid,
        },
        MatrixCase {
            capability: Capability::MutateTask,
            missing_field: "policy_allows",
            expected_precondition: Precondition::PolicyAllows,
        },
        MatrixCase {
            capability: Capability::PlanAssimilation,
            missing_field: "target_supplied",
            expected_precondition: Precondition::TargetSupplied,
        },
        MatrixCase {
            capability: Capability::PlanAssimilation,
            missing_field: "target_syntactically_valid",
            expected_precondition: Precondition::TargetSyntacticallyValid,
        },
        MatrixCase {
            capability: Capability::PlanAssimilation,
            missing_field: "target_exists",
            expected_precondition: Precondition::TargetExists,
        },
        MatrixCase {
            capability: Capability::PlanAssimilation,
            missing_field: "policy_allows",
            expected_precondition: Precondition::PolicyAllows,
        },
    ];
    let strict_surfaces = [
        RequestSurface::Sdk,
        RequestSurface::Gateway,
        RequestSurface::Dashboard,
        RequestSurface::Cli,
    ];

    let mut executed_cases = 0usize;
    for surface in strict_surfaces {
        for case in &matrix {
            let request =
                adapted_typed_request(surface, case.capability.clone(), case.missing_field);
            let probe = infring_orchestration_surface_v1::planner::preconditions::probe_capability(
                &request,
                &case.capability,
            );
            let expected_source = format!(
                "probe.required_for_typed_surface.{}.{}",
                capability_key(case.capability.clone()),
                case.missing_field
            );
            assert!(
                probe.blocked_on.contains(&case.expected_precondition),
                "missing precondition for surface={surface:?} capability={:?} field={}",
                case.capability,
                case.missing_field
            );
            assert!(
                probe
                    .probe_sources
                    .iter()
                    .any(|source| source == &expected_source),
                "missing strict probe source for surface={surface:?} capability={:?} field={}",
                case.capability,
                case.missing_field
            );
            assert!(
                !probe
                    .probe_sources
                    .iter()
                    .any(|source| source.starts_with("heuristic.")),
                "strict adapted surfaces must not consume heuristic probes"
            );
            executed_cases += 1;
        }
    }

    let legacy_tool_probe =
        infring_orchestration_surface_v1::planner::preconditions::probe_capability(
            &legacy_tool_request_without_probe_envelope(),
            &Capability::ExecuteTool,
        );
    assert!(!legacy_tool_probe
        .blocked_on
        .contains(&Precondition::ToolAvailable));
    assert!(legacy_tool_probe
        .probe_sources
        .iter()
        .any(|source| source.starts_with("heuristic.")));
    assert!(!legacy_tool_probe
        .probe_sources
        .iter()
        .any(|source| { source.starts_with("probe.required_for_typed_surface.execute_tool.") }));
    executed_cases += 1;

    let legacy_assimilation_probe =
        infring_orchestration_surface_v1::planner::preconditions::probe_capability(
            &legacy_assimilation_request_without_probe_envelope(),
            &Capability::PlanAssimilation,
        );
    assert!(legacy_assimilation_probe
        .blocked_on
        .contains(&Precondition::PolicyAllows));
    assert!(legacy_assimilation_probe
        .probe_sources
        .iter()
        .any(|source| source == "heuristic.policy_scope_and_mutability"));
    assert!(!legacy_assimilation_probe
        .probe_sources
        .iter()
        .any(|source| {
            source.starts_with("probe.required_for_typed_surface.plan_assimilation.")
        }));
    executed_cases += 1;

    assert_eq!(
        executed_cases, 50,
        "probe authority matrix must execute 50 cases"
    );
}
