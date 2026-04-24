// Layer ownership: surface/orchestration (non-canonical orchestration coordination only).
use crate::contracts::{
    Capability, CapabilityProbeResult, DegradationReason, Mutability, OperationKind, PolicyScope,
    Precondition, RequestSurface, ResourceKind, TargetDescriptor, TypedOrchestrationRequest,
};
use serde_json::{json, Value};

fn capability_key(capability: &Capability) -> &'static str {
    match capability.probe_keys().first().copied() {
        Some(key) => key,
        None => unreachable!("capability must declare at least one probe key"),
    }
}

fn allow_payload_probe_shortcuts(request: &TypedOrchestrationRequest) -> bool {
    matches!(request.surface, RequestSurface::Legacy)
}

fn probe_bool(request: &TypedOrchestrationRequest, path: &[&str], top_level: &str) -> Option<bool> {
    if !allow_payload_probe_shortcuts(request) {
        return None;
    }
    let mut cursor = request.payload.get("capability_probes");
    if let Some(capability_key) = path.first() {
        cursor = cursor.and_then(|row| row.get(*capability_key));
    }
    for segment in path.iter().skip(1) {
        cursor = cursor.and_then(|row| row.get(*segment));
    }
    cursor
        .and_then(Value::as_bool)
        .or_else(|| {
            request
                .payload
                .get("probes")
                .and_then(|row| traverse_bool(row, path))
        })
        .or_else(|| request.payload.get(top_level).and_then(Value::as_bool))
}

fn envelope_probe_bool(
    request: &TypedOrchestrationRequest,
    capability: &Capability,
    field: Option<&str>,
) -> Option<(bool, String)> {
    let field = field?;
    for probe_key in capability.probe_keys() {
        let parsed = parse_capability_key(probe_key)?;
        let Some(row) = request
            .core_probe_envelope
            .as_ref()?
            .probes
            .iter()
            .find(|row| row.capability == parsed)
        else {
            continue;
        };
        let value = match field {
            "tool_available" => row.tool_available,
            "target_supplied" => row.target_supplied,
            "target_syntactically_valid" => row.target_syntactically_valid,
            "target_exists" => row.target_exists,
            "authorization_valid" => row.authorization_valid,
            "policy_allows" => row.policy_allows,
            "transport_available" => row.transport_available,
            _ => None,
        };
        if let Some(value) = value {
            return Some((value, envelope_probe_source(probe_key, field, value)));
        }
    }
    None
}

fn envelope_probe_source(capability_key: &str, field: &str, value: bool) -> String {
    if value {
        format!("probe.core_probe_envelope.{capability_key}.{field}")
    } else {
        format!("denied_probe: {capability_key}.{field}")
    }
}

fn parse_capability_key(value: &str) -> Option<Capability> {
    match value {
        "read_memory" => Some(Capability::ReadMemory),
        "mutate_task" => Some(Capability::MutateTask),
        "workspace_read" => Some(Capability::WorkspaceRead),
        "file_read" => Some(Capability::WorkspaceRead),
        "read_file" => Some(Capability::WorkspaceRead),
        "workspace_search" => Some(Capability::WorkspaceSearch),
        "file_search" => Some(Capability::WorkspaceSearch),
        "file_list" => Some(Capability::WorkspaceSearch),
        "workspace_analyze" => Some(Capability::WorkspaceSearch),
        "web_search" => Some(Capability::WebSearch),
        "web_lookup" => Some(Capability::WebSearch),
        "web_fetch" => Some(Capability::WebFetch),
        "tool_route" => Some(Capability::ToolRoute),
        "plan_assimilation" => Some(Capability::PlanAssimilation),
        "verify_claim" => Some(Capability::VerifyClaim),
        _ => None,
    }
}

fn required_probe_key(capability: &Capability) -> &'static str {
    capability_key(capability)
}

fn missing_probe_source(capability: &Capability) -> String {
    format!("missing_probe: {}", required_probe_key(capability))
}

fn missing_probe_field_source(capability: &Capability, probe_name: &str) -> String {
    format!("missing_probe: {}.{}", required_probe_key(capability), probe_name)
}

