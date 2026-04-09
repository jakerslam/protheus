use crate::contracts::{OrchestrationRequest, RequestClass};

pub fn clarification_prompt_for(
    request: &OrchestrationRequest,
    request_class: RequestClass,
) -> Option<String> {
    if request.session_id.is_empty() {
        return Some("missing session_id for orchestration context".to_string());
    }
    match request_class {
        RequestClass::Mutation => {
            Some("confirm mutation scope and target contract before execution".to_string())
        }
        RequestClass::Assimilation if request.intent.contains("unknown") => {
            Some("specify target artifacts for assimilation planning".to_string())
        }
        _ => None,
    }
}
