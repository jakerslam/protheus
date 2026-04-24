use super::*;
use infring_orchestration_surface_v1::planner::preconditions::deterministic_routing_decision_trace;

fn dashboard_tool_request_for_trace(
    resource_kind: ResourceKind,
    operation_kind: OperationKind,
    target_descriptors: Vec<TargetDescriptor>,
    target_refs: Vec<String>,
    probe: Option<CapabilityProbeSnapshot>,
) -> TypedOrchestrationRequest {
    TypedOrchestrationRequest {
        session_id: "dashboard-tool-routing-trace".to_string(),
        surface: RequestSurface::Dashboard,
        legacy_intent: "trace tool route".to_string(),
        adapted: false,
        payload: serde_json::json!({}),
        request_kind: RequestKind::Direct,
        operation_kind,
        resource_kind,
        mutability: Mutability::ReadOnly,
        target_descriptors,
        target_refs,
        tool_hints: Vec::new(),
        policy_scope: PolicyScope::Default,
        user_constraints: Vec::new(),
        core_probe_envelope: probe.map(|row| CoreProbeEnvelope { probes: vec![row] }),
    }
}

fn full_positive_probe(capability: Capability) -> CapabilityProbeSnapshot {
    CapabilityProbeSnapshot {
        capability,
        tool_available: Some(true),
        target_supplied: Some(true),
        target_syntactically_valid: Some(true),
        target_exists: Some(true),
        authorization_valid: Some(true),
        policy_allows: Some(true),
        transport_available: Some(true),
    }
}

#[test]
fn non_legacy_tool_routing_requires_authoritative_probe_even_when_unadapted() {
    let request = dashboard_tool_request_for_trace(
        ResourceKind::Web,
        OperationKind::Search,
        vec![TargetDescriptor::Url {
            value: "https://example.com".to_string(),
        }],
        vec!["https://example.com".to_string()],
        None,
    );

    let trace = deterministic_routing_decision_trace(&request);

    assert_eq!(trace["selected"], "web_search");
    assert_eq!(trace["reason"], "missing_probe: web_search.tool_available");
    assert_eq!(trace["confidence"], 0.0);
}

#[test]
fn routing_decision_trace_records_selected_rejected_reason_and_confidence() {
    let request = dashboard_tool_request_for_trace(
        ResourceKind::Workspace,
        OperationKind::Search,
        vec![TargetDescriptor::WorkspacePath {
            value: "README.md".to_string(),
        }],
        vec!["README.md".to_string()],
        Some(full_positive_probe(Capability::WorkspaceSearch)),
    );

    let trace = deterministic_routing_decision_trace(&request);

    assert_eq!(trace["selected"], "workspace_search");
    assert_eq!(
        trace["reason"],
        "probe.core_probe_envelope.workspace_search.tool_available"
    );
    assert_eq!(trace["confidence"], 1.0);
    assert!(trace["rejected"]
        .as_array()
        .expect("rejected alternatives must be an array")
        .iter()
        .any(|row| row == "web_search"));
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
        capability
            .probe_keys()
            .first()
            .copied()
            .unwrap_or_else(|| panic!("capability must expose at least one probe key"))
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
            Capability::WorkspaceRead | Capability::WorkspaceSearch => (
                RequestKind::Direct,
                OperationKind::Search,
                ResourceKind::Workspace,
                Mutability::ReadOnly,
                PolicyScope::WorkspaceOnly,
                vec![TargetDescriptor::WorkspacePath {
                    value: "README.md".to_string(),
                }],
                vec!["README.md".to_string()],
                vec!["workspace_search".to_string()],
            ),
            Capability::WebSearch | Capability::WebFetch => (
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
            Capability::ToolRoute | Capability::ExecuteTool => (
                RequestKind::Direct,
                OperationKind::InspectTooling,
                ResourceKind::Tooling,
                Mutability::ReadOnly,
                PolicyScope::Default,
                vec![TargetDescriptor::ToolName {
                    value: "shell.exec".to_string(),
                }],
                vec!["shell.exec".to_string()],
                vec!["tool_route".to_string()],
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
            capability: Capability::WorkspaceRead,
            missing_field: "tool_available",
            expected_precondition: Precondition::ToolAvailable,
        },
        MatrixCase {
            capability: Capability::WorkspaceRead,
            missing_field: "transport_available",
            expected_precondition: Precondition::TransportAvailable,
        },
        MatrixCase {
            capability: Capability::WorkspaceSearch,
            missing_field: "tool_available",
            expected_precondition: Precondition::ToolAvailable,
        },
        MatrixCase {
            capability: Capability::WorkspaceSearch,
            missing_field: "transport_available",
            expected_precondition: Precondition::TransportAvailable,
        },
        MatrixCase {
            capability: Capability::WebSearch,
            missing_field: "tool_available",
            expected_precondition: Precondition::ToolAvailable,
        },
        MatrixCase {
            capability: Capability::WebSearch,
            missing_field: "transport_available",
            expected_precondition: Precondition::TransportAvailable,
        },
        MatrixCase {
            capability: Capability::WebFetch,
            missing_field: "tool_available",
            expected_precondition: Precondition::ToolAvailable,
        },
        MatrixCase {
            capability: Capability::WebFetch,
            missing_field: "transport_available",
            expected_precondition: Precondition::TransportAvailable,
        },
        MatrixCase {
            capability: Capability::ToolRoute,
            missing_field: "tool_available",
            expected_precondition: Precondition::ToolAvailable,
        },
        MatrixCase {
            capability: Capability::ToolRoute,
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
                    .any(|source| source.contains(".execute_tool.")),
                "strict adapted surfaces must not collapse typed capability probe source to execute_tool"
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
    assert!(legacy_tool_probe
        .blocked_on
        .contains(&Precondition::ToolAvailable));
    assert!(legacy_tool_probe
        .probe_sources
        .iter()
        .any(|source| source == "missing_probe: tool_route"));
    assert!(!legacy_tool_probe
        .probe_sources
        .iter()
        .any(|source| source == "heuristic.tool_hints_or_resource_kind"));
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
        executed_cases, 82,
        "probe authority matrix must execute 82 cases"
    );
}
