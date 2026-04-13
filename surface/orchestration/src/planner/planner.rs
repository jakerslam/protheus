// Layer ownership: surface/orchestration (non-canonical orchestration coordination only).
use crate::contracts::{
    Capability, CapabilityProbeResult, OrchestrationPlanStep, PlanCandidate, PlanScore,
    PlanVariant, Precondition, RequestClassification, TypedOrchestrationRequest,
};

use super::{capability_registry, preconditions, scoring};

pub fn build_plan_candidates(
    request: &TypedOrchestrationRequest,
    classification: &RequestClassification,
) -> Vec<PlanCandidate> {
    let capabilities = classification.required_capabilities.clone();
    let probes = preconditions::probe_capabilities(request, &capabilities);
    let mut candidates = vec![
        build_candidate_for_variant(
            request,
            classification,
            &capabilities,
            &probes,
            PlanVariant::Safest,
        ),
        build_candidate_for_variant(
            request,
            classification,
            &capabilities,
            &probes,
            PlanVariant::Fastest,
        ),
        build_candidate_for_variant(
            request,
            classification,
            &capabilities,
            &probes,
            PlanVariant::DegradedFallback,
        ),
        build_candidate_for_variant(
            request,
            classification,
            &capabilities,
            &probes,
            PlanVariant::ClarificationFirst,
        ),
    ];
    candidates.sort_by(|left, right| {
        let left_executable = !left.steps.is_empty();
        let right_executable = !right.steps.is_empty();
        right_executable
            .cmp(&left_executable)
            .then(left.blocked_on.len().cmp(&right.blocked_on.len()))
            .then(left.degradation.len().cmp(&right.degradation.len()))
            .then(
                right
                    .score
                    .overall
                    .partial_cmp(&left.score.overall)
                    .unwrap_or(std::cmp::Ordering::Equal),
            )
            .then(
                left.requires_clarification
                    .cmp(&right.requires_clarification),
            )
            .then(variant_priority(&left.variant).cmp(&variant_priority(&right.variant)))
            .then(right.steps.len().cmp(&left.steps.len()))
    });
    candidates
}

pub fn build_plan_candidate(
    request: &TypedOrchestrationRequest,
    classification: &RequestClassification,
) -> PlanCandidate {
    build_plan_candidates(request, classification)
        .into_iter()
        .next()
        .unwrap_or_else(|| {
            empty_candidate(classification, Vec::new(), PlanVariant::ClarificationFirst)
        })
}

