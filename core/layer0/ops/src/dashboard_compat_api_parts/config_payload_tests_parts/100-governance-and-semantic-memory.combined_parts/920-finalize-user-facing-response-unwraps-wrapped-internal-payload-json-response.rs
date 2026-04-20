fn finalize_user_facing_response_unwraps_wrapped_internal_payload_json_response() {
    let raw = format!(
        "tool output follows:\n{}\nend",
        json!({
            "agent_id": "agent-83ed64e07515",
            "response": "Synthesized answer with linked sources.",
            "response_finalization": {"tool_completion": {"completion_state": "reported_findings"}},
            "tools": [{"name": "batch_query", "is_error": false, "result": "raw tool output"}],
            "turn_loop_tracking": {"ok": true},
            "turn_transaction": {"tool_execute": "complete"}
        })
    );
    let finalized = finalize_user_facing_response(raw, None);
    assert_eq!(finalized, "Synthesized answer with linked sources.");
    assert!(!finalized.contains("agent_id"));
}

include!("030-part.tail.rs");

#[test]
