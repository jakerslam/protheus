use crate::contracts::{ClarificationReason, RequestClassification, TypedOrchestrationRequest};

pub fn build_clarification_prompt(
    request: &TypedOrchestrationRequest,
    classification: &RequestClassification,
) -> Option<String> {
    let primary = classification.clarification_reasons.first()?;
    match primary {
        ClarificationReason::MissingSessionId => {
            Some("missing session_id for orchestration context".to_string())
        }
        ClarificationReason::AmbiguousOperation => Some(format!(
            "clarify requested operation before orchestration planning (parsed operation={:?}, resource={:?})",
            request.operation_kind, request.resource_kind
        ).to_lowercase()),
        ClarificationReason::MissingTargetRefs => {
            Some("specify target artifacts for assimilation planning".to_string())
        }
        ClarificationReason::MutationScopeRequired => {
            Some("confirm mutation scope and target contract before execution".to_string())
        }
        ClarificationReason::PlannerGap => {
            Some("no executable plan steps were generated".to_string())
        }
    }
}

// Compatibility alias during control-plane naming transition.
pub fn clarification_prompt_for(
    request: &TypedOrchestrationRequest,
    classification: &RequestClassification,
) -> Option<String> {
    build_clarification_prompt(request, classification)
}
