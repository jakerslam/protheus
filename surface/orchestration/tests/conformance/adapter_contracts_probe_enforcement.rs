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
        infring_orchestration_surface_v1::contracts::PlanStatus::ClarificationRequired
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
        .contains(&infring_orchestration_surface_v1::contracts::Precondition::ToolAvailable));
    assert!(package.selected_plan.capability_probes.iter().any(|row| {
        row.capability.is_tool_family()
            && row.probe_sources.iter().any(|source| {
                source == "probe.required_for_typed_surface.web_search.tool_available"
            })
    }));
    assert!(package
        .classification
        .reasons
        .iter()
        .any(|row| row == "missing_probe: web_search.tool_available"));
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
    assert!(package
        .classification
        .reasons
        .iter()
        .any(|row| row == "missing_probe: web_search.tool_available"));
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
fn adapted_workspace_request_requires_workspace_transport_probe_field() {
    let mut runtime = OrchestrationSurfaceRuntime::new();
    let package = runtime.orchestrate(
        OrchestrationRequest {
            session_id: "sdk-workspace-missing-transport-probe".to_string(),
            intent: "opaque".to_string(),
            surface: RequestSurface::Sdk,
            payload: json!({
                "sdk": {
                    "operation_kind": "search",
                    "resource_kind": "workspace",
                    "request_kind": "direct",
                    "targets": [{ "kind": "workspace_path", "value": "README.md" }]
                },
                "core_probe_envelope": {
                    "workspace_search": {
                        "tool_available": true
                    }
                }
            }),
        },
        43_292_1,
    );

    assert!(package
        .selected_plan
        .blocked_on
        .contains(&infring_orchestration_surface_v1::contracts::Precondition::TransportAvailable));
    assert!(package.selected_plan.capability_probes.iter().any(|row| {
        matches!(
            row.capability,
            infring_orchestration_surface_v1::contracts::Capability::WorkspaceRead
                | infring_orchestration_surface_v1::contracts::Capability::WorkspaceSearch
        ) && row.probe_sources.iter().any(|source| {
            source == "probe.required_for_typed_surface.workspace_search.transport_available"
                || source == "probe.required_for_typed_surface.workspace_read.transport_available"
        })
    }));
}

