use crate::contracts::{
    Capability, DegradationReason, Mutability, OperationKind, PolicyScope, Precondition,
    ResourceKind, TypedOrchestrationRequest,
};
use serde_json::Value;

fn payload_bool(payload: &Value, key: &str) -> Option<bool> {
    payload.get(key).and_then(Value::as_bool)
}

fn tool_available(request: &TypedOrchestrationRequest) -> bool {
    payload_bool(&request.payload, "tool_available").unwrap_or_else(|| {
        !request.tool_hints.is_empty()
            || matches!(
                request.resource_kind,
                ResourceKind::Web | ResourceKind::Tooling | ResourceKind::Mixed
            )
    })
}

fn target_exists(request: &TypedOrchestrationRequest) -> bool {
    payload_bool(&request.payload, "target_exists").unwrap_or_else(|| {
        match request.mutability {
            Mutability::ReadOnly => true,
            Mutability::Proposal | Mutability::Mutation => !request.target_refs.is_empty(),
        }
    })
}

fn authorization_valid(request: &TypedOrchestrationRequest) -> bool {
    payload_bool(&request.payload, "authorization_valid").unwrap_or_else(|| {
        !(request.mutability == Mutability::Mutation
            && request.policy_scope == PolicyScope::CrossBoundary)
    })
}

fn policy_allows(request: &TypedOrchestrationRequest) -> bool {
    payload_bool(&request.payload, "policy_allows").unwrap_or(true)
}

fn transport_available(request: &TypedOrchestrationRequest) -> bool {
    payload_bool(&request.payload, "transport_available").unwrap_or(true)
}

pub fn blocked_preconditions(
    request: &TypedOrchestrationRequest,
    capabilities: &[Capability],
) -> Vec<Precondition> {
    let mut blocked = Vec::new();
    let needs_tooling = capabilities
        .iter()
        .any(|row| matches!(row, Capability::ExecuteTool | Capability::VerifyClaim));
    let needs_targets = capabilities.iter().any(|row| row == &Capability::PlanAssimilation)
        || request.mutability == Mutability::Mutation;
    let needs_mutation_authority = capabilities.iter().any(|row| row == &Capability::MutateTask)
        || request.mutability == Mutability::Mutation;

    if needs_tooling && !tool_available(request) {
        blocked.push(Precondition::ToolAvailable);
    }
    if needs_tooling && !transport_available(request) {
        blocked.push(Precondition::TransportAvailable);
    }
    if needs_targets && !target_exists(request) {
        blocked.push(Precondition::TargetExists);
    }
    if needs_mutation_authority && !authorization_valid(request) {
        blocked.push(Precondition::AuthorizationValid);
    }
    if (needs_mutation_authority || request.operation_kind == OperationKind::Assimilate)
        && !policy_allows(request)
    {
        blocked.push(Precondition::PolicyAllows);
    }

    blocked
}

pub fn degradation_reason(blocked_on: &[Precondition]) -> Option<DegradationReason> {
    if blocked_on.contains(&Precondition::ToolAvailable) {
        return Some(DegradationReason::ToolUnavailable);
    }
    if blocked_on.contains(&Precondition::TransportAvailable) {
        return Some(DegradationReason::TransportFailure);
    }
    if blocked_on.contains(&Precondition::TargetExists) {
        return Some(DegradationReason::MissingTarget);
    }
    if blocked_on.contains(&Precondition::AuthorizationValid) {
        return Some(DegradationReason::AuthFailure);
    }
    if blocked_on.contains(&Precondition::PolicyAllows) {
        return Some(DegradationReason::PolicyDenied);
    }
    None
}

pub fn can_degrade(request: &TypedOrchestrationRequest, reason: &DegradationReason) -> bool {
    match reason {
        DegradationReason::ToolUnavailable | DegradationReason::TransportFailure => {
            matches!(
                request.resource_kind,
                ResourceKind::Workspace | ResourceKind::Memory | ResourceKind::Mixed
            ) || request.operation_kind == OperationKind::Compare
        }
        DegradationReason::MissingTarget => matches!(request.mutability, Mutability::ReadOnly),
        DegradationReason::AuthFailure | DegradationReason::PolicyDenied => false,
    }
}