fn build_candidate_for_variant(
    request: &TypedOrchestrationRequest,
    classification: &RequestClassification,
    capabilities: &[Capability],
    probes: &[CapabilityProbeResult],
    variant: PlanVariant,
) -> PlanCandidate {
    let mut steps = Vec::new();
    let mut reasons = classification.reasons.clone();

    for capability in capabilities {
        let probe = probes
            .iter()
            .find(|row| &row.capability == capability)
            .cloned()
            .unwrap_or_else(|| CapabilityProbeResult {
                capability: capability.clone(),
                blocked_on: Vec::new(),
                degradation_reasons: Vec::new(),
                can_degrade: false,
                probe_sources: vec!["probe.missing".to_string()],
            });
        let spec = capability_registry::spec_for(capability);
        let blocked = !probe.blocked_on.is_empty();
        let wants_clarification = matches!(variant, PlanVariant::ClarificationFirst)
            && (blocked || classification.needs_clarification);
        if wants_clarification {
            reasons.push(format!("clarification_first:{capability:?}").to_lowercase());
            continue;
        }
        if blocked {
            reasons.push(format!("capability_blocked:{capability:?}").to_lowercase());
            if matches!(
                variant,
                PlanVariant::Safest | PlanVariant::ClarificationFirst
            ) || !probe.can_degrade
                || spec.degraded_steps.is_empty()
            {
                continue;
            }
        }

        let using_degraded = blocked
            && probe.can_degrade
            && !spec.degraded_steps.is_empty()
            && matches!(
                variant,
                PlanVariant::Fastest | PlanVariant::DegradedFallback
            );
        let mut chain = if using_degraded {
            reasons.push(format!("capability_degraded:{capability:?}").to_lowercase());
            spec.degraded_steps
        } else {
            spec.primary_steps
        };
        for step in &mut chain {
            if blocked {
                step.blocked_on.extend(probe.blocked_on.clone());
                step.blocked_on.sort();
                step.blocked_on.dedup();
            }
            step.rationale
                .push(format!("variant:{variant:?}").to_lowercase());
            step.rationale.extend(probe.probe_sources.iter().cloned());
            step.rationale.sort();
            step.rationale.dedup();
        }
        steps.extend(chain);
    }

    dedupe_steps(&mut steps);

    let blocked_on = preconditions::blocked_preconditions(probes);
    let degradation = preconditions::degradation_reasons(probes);
    let requires_clarification = classification.needs_clarification
        || blocked_on.iter().any(|row| {
            matches!(
                row,
                Precondition::TargetSupplied
                    | Precondition::TargetSyntacticallyValid
                    | Precondition::TargetExists
                    | Precondition::AuthorizationValid
                    | Precondition::PolicyAllows
            )
        })
        || matches!(variant, PlanVariant::ClarificationFirst)
            && (!blocked_on.is_empty() || classification.needs_clarification);
    let contracts = steps
        .iter()
        .map(|row| row.target_contract.clone())
        .collect::<Vec<_>>();
    let score = scoring::score_candidate(
        request,
        classification,
        contracts.as_slice(),
        blocked_on.len(),
        degradation.len(),
        requires_clarification,
    );
    if steps.is_empty() {
        reasons.push("candidate_empty_after_capability_resolution".to_string());
    }

    PlanCandidate {
        plan_id: format!(
            "plan_{:?}_{:?}_{:?}_{:?}",
            classification.request_class, request.operation_kind, request.resource_kind, variant
        )
        .to_lowercase(),
        variant,
        steps,
        confidence: score.overall,
        score,
        requires_clarification,
        blocked_on,
        degradation,
        capabilities: capabilities.to_vec(),
        capability_probes: probes.to_vec(),
        reasons,
    }
}

fn empty_candidate(
    classification: &RequestClassification,
    capabilities: Vec<Capability>,
    variant: PlanVariant,
) -> PlanCandidate {
    PlanCandidate {
        plan_id: "plan_empty".to_string(),
        variant,
        steps: Vec::new(),
        confidence: 0.0,
        score: PlanScore {
            overall: 0.0,
            authority_cost: 0.0,
            transport_dependency: 0.0,
            mutation_risk: 0.0,
            fallback_quality: 0.0,
            target_specificity: 0.0,
        },
        requires_clarification: true,
        blocked_on: Vec::new(),
        degradation: Vec::new(),
        capabilities,
        capability_probes: Vec::new(),
        reasons: classification.reasons.clone(),
    }
}

fn dedupe_steps(steps: &mut Vec<OrchestrationPlanStep>) {
    let mut merged = Vec::<OrchestrationPlanStep>::new();
    let mut indices = std::collections::BTreeMap::<String, usize>::new();
    for step in steps.drain(..) {
        let key = format!(
            "{:?}:{}:{:?}",
            step.target_contract, step.operation, step.blocked_on
        );
        if let Some(index) = indices.get(&key).copied() {
            let existing = &mut merged[index];
            existing
                .merged_capabilities
                .extend(step.merged_capabilities);
            existing
                .merged_capabilities
                .sort_by_key(|row| format!("{row:?}"));
            existing.merged_capabilities.dedup();
            existing.rationale.extend(step.rationale);
            existing.rationale.sort();
            existing.rationale.dedup();
        } else {
            indices.insert(key, merged.len());
            merged.push(step);
        }
    }
    *steps = merged;
}

fn variant_priority(variant: &PlanVariant) -> usize {
    match variant {
        PlanVariant::Safest => 0,
        PlanVariant::Fastest => 1,
        PlanVariant::DegradedFallback => 2,
        PlanVariant::ClarificationFirst => 3,
    }
}
