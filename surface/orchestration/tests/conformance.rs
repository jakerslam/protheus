// Layer ownership: tests (regression proof for orchestration surface contracts).
use infring_orchestration_surface_v1::contracts::{
    ClarificationReason, CoreContractCall, OrchestrationRequest, RequestClass, RequestSurface,
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

    assert!(ready
        .core_contract_calls
        .contains(&CoreContractCall::ToolBrokerRequest));
    assert_eq!(
        degraded.core_contract_calls,
        vec![CoreContractCall::UnifiedMemoryRead]
    );
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
        .contains(&CoreContractCall::UnifiedMemoryRead));
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
                source == "probe.required_for_adapted_surface.execute_tool.transport_available"
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
                source == "probe.required_for_adapted_surface.plan_assimilation.policy_allows"
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
        .find(|row| row.target_contract == CoreContractCall::UnifiedMemoryRead)
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
    let package = runtime.orchestrate(
        OrchestrationRequest {
            session_id: "core-running".to_string(),
            intent: "compare this workspace state to the web".to_string(),
            surface: RequestSurface::Legacy,
            payload: json!({
                "path": "README.md",
                "url": "https://example.com",
                "core_execution_observation": {
                    "status": "completed",
                    "receipt_ids": ["receipt-1", "receipt-2"],
                    "outcome_refs": ["outcome-1"],
                    "step_statuses": {
                        "step_tool_capability_probe": "succeeded",
                        "step_tool_broker_request": "failed"
                    }
                }
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
        row.step_id == "step_tool_broker_request"
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
