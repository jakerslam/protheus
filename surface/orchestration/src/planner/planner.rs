use crate::contracts::{
    OrchestrationPlanStep, PlanCandidate, Precondition, RequestClassification,
    TypedOrchestrationRequest,
};

use super::{capability_registry, preconditions, scoring};

pub fn build_plan_candidate(
    request: &TypedOrchestrationRequest,
    classification: &RequestClassification,
) -> PlanCandidate {
    let capabilities = classification.required_capabilities.clone();
    let blocked_on = preconditions::blocked_preconditions(request, &capabilities);
    let degradation = preconditions::degradation_reason(&blocked_on);
    let requires_clarification = classification.needs_clarification
        || blocked_on.contains(&Precondition::TargetExists)
        || blocked_on.contains(&Precondition::AuthorizationValid)
        || blocked_on.contains(&Precondition::PolicyAllows);
    let allow_degraded_chain = degradation
        .as_ref()
        .map(|reason| preconditions::can_degrade(request, reason))
        .unwrap_or(false);
    let mut steps = Vec::new();
    let mut reasons = classification.reasons.clone();

    for capability in &capabilities {
        let spec = capability_registry::spec_for(capability);
        let capability_blocked = spec.requires.iter().any(|row| blocked_on.contains(row));
        if capability_blocked && !allow_degraded_chain {
            reasons.push(format!("capability_blocked:{capability:?}").to_lowercase());
            continue;
        }
        let mut chain = if capability_blocked {
            reasons.push(format!("capability_degraded:{capability:?}").to_lowercase());
            spec.degraded_steps
        } else {
            spec.primary_steps
        };
        if capability_blocked {
            for step in &mut chain {
                step.blocked_on = spec.requires.clone();
            }
        }
        steps.extend(chain);
    }

    dedupe_steps(&mut steps);

    let confidence = scoring::score_candidate(
        request,
        classification,
        steps.len(),
        &blocked_on,
        degradation.as_ref(),
        requires_clarification,
    );

    if steps.is_empty() {
        reasons.push("candidate_empty_after_capability_resolution".to_string());
    }

    PlanCandidate {
        plan_id: format!(
            "plan_{:?}_{:?}_{:?}",
            classification.request_class, request.operation_kind, request.resource_kind
        )
        .to_lowercase(),
        steps,
        confidence,
        requires_clarification,
        blocked_on,
        degradation,
        capabilities,
        reasons,
    }
}

fn dedupe_steps(steps: &mut Vec<OrchestrationPlanStep>) {
    let mut seen = std::collections::BTreeSet::new();
    steps.retain(|row| seen.insert(format!("{:?}:{}", row.target_contract, row.operation)));
}
