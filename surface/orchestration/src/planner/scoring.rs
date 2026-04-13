use crate::contracts::{
    CoreContractCall, Mutability, PlanScore, RequestClassification, TypedOrchestrationRequest,
};

pub fn score_candidate(
    request: &TypedOrchestrationRequest,
    classification: &RequestClassification,
    contracts: &[CoreContractCall],
    blocked_count: usize,
    degradation_count: usize,
    requires_clarification: bool,
) -> PlanScore {
    let authority_cost = if contracts
        .iter()
        .any(|row| matches!(row, CoreContractCall::TaskFabricProposal))
    {
        0.85
    } else if contracts.iter().any(|row| {
        matches!(
            row,
            CoreContractCall::ToolBrokerRequest | CoreContractCall::VerifierRequest
        )
    }) {
        0.45
    } else {
        0.15
    };
    let transport_dependency = if contracts.is_empty() {
        0.0
    } else {
        contracts
            .iter()
            .filter(|row| {
                matches!(
                    row,
                    CoreContractCall::ToolCapabilityProbe
                        | CoreContractCall::ToolBrokerRequest
                        | CoreContractCall::VerifierRequest
                )
            })
            .count() as f32
            / contracts.len() as f32
    };
    let mutation_risk = if matches!(request.mutability, Mutability::Mutation) {
        0.90
    } else if matches!(request.mutability, Mutability::Proposal) {
        0.45
    } else {
        0.10
    };
    let fallback_quality = if requires_clarification {
        0.20
    } else if degradation_count > 0 {
        0.55
    } else {
        0.92
    };
    let known_targets = request
        .target_descriptors
        .iter()
        .filter(|row| !matches!(row, crate::contracts::TargetDescriptor::Unknown { .. }))
        .count();
    let target_specificity = if request.target_descriptors.is_empty() {
        0.15
    } else {
        (known_targets as f32 / request.target_descriptors.len() as f32).clamp(0.0, 1.0)
    };

    let mut overall = classification.confidence;
    overall += 0.18 * fallback_quality;
    overall += 0.10 * target_specificity;
    if !request.tool_hints.is_empty() {
        overall += 0.03;
    }
    overall -= 0.08 * authority_cost;
    overall -= 0.08 * transport_dependency;
    overall -= 0.08 * mutation_risk;
    overall -= 0.10 * blocked_count as f32;
    overall -= 0.05 * degradation_count as f32;
    if requires_clarification {
        overall -= 0.18;
    }

    PlanScore {
        overall: overall.clamp(0.0, 0.99),
        authority_cost,
        transport_dependency,
        mutation_risk,
        fallback_quality,
        target_specificity,
    }
}
