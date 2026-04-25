// Layer ownership: surface/orchestration (non-canonical orchestration coordination only).
mod chain;
mod common;
mod ranking;
mod strategy;

use crate::contracts::{
    Capability, CapabilityProbeResult, CoreContractCall, OrchestrationPlanStep, PlanCandidate, PlanScore,
    PlanVariant, Precondition, RequestClassification, TypedOrchestrationRequest, WorkflowTemplate,
};

use super::{capability_registry, preconditions, scoring};
use chain::{chain_for_variant, maybe_prepend_context_preparation_step};
use ranking::{template_variant_bias, variant_priority};
use strategy::{
    ordered_capabilities_for_variant, strategy_capabilities_for_variant, strategy_family_for,
};

pub fn propose_decomposition_candidates(
    request: &TypedOrchestrationRequest,
    classification: &RequestClassification,
) -> Vec<PlanCandidate> {
    propose_decomposition_candidates_with_template(request, classification, None)
}

pub fn propose_decomposition_candidates_with_template(
    request: &TypedOrchestrationRequest,
    classification: &RequestClassification,
    template_hint: Option<&WorkflowTemplate>,
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
            template_hint,
        ),
        build_candidate_for_variant(
            request,
            classification,
            &capabilities,
            &probes,
            PlanVariant::Fastest,
            template_hint,
        ),
        build_candidate_for_variant(
            request,
            classification,
            &capabilities,
            &probes,
            PlanVariant::DegradedFallback,
            template_hint,
        ),
        build_candidate_for_variant(
            request,
            classification,
            &capabilities,
            &probes,
            PlanVariant::ClarificationFirst,
            template_hint,
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
            .then(left.contract_family.cmp(&right.contract_family))
            .then(left.capability_graph.len().cmp(&right.capability_graph.len()))
            .then(left.decomposition_family.cmp(&right.decomposition_family))
            .then(right.steps.len().cmp(&left.steps.len()))
    });
    candidates
}

pub fn propose_decomposition_candidate(
    request: &TypedOrchestrationRequest,
    classification: &RequestClassification,
) -> PlanCandidate {
    propose_decomposition_candidate_with_template(request, classification, None)
}

pub fn propose_decomposition_candidate_with_template(
    request: &TypedOrchestrationRequest,
    classification: &RequestClassification,
    template_hint: Option<&WorkflowTemplate>,
) -> PlanCandidate {
    propose_decomposition_candidates_with_template(request, classification, template_hint)
        .into_iter()
        .next()
        .unwrap_or_else(|| {
            empty_candidate(classification, Vec::new(), PlanVariant::ClarificationFirst)
        })
}

// Compatibility aliases for existing callers during control-plane naming transition.
pub fn build_plan_candidates(
    request: &TypedOrchestrationRequest,
    classification: &RequestClassification,
) -> Vec<PlanCandidate> {
    propose_decomposition_candidates_with_template(request, classification, None)
}

pub fn build_plan_candidate(
    request: &TypedOrchestrationRequest,
    classification: &RequestClassification,
) -> PlanCandidate {
    propose_decomposition_candidate_with_template(request, classification, None)
}

fn contract_family_for(contracts: &[CoreContractCall], capabilities: &[Capability]) -> String {
    let mut parts = Vec::new();
    if contracts
        .iter()
        .any(|row| matches!(row, CoreContractCall::ToolCapabilityProbe | CoreContractCall::ToolBrokerRequest))
        || capabilities.iter().any(Capability::is_tool_family)
    {
        parts.push("tool_route");
    }
    if contracts.iter().any(|row| {
        matches!(
            row,
            CoreContractCall::ContextTopologyInspect
                | CoreContractCall::ContextTopologyMaterialize
                | CoreContractCall::ContextAtomAppend
        )
    }) || capabilities.contains(&Capability::ReadMemory)
    {
        parts.push("context_topology");
    }
    if contracts
        .iter()
        .any(|row| matches!(row, CoreContractCall::UnifiedMemoryRead))
    {
        parts.push("memory_compatibility");
    }
    if contracts
        .iter()
        .any(|row| matches!(row, CoreContractCall::VerifierRequest))
        || capabilities.contains(&Capability::VerifyClaim)
    {
        parts.push("verification");
    }
    if contracts
        .iter()
        .any(|row| matches!(row, CoreContractCall::AssimilationPlanRequest))
        || capabilities.contains(&Capability::PlanAssimilation)
    {
        parts.push("assimilation");
    }
    if contracts
        .iter()
        .any(|row| matches!(row, CoreContractCall::TaskFabricProposal))
        || capabilities.contains(&Capability::MutateTask)
    {
        parts.push("task_fabric");
    }
    if parts.is_empty() {
        "empty_contract_graph".to_string()
    } else {
        parts.join("+")
    }
}

