// Layer ownership: surface/orchestration (non-canonical orchestration coordination only).
use crate::contracts::{
    Capability, CoreContractCall, OrchestrationPlanStep, RequestKind, ResourceKind,
    TypedOrchestrationRequest,
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
    let Some(envelope) = request.core_probe_envelope.as_ref() else {
        return false;
    };
    let primary_transport_capability =
        Capability::primary_tool_for(&request.operation_kind, &request.resource_kind);
    envelope.probes.iter().any(|row| {
        matches!(
            row.capability,
            Capability::WorkspaceRead
                | Capability::WorkspaceSearch
                | Capability::WebSearch
                | Capability::WebFetch
                | Capability::ToolRoute
                | Capability::ExecuteTool
                | Capability::VerifyClaim
        ) && (row.capability == primary_transport_capability
            || row.capability == Capability::VerifyClaim)
            && row.transport_available == Some(false)
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::contracts::{
        CapabilityProbeSnapshot, CoreProbeEnvelope, Mutability, OperationKind, PolicyScope,
        RequestKind, RequestSurface, ResourceKind, TypedOrchestrationRequest,
    };
    use serde_json::json;

    fn non_legacy_workspace_search_request(
        payload_transport_available: bool,
        envelope_transport_available: bool,
    ) -> TypedOrchestrationRequest {
        TypedOrchestrationRequest {
            session_id: "s".to_string(),
            surface: RequestSurface::Sdk,
            legacy_intent: "search workspace".to_string(),
            adapted: true,
            payload: json!({
                "transport_available": payload_transport_available
            }),
            request_kind: RequestKind::Comparative,
            operation_kind: OperationKind::Search,
            resource_kind: ResourceKind::Workspace,
            mutability: Mutability::ReadOnly,
            target_descriptors: Vec::new(),
            target_refs: Vec::new(),
            tool_hints: vec!["workspace_search".to_string()],
            policy_scope: PolicyScope::WorkspaceOnly,
            user_constraints: Vec::new(),
            core_probe_envelope: Some(CoreProbeEnvelope {
                probes: vec![CapabilityProbeSnapshot {
                    capability: Capability::WorkspaceSearch,
                    tool_available: Some(true),
                    target_supplied: Some(true),
                    target_syntactically_valid: Some(true),
                    target_exists: Some(true),
                    authorization_valid: Some(true),
                    policy_allows: Some(true),
                    transport_available: Some(envelope_transport_available),
                }],
            }),
        }
    }

    #[test]
    fn non_legacy_transport_unavailable_ignores_raw_payload_shortcut() {
        let request = non_legacy_workspace_search_request(false, true);

        assert!(!transport_explicitly_unavailable(&request));
    }

    #[test]
    fn non_legacy_transport_unavailable_uses_core_probe_envelope() {
        let request = non_legacy_workspace_search_request(true, false);

        assert!(transport_explicitly_unavailable(&request));
    }
}
