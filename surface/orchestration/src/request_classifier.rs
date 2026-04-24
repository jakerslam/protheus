// Layer ownership: surface/orchestration (non-canonical orchestration coordination only).
use crate::contracts::{
    AmbiguityReason, Capability, ClarificationReason, Mutability, OperationKind, ParseResult,
    RequestClass, RequestClassification, RequestKind, RequestSurface, ResourceKind,
    TypedOrchestrationRequest,
};

pub fn classify_request(parsed: &ParseResult) -> RequestClassification {
    let request = &parsed.typed_request;
    let request_class = match (
        &request.operation_kind,
        &request.mutability,
        &request.resource_kind,
    ) {
        (_, _, ResourceKind::Tooling) => RequestClass::ToolCall,
        (OperationKind::Search | OperationKind::Fetch | OperationKind::InspectTooling, _, _) => {
            RequestClass::ToolCall
        }
        (OperationKind::Assimilate, _, _) => RequestClass::Assimilation,
        (OperationKind::Plan, _, _) => RequestClass::TaskProposal,
        (_, Mutability::Mutation, _) => RequestClass::Mutation,
        (OperationKind::Compare, _, ResourceKind::Web) => RequestClass::ToolCall,
        _ => RequestClass::ReadOnly,
    };

    let required_capabilities = required_capabilities_for_typed_request(request);

    let mut clarification_reasons = Vec::new();
    if request.session_id.is_empty() {
        clarification_reasons.push(ClarificationReason::MissingSessionId);
    }
    if should_add_ambiguous_operation_clarification(parsed) {
        clarification_reasons.push(ClarificationReason::AmbiguousOperation);
    }
    if has_typed_probe_contract_violation(parsed) {
        clarification_reasons.push(ClarificationReason::TypedProbeContractViolation);
    }
    if request.operation_kind == OperationKind::Assimilate && request.target_refs.is_empty() {
        clarification_reasons.push(ClarificationReason::MissingTargetRefs);
    }
    if request.mutability == Mutability::Mutation && request.target_refs.is_empty() {
        clarification_reasons.push(ClarificationReason::MutationScopeRequired);
    }

    let mut reasons = parsed.reasons.clone();
    reasons.push(format!("classified_as:{request_class:?}").to_lowercase());
    reasons.push(format!("policy_scope:{:?}", request.policy_scope).to_lowercase());
    if parsed
        .ambiguity
        .iter()
        .any(|row| matches!(row, AmbiguityReason::LowConfidence))
    {
        reasons.push("parse_confidence_below_threshold".to_string());
    }
    if !request.tool_hints.is_empty() {
        reasons.push(format!("tool_hints:{}", request.tool_hints.join(",")));
    }
    if !required_capabilities.is_empty() {
        reasons.push(format!(
            "capabilities:{}",
            required_capabilities
                .iter()
                .map(|row| format!("{row:?}").to_lowercase())
                .collect::<Vec<_>>()
                .join(",")
        ));
    }

    let confidence = if clarification_reasons.is_empty() {
        parsed.confidence
    } else {
        (parsed.confidence - 0.15).clamp(0.0, 0.95)
    };

    RequestClassification {
        request_class,
        confidence,
        reasons,
        required_capabilities,
        clarification_reasons: clarification_reasons.clone(),
        needs_clarification: !clarification_reasons.is_empty(),
        surface_adapter_used: parsed.surface_adapter_used,
        surface_adapter_fallback: parsed.surface_adapter_fallback,
    }
}

fn required_capabilities_for(
    request_class: RequestClass,
    request_kind: RequestKind,
    operation_kind: OperationKind,
    resource_kind: ResourceKind,
) -> Vec<Capability> {
    let mut required_capabilities = match request_class {
        RequestClass::ToolCall => {
            vec![Capability::primary_tool_for(
                &operation_kind,
                &resource_kind,
            )]
        }
        RequestClass::Assimilation => vec![Capability::PlanAssimilation, Capability::MutateTask],
        RequestClass::TaskProposal | RequestClass::Mutation => vec![Capability::MutateTask],
        RequestClass::ReadOnly => {
            if resource_kind == ResourceKind::Workspace {
                vec![Capability::WorkspaceRead]
            } else {
                vec![Capability::ReadMemory]
            }
        }
    };
    if request_kind == RequestKind::Comparative || resource_kind == ResourceKind::Mixed {
        if !required_capabilities.iter().any(Capability::is_tool_family) {
            required_capabilities.push(Capability::primary_tool_for(
                &operation_kind,
                &resource_kind,
            ));
        }
        required_capabilities.push(Capability::VerifyClaim);
    }
    required_capabilities
}