fn traverse_bool(value: &Value, path: &[&str]) -> Option<bool> {
    let mut cursor = value;
    for segment in path {
        cursor = cursor.get(*segment)?;
    }
    cursor.as_bool()
}

fn fail_closed_on_missing_probe_for_typed_surface(
    request: &TypedOrchestrationRequest,
    capability: &Capability,
    probe_name: &str,
) -> Option<(bool, String)> {
    if !matches!(request.surface, RequestSurface::Legacy) {
        return Some((false, missing_probe_field_source(capability, probe_name)));
    }
    None
}

pub fn deterministic_routing_decision_trace(request: &TypedOrchestrationRequest) -> Value {
    let selected = Capability::primary_tool_for(&request.operation_kind, &request.resource_kind);
    let selected_key = required_probe_key(&selected);
    let (available, reason) = tool_available(request, &selected);
    let rejected: Vec<&'static str> = [
        Capability::WorkspaceRead,
        Capability::WorkspaceSearch,
        Capability::WebSearch,
        Capability::WebFetch,
        Capability::ToolRoute,
    ]
    .iter()
    .filter(|candidate| required_probe_key(candidate) != selected_key)
    .map(required_probe_key)
    .collect();
    json!({
        "selected": selected_key,
        "rejected": rejected,
        "reason": reason,
        "confidence": if available { 1.0 } else { 0.0 },
    })
}

fn authoritative_probe_required(
    request: &TypedOrchestrationRequest,
    capability: &Capability,
    probe_name: &str,
) -> Option<(bool, String)> {
    fail_closed_on_missing_probe_for_typed_surface(request, capability, probe_name)
}

fn tool_available(request: &TypedOrchestrationRequest, capability: &Capability) -> (bool, String) {
    if let Some((value, source)) = envelope_probe_bool(request, capability, Some("tool_available"))
    {
        return (value, source);
    }
    if let Some(required) = authoritative_probe_required(request, capability, "tool_available") {
        return required;
    }
    for probe_key in capability.probe_keys() {
        if let Some(value) = probe_bool(request, &[probe_key, "tool_available"], "tool_available") {
            return (
                value,
                format!("probe.capability_probes.{probe_key}.tool_available"),
            );
        }
    }
    (false, missing_probe_source(capability))
}

fn target_supplied(request: &TypedOrchestrationRequest, capability: &Capability) -> (bool, String) {
    if let Some((value, source)) = envelope_probe_bool(request, capability, Some("target_supplied"))
    {
        return (value, source);
    }
    if let Some(required) = authoritative_probe_required(request, capability, "target_supplied") {
        return required;
    }
    for probe_key in capability.probe_keys() {
        if let Some(value) = probe_bool(request, &[probe_key, "target_supplied"], "target_supplied")
        {
            return (
                value,
                format!("probe.capability_probes.{probe_key}.target_supplied"),
            );
        }
    }
    let supplied = !request.target_descriptors.is_empty();
    (supplied, "heuristic.target_descriptors_present".to_string())
}

fn target_syntax_valid(
    request: &TypedOrchestrationRequest,
    capability: &Capability,
) -> (bool, String) {
    if let Some((value, source)) =
        envelope_probe_bool(request, capability, Some("target_syntactically_valid"))
    {
        return (value, source);
    }
    if let Some(required) =
        authoritative_probe_required(request, capability, "target_syntactically_valid")
    {
        return required;
    }
    for probe_key in capability.probe_keys() {
        if let Some(value) = probe_bool(
            request,
            &[probe_key, "target_syntactically_valid"],
            "target_syntactically_valid",
        ) {
            return (
                value,
                format!("probe.capability_probes.{probe_key}.target_syntactically_valid"),
            );
        }
    }
    let valid = request
        .target_descriptors
        .iter()
        .any(|row| !matches!(row, TargetDescriptor::Unknown { .. }));
    (valid, "heuristic.target_descriptor_domain".to_string())
}

