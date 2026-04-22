// Layer ownership: surface/orchestration (non-canonical orchestration coordination only).
use crate::contracts::{
    AmbiguityReason, Capability, CapabilityProbeSnapshot, Mutability, OperationKind, ParseResult,
    PolicyScope, RequestKind, RequestSurface, ResourceKind, TargetDescriptor,
    TypedOrchestrationRequest,
};

pub fn select_operation_kind(candidates: &[OperationKind]) -> OperationKind {
    match candidates {
        [] => OperationKind::Unknown,
        [only] => only.clone(),
        many if many.iter().any(|row| row == &OperationKind::Mutate) => OperationKind::Mutate,
        many if many.iter().any(|row| row == &OperationKind::Search) => OperationKind::Search,
        many if many.iter().any(|row| row == &OperationKind::Fetch) => OperationKind::Fetch,
        many if many.iter().any(|row| row == &OperationKind::Plan) => OperationKind::Plan,
        [first, ..] => first.clone(),
    }
}

pub fn select_resource_kind(candidates: &[ResourceKind]) -> ResourceKind {
    match candidates {
        [] => ResourceKind::Unspecified,
        [only] => only.clone(),
        _ => ResourceKind::Mixed,
    }
}

pub fn infer_request_kind(
    operation_candidates: &[OperationKind],
    operation_kind: &OperationKind,
) -> RequestKind {
    if operation_candidates.len() > 1 {
        return RequestKind::Ambiguous;
    }
    match operation_kind {
        OperationKind::Compare => RequestKind::Comparative,
        OperationKind::Assimilate | OperationKind::Plan => RequestKind::Workflow,
        OperationKind::Unknown => RequestKind::Ambiguous,
        _ => RequestKind::Direct,
    }
}

pub fn infer_policy_scope(resource_kind: &ResourceKind, mutability: &Mutability) -> PolicyScope {
    match (resource_kind, mutability) {
        (ResourceKind::Web, _) => PolicyScope::WebOnly,
        (ResourceKind::Workspace, _) => PolicyScope::WorkspaceOnly,
        (_, Mutability::Proposal | Mutability::Mutation) => PolicyScope::CoreProposal,
        (ResourceKind::Mixed, _) => PolicyScope::CrossBoundary,
        _ => PolicyScope::Default,
    }
}

pub fn build_parse_result(
    typed_request: TypedOrchestrationRequest,
    operation_candidates: &[OperationKind],
    resource_candidates: &[ResourceKind],
    adapter_reasons: &[String],
) -> ParseResult {
    parse_diagnostics(
        typed_request,
        operation_candidates,
        resource_candidates,
        adapter_reasons,
    )
}

// Compatibility alias during control-plane naming transition.
pub fn parse_diagnostics(
    typed_request: TypedOrchestrationRequest,
    operation_candidates: &[OperationKind],
    resource_candidates: &[ResourceKind],
    adapter_reasons: &[String],
) -> ParseResult {
    let surface_adapter_used = typed_request.adapted;
    let surface_adapter_fallback =
        !surface_adapter_used && !matches!(typed_request.surface, RequestSurface::Legacy);
    let mut confidence: f32 = 0.20;
    let mut ambiguity = Vec::new();
    let mut reasons = if surface_adapter_used {
        adapter_reasons.to_vec()
    } else {
        vec!["legacy_intent_compatibility_shim".to_string()]
    };
    let typed_probe_contract = typed_probe_contract_diagnostics(&typed_request);
    if !typed_probe_contract.messages.is_empty() {
        reasons.extend(typed_probe_contract.messages);
    }
    if typed_probe_contract.missing_count > 0 {
        ambiguity.push(AmbiguityReason::TypedProbeContractViolation);
        confidence -= (0.05 + (typed_probe_contract.missing_count as f32 * 0.01)).min(0.25);
    }

    if typed_request.operation_kind != OperationKind::Unknown {
        confidence += 0.30;
        reasons.push(format!("operation_kind:{:?}", typed_request.operation_kind).to_lowercase());
    } else {
        ambiguity.push(AmbiguityReason::UnknownOperation);
        reasons.push("operation_kind:unknown".to_string());
    }
    if typed_request.resource_kind != ResourceKind::Unspecified {
        confidence += 0.20;
        reasons.push(format!("resource_kind:{:?}", typed_request.resource_kind).to_lowercase());
    }
    if typed_request.request_kind != RequestKind::Ambiguous {
        confidence += 0.10;
    }
    if surface_adapter_used {
        confidence += 0.15;
        reasons.push("surface_native_typed_adapter".to_string());
    } else if surface_adapter_fallback {
        ambiguity.push(AmbiguityReason::SurfaceAdapterFallback);
        confidence -= 0.15;
        reasons
            .push(format!("surface_adapter_fallback:{:?}", typed_request.surface).to_lowercase());
    }
    if operation_candidates.len() > 1 {
        ambiguity.push(AmbiguityReason::MultipleOperationCandidates);
        confidence -= 0.20;
        reasons.push(format!(
            "operation_candidates:{}",
            operation_candidates.len()
        ));
    }
    if resource_candidates.len() > 1 {
        ambiguity.push(AmbiguityReason::MultipleResourceCandidates);
        confidence -= 0.10;
        reasons.push(format!("resource_candidates:{}", resource_candidates.len()));
    }
    if !typed_request.target_refs.is_empty() {
        confidence += 0.10;
        reasons.push("targets:present".to_string());
    } else if matches!(
        typed_request.operation_kind,
        OperationKind::Assimilate | OperationKind::Mutate
    ) {
        ambiguity.push(AmbiguityReason::MissingTargetSignals);
        confidence -= 0.10;
    }
    if typed_request
        .payload
        .as_object()
        .map(|row| row.is_empty())
        .unwrap_or(true)
    {
        ambiguity.push(AmbiguityReason::LegacyCompatOnly);
    }
    if typed_request
        .target_descriptors
        .iter()
        .any(|row| matches!(row, TargetDescriptor::Unknown { .. }))
    {
        ambiguity.push(AmbiguityReason::UnresolvedTargetDomain);
        confidence -= 0.05;
    }

    confidence = confidence.clamp(0.0, 0.99);
    if confidence < 0.55 {
        ambiguity.push(AmbiguityReason::LowConfidence);
    }

    ParseResult {
        typed_request,
        confidence,
        ambiguity,
        reasons,
        surface_adapter_used,
        surface_adapter_fallback,
    }
}

