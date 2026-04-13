mod classifier;
mod parser;

use crate::contracts::{OrchestrationRequest, ParseResult, TypedOrchestrationRequest};
use serde_json::Value;

pub fn normalize_request(input: OrchestrationRequest) -> ParseResult {
    let session_id = input.session_id.trim().to_string();
    let legacy_intent = input.intent.trim().to_lowercase();
    let payload = match input.payload {
        Value::Null => Value::Object(Default::default()),
        other => other,
    };
    let tokens = parser::tokenize(&legacy_intent);
    let operation_candidates = parser::operation_candidates(&tokens, &payload);
    let resource_candidates = parser::resource_candidates(&tokens, &payload);
    let operation_kind = classifier::select_operation_kind(&operation_candidates);
    let resource_kind = classifier::select_resource_kind(&resource_candidates);
    let mutability = parser::infer_mutability(&operation_kind);
    let request_kind = classifier::infer_request_kind(&operation_candidates, &operation_kind);
    let target_refs = parser::extract_target_refs(&payload);
    let tool_hints = parser::extract_tool_hints(&payload, &operation_kind);
    let policy_scope = classifier::infer_policy_scope(&resource_kind, &mutability);
    let user_constraints = parser::extract_user_constraints(&payload);

    classifier::parse_diagnostics(
        TypedOrchestrationRequest {
            session_id,
            legacy_intent,
            payload,
            request_kind,
            operation_kind,
            resource_kind,
            mutability,
            target_refs,
            tool_hints,
            policy_scope,
            user_constraints,
        },
        &operation_candidates,
        &resource_candidates,
    )
}
