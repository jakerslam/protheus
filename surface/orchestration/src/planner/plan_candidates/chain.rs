// Layer ownership: surface/orchestration (non-canonical orchestration coordination only).
use crate::contracts::{
    Capability, CapabilityProbeResult, CoreContractCall, OperationKind, OrchestrationPlanStep,
    PlanVariant, RequestClassification, RequestKind, RequestSurface, TypedOrchestrationRequest,
};

use super::{
    capability_registry,
    common::{
        filter_steps_by_contract, is_structural_comparative_request,
        transport_explicitly_unavailable,
    },
    strategy::StrategyFamily,
};

pub(super) fn maybe_prepend_context_preparation_step(
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

pub(super) fn chain_for_variant(
    request: &TypedOrchestrationRequest,
    capability: &Capability,
    probe: &CapabilityProbeResult,
    variant: &PlanVariant,
    strategy_family: &StrategyFamily,
    spec: &capability_registry::CapabilitySpec,
) -> (Vec<OrchestrationPlanStep>, bool, bool) {
    match strategy_family {
        StrategyFamily::MemoryFirst => {
            if matches!(capability, Capability::ReadMemory) {
                return (spec.primary_steps.clone(), false, false);
            }
            if (capability.is_tool_family() || matches!(capability, Capability::VerifyClaim))
                && !spec.degraded_steps.is_empty()
                && (!probe.blocked_on.is_empty() || transport_explicitly_unavailable(request))
            {
                return (spec.degraded_steps.clone(), true, false);
            }
        }
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
                && capability.is_tool_family()
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
                _ if (capability.is_tool_family()
                    || matches!(capability, Capability::VerifyClaim))
                    && !probe.blocked_on.is_empty()
                    && !spec.degraded_steps.is_empty() =>
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
            _ if (capability.is_tool_family() || matches!(capability, Capability::VerifyClaim))
                && !spec.degraded_steps.is_empty() =>
            {
                if !matches!(request.surface, RequestSurface::Legacy) {
                    return (spec.degraded_steps.clone(), true, false);
                }
            }
            _ => {}
        },
        PlanVariant::ClarificationFirst => {
            if capability.is_tool_family()
                || matches!(
                    capability,
                    Capability::MutateTask | Capability::PlanAssimilation
                )
            {
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
        OperationKind::Plan
            | OperationKind::Assimilate
            | OperationKind::Mutate
            | OperationKind::Compare
    ) {
        return true;
    }
    matches!(
        request.request_kind,
        RequestKind::Workflow | RequestKind::Comparative
    ) || matches!(request.surface, RequestSurface::Legacy)
}