#[test]
fn adapted_tool_route_request_requires_transport_probe_field() {
    let mut runtime = OrchestrationSurfaceRuntime::new();
    let package = runtime.orchestrate(
        OrchestrationRequest {
            session_id: "sdk-tool-route-missing-transport-probe".to_string(),
            intent: "opaque".to_string(),
            surface: RequestSurface::Sdk,
            payload: json!({
                "sdk": {
                    "operation_kind": "inspect_tooling",
                    "resource_kind": "tooling",
                    "request_kind": "direct",
                    "tool_hints": ["tool_route"],
                    "targets": [{ "kind": "tool_name", "value": "shell.exec" }]
                },
                "core_probe_envelope": {
                    "tool_route": {
                        "tool_available": true
                    }
                }
            }),
        },
        43_292_2,
    );

    assert!(package
        .selected_plan
        .blocked_on
        .contains(&infring_orchestration_surface_v1::contracts::Precondition::TransportAvailable));
    assert!(package.selected_plan.capability_probes.iter().any(|row| {
        row.capability == infring_orchestration_surface_v1::contracts::Capability::ToolRoute
            && row.probe_sources.iter().any(|source| {
                source == "probe.required_for_typed_surface.tool_route.transport_available"
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
fn non_legacy_tool_family_missing_capability_denials_are_exact() {
    let mut runtime = OrchestrationSurfaceRuntime::new();
    let cases = vec![
        (
            "workspace_read",
            json!({
                "sdk": {
                    "operation_kind": "read",
                    "resource_kind": "workspace",
                    "request_kind": "direct",
                    "targets": [{ "kind": "workspace_path", "value": "README.md" }]
                },
                "core_probe_envelope": {
                    "verify_claim": {
                        "transport_available": true
                    }
                }
            }),
        ),
        (
            "workspace_search",
            json!({
                "sdk": {
                    "operation_kind": "search",
                    "resource_kind": "workspace",
                    "request_kind": "direct",
                    "targets": [{ "kind": "workspace_path", "value": "README.md" }]
                },
                "core_probe_envelope": {
                    "verify_claim": {
                        "transport_available": true
                    }
                }
            }),
        ),
        (
            "web_search",
            json!({
                "sdk": {
                    "operation_kind": "search",
                    "resource_kind": "web",
                    "request_kind": "direct",
                    "targets": [{ "kind": "url", "value": "https://example.com/releases" }]
                },
                "core_probe_envelope": {
                    "verify_claim": {
                        "transport_available": true
                    }
                }
            }),
        ),
        (
            "web_fetch",
            json!({
                "sdk": {
                    "operation_kind": "fetch",
                    "resource_kind": "web",
                    "request_kind": "direct",
                    "targets": [{ "kind": "url", "value": "https://example.com/releases" }]
                },
                "core_probe_envelope": {
                    "verify_claim": {
                        "transport_available": true
                    }
                }
            }),
        ),
        (
            "tool_route",
            json!({
                "sdk": {
                    "operation_kind": "inspect_tooling",
                    "resource_kind": "tooling",
                    "request_kind": "direct",
                    "tool_hints": ["tool_route"],
                    "targets": [{ "kind": "tool_name", "value": "shell.exec" }]
                },
                "core_probe_envelope": {
                    "verify_claim": {
                        "transport_available": true
                    }
                }
            }),
        ),
    ];

    for (offset, (capability_key, payload)) in cases.into_iter().enumerate() {
        let package = runtime.orchestrate(
            OrchestrationRequest {
                session_id: format!("sdk-missing-capability-{capability_key}"),
                intent: "typed fixture".to_string(),
                surface: RequestSurface::Sdk,
                payload,
            },
            60_000 + offset as u64,
        );
        let expected_reason = format!("typed_probe_contract_missing:capability.{capability_key}");
        assert!(package.classification.surface_adapter_used);
        assert!(
            package
                .classification
                .reasons
                .iter()
                .any(|row| row == &expected_reason),
            "missing expected exact denial reason for {capability_key}"
        );
        let expected_missing_probe = format!("missing_probe: {capability_key}");
        assert!(
            package
                .classification
                .reasons
                .iter()
                .any(|row| row == &expected_missing_probe),
            "missing precise missing-probe code for {capability_key}"
        );
        assert!(!package.classification.reasons.iter().any(|row| {
            row == "typed_probe_contract_missing:capability.execute_tool"
                || row == "tool_unavailable"
        }));
        assert!(package
            .classification
            .clarification_reasons
            .contains(&ClarificationReason::TypedProbeContractViolation));
    }
}

#[test]
fn non_legacy_tool_family_partial_probe_fields_emit_exact_field_denials() {
    let mut runtime = OrchestrationSurfaceRuntime::new();
    let cases = vec![
        (
            "workspace_read",
            "tool_available",
            json!({ "transport_available": true }),
        ),
        (
            "workspace_read",
            "transport_available",
            json!({ "tool_available": true }),
        ),
        (
            "workspace_search",
            "tool_available",
            json!({ "transport_available": true }),
        ),
        (
            "workspace_search",
            "transport_available",
            json!({ "tool_available": true }),
        ),
        (
            "web_search",
            "tool_available",
            json!({ "transport_available": true }),
        ),
        (
            "web_search",
            "transport_available",
            json!({ "tool_available": true }),
        ),
        (
            "web_fetch",
            "tool_available",
            json!({ "transport_available": true }),
        ),
        (
            "web_fetch",
            "transport_available",
            json!({ "tool_available": true }),
        ),
        (
            "tool_route",
            "tool_available",
            json!({ "transport_available": true }),
        ),
        (
            "tool_route",
            "transport_available",
            json!({ "tool_available": true }),
        ),
    ];

    for (offset, (capability_key, missing_field, probe_row)) in cases.into_iter().enumerate() {
        let sdk_payload = match capability_key {
            "workspace_read" => json!({
                "sdk": {
                    "operation_kind": "read",
                    "resource_kind": "workspace",
                    "request_kind": "direct",
                    "targets": [{ "kind": "workspace_path", "value": "README.md" }]
                }
            }),
            "workspace_search" => json!({
                "sdk": {
                    "operation_kind": "search",
                    "resource_kind": "workspace",
                    "request_kind": "direct",
                    "targets": [{ "kind": "workspace_path", "value": "README.md" }]
                }
            }),
            "web_search" => json!({
                "sdk": {
                    "operation_kind": "search",
                    "resource_kind": "web",
                    "request_kind": "direct",
                    "targets": [{ "kind": "url", "value": "https://example.com/releases" }]
                }
            }),
            "web_fetch" => json!({
                "sdk": {
                    "operation_kind": "fetch",
                    "resource_kind": "web",
                    "request_kind": "direct",
                    "targets": [{ "kind": "url", "value": "https://example.com/releases" }]
                }
            }),
            "tool_route" => json!({
                "sdk": {
                    "operation_kind": "inspect_tooling",
                    "resource_kind": "tooling",
                    "request_kind": "direct",
                    "tool_hints": ["tool_route"],
                    "targets": [{ "kind": "tool_name", "value": "shell.exec" }]
                }
            }),
            other => panic!("unexpected capability key {other}"),
        };
        let mut core_probe_envelope = serde_json::Map::new();
        core_probe_envelope.insert(capability_key.to_string(), probe_row);
        let mut payload_object = serde_json::Map::new();
        payload_object.insert("sdk".to_string(), sdk_payload["sdk"].clone());
        payload_object.insert(
            "core_probe_envelope".to_string(),
            serde_json::Value::Object(core_probe_envelope),
        );
        let payload = serde_json::Value::Object(payload_object);
        let package = runtime.orchestrate(
            OrchestrationRequest {
                session_id: format!("sdk-missing-field-{capability_key}-{missing_field}"),
                intent: "typed fixture".to_string(),
                surface: RequestSurface::Sdk,
                payload,
            },
            70_000 + offset as u64,
        );
        let expected_field_reason =
            format!("typed_probe_contract_missing:field.{capability_key}.{missing_field}");
        let unexpected_capability_reason =
            format!("typed_probe_contract_missing:capability.{capability_key}");
        assert!(package.classification.surface_adapter_used);
        assert!(
            package
                .classification
                .reasons
                .iter()
                .any(|row| row == &expected_field_reason),
            "missing exact field denial reason for {capability_key}.{missing_field}"
        );
        let expected_missing_probe = format!("missing_probe: {capability_key}.{missing_field}");
        assert!(
            package
                .classification
                .reasons
                .iter()
                .any(|row| row == &expected_missing_probe),
            "missing precise missing-probe code for {capability_key}.{missing_field}"
        );
        assert!(!package
            .classification
            .reasons
            .iter()
            .any(|row| row == &unexpected_capability_reason));
        assert!(package
            .classification
            .clarification_reasons
            .contains(&ClarificationReason::TypedProbeContractViolation));
    }
}

#[test]
fn non_legacy_tool_family_stale_payload_probe_shortcuts_emit_missing_probe_codes() {
    let mut runtime = OrchestrationSurfaceRuntime::new();
    let cases = vec![
        (
            "workspace_read",
            json!({
                "sdk": {
                    "operation_kind": "read",
                    "resource_kind": "workspace",
                    "request_kind": "direct",
                    "targets": [{ "kind": "workspace_path", "value": "README.md" }]
                }
            }),
        ),
        (
            "workspace_search",
            json!({
                "sdk": {
                    "operation_kind": "search",
                    "resource_kind": "workspace",
                    "request_kind": "direct",
                    "targets": [{ "kind": "workspace_path", "value": "README.md" }]
                }
            }),
        ),
        (
            "web_search",
            json!({
                "sdk": {
                    "operation_kind": "search",
                    "resource_kind": "web",
                    "request_kind": "direct",
                    "targets": [{ "kind": "url", "value": "https://example.com/releases" }]
                }
            }),
        ),
        (
            "web_fetch",
            json!({
                "sdk": {
                    "operation_kind": "fetch",
                    "resource_kind": "web",
                    "request_kind": "direct",
                    "targets": [{ "kind": "url", "value": "https://example.com/releases" }]
                }
            }),
        ),
        (
            "tool_route",
            json!({
                "sdk": {
                    "operation_kind": "inspect_tooling",
                    "resource_kind": "tooling",
                    "request_kind": "direct",
                    "tool_hints": ["tool_route"],
                    "targets": [{ "kind": "tool_name", "value": "shell.exec" }]
                }
            }),
        ),
    ];

    for (offset, (capability_key, sdk_payload)) in cases.into_iter().enumerate() {
        let mut payload_object = serde_json::Map::new();
        payload_object.insert("sdk".to_string(), sdk_payload["sdk"].clone());
        payload_object.insert(
            "capability_probes".to_string(),
            json!({
                capability_key: {
                    "tool_available": true,
                    "transport_available": true
                }
            }),
        );
        let package = runtime.orchestrate(
            OrchestrationRequest {
                session_id: format!("sdk-stale-payload-shortcut-{capability_key}"),
                intent: "typed fixture".to_string(),
                surface: RequestSurface::Sdk,
                payload: serde_json::Value::Object(payload_object),
            },
            75_000 + offset as u64,
        );
        assert!(package
            .classification
            .reasons
            .iter()
            .any(|row| row == "typed_probe_contract_missing:core_probe_envelope"));
        let expected_missing_probe = format!("missing_probe: {capability_key}");
        assert!(
            package
                .classification
                .reasons
                .iter()
                .any(|row| row == &expected_missing_probe),
            "missing stale-probe denial code for {capability_key}"
        );
        assert!(package
            .classification
            .clarification_reasons
            .contains(&ClarificationReason::TypedProbeContractViolation));
    }
}

#[test]
fn non_legacy_typed_surface_rejects_execute_tool_alias_in_core_probe_envelope() {
    let mut runtime = OrchestrationSurfaceRuntime::new();
    let package = runtime.orchestrate(
        OrchestrationRequest {
            session_id: "sdk-execute-tool-alias-rejected".to_string(),
            intent: "typed fixture".to_string(),
            surface: RequestSurface::Sdk,
            payload: json!({
                "sdk": {
                    "operation_kind": "inspect_tooling",
                    "resource_kind": "tooling",
                    "request_kind": "direct",
                    "tool_hints": ["tool_route"],
                    "targets": [{ "kind": "tool_name", "value": "shell.exec" }]
                },
                "core_probe_envelope": {
                    "execute_tool": {
                        "tool_available": true,
                        "transport_available": true
                    }
                }
            }),
        },
        80_000,
    );
    assert!(package.classification.surface_adapter_used);
    assert!(package
        .classification
        .reasons
        .iter()
        .any(|row| row == "typed_probe_contract_missing:core_probe_envelope"));
    assert!(package
        .classification
        .reasons
        .iter()
        .any(|row| row == "missing_probe: tool_route"));
    assert!(!package
        .classification
        .reasons
        .iter()
        .any(|row| row == "typed_probe_contract_missing:capability.execute_tool"));
}
