#[test]
fn finalize_user_facing_response_blocks_internal_payload_json_without_response() {
    let raw = json!({
        "agent_id": "agent-83ed64e07515",
        "response_finalization": {"tool_completion": {"completion_state": "reported_reason"}},
        "tools": [{"name": "manage_agent", "is_error": false, "result": "{\"ok\":true}"}],
        "turn_loop_tracking": {"ok": true},
        "turn_transaction": {"tool_execute": "complete"}
    })
    .to_string();
    let finalized = finalize_user_facing_response(raw, None);
    let lowered = finalized.to_ascii_lowercase();
    assert!(
        lowered.contains("no synthesized response")
            || lowered.contains("usable tool findings from this turn yet")
    );
    assert!(!lowered.contains("agent_id"));
    assert!(!finalized.starts_with('{'));
}

#[test]
fn summarize_tool_payload_unknown_tool_avoids_raw_json_dump() {
    let payload = json!({
        "ok": true,
        "agent_id": "agent-raw-dump",
        "runtime_model": "tool-router",
        "turn_loop_tracking": {"ok": true},
        "response_finalization": {"tool_completion": {"completion_state": "reported_findings"}},
        "result_count": 3,
        "source": "web"
    });
    let summary = summarize_tool_payload("manage_agent", &payload);
    let lowered = summary.to_ascii_lowercase();
    assert!(!summary.trim_start().starts_with('{'));
    assert!(!lowered.contains("\"agent_id\""));
    assert!(lowered.contains("completed"));
}
