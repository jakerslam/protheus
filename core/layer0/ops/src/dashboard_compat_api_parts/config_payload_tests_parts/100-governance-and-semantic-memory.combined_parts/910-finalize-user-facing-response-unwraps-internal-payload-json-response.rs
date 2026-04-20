fn finalize_user_facing_response_unwraps_internal_payload_json_response() {
    let raw = json!({
        "agent_id": "agent-83ed64e07515",
        "response": "From web retrieval: benchmark summary with sources. https://example.com/benchmarks",
        "response_finalization": {"tool_completion": {"completion_state": "reported_findings"}},
        "tools": [{"name": "batch_query", "is_error": false, "result": "raw tool output"}],
        "turn_loop_tracking": {"ok": true},
        "turn_transaction": {"tool_execute": "complete"}
    })
    .to_string();
    let finalized = finalize_user_facing_response(raw, None);
    assert_eq!(
        finalized,
        "From web retrieval: benchmark summary with sources. https://example.com/benchmarks"
    );
    assert!(!finalized.contains("agent_id"));
    assert!(!finalized.starts_with('{'));
}

#[test]
