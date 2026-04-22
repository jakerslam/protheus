// Layer ownership: surface/orchestration (non-canonical orchestration coordination only).
use crate::contracts::{
    Capability, CapabilityProbeResult, PlanVariant, RequestClass, RequestClassification,
    ResourceKind, TypedOrchestrationRequest,
};

use super::common::{is_structural_comparative_request, transport_explicitly_unavailable};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum StrategyFamily {
    Balanced,
    ToolFirst,
    TopologyFirst,
    MemoryFirst,
}

pub(super) fn strategy_capabilities_for_variant(
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
                if let Some(tool_capability) = selected_tool_capability(capabilities, request) {
                    out.push(tool_capability);
                }
                if has_capability(capabilities, Capability::VerifyClaim) {
                    out.push(Capability::VerifyClaim);
                }
                if has_capability(capabilities, Capability::ReadMemory)
                    && (transport_explicitly_unavailable(request)
                        || tool_capability_blocked(capabilities, request, probes)
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
                if has_tool_capability(capabilities, request)
                    && !tool_capability_blocked(capabilities, request, probes)
                    && !transport_explicitly_unavailable(request)
                {
                    if let Some(tool_capability) = selected_tool_capability(capabilities, request) {
                        out.push(tool_capability);
                    }
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
                if let Some(tool_capability) = selected_tool_capability(capabilities, request) {
                    out.push(tool_capability);
                }
                if has_capability(capabilities, Capability::VerifyClaim) {
                    out.push(Capability::VerifyClaim);
                }
                if has_capability(capabilities, Capability::ReadMemory)
                    && (out.is_empty()
                        || tool_capability_blocked(capabilities, request, probes)
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
                if has_tool_capability(capabilities, request) && out.is_empty() {
                    if let Some(tool_capability) = selected_tool_capability(capabilities, request) {
                        out.push(tool_capability);
                    }
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
                if has_tool_capability(capabilities, request)
                    && !tool_capability_blocked(capabilities, request, probes)
                    && !transport_explicitly_unavailable(request)
                    && out.is_empty()
                {
                    if let Some(tool_capability) = selected_tool_capability(capabilities, request) {
                        out.push(tool_capability);
                    }
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
        && selected.iter().any(Capability::is_tool_family)
    {
        selected.retain(|row| !row.is_tool_family());
    }

    selected.retain(|row| capabilities.iter().any(|capability| capability == row));
    selected.sort_by_key(|row| format!("{row:?}"));
    selected.dedup();
    if selected.is_empty() {
        return capabilities.to_vec();
    }
    selected
}

pub(super) fn ordered_capabilities_for_variant(
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

pub(super) fn strategy_family_for(
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
                    ResourceKind::Web
                        | ResourceKind::Workspace
                        | ResourceKind::Tooling
                        | ResourceKind::Mixed
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
                    ResourceKind::Web
                        | ResourceKind::Workspace
                        | ResourceKind::Tooling
                        | ResourceKind::Mixed
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

fn has_capability(capabilities: &[Capability], capability: Capability) -> bool {
    capabilities.iter().any(|row| row == &capability)
}

fn selected_tool_capability(
    capabilities: &[Capability],
    request: &TypedOrchestrationRequest,
) -> Option<Capability> {
    let preferred = Capability::primary_tool_for(&request.operation_kind, &request.resource_kind);
    if capabilities.iter().any(|row| row == &preferred) {
        return Some(preferred);
    }
    capabilities
        .iter()
        .find(|row| row.is_tool_family())
        .cloned()
}

fn has_tool_capability(capabilities: &[Capability], request: &TypedOrchestrationRequest) -> bool {
    selected_tool_capability(capabilities, request).is_some()
}

fn tool_capability_blocked(
    capabilities: &[Capability],
    request: &TypedOrchestrationRequest,
    probes: &[CapabilityProbeResult],
) -> bool {
    selected_tool_capability(capabilities, request)
        .map(|capability| capability_blocked(capability, probes))
        .unwrap_or(false)
}

fn capability_blocked(capability: Capability, probes: &[CapabilityProbeResult]) -> bool {
    probes
        .iter()
        .find(|row| row.capability == capability)
        .map(|row| !row.blocked_on.is_empty())
        .unwrap_or(false)
}

fn capability_priority(
    strategy_family: &StrategyFamily,
    comparative: bool,
    variant: &PlanVariant,
    capability: &Capability,
) -> usize {
    let is_tool = capability.is_tool_family();
    match strategy_family {
        StrategyFamily::ToolFirst => {
            if is_tool {
                0
            } else {
                match capability {
                    Capability::VerifyClaim => 1,
                    Capability::ReadMemory => 2,
                    Capability::PlanAssimilation => 3,
                    Capability::MutateTask => 4,
                    _ => 5,
                }
            }
        }
        StrategyFamily::TopologyFirst => {
            if is_tool {
                2
            } else {
                match capability {
                    Capability::ReadMemory => 0,
                    Capability::VerifyClaim => 1,
                    Capability::PlanAssimilation => 3,
                    Capability::MutateTask => 4,
                    _ => 5,
                }
            }
        }
        StrategyFamily::MemoryFirst => {
            if is_tool {
                2
            } else {
                match capability {
                    Capability::ReadMemory => 0,
                    Capability::VerifyClaim => 1,
                    Capability::PlanAssimilation => 3,
                    Capability::MutateTask => 4,
                    _ => 5,
                }
            }
        }
        StrategyFamily::Balanced => {
            if comparative {
                match variant {
                    PlanVariant::Safest => match capability {
                        Capability::ReadMemory => 0,
                        Capability::VerifyClaim => 2,
                        Capability::PlanAssimilation => 3,
                        Capability::MutateTask => 4,
                        _ if is_tool => 1,
                        _ => 5,
                    },
                    PlanVariant::Fastest => match capability {
                        Capability::VerifyClaim => 1,
                        Capability::ReadMemory => 2,
                        Capability::PlanAssimilation => 3,
                        Capability::MutateTask => 4,
                        _ if is_tool => 0,
                        _ => 5,
                    },
                    PlanVariant::DegradedFallback => match capability {
                        Capability::ReadMemory => 0,
                        Capability::VerifyClaim => 1,
                        Capability::PlanAssimilation => 3,
                        Capability::MutateTask => 4,
                        _ if is_tool => 2,
                        _ => 5,
                    },
                    PlanVariant::ClarificationFirst => match capability {
                        Capability::ReadMemory => 0,
                        Capability::VerifyClaim => 2,
                        Capability::PlanAssimilation => 3,
                        Capability::MutateTask => 4,
                        _ if is_tool => 1,
                        _ => 5,
                    },
                }
            } else {
                match capability {
                    Capability::ReadMemory => 0,
                    Capability::VerifyClaim => 2,
                    Capability::PlanAssimilation => 3,
                    Capability::MutateTask => 4,
                    _ if is_tool => 1,
                    _ => 5,
                }
            }
        }
    }
}
