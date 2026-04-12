use crate::contracts::{
    ClarificationReason, CoreContractCall, Mutability, OperationKind, RequestClass,
    RequestClassification, RequestKind, ResourceKind, TypedOrchestrationRequest,
};

pub fn classify_request(request: &TypedOrchestrationRequest) -> RequestClassification {
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

    let required_contracts = match request_class {
        RequestClass::ToolCall => vec![
            CoreContractCall::ToolCapabilityProbe,
            CoreContractCall::ToolBrokerRequest,
        ],
        RequestClass::Assimilation => vec![
            CoreContractCall::AssimilationPlanRequest,
            CoreContractCall::TaskFabricProposal,
        ],
        RequestClass::TaskProposal | RequestClass::Mutation => {
            vec![CoreContractCall::TaskFabricProposal]
        }
        RequestClass::ReadOnly => vec![CoreContractCall::UnifiedMemoryRead],
    };

    let mut clarification_reasons = Vec::new();
    if request.session_id.is_empty() {
        clarification_reasons.push(ClarificationReason::MissingSessionId);
    }
    if request.request_kind == RequestKind::Ambiguous
        || request.operation_kind == OperationKind::Unknown
        || request.parse_confidence < 0.45
    {
        clarification_reasons.push(ClarificationReason::AmbiguousOperation);
    }
    if request.operation_kind == OperationKind::Assimilate && request.target_refs.is_empty() {
        clarification_reasons.push(ClarificationReason::MissingTargetRefs);
    }
    if request.mutability == Mutability::Mutation && request.target_refs.is_empty() {
        clarification_reasons.push(ClarificationReason::MutationScopeRequired);
    }

    let mut reasons = request.parse_reasons.clone();
    reasons.push(format!("classified_as:{request_class:?}").to_lowercase());
    reasons.push(format!("policy_scope:{:?}", request.policy_scope).to_lowercase());
    if !request.tool_hints.is_empty() {
        reasons.push(format!("tool_hints:{}", request.tool_hints.join(",")));
    }

    let confidence = if clarification_reasons.is_empty() {
        request.parse_confidence
    } else {
        (request.parse_confidence - 0.20).clamp(0.0, 0.95)
    };

    RequestClassification {
        request_class,
        confidence,
        reasons,
        required_contracts,
        clarification_reasons: clarification_reasons.clone(),
        needs_clarification: !clarification_reasons.is_empty(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn classification_uses_typed_operation_not_legacy_intent_text() {
        let request = TypedOrchestrationRequest {
            session_id: "s1".to_string(),
            legacy_intent: "search the web".to_string(),
            payload: json!({}),
            request_kind: RequestKind::Direct,
            operation_kind: OperationKind::Read,
            resource_kind: ResourceKind::Memory,
            mutability: Mutability::ReadOnly,
            target_refs: Vec::new(),
            tool_hints: Vec::new(),
            policy_scope: crate::contracts::PolicyScope::Default,
            user_constraints: Vec::new(),
            parse_confidence: 0.90,
            parse_reasons: vec!["typed_override".to_string()],
        };
        let classification = classify_request(&request);
        assert_eq!(classification.request_class, RequestClass::ReadOnly);
        assert!(!classification.needs_clarification);
    }

    #[test]
    fn ambiguous_typed_request_emits_machine_readable_reason() {
        let request = TypedOrchestrationRequest {
            session_id: "s1".to_string(),
            legacy_intent: "maybe do something".to_string(),
            payload: json!({}),
            request_kind: RequestKind::Ambiguous,
            operation_kind: OperationKind::Unknown,
            resource_kind: ResourceKind::Unspecified,
            mutability: Mutability::ReadOnly,
            target_refs: Vec::new(),
            tool_hints: Vec::new(),
            policy_scope: crate::contracts::PolicyScope::Default,
            user_constraints: Vec::new(),
            parse_confidence: 0.20,
            parse_reasons: vec!["request_kind:ambiguous".to_string()],
        };
        let classification = classify_request(&request);
        assert!(classification.needs_clarification);
        assert!(classification
            .clarification_reasons
            .contains(&ClarificationReason::AmbiguousOperation));
    }
}
