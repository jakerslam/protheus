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
        ClarificationReason::TypedProbeContractViolation => Some(
            build_typed_probe_gap_prompt(classification),
        ),
        ClarificationReason::MissingTargetRefs => {
            Some("specify target artifacts for assimilation planning".to_string())
        }
        ClarificationReason::MutationScopeRequired => {
            Some("confirm mutation scope and target contract before execution".to_string())
        }
        ClarificationReason::PlannerGap => {
            Some(build_planner_gap_prompt(classification))
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

fn build_typed_probe_gap_prompt(classification: &RequestClassification) -> String {
    let missing = classification
        .reasons
        .iter()
        .filter_map(|reason| reason.strip_prefix("typed_probe_contract_missing:"))
        .take(4)
        .collect::<Vec<_>>();
    if missing.is_empty() {
        return "typed surface request is missing required core_probe_envelope fields; refresh gateway/sdk probe contract and retry".to_string();
    }
    format!(
        "typed probe contract is incomplete (missing: {}); refresh probe envelope and retry",
        missing.join(", ")
    )
}

fn build_planner_gap_prompt(classification: &RequestClassification) -> String {
    if classification
        .reasons
        .iter()
        .any(|reason| reason == "feedback_no_viable_reroute")
    {
        return "workflow reroute options were exhausted; provide a narrower target or direct source context".to_string();
    }
    "no executable plan steps were generated; provide explicit route, target, or direct evidence"
        .to_string()
}
