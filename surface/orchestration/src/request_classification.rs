use crate::contracts::{
    AmbiguityReason, Capability, ClarificationReason, Mutability, OperationKind, ParseResult,
    RequestClass, RequestClassification, RequestKind, RequestSurface, ResourceKind,
};

pub fn classify_request(parsed: &ParseResult) -> RequestClassification {
    let request = &parsed.typed_request;
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

    let mut required_capabilities = match request_class {
        RequestClass::ToolCall => vec![Capability::ExecuteTool],
        RequestClass::Assimilation => vec![Capability::PlanAssimilation, Capability::MutateTask],
        RequestClass::TaskProposal | RequestClass::Mutation => vec![Capability::MutateTask],
        RequestClass::ReadOnly => vec![Capability::ReadMemory],
    };
    if request.request_kind == RequestKind::Comparative
        || request.resource_kind == ResourceKind::Mixed
    {
        if matches!(request.resource_kind, ResourceKind::Web | ResourceKind::Mixed) {
            required_capabilities.push(Capability::ExecuteTool);
        }
        required_capabilities.push(Capability::VerifyClaim);
    }
    required_capabilities.sort_by_key(|row| format!("{row:?}"));
    required_capabilities.dedup();

    let mut clarification_reasons = Vec::new();
    if request.session_id.is_empty() {
        clarification_reasons.push(ClarificationReason::MissingSessionId);
    }
    if request.request_kind == RequestKind::Ambiguous
        || request.operation_kind == OperationKind::Unknown
        || parsed.confidence < 0.55
        || parsed
            .ambiguity
            .iter()
            .any(|row| !matches!(row, AmbiguityReason::LegacyCompatOnly))
        || (request.surface != RequestSurface::Legacy
            && parsed
                .ambiguity
                .iter()
                .any(|row| matches!(row, AmbiguityReason::LegacyCompatOnly)))
    {
        clarification_reasons.push(ClarificationReason::AmbiguousOperation);
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
    }
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
            },
            confidence: 0.90,
            ambiguity: Vec::new(),
            reasons: vec!["typed_override".to_string()],
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
            },
            confidence: 0.20,
            ambiguity: vec![AmbiguityReason::UnknownOperation, AmbiguityReason::LowConfidence],
            reasons: vec!["request_kind:ambiguous".to_string()],
        };
        let classification = classify_request(&request);
        assert!(classification.needs_clarification);
        assert!(classification
            .clarification_reasons
            .contains(&ClarificationReason::AmbiguousOperation));
    }
}
