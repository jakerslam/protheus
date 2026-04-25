
fn output_tokens_estimate(payload: &Value) -> usize {
    let mut total = 0usize;
    for key in ["summary", "content", "result", "message", "error"] {
        total += payload
            .get(key)
            .and_then(Value::as_str)
            .map(|row| row.len())
            .unwrap_or(0);
    }
    if total == 0 {
        total = payload.to_string().len().min(32_000);
    }
    (total / 4).max(1)
}

pub(crate) fn record_tool_turn_tracking(
    root: &Path,
    session_id: &str,
    tool_name: &str,
    payload: &Value,
) -> Option<Value> {
    let clean_session = clean_text(session_id, 120);
    if clean_session.is_empty() {
        return None;
    }
    let command = format!("tool::{}", normalize_tool_name(tool_name));
    let batch = json!({
        "session_id": clean_session,
        "records": [
            {
                "session_id": clean_session,
                "command": command,
                "output_tokens": output_tokens_estimate(payload)
            }
        ]
    });
    crate::session_command_tracking_kernel::record_batch_for_kernel(root, &batch).ok()
}

pub(crate) fn turn_transaction_payload(
    hydrate: &str,
    tool_execute: &str,
    synthesize: &str,
    session_persist: &str,
) -> Value {
    let hydrate = clean_text(hydrate, 60);
    let tool_execute = clean_text(tool_execute, 60);
    let synthesize = clean_text(synthesize, 60);
    let session_persist = clean_text(session_persist, 60);
    let stages = [
        ("hydrate", hydrate.as_str()),
        ("tool_execute", tool_execute.as_str()),
        ("synthesize", synthesize.as_str()),
        ("session_persist", session_persist.as_str()),
    ];
    let complete = stages.iter().all(|(_, status)| *status == "complete");
    let first_incomplete_stage = stages
        .iter()
        .find(|(_, status)| *status != "complete")
        .map(|(stage, _)| *stage)
        .unwrap_or("");
    let receipt_id = crate::deterministic_receipt_hash(&json!({
        "type": "turn_transaction_lifecycle_receipt",
        "contract_version": 2,
        "hydrate": &hydrate,
        "tool_execute": &tool_execute,
        "synthesize": &synthesize,
        "session_persist": &session_persist,
        "complete": complete,
        "first_incomplete_stage": first_incomplete_stage
    }));
    json!({
        "contract_version": 2,
        "receipt_id": receipt_id,
        "hydrate": hydrate,
        "tool_execute": tool_execute,
        "synthesize": synthesize,
        "session_persist": session_persist,
        "complete": complete,
        "first_incomplete_stage": first_incomplete_stage
    })
}

pub(crate) fn hydration_failed_payload(agent_id: &str) -> Value {
    json!({
        "ok": false,
        "error": "context_hydration_incomplete",
        "agent_id": clean_text(agent_id, 120),
        "message": "Conversation context hydration failed closed before model execution. Retry once; if it persists, run `infringctl doctor --json` and `/context`.",
        "turn_transaction": turn_transaction_payload("failed_closed", "skipped", "skipped", "skipped")
    })
}
