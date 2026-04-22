// Layer ownership: surface/orchestration (non-canonical orchestration coordination only).
use crate::contracts::{
    CoreContractCall, OrchestrationPlanStep, RequestKind, ResourceKind, TypedOrchestrationRequest,
};

pub(super) fn filter_steps_by_contract(
    steps: &[OrchestrationPlanStep],
    allowed: &[CoreContractCall],
) -> Vec<OrchestrationPlanStep> {
    steps
        .iter()
        .filter(|step| allowed.contains(&step.target_contract))
        .cloned()
        .collect()
}

pub(super) fn is_structural_comparative_request(request: &TypedOrchestrationRequest) -> bool {
    request.request_kind == RequestKind::Comparative || request.resource_kind == ResourceKind::Mixed
}

pub(super) fn transport_explicitly_unavailable(request: &TypedOrchestrationRequest) -> bool {
    request
        .payload
        .get("transport_available")
        .and_then(|row| row.as_bool())
        == Some(false)
}