#[derive(Debug, Default)]
struct TypedProbeContractDiagnostics {
    messages: Vec<String>,
    missing_count: usize,
}

fn typed_probe_contract_diagnostics(
    typed_request: &TypedOrchestrationRequest,
) -> TypedProbeContractDiagnostics {
    if !typed_request.adapted || matches!(typed_request.surface, RequestSurface::Legacy) {
        return TypedProbeContractDiagnostics::default();
    }

    let required_capabilities =
        crate::request_classifier::required_capabilities_for_typed_request(typed_request);
    let mut requirements = required_capabilities
        .iter()
        .filter_map(required_probe_contract_for_capability)
        .collect::<Vec<_>>();
    requirements.sort_by_key(|row| row.0);
    requirements.dedup_by_key(|row| row.0);
    if requirements.is_empty() {
        return TypedProbeContractDiagnostics {
            messages: vec!["typed_probe_contract_complete".to_string()],
            missing_count: 0,
        };
    }

    let Some(envelope) = typed_request.core_probe_envelope.as_ref() else {
        return TypedProbeContractDiagnostics {
            messages: vec![
                "typed_probe_contract_missing:core_probe_envelope".to_string(),
                format!(
                    "typed_probe_contract_expected:{}",
                    requirements
                        .iter()
                        .map(|row| row.0)
                        .collect::<Vec<_>>()
                        .join(",")
                ),
            ],
            missing_count: 1,
        };
    };

    let mut diagnostics = TypedProbeContractDiagnostics::default();
    for (capability_key, fields) in requirements {
        let Some(snapshot) =
            probe_snapshot_for_contract_key(envelope.probes.as_slice(), capability_key)
        else {
            diagnostics.messages.push(format!(
                "typed_probe_contract_missing:capability.{capability_key}"
            ));
            diagnostics.missing_count += 1;
            continue;
        };
        for field in fields {
            if probe_field_is_missing(snapshot, field) {
                diagnostics.messages.push(format!(
                    "typed_probe_contract_missing:field.{capability_key}.{field}"
                ));
                diagnostics.missing_count += 1;
            }
        }
    }

    if diagnostics.messages.is_empty() {
        diagnostics
            .messages
            .push("typed_probe_contract_complete".to_string());
    }
    diagnostics
}

fn required_probe_contract_for_capability(
    capability: &Capability,
) -> Option<(&'static str, &'static [&'static str])> {
    match capability {
        Capability::ReadMemory => None,
        Capability::MutateTask => Some((
            "mutate_task",
            &[
                "target_supplied",
                "target_syntactically_valid",
                "target_exists",
                "authorization_valid",
                "policy_allows",
            ],
        )),
        Capability::PlanAssimilation => Some((
            "plan_assimilation",
            &[
                "target_supplied",
                "target_syntactically_valid",
                "target_exists",
                "policy_allows",
            ],
        )),
        Capability::VerifyClaim => Some(("verify_claim", &["transport_available"])),
        Capability::WorkspaceRead => Some(("workspace_read", &["tool_available", "transport_available"])),
        Capability::WorkspaceSearch => Some(("workspace_search", &["tool_available", "transport_available"])),
        Capability::WebSearch => Some(("web_search", &["tool_available", "transport_available"])),
        Capability::WebFetch => Some(("web_fetch", &["tool_available", "transport_available"])),
        Capability::ToolRoute => Some(("tool_route", &["tool_available", "transport_available"])),
        Capability::ExecuteTool => {
            Some(("execute_tool", &["tool_available", "transport_available"]))
        }
    }
}

fn probe_snapshot_for_contract_key<'a>(
    probes: &'a [CapabilityProbeSnapshot],
    capability_key: &str,
) -> Option<&'a CapabilityProbeSnapshot> {
    let capability = match capability_key {
        "read_memory" => Capability::ReadMemory,
        "mutate_task" => Capability::MutateTask,
        "workspace_read" => Capability::WorkspaceRead,
        "workspace_search" => Capability::WorkspaceSearch,
        "web_search" => Capability::WebSearch,
        "web_fetch" => Capability::WebFetch,
        "tool_route" => Capability::ToolRoute,
        "execute_tool" => Capability::ExecuteTool,
        "plan_assimilation" => Capability::PlanAssimilation,
        "verify_claim" => Capability::VerifyClaim,
        _ => return None,
    };
    probes.iter().find(|row| row.capability == capability)
}

fn probe_field_is_missing(snapshot: &CapabilityProbeSnapshot, field: &str) -> bool {
    match field {
        "tool_available" => snapshot.tool_available.is_none(),
        "target_supplied" => snapshot.target_supplied.is_none(),
        "target_syntactically_valid" => snapshot.target_syntactically_valid.is_none(),
        "target_exists" => snapshot.target_exists.is_none(),
        "authorization_valid" => snapshot.authorization_valid.is_none(),
        "policy_allows" => snapshot.policy_allows.is_none(),
        "transport_available" => snapshot.transport_available.is_none(),
        _ => false,
    }
}
