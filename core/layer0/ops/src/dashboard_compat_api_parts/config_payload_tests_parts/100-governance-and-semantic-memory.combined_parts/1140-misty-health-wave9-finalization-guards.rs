// SRS: V12-MISTY-HEALTH-WAVE9-001

#[test]
fn misty_wave9_gate_choice_prefix_is_recovered_before_visible_chat() {
    let root = governance_temp_root();
    let snapshot = governance_ok_snapshot();
    let created = handle(
        root.path(),
        "POST",
        "/api/agents",
        br#"{"name":"misty-wave9-gate-prefix-agent","role":"assistant"}"#,
        &snapshot,
    )
    .expect("agent create");
    let agent_id = clean_agent_id(
        created
            .payload
            .get("agent_id")
            .or_else(|| created.payload.get("id"))
            .and_then(Value::as_str)
            .unwrap_or(""),
    );
    write_json(
        &governance_test_chat_script_path(root.path()),
        &json!({"queue": [
            {"response": "Yes, tool family: Conversation. Tool: Answer directly."},
            {"response": "I can answer normally and keep tool-menu choices out of visible chat."}
        ], "calls": []}),
    );

    let response = handle(
        root.path(),
        "POST",
        &format!("/api/agents/{agent_id}/message"),
        br#"{"message":"Do you feel like you can answer normally and decide whether tools are needed? Answer naturally."}"#,
        &snapshot,
    )
    .expect("message response");

    assert_eq!(response.status, 200);
    assert_eq!(response.payload.get("ok").and_then(Value::as_bool), Some(true));
    let response_text = response
        .payload
        .get("response")
        .and_then(Value::as_str)
        .unwrap_or("");
    assert_eq!(
        response_text,
        "I can answer normally and keep tool-menu choices out of visible chat."
    );
    assert!(!response_text.starts_with("Yes,"), "{response_text}");
    assert_eq!(
        response
            .payload
            .get("system_chat_injection_used")
            .and_then(Value::as_bool),
        Some(false)
    );
    assert!(
        response
            .payload
            .pointer("/response_finalization/outcome")
            .and_then(Value::as_str)
            .unwrap_or("")
            .contains("final_response_guard_recovered_by_llm")
    );
}

#[test]
fn misty_wave9_web_no_findings_without_tool_receipt_becomes_pending_choice() {
    let root = governance_temp_root();
    let snapshot = governance_ok_snapshot();
    let created = handle(
        root.path(),
        "POST",
        "/api/agents",
        br#"{"name":"misty-wave9-web-evidence-agent","role":"assistant"}"#,
        &snapshot,
    )
    .expect("agent create");
    let agent_id = clean_agent_id(
        created
            .payload
            .get("agent_id")
            .or_else(|| created.payload.get("id"))
            .and_then(Value::as_str)
            .unwrap_or(""),
    );
    write_json(
        &governance_test_chat_script_path(root.path()),
        &json!({"queue": [
            {"response": "No. The web search returned no findings about current OpenHands and AutoGPT status."},
            {"response": "I would choose web search next, then summarize only after results are available."}
        ], "calls": []}),
    );

    let response = handle(
        root.path(),
        "POST",
        &format!("/api/agents/{agent_id}/message"),
        br#"{"message":"Use web search to check current OpenHands and AutoGPT status, then summarize whether you need another tool before answering."}"#,
        &snapshot,
    )
    .expect("message response");

    assert_eq!(response.status, 200);
    assert_eq!(response.payload.get("ok").and_then(Value::as_bool), Some(true));
    let response_text = response
        .payload
        .get("response")
        .and_then(Value::as_str)
        .unwrap_or("");
    assert!(
        response_text.to_ascii_lowercase().contains("web search"),
        "{response_text}"
    );
    assert!(!response_text.starts_with("No."), "{response_text}");
    assert!(
        !response_text
            .to_ascii_lowercase()
            .contains("returned no findings"),
        "{response_text}"
    );
    assert_eq!(
        response
            .payload
            .get("tools")
            .and_then(Value::as_array)
            .map(Vec::len),
        Some(0)
    );
    assert_eq!(
        response
            .payload
            .pointer("/pending_tool_request/status")
            .and_then(Value::as_str),
        Some("pending_confirmation")
    );
    assert_eq!(
        response
            .payload
            .pointer("/pending_tool_request/tool_name")
            .and_then(Value::as_str),
        Some("batch_query")
    );
    assert_eq!(
        response
            .payload
            .pointer("/pending_tool_request/execution_claim_allowed")
            .and_then(Value::as_bool),
        Some(false)
    );
    assert_eq!(
        response
            .payload
            .pointer("/web_intent/detected")
            .and_then(Value::as_bool),
        Some(true)
    );
    assert_eq!(
        response
            .payload
            .get("system_chat_injection_used")
            .and_then(Value::as_bool),
        Some(false)
    );
}