fn target_exists(request: &TypedOrchestrationRequest, capability: &Capability) -> (bool, String) {
    if let Some((value, source)) = envelope_probe_bool(request, capability, Some("target_exists")) {
        return (value, source);
    }
    if let Some(required) = authoritative_probe_required(request, capability, "target_exists") {
        return required;
    }
    for probe_key in capability.probe_keys() {
        if let Some(value) = probe_bool(request, &[probe_key, "target_exists"], "target_exists") {
            return (
                value,
                format!("probe.capability_probes.{probe_key}.target_exists"),
            );
        }
    }
    let exists = match request.mutability {
        Mutability::ReadOnly => true,
        Mutability::Proposal | Mutability::Mutation => !request.target_refs.is_empty(),
    };
    (exists, "heuristic.target_refs_present".to_string())
}

fn authorization_valid(
    request: &TypedOrchestrationRequest,
    capability: &Capability,
) -> (bool, String) {
    if let Some((value, source)) =
        envelope_probe_bool(request, capability, Some("authorization_valid"))
    {
        return (value, source);
    }
    if let Some(required) = authoritative_probe_required(request, capability, "authorization_valid")
    {
        return required;
    }
    for probe_key in capability.probe_keys() {
        if let Some(value) = probe_bool(
            request,
            &[probe_key, "authorization_valid"],
            "authorization_valid",
        ) {
            return (
                value,
                format!("probe.capability_probes.{probe_key}.authorization_valid"),
            );
        }
    }
    (
        !(request.mutability == Mutability::Mutation
            && request.policy_scope == PolicyScope::CrossBoundary),
        "heuristic.mutation_cross_boundary".to_string(),
    )
}

fn policy_allows(request: &TypedOrchestrationRequest, capability: &Capability) -> (bool, String) {
    if let Some((value, source)) = envelope_probe_bool(request, capability, Some("policy_allows")) {
        return (value, source);
    }
    if let Some(required) = authoritative_probe_required(request, capability, "policy_allows") {
        return required;
    }
    for probe_key in capability.probe_keys() {
        if let Some(value) = probe_bool(request, &[probe_key, "policy_allows"], "policy_allows") {
            return (
                value,
                format!("probe.capability_probes.{probe_key}.policy_allows"),
            );
        }
    }
    let allows = if request.mutability == Mutability::ReadOnly {
        true
    } else {
        !matches!(request.policy_scope, PolicyScope::CrossBoundary)
            && !matches!(request.operation_kind, OperationKind::Assimilate)
    };
    (allows, "heuristic.policy_scope_and_mutability".to_string())
}

fn transport_available(
    request: &TypedOrchestrationRequest,
    capability: &Capability,
) -> (bool, String) {
    if let Some((value, source)) =
        envelope_probe_bool(request, capability, Some("transport_available"))
    {
        return (value, source);
    }
    if let Some(required) = authoritative_probe_required(request, capability, "transport_available")
    {
        return required;
    }
    for probe_key in capability.probe_keys() {
        if let Some(value) = probe_bool(
            request,
            &[probe_key, "transport_available"],
            "transport_available",
        ) {
            return (
                value,
                format!("probe.capability_probes.{probe_key}.transport_available"),
            );
        }
    }
    let likely_transport = !request.tool_hints.is_empty()
        || matches!(
            request.resource_kind,
            ResourceKind::Web
                | ResourceKind::Workspace
                | ResourceKind::Tooling
                | ResourceKind::Mixed
        )
        || matches!(
            request.operation_kind,
            OperationKind::Search
                | OperationKind::Fetch
                | OperationKind::Compare
                | OperationKind::InspectTooling
        )
        || capability.is_tool_family();
    (
        likely_transport,
        "heuristic.transport_hints_or_operation".to_string(),
    )
}

fn dedupe<T: Ord>(rows: &mut Vec<T>) {
    rows.sort();
    rows.dedup();
}

