use crate::contracts::{
    Capability, CapabilityProbeResult, DegradationReason, Mutability, OperationKind, PolicyScope,
    Precondition, ResourceKind, TargetDescriptor, TypedOrchestrationRequest,
};
use serde_json::Value;

fn capability_key(capability: &Capability) -> &'static str {
    match capability {
        Capability::ReadMemory => "read_memory",
        Capability::MutateTask => "mutate_task",
        Capability::ExecuteTool => "execute_tool",
        Capability::PlanAssimilation => "plan_assimilation",
        Capability::VerifyClaim => "verify_claim",
    }
}

fn probe_bool(request: &TypedOrchestrationRequest, path: &[&str], top_level: &str) -> Option<bool> {
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

fn traverse_bool(value: &Value, path: &[&str]) -> Option<bool> {
    let mut cursor = value;
    for segment in path {
        cursor = cursor.get(*segment)?;
    }
    cursor.as_bool()
}

fn tool_available(request: &TypedOrchestrationRequest, capability: &Capability) -> (bool, String) {
    let key = capability_key(capability);
    if let Some(value) = probe_bool(request, &[key, "tool_available"], "tool_available") {
        return (
            value,
            format!("probe.capability_probes.{key}.tool_available"),
        );
    }
    (
        !request.tool_hints.is_empty()
            || matches!(
                request.resource_kind,
                ResourceKind::Web | ResourceKind::Tooling | ResourceKind::Mixed
            ),
        "heuristic.tool_hints_or_resource_kind".to_string(),
    )
}

fn target_supplied(request: &TypedOrchestrationRequest, capability: &Capability) -> (bool, String) {
    let key = capability_key(capability);
    if let Some(value) = probe_bool(request, &[key, "target_supplied"], "target_supplied") {
        return (
            value,
            format!("probe.capability_probes.{key}.target_supplied"),
        );
    }
    let supplied = !request.target_descriptors.is_empty();
    (supplied, "heuristic.target_descriptors_present".to_string())
}

fn target_syntax_valid(
    request: &TypedOrchestrationRequest,
    capability: &Capability,
) -> (bool, String) {
    let key = capability_key(capability);
    if let Some(value) = probe_bool(
        request,
        &[key, "target_syntactically_valid"],
        "target_syntactically_valid",
    ) {
        return (
            value,
            format!("probe.capability_probes.{key}.target_syntactically_valid"),
        );
    }
    let valid = request
        .target_descriptors
        .iter()
        .any(|row| !matches!(row, TargetDescriptor::Unknown { .. }));
    (valid, "heuristic.target_descriptor_domain".to_string())
}

fn target_exists(request: &TypedOrchestrationRequest, capability: &Capability) -> (bool, String) {
    let key = capability_key(capability);
    if let Some(value) = probe_bool(request, &[key, "target_exists"], "target_exists") {
        return (
            value,
            format!("probe.capability_probes.{key}.target_exists"),
        );
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
    let key = capability_key(capability);
    if let Some(value) = probe_bool(
        request,
        &[key, "authorization_valid"],
        "authorization_valid",
    ) {
        return (
            value,
            format!("probe.capability_probes.{key}.authorization_valid"),
        );
    }
    (
        !(request.mutability == Mutability::Mutation
            && request.policy_scope == PolicyScope::CrossBoundary),
        "heuristic.mutation_cross_boundary".to_string(),
    )
}

fn policy_allows(request: &TypedOrchestrationRequest, capability: &Capability) -> (bool, String) {
    let key = capability_key(capability);
    if let Some(value) = probe_bool(request, &[key, "policy_allows"], "policy_allows") {
        return (
            value,
            format!("probe.capability_probes.{key}.policy_allows"),
        );
    }
    (true, "heuristic.policy_default_allow".to_string())
}

fn transport_available(
    request: &TypedOrchestrationRequest,
    capability: &Capability,
) -> (bool, String) {
    let key = capability_key(capability);
    if let Some(value) = probe_bool(
        request,
        &[key, "transport_available"],
        "transport_available",
    ) {
        return (
            value,
            format!("probe.capability_probes.{key}.transport_available"),
        );
    }
    (true, "heuristic.transport_default_available".to_string())
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

    if matches!(capability, Capability::ExecuteTool) {
        let (available, source) = tool_available(request, capability);
        probe_sources.push(source);
        if !available {
            blocked_on.push(Precondition::ToolAvailable);
            degradation_reasons.push(DegradationReason::ToolUnavailable);
        }
    }

    if matches!(
        capability,
        Capability::ExecuteTool | Capability::VerifyClaim
    ) {
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
