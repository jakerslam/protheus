use crate::contracts::OrchestrationRequest;
use serde_json::Value;

pub fn normalize_request(input: OrchestrationRequest) -> OrchestrationRequest {
    let normalized_intent = input.intent.trim().to_lowercase();
    let normalized_payload = match input.payload {
        Value::Null => Value::Object(Default::default()),
        other => other,
    };
    OrchestrationRequest {
        session_id: input.session_id.trim().to_string(),
        intent: normalized_intent,
        payload: normalized_payload,
    }
}