pub fn required_capabilities_for_typed_request(
    request: &TypedOrchestrationRequest,
) -> Vec<Capability> {
    let request_class = match (
        &request.operation_kind,
        &request.mutability,
        &request.resource_kind,
    ) {
        (OperationKind::Search | OperationKind::Fetch | OperationKind::InspectTooling, _, _) => {
            RequestClass::ToolCall
        }
        (OperationKind::Assimilate, _, _) => RequestClass::Assimilation,
        (OperationKind::Plan, _, _) => RequestClass::TaskProposal,
        (_, Mutability::Mutation, _) => RequestClass::Mutation,
        (OperationKind::Compare, _, ResourceKind::Web) => RequestClass::ToolCall,
        _ => RequestClass::ReadOnly,
    };

    let mut required = required_capabilities_for(
        request_class,
        request.request_kind.clone(),
        request.operation_kind.clone(),
        request.resource_kind.clone(),
    );
    required.sort_by_key(|row| format!("{row:?}"));
    required.dedup();
    required
}

fn should_add_ambiguous_operation_clarification(parsed: &ParseResult) -> bool {
    let request = &parsed.typed_request;
    request.request_kind == RequestKind::Ambiguous
        || request.operation_kind == OperationKind::Unknown
        || parsed.confidence < 0.55
        || parsed.ambiguity.iter().any(|row| {
            !matches!(
                row,
                AmbiguityReason::LegacyCompatOnly | AmbiguityReason::TypedProbeContractViolation
            )
        })
        || (request.surface != RequestSurface::Legacy
            && parsed
                .ambiguity
                .iter()
                .any(|row| matches!(row, AmbiguityReason::LegacyCompatOnly)))
}

