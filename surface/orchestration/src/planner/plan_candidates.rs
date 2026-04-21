// Layer ownership: surface/orchestration (non-canonical orchestration coordination only).
use crate::contracts::{
    Capability, CapabilityProbeResult, CoreContractCall, OrchestrationPlanStep, PlanCandidate,
    PlanScore, PlanVariant, Precondition, RequestClass, RequestClassification, RequestKind,
    ResourceKind, TypedOrchestrationRequest,
};

use super::{capability_registry, preconditions, scoring};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum StrategyFamily {
    Balanced,
    ToolFirst,
    TopologyFirst,
    MemoryFirst,
}

pub fn propose_decomposition_candidates(
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

pub fn propose_decomposition_candidate(
    request: &TypedOrchestrationRequest,
    classification: &RequestClassification,
) -> PlanCandidate {
    propose_decomposition_candidates(request, classification)
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
    propose_decomposition_candidates(request, classification)
}

pub fn build_plan_candidate(
    request: &TypedOrchestrationRequest,
    classification: &RequestClassification,
) -> PlanCandidate {
    propose_decomposition_candidate(request, classification)
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
    let score = scoring::score_candidate(
        request,
        classification,
        contracts.as_slice(),
        blocked_on.len(),
        degradation.len() + usize::from(variant_used_degraded),
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

fn has_capability(capabilities: &[Capability], capability: Capability) -> bool {
    capabilities.iter().any(|row| row == &capability)
}

fn capability_blocked(capability: Capability, probes: &[CapabilityProbeResult]) -> bool {
    probes
        .iter()
        .find(|row| row.capability == capability)
        .map(|row| !row.blocked_on.is_empty())
        .unwrap_or(false)
}

fn strategy_capabilities_for_variant(
    request: &TypedOrchestrationRequest,
    classification: &RequestClassification,
    capabilities: &[Capability],
    probes: &[CapabilityProbeResult],
    variant: &PlanVariant,
    strategy_family: &StrategyFamily,
) -> Vec<Capability> {
    let comparative = is_structural_comparative_request(request);
    let mut selected = if comparative {
        match strategy_family {
            StrategyFamily::ToolFirst => {
                let mut out = Vec::new();
                if has_capability(capabilities, Capability::ExecuteTool) {
                    out.push(Capability::ExecuteTool);
                }
                if has_capability(capabilities, Capability::VerifyClaim) {
                    out.push(Capability::VerifyClaim);
                }
                if has_capability(capabilities, Capability::ReadMemory)
                    && (transport_explicitly_unavailable(request)
                        || capability_blocked(Capability::ExecuteTool, probes)
                        || capability_blocked(Capability::VerifyClaim, probes))
                {
                    out.push(Capability::ReadMemory);
                }
                out
            }
            StrategyFamily::TopologyFirst => {
                let mut out = Vec::new();
                if has_capability(capabilities, Capability::ReadMemory) {
                    out.push(Capability::ReadMemory);
                }
                if has_capability(capabilities, Capability::VerifyClaim) {
                    out.push(Capability::VerifyClaim);
                }
                out
            }
            StrategyFamily::MemoryFirst => {
                let mut out = Vec::new();
                if has_capability(capabilities, Capability::ReadMemory) {
                    out.push(Capability::ReadMemory);
                }
                if has_capability(capabilities, Capability::ExecuteTool)
                    && !capability_blocked(Capability::ExecuteTool, probes)
                    && !transport_explicitly_unavailable(request)
                {
                    out.push(Capability::ExecuteTool);
                }
                if has_capability(capabilities, Capability::VerifyClaim)
                    && !capability_blocked(Capability::VerifyClaim, probes)
                    && !transport_explicitly_unavailable(request)
                {
                    out.push(Capability::VerifyClaim);
                }
                out
            }
            StrategyFamily::Balanced => capabilities.to_vec(),
        }
    } else {
        match strategy_family {
            StrategyFamily::ToolFirst => {
                let mut out = Vec::new();
                if has_capability(capabilities, Capability::ExecuteTool) {
                    out.push(Capability::ExecuteTool);
                }
                if has_capability(capabilities, Capability::VerifyClaim) {
                    out.push(Capability::VerifyClaim);
                }
                if has_capability(capabilities, Capability::ReadMemory)
                    && (out.is_empty()
                        || capability_blocked(Capability::ExecuteTool, probes)
                        || capability_blocked(Capability::VerifyClaim, probes))
                {
                    out.push(Capability::ReadMemory);
                }
                out
            }
            StrategyFamily::TopologyFirst => {
                let mut out = Vec::new();
                if has_capability(capabilities, Capability::ReadMemory) {
                    out.push(Capability::ReadMemory);
                }
                if has_capability(capabilities, Capability::VerifyClaim) {
                    out.push(Capability::VerifyClaim);
                }
                if has_capability(capabilities, Capability::ExecuteTool) && out.is_empty() {
                    out.push(Capability::ExecuteTool);
                }
                out
            }
            StrategyFamily::MemoryFirst => {
                let mut out = Vec::new();
                if has_capability(capabilities, Capability::ReadMemory) {
                    out.push(Capability::ReadMemory);
                }
                if has_capability(capabilities, Capability::MutateTask)
                    && matches!(
                        classification.request_class,
                        RequestClass::TaskProposal | RequestClass::Mutation
                    )
                {
                    out.push(Capability::MutateTask);
                }
                if has_capability(capabilities, Capability::PlanAssimilation)
                    && classification.request_class == RequestClass::Assimilation
                {
                    out.push(Capability::PlanAssimilation);
                }
                if has_capability(capabilities, Capability::VerifyClaim)
                    && !capability_blocked(Capability::VerifyClaim, probes)
                {
                    out.push(Capability::VerifyClaim);
                }
                if has_capability(capabilities, Capability::ExecuteTool)
                    && !capability_blocked(Capability::ExecuteTool, probes)
                    && !transport_explicitly_unavailable(request)
                    && out.is_empty()
                {
                    out.push(Capability::ExecuteTool);
                }
                out
            }
            StrategyFamily::Balanced => capabilities.to_vec(),
        }
    };

    match classification.request_class {
        RequestClass::Assimilation => {
            if has_capability(capabilities, Capability::PlanAssimilation)
                && !has_capability(&selected, Capability::PlanAssimilation)
            {
                selected.push(Capability::PlanAssimilation);
            }
            if has_capability(capabilities, Capability::MutateTask)
                && !has_capability(&selected, Capability::MutateTask)
            {
                selected.push(Capability::MutateTask);
            }
        }
        RequestClass::TaskProposal | RequestClass::Mutation => {
            if has_capability(capabilities, Capability::MutateTask)
                && !has_capability(&selected, Capability::MutateTask)
            {
                selected.push(Capability::MutateTask);
            }
        }
        RequestClass::ReadOnly | RequestClass::ToolCall => {}
    }

    if matches!(variant, PlanVariant::ClarificationFirst)
        && has_capability(&selected, Capability::ExecuteTool)
    {
        selected.retain(|row| row != &Capability::ExecuteTool);
    }

    selected.retain(|row| capabilities.iter().any(|capability| capability == row));
    selected.sort_by_key(|row| format!("{row:?}"));
    selected.dedup();
    if selected.is_empty() {
        return capabilities.to_vec();
    }
    selected
}

fn should_prepare_session_context(
    request: &TypedOrchestrationRequest,
    classification: &RequestClassification,
    variant: &PlanVariant,
    strategy_family: &StrategyFamily,
) -> bool {
    if matches!(variant, PlanVariant::Fastest) {
        return false;
    }
    if matches!(strategy_family, StrategyFamily::ToolFirst) {
        return false;
    }
    if classification.needs_clarification {
        return true;
    }
    if !request.user_constraints.is_empty() {
        return true;
    }
    if matches!(
        request.operation_kind,
        crate::contracts::OperationKind::Plan
            | crate::contracts::OperationKind::Assimilate
            | crate::contracts::OperationKind::Mutate
            | crate::contracts::OperationKind::Compare
    ) {
        return true;
    }
    matches!(
        request.request_kind,
        RequestKind::Workflow | RequestKind::Comparative
    ) || matches!(request.surface, crate::contracts::RequestSurface::Legacy)
}

fn maybe_prepend_context_preparation_step(
    request: &TypedOrchestrationRequest,
    classification: &RequestClassification,
    capability: &Capability,
    variant: &PlanVariant,
    strategy_family: &StrategyFamily,
    mut chain: Vec<OrchestrationPlanStep>,
    structurally_deferred: bool,
) -> Vec<OrchestrationPlanStep> {
    if structurally_deferred
        || chain.is_empty()
        || !matches!(capability, Capability::ReadMemory)
        || !should_prepare_session_context(request, classification, variant, strategy_family)
    {
        return chain;
    }
    if chain
        .iter()
        .any(|row| row.target_contract == CoreContractCall::ContextAtomAppend)
    {
        return chain;
    }
    let mut prefixed = vec![capability_registry::context_preparation_step()];
    prefixed.append(&mut chain);
    prefixed
}

fn ordered_capabilities_for_variant(
    request: &TypedOrchestrationRequest,
    capabilities: &[Capability],
    variant: &PlanVariant,
    strategy_family: &StrategyFamily,
) -> Vec<Capability> {
    let comparative = is_structural_comparative_request(request);
    let mut out = capabilities.to_vec();
    out.sort_by_key(|capability| {
        capability_priority(strategy_family, comparative, variant, capability)
    });
    out
}

fn chain_for_variant(
    request: &TypedOrchestrationRequest,
    capability: &Capability,
    probe: &CapabilityProbeResult,
    variant: &PlanVariant,
    strategy_family: &StrategyFamily,
    spec: &capability_registry::CapabilitySpec,
) -> (Vec<OrchestrationPlanStep>, bool, bool) {
    match strategy_family {
        StrategyFamily::MemoryFirst => match capability {
            Capability::ReadMemory => return (spec.primary_steps.clone(), false, false),
            Capability::ExecuteTool | Capability::VerifyClaim
                if !spec.degraded_steps.is_empty()
                    && (!probe.blocked_on.is_empty()
                        || transport_explicitly_unavailable(request)) =>
            {
                return (spec.degraded_steps.clone(), true, false);
            }
            _ => {}
        },
        StrategyFamily::TopologyFirst => {
            if is_structural_comparative_request(request)
                && matches!(capability, Capability::ReadMemory)
                && matches!(
                    variant,
                    PlanVariant::ClarificationFirst | PlanVariant::Fastest
                )
            {
                return (
                    filter_steps_by_contract(
                        spec.primary_steps.as_slice(),
                        &[CoreContractCall::ContextTopologyInspect],
                    ),
                    false,
                    false,
                );
            }
            if is_structural_comparative_request(request)
                && matches!(capability, Capability::ExecuteTool)
                && matches!(variant, PlanVariant::ClarificationFirst)
                && !transport_explicitly_unavailable(request)
            {
                return (Vec::new(), false, true);
            }
        }
        StrategyFamily::ToolFirst => {
            if is_structural_comparative_request(request)
                && matches!(capability, Capability::ReadMemory)
                && !transport_explicitly_unavailable(request)
            {
                return (Vec::new(), false, true);
            }
        }
        StrategyFamily::Balanced => {}
    }

    if is_structural_comparative_request(request) {
        match variant {
            PlanVariant::Fastest => match capability {
                Capability::ReadMemory => {
                    if transport_explicitly_unavailable(request) {
                        return (spec.primary_steps.clone(), false, false);
                    }
                    return (Vec::new(), false, true);
                }
                Capability::VerifyClaim if !probe.blocked_on.is_empty() && probe.can_degrade => {
                    return (spec.degraded_steps.clone(), true, false);
                }
                Capability::VerifyClaim => {
                    return (
                        filter_steps_by_contract(
                            spec.primary_steps.as_slice(),
                            &[CoreContractCall::VerifierRequest],
                        ),
                        false,
                        false,
                    );
                }
                _ => {}
            },
            PlanVariant::DegradedFallback => match capability {
                Capability::ReadMemory => return (spec.primary_steps.clone(), false, false),
                Capability::ExecuteTool | Capability::VerifyClaim
                    if !probe.blocked_on.is_empty() && !spec.degraded_steps.is_empty() =>
                {
                    return (spec.degraded_steps.clone(), true, false);
                }
                _ => {}
            },
            PlanVariant::Safest | PlanVariant::ClarificationFirst => {}
        }
    }

    match variant {
        PlanVariant::Fastest => match capability {
            Capability::ReadMemory => {
                return (
                    filter_steps_by_contract(
                        spec.primary_steps.as_slice(),
                        &[CoreContractCall::ContextTopologyInspect],
                    ),
                    false,
                    false,
                );
            }
            Capability::VerifyClaim => {
                return (
                    filter_steps_by_contract(
                        spec.primary_steps.as_slice(),
                        &[CoreContractCall::VerifierRequest],
                    ),
                    false,
                    false,
                );
            }
            _ => {}
        },
        PlanVariant::DegradedFallback => match capability {
            Capability::ExecuteTool | Capability::VerifyClaim
                if !spec.degraded_steps.is_empty() =>
            {
                if !matches!(request.surface, crate::contracts::RequestSurface::Legacy) {
                    return (spec.degraded_steps.clone(), true, false);
                }
            }
            _ => {}
        },
        PlanVariant::ClarificationFirst => {
            if matches!(
                capability,
                Capability::ExecuteTool | Capability::MutateTask | Capability::PlanAssimilation
            ) {
                return (Vec::new(), false, true);
            }
        }
        PlanVariant::Safest => {}
    }

    let using_degraded = !probe.blocked_on.is_empty()
        && probe.can_degrade
        && !spec.degraded_steps.is_empty()
        && matches!(
            variant,
            PlanVariant::Fastest | PlanVariant::DegradedFallback
        );
    let chain = if using_degraded {
        spec.degraded_steps.clone()
    } else {
        spec.primary_steps.clone()
    };
    (chain, using_degraded, false)
}

fn strategy_family_for(
    request: &TypedOrchestrationRequest,
    classification: &RequestClassification,
    variant: &PlanVariant,
) -> StrategyFamily {
    let comparative = is_structural_comparative_request(request);
    match variant {
        PlanVariant::Fastest => {
            if comparative
                || matches!(
                    classification.request_class,
                    RequestClass::ToolCall | RequestClass::Assimilation
                )
                || matches!(
                    request.resource_kind,
                    ResourceKind::Web | ResourceKind::Tooling | ResourceKind::Mixed
                )
            {
                StrategyFamily::ToolFirst
            } else {
                StrategyFamily::Balanced
            }
        }
        PlanVariant::Safest => {
            if !comparative
                && matches!(
                    classification.request_class,
                    RequestClass::ReadOnly | RequestClass::ToolCall
                )
            {
                StrategyFamily::TopologyFirst
            } else {
                StrategyFamily::Balanced
            }
        }
        PlanVariant::DegradedFallback => {
            if comparative
                || matches!(
                    request.resource_kind,
                    ResourceKind::Web | ResourceKind::Tooling | ResourceKind::Mixed
                )
            {
                StrategyFamily::MemoryFirst
            } else if matches!(
                classification.request_class,
                RequestClass::ReadOnly | RequestClass::ToolCall
            ) {
                StrategyFamily::TopologyFirst
            } else {
                StrategyFamily::Balanced
            }
        }
        PlanVariant::ClarificationFirst => {
            if comparative
                || matches!(
                    classification.request_class,
                    RequestClass::ReadOnly | RequestClass::ToolCall
                )
            {
                StrategyFamily::TopologyFirst
            } else {
                StrategyFamily::Balanced
            }
        }
    }
}

fn capability_priority(
    strategy_family: &StrategyFamily,
    comparative: bool,
    variant: &PlanVariant,
    capability: &Capability,
) -> usize {
    match strategy_family {
        StrategyFamily::ToolFirst => match capability {
            Capability::ExecuteTool => 0,
            Capability::VerifyClaim => 1,
            Capability::ReadMemory => 2,
            Capability::PlanAssimilation => 3,
            Capability::MutateTask => 4,
        },
        StrategyFamily::TopologyFirst => match capability {
            Capability::ReadMemory => 0,
            Capability::VerifyClaim => 1,
            Capability::ExecuteTool => 2,
            Capability::PlanAssimilation => 3,
            Capability::MutateTask => 4,
        },
        StrategyFamily::MemoryFirst => match capability {
            Capability::ReadMemory => 0,
            Capability::VerifyClaim => 1,
            Capability::ExecuteTool => 2,
            Capability::PlanAssimilation => 3,
            Capability::MutateTask => 4,
        },
        StrategyFamily::Balanced => {
            if comparative {
                match variant {
                    PlanVariant::Safest => match capability {
                        Capability::ReadMemory => 0,
                        Capability::ExecuteTool => 1,
                        Capability::VerifyClaim => 2,
                        Capability::PlanAssimilation => 3,
                        Capability::MutateTask => 4,
                    },
                    PlanVariant::Fastest => match capability {
                        Capability::ExecuteTool => 0,
                        Capability::VerifyClaim => 1,
                        Capability::ReadMemory => 2,
                        Capability::PlanAssimilation => 3,
                        Capability::MutateTask => 4,
                    },
                    PlanVariant::DegradedFallback => match capability {
                        Capability::ReadMemory => 0,
                        Capability::VerifyClaim => 1,
                        Capability::ExecuteTool => 2,
                        Capability::PlanAssimilation => 3,
                        Capability::MutateTask => 4,
                    },
                    PlanVariant::ClarificationFirst => match capability {
                        Capability::ReadMemory => 0,
                        Capability::ExecuteTool => 1,
                        Capability::VerifyClaim => 2,
                        Capability::PlanAssimilation => 3,
                        Capability::MutateTask => 4,
                    },
                }
            } else {
                match capability {
                    Capability::ReadMemory => 0,
                    Capability::ExecuteTool => 1,
                    Capability::VerifyClaim => 2,
                    Capability::PlanAssimilation => 3,
                    Capability::MutateTask => 4,
                }
            }
        }
    }
}

fn filter_steps_by_contract(
    steps: &[OrchestrationPlanStep],
    allowed: &[CoreContractCall],
) -> Vec<OrchestrationPlanStep> {
    steps
        .iter()
        .filter(|step| allowed.contains(&step.target_contract))
        .cloned()
        .collect()
}

fn is_structural_comparative_request(request: &TypedOrchestrationRequest) -> bool {
    request.request_kind == RequestKind::Comparative || request.resource_kind == ResourceKind::Mixed
}

fn transport_explicitly_unavailable(request: &TypedOrchestrationRequest) -> bool {
    request
        .payload
        .get("transport_available")
        .and_then(|row| row.as_bool())
        == Some(false)
}

fn variant_priority(variant: &PlanVariant) -> usize {
    match variant {
        PlanVariant::Safest => 0,
        PlanVariant::Fastest => 1,
        PlanVariant::DegradedFallback => 2,
        PlanVariant::ClarificationFirst => 3,
    }
}