fn build_candidate_for_variant(
    request: &TypedOrchestrationRequest,
    classification: &RequestClassification,
    capabilities: &[Capability],
    probes: &[CapabilityProbeResult],
    variant: PlanVariant,
    template_hint: Option<&WorkflowTemplate>,
) -> PlanCandidate {
    let mut steps = Vec::new();
    let mut reasons = classification.reasons.clone();
    let mut variant_used_degraded = false;
    let strategy_family = strategy_family_for(request, classification, &variant);
    reasons.push(format!("strategy_family:{strategy_family:?}").to_lowercase());
    let strategy_capabilities = strategy_capabilities_for_variant(
        request,
        classification,
        capabilities,
        probes,
        &variant,
        &strategy_family,
    );
    reasons.push(format!(
        "strategy_capability_graph:{}",
        strategy_capabilities
            .iter()
            .map(|row| format!("{row:?}").to_lowercase())
            .collect::<Vec<_>>()
            .join(",")
    ));
    let strategy_probes = strategy_capabilities
        .iter()
        .map(|capability| probe_for_capability(capability, probes))
        .collect::<Vec<_>>();
    let ordered_capabilities = ordered_capabilities_for_variant(
        request,
        &strategy_capabilities,
        &variant,
        &strategy_family,
    );

    for capability in &ordered_capabilities {
        let probe = probe_for_capability(capability, strategy_probes.as_slice());
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

        let (mut chain, using_degraded, structurally_deferred) = chain_for_variant(
            request,
            capability,
            &probe,
            &variant,
            &strategy_family,
            &spec,
        );
        chain = maybe_prepend_context_preparation_step(
            request,
            classification,
            capability,
            &variant,
            &strategy_family,
            chain,
            structurally_deferred,
        );
        if structurally_deferred {
            reasons.push(format!("capability_structurally_deferred:{capability:?}").to_lowercase());
        }
        if using_degraded {
            reasons.push(format!("capability_degraded:{capability:?}").to_lowercase());
            variant_used_degraded = true;
        }
        for step in &mut chain {
            if blocked {
                step.blocked_on.extend(probe.blocked_on.clone());
                step.blocked_on.sort();
                step.blocked_on.dedup();
            }
            step.rationale
                .push(format!("variant:{variant:?}").to_lowercase());
            step.rationale
                .push(format!("strategy_family:{strategy_family:?}").to_lowercase());
            step.rationale.extend(probe.probe_sources.iter().cloned());
            step.rationale.sort();
            step.rationale.dedup();
        }
        steps.extend(chain);
    }

    dedupe_steps(&mut steps);

    let blocked_on = preconditions::blocked_preconditions(strategy_probes.as_slice());
    let degradation = preconditions::degradation_reasons(strategy_probes.as_slice());
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
        || matches!(variant, PlanVariant::ClarificationFirst) && !blocked_on.is_empty();
    let contracts = steps
        .iter()
        .map(|row| row.target_contract.clone())
        .collect::<Vec<_>>();
    let decomposition_family = format!("decomposition_{strategy_family:?}_{variant:?}")
        .to_lowercase();
    let capability_graph = strategy_capabilities.clone();
    let contract_family = contract_family_for(contracts.as_slice(), capability_graph.as_slice());
    let mut score = scoring::score_candidate(
        request,
        classification,
        contracts.as_slice(),
        blocked_on.len(),
        degradation.len() + usize::from(variant_used_degraded),
        requires_clarification,
    );
    let template_bias = template_variant_bias(template_hint, &variant);
    if template_bias != 0.0 {
        score.overall = (score.overall + template_bias).clamp(0.0, 0.99);
        reasons
            .push(format!("workflow_template_bias:{variant:?}:{template_bias:+.2}").to_lowercase());
    }
    if steps.is_empty() {
        reasons.push("candidate_empty_after_capability_resolution".to_string());
    }
    let mutates_session_context = steps
        .iter()
        .any(|step| step.target_contract == crate::contracts::CoreContractCall::ContextAtomAppend);
    let context_preparation_rationale = mutates_session_context.then(|| {
        "explicit_context_preparation_pre_step:selected_by_planner_rationale".to_string()
    });

    PlanCandidate {
        plan_id: format!(
            "plan_{:?}_{:?}_{:?}_{:?}",
            classification.request_class, request.operation_kind, request.resource_kind, variant
        )
        .to_lowercase(),
        variant,
        steps,
        mutates_session_context,
        context_preparation_rationale,
        decomposition_family,
        capability_graph,
        contract_family,
        confidence: score.overall,
        score,
        requires_clarification,
        blocked_on,
        degradation,
        capabilities: strategy_capabilities,
        capability_probes: strategy_probes,
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
        mutates_session_context: false,
        context_preparation_rationale: None,
        decomposition_family: "empty".to_string(),
        capability_graph: classification.required_capabilities.clone(),
        contract_family: "empty_contract_graph".to_string(),
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
            existing
                .expected_contract_refs
                .extend(step.expected_contract_refs);
            existing.expected_contract_refs.sort();
            existing.expected_contract_refs.dedup();
        } else {
            indices.insert(key, merged.len());
            merged.push(step);
        }
    }
    *steps = merged;
}

fn probe_for_capability(
    capability: &Capability,
    probes: &[CapabilityProbeResult],
) -> CapabilityProbeResult {
    probes
        .iter()
        .find(|row| row.capability == *capability)
        .cloned()
        .unwrap_or_else(|| CapabilityProbeResult {
            capability: capability.clone(),
            blocked_on: Vec::new(),
            degradation_reasons: Vec::new(),
            can_degrade: false,
            probe_sources: vec!["probe.missing".to_string()],
        })
}