fn has_typed_probe_contract_violation(parsed: &ParseResult) -> bool {
    parsed
        .ambiguity
        .iter()
        .any(|row| matches!(row, AmbiguityReason::TypedProbeContractViolation))
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn classification_uses_typed_operation_not_legacy_intent_text() {
        let request = ParseResult {
            typed_request: crate::contracts::TypedOrchestrationRequest {
                session_id: "s1".to_string(),
                surface: RequestSurface::Legacy,
                legacy_intent: "search the web".to_string(),
                adapted: false,
                payload: json!({}),
                request_kind: RequestKind::Direct,
                operation_kind: OperationKind::Read,
                resource_kind: ResourceKind::Memory,
                mutability: Mutability::ReadOnly,
                target_descriptors: Vec::new(),
                target_refs: Vec::new(),
                tool_hints: Vec::new(),
                policy_scope: crate::contracts::PolicyScope::Default,
                user_constraints: Vec::new(),
                core_probe_envelope: None,
            },
            confidence: 0.90,
            ambiguity: Vec::new(),
            reasons: vec!["typed_override".to_string()],
            surface_adapter_used: false,
            surface_adapter_fallback: false,
        };
        let classification = classify_request(&request);
        assert_eq!(classification.request_class, RequestClass::ReadOnly);
        assert!(!classification.needs_clarification);
    }

    #[test]
    fn ambiguous_typed_request_emits_machine_readable_reason() {
        let request = ParseResult {
            typed_request: crate::contracts::TypedOrchestrationRequest {
                session_id: "s1".to_string(),
                surface: RequestSurface::Legacy,
                legacy_intent: "maybe do something".to_string(),
                adapted: false,
                payload: json!({}),
                request_kind: RequestKind::Ambiguous,
                operation_kind: OperationKind::Unknown,
                resource_kind: ResourceKind::Unspecified,
                mutability: Mutability::ReadOnly,
                target_descriptors: Vec::new(),
                target_refs: Vec::new(),
                tool_hints: Vec::new(),
                policy_scope: crate::contracts::PolicyScope::Default,
                user_constraints: Vec::new(),
                core_probe_envelope: None,
            },
            confidence: 0.20,
            ambiguity: vec![
                AmbiguityReason::UnknownOperation,
                AmbiguityReason::LowConfidence,
            ],
            reasons: vec!["request_kind:ambiguous".to_string()],
            surface_adapter_used: false,
            surface_adapter_fallback: false,
        };
        let classification = classify_request(&request);
        assert!(classification.needs_clarification);
        assert!(classification
            .clarification_reasons
            .contains(&ClarificationReason::AmbiguousOperation));
    }

    #[test]
    fn comparative_mixed_request_adds_verify_and_execute_capabilities() {
        let request = ParseResult {
            typed_request: crate::contracts::TypedOrchestrationRequest {
                session_id: "s1".to_string(),
                surface: RequestSurface::Legacy,
                legacy_intent: "compare options".to_string(),
                adapted: false,
                payload: json!({}),
                request_kind: RequestKind::Comparative,
                operation_kind: OperationKind::Compare,
                resource_kind: ResourceKind::Mixed,
                mutability: Mutability::ReadOnly,
                target_descriptors: Vec::new(),
                target_refs: Vec::new(),
                tool_hints: Vec::new(),
                policy_scope: crate::contracts::PolicyScope::Default,
                user_constraints: Vec::new(),
                core_probe_envelope: None,
            },
            confidence: 0.80,
            ambiguity: Vec::new(),
            reasons: vec!["comparative".to_string()],
            surface_adapter_used: false,
            surface_adapter_fallback: false,
        };
        let classification = classify_request(&request);
        assert!(classification
            .required_capabilities
            .contains(&Capability::VerifyClaim));
        assert!(classification
            .required_capabilities
            .contains(&Capability::ToolRoute));
    }

    #[test]
    fn non_legacy_surface_with_legacy_ambiguity_requires_clarification() {
        let request = ParseResult {
            typed_request: crate::contracts::TypedOrchestrationRequest {
                session_id: "s1".to_string(),
                surface: RequestSurface::Sdk,
                legacy_intent: "sdk request".to_string(),
                adapted: true,
                payload: json!({}),
                request_kind: RequestKind::Direct,
                operation_kind: OperationKind::Read,
                resource_kind: ResourceKind::Memory,
                mutability: Mutability::ReadOnly,
                target_descriptors: Vec::new(),
                target_refs: Vec::new(),
                tool_hints: Vec::new(),
                policy_scope: crate::contracts::PolicyScope::Default,
                user_constraints: Vec::new(),
                core_probe_envelope: None,
            },
            confidence: 0.80,
            ambiguity: vec![AmbiguityReason::LegacyCompatOnly],
            reasons: vec!["sdk".to_string()],
            surface_adapter_used: true,
            surface_adapter_fallback: false,
        };
        let classification = classify_request(&request);
        assert!(classification.needs_clarification);
        assert!(classification
            .clarification_reasons
            .contains(&ClarificationReason::AmbiguousOperation));
    }

    #[test]
    fn typed_probe_contract_violation_emits_dedicated_clarification_reason() {
        let request = ParseResult {
            typed_request: crate::contracts::TypedOrchestrationRequest {
                session_id: "sdk-probe-violation".to_string(),
                surface: RequestSurface::Sdk,
                legacy_intent: "search".to_string(),
                adapted: true,
                payload: json!({}),
                request_kind: RequestKind::Direct,
                operation_kind: OperationKind::Search,
                resource_kind: ResourceKind::Web,
                mutability: Mutability::ReadOnly,
                target_descriptors: Vec::new(),
                target_refs: Vec::new(),
                tool_hints: vec!["web_search".to_string()],
                policy_scope: crate::contracts::PolicyScope::WebOnly,
                user_constraints: Vec::new(),
                core_probe_envelope: None,
            },
            confidence: 0.72,
            ambiguity: vec![AmbiguityReason::TypedProbeContractViolation],
            reasons: vec!["typed_probe_contract_missing:core_probe_envelope".to_string()],
            surface_adapter_used: true,
            surface_adapter_fallback: false,
        };
        let classification = classify_request(&request);
        assert!(classification.needs_clarification);
        assert!(classification
            .clarification_reasons
            .contains(&ClarificationReason::TypedProbeContractViolation));
        assert!(!classification
            .clarification_reasons
            .contains(&ClarificationReason::AmbiguousOperation));
    }

    #[test]
    fn tooling_resource_read_is_classified_as_tool_call() {
        let request = ParseResult {
            typed_request: crate::contracts::TypedOrchestrationRequest {
                session_id: "tooling-read".to_string(),
                surface: RequestSurface::Sdk,
                legacy_intent: "inspect tooling".to_string(),
                adapted: true,
                payload: json!({}),
                request_kind: RequestKind::Direct,
                operation_kind: OperationKind::Read,
                resource_kind: ResourceKind::Tooling,
                mutability: Mutability::ReadOnly,
                target_descriptors: Vec::new(),
                target_refs: Vec::new(),
                tool_hints: vec!["tool_route".to_string()],
                policy_scope: crate::contracts::PolicyScope::Default,
                user_constraints: Vec::new(),
                core_probe_envelope: None,
            },
            confidence: 0.78,
            ambiguity: Vec::new(),
            reasons: vec!["typed_tooling_read".to_string()],
            surface_adapter_used: true,
            surface_adapter_fallback: false,
        };
        let classification = classify_request(&request);
        assert_eq!(classification.request_class, RequestClass::ToolCall);
        assert!(classification
            .required_capabilities
            .contains(&Capability::ToolRoute));
    }

    #[test]
    fn workspace_read_request_classification_excludes_web_capabilities() {
        let request = ParseResult {
            typed_request: crate::contracts::TypedOrchestrationRequest {
                session_id: "workspace-read-only".to_string(),
                surface: RequestSurface::Legacy,
                legacy_intent: "read local file".to_string(),
                adapted: false,
                payload: json!({}),
                request_kind: RequestKind::Direct,
                operation_kind: OperationKind::Read,
                resource_kind: ResourceKind::Workspace,
                mutability: Mutability::ReadOnly,
                target_descriptors: Vec::new(),
                target_refs: vec!["README.md".to_string()],
                tool_hints: vec!["workspace_read".to_string()],
                policy_scope: crate::contracts::PolicyScope::WorkspaceOnly,
                user_constraints: Vec::new(),
                core_probe_envelope: None,
            },
            confidence: 0.82,
            ambiguity: Vec::new(),
            reasons: vec!["workspace_local_intent".to_string()],
            surface_adapter_used: false,
            surface_adapter_fallback: false,
        };
        let classification = classify_request(&request);
        assert!(classification
            .required_capabilities
            .contains(&Capability::WorkspaceRead));
        assert!(!classification
            .required_capabilities
            .contains(&Capability::WebSearch));
        assert!(!classification
            .required_capabilities
            .contains(&Capability::WebFetch));
    }

    #[test]
    fn tooling_route_classification_excludes_web_capabilities_for_local_tooling_intent() {
        let request = ParseResult {
            typed_request: crate::contracts::TypedOrchestrationRequest {
                session_id: "local-tool-route".to_string(),
                surface: RequestSurface::Legacy,
                legacy_intent: "route local tooling request".to_string(),
                adapted: false,
                payload: json!({}),
                request_kind: RequestKind::Direct,
                operation_kind: OperationKind::InspectTooling,
                resource_kind: ResourceKind::Tooling,
                mutability: Mutability::ReadOnly,
                target_descriptors: Vec::new(),
                target_refs: vec!["tool_route".to_string()],
                tool_hints: vec!["tool_route".to_string()],
                policy_scope: crate::contracts::PolicyScope::Default,
                user_constraints: Vec::new(),
                core_probe_envelope: None,
            },
            confidence: 0.81,
            ambiguity: Vec::new(),
            reasons: vec!["local_tool_intent".to_string()],
            surface_adapter_used: false,
            surface_adapter_fallback: false,
        };
        let classification = classify_request(&request);
        assert!(classification
            .required_capabilities
            .contains(&Capability::ToolRoute));
        assert!(!classification
            .required_capabilities
            .contains(&Capability::WebSearch));
        assert!(!classification
            .required_capabilities
            .contains(&Capability::WebFetch));
    }
}