pub fn probe_capability(
    request: &TypedOrchestrationRequest,
    capability: &Capability,
) -> CapabilityProbeResult {
    let mut blocked_on = Vec::new();
    let mut degradation_reasons = Vec::new();
    let mut probe_sources = Vec::new();

    let requires_target = matches!(
        capability,
        Capability::PlanAssimilation | Capability::MutateTask
    ) || request.mutability == Mutability::Mutation;
    if requires_target {
        let (supplied, source) = target_supplied(request, capability);
        probe_sources.push(source);
        if !supplied {
            blocked_on.push(Precondition::TargetSupplied);
            degradation_reasons.push(DegradationReason::MissingTarget);
        } else {
            let (valid, source) = target_syntax_valid(request, capability);
            probe_sources.push(source);
            if !valid {
                blocked_on.push(Precondition::TargetSyntacticallyValid);
                degradation_reasons.push(DegradationReason::TargetInvalid);
            } else {
                let (exists, source) = target_exists(request, capability);
                probe_sources.push(source);
                if !exists {
                    blocked_on.push(Precondition::TargetExists);
                    degradation_reasons.push(DegradationReason::TargetNotFound);
                }
            }
        }
    }

    if capability.is_tool_family() {
        let (available, source) = tool_available(request, capability);
        probe_sources.push(source);
        if !available {
            blocked_on.push(Precondition::ToolAvailable);
            degradation_reasons.push(DegradationReason::ToolUnavailable);
        }
    }

    if capability.is_tool_family() || matches!(capability, Capability::VerifyClaim) {
        let (available, source) = transport_available(request, capability);
        probe_sources.push(source);
        if !available {
            blocked_on.push(Precondition::TransportAvailable);
            degradation_reasons.push(DegradationReason::TransportFailure);
        }
    }

    if matches!(capability, Capability::MutateTask) {
        let (allowed, source) = authorization_valid(request, capability);
        probe_sources.push(source);
        if !allowed {
            blocked_on.push(Precondition::AuthorizationValid);
            degradation_reasons.push(DegradationReason::AuthFailure);
        }
    }

    if matches!(
        capability,
        Capability::MutateTask | Capability::PlanAssimilation
    ) || request.operation_kind == OperationKind::Assimilate
    {
        let (allowed, source) = policy_allows(request, capability);
        probe_sources.push(source);
        if !allowed {
            blocked_on.push(Precondition::PolicyAllows);
            degradation_reasons.push(DegradationReason::PolicyDenied);
        }
    }

    dedupe(&mut blocked_on);
    dedupe(&mut degradation_reasons);
    probe_sources.sort();
    probe_sources.dedup();

    let can_degrade = degradation_reasons
        .iter()
        .all(|reason| can_degrade_reason(request, capability, reason));

    CapabilityProbeResult {
        capability: capability.clone(),
        blocked_on,
        degradation_reasons,
        can_degrade,
        probe_sources,
    }
}

pub fn probe_capabilities(
    request: &TypedOrchestrationRequest,
    capabilities: &[Capability],
) -> Vec<CapabilityProbeResult> {
    capabilities
        .iter()
        .map(|capability| probe_capability(request, capability))
        .collect()
}

pub fn blocked_preconditions(probes: &[CapabilityProbeResult]) -> Vec<Precondition> {
    let mut blocked = probes
        .iter()
        .flat_map(|row| row.blocked_on.iter().cloned())
        .collect::<Vec<_>>();
    dedupe(&mut blocked);
    blocked
}

pub fn degradation_reasons(probes: &[CapabilityProbeResult]) -> Vec<DegradationReason> {
    let mut reasons = probes
        .iter()
        .flat_map(|row| row.degradation_reasons.iter().cloned())
        .collect::<Vec<_>>();
    dedupe(&mut reasons);
    reasons
}

fn can_degrade_reason(
    request: &TypedOrchestrationRequest,
    capability: &Capability,
    reason: &DegradationReason,
) -> bool {
    match reason {
        DegradationReason::ToolUnavailable | DegradationReason::TransportFailure => {
            matches!(
                request.resource_kind,
                ResourceKind::Workspace | ResourceKind::Memory | ResourceKind::Mixed
            ) || matches!(capability, Capability::VerifyClaim)
                || request.operation_kind == OperationKind::Compare
        }
        DegradationReason::MissingTarget
        | DegradationReason::TargetInvalid
        | DegradationReason::TargetNotFound => matches!(request.mutability, Mutability::ReadOnly),
        DegradationReason::AuthFailure | DegradationReason::PolicyDenied => false,
    }
}
