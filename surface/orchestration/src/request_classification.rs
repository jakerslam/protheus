use crate::contracts::{OrchestrationRequest, RequestClass};

pub fn classify_request(request: &OrchestrationRequest) -> RequestClass {
    let intent = request.intent.as_str();
    if intent.contains("tool") || intent.contains("search") || intent.contains("fetch") {
        return RequestClass::ToolCall;
    }
    if intent.contains("assimilat") || intent.contains("ingest") {
        return RequestClass::Assimilation;
    }
    if intent.contains("task") || intent.contains("plan") || intent.contains("backlog") {
        return RequestClass::TaskProposal;
    }
    if intent.contains("update") || intent.contains("write") || intent.contains("apply") {
        return RequestClass::Mutation;
    }
    RequestClass::ReadOnly
}
