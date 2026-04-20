fn workflow_low_signal_tool_reply_persists_repaired_visible_response() {
    let root = governance_temp_root();
    let snapshot = governance_ok_snapshot();
    let created = handle(
        root.path(),
        "POST",
        "/api/agents",
        br#"{"name":"workflow-low-signal-persistence-agent","role":"researcher"}"#,
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
    assert!(!agent_id.is_empty());
    write_json(
        &governance_test_chat_script_path(root.path()),
        &json!({
            "queue": [
                {
                    "response": "<function=batch_query>{\"source\":\"web\",\"query\":\"top AI agentic frameworks\",\"aperture\":\"medium\"}</function>"
                },
                {"response": "I don't have usable tool findings from this turn yet. Ask me to retry with a narrower query or a specific source URL."},
                {"response": "I don't have usable tool findings from this turn yet. Ask me to retry with a narrower query or a specific source URL."}
            ],
            "calls": []
        }),
    );
    write_json(
        &governance_test_tool_script_path(root.path()),
        &json!({
            "queue": [
                {
                    "tool": "batch_query",
                    "payload": {
                        "ok": true,
                        "status": "no_results",
                        "summary": "Web retrieval ran, but no usable findings were extracted in this turn. Retry with a narrower query or one specific source URL for source-backed findings."
                    }
                }
            ],
            "calls": []
        }),
    );
    let response = handle(
        root.path(),
        "POST",
        &format!("/api/agents/{agent_id}/message"),
        br#"{"message":"Try to web search \"top AI agentic frameworks\" and return the results"}"#,
        &snapshot,
    )
    .expect("message response");
    assert_eq!(response.status, 200);
    assert_eq!(
        response
            .payload
            .pointer("/response_workflow/final_llm_response/status")
            .and_then(Value::as_str),
        Some("synthesis_failed")
    );
    let response_text = response
        .payload
        .get("response")
        .and_then(Value::as_str)
        .unwrap_or("");
    assert!(!response_text.trim().is_empty(), "expected repaired reply");
    assert!(!response_is_no_findings_placeholder(response_text));
    let persisted = latest_persisted_assistant_text_for_test(root.path(), &agent_id);
    assert_eq!(
        normalize_test_text_whitespace(&persisted),
        normalize_test_text_whitespace(response_text)
    );
}

#[test]
fn workflow_initial_model_invoke_failure_still_persists_visible_reply() {
    let root = governance_temp_root();
    let snapshot = governance_ok_snapshot();
    let created = handle(
        root.path(),
        "POST",
        "/api/agents",
        br#"{"name":"workflow-invoke-failure-agent","role":"researcher"}"#,
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
    assert!(!agent_id.is_empty());
    write_json(
        &governance_test_chat_script_path(root.path()),
        &json!({
            "queue": [
                {"error": "provider timeout after 30s"}
            ],
            "calls": []
        }),
    );
    let response = handle(
        root.path(),
        "POST",
        &format!("/api/agents/{agent_id}/message"),
        br#"{"message":"Say hello and confirm workflow ownership."}"#,
        &snapshot,
    )
    .expect("message response");
    assert_eq!(response.status, 200);
    let final_stage_status = response
        .payload
        .pointer("/response_workflow/final_llm_response/status")
        .and_then(Value::as_str)
        .unwrap_or("");
    assert!(
        final_stage_status == "invoke_failed" || final_stage_status == "synthesized",
        "{final_stage_status}"
    );
    let response_text = response
        .payload
        .get("response")
        .and_then(Value::as_str)
        .unwrap_or("");
    assert!(!response_text.trim().is_empty(), "expected visible fallback reply");
    assert!(!response_is_no_findings_placeholder(response_text));
    assert!(
        response_text.to_ascii_lowercase().contains("retry")
            || response_text.to_ascii_lowercase().contains("workflow"),
        "{response_text}"
    );
    let persisted = latest_persisted_assistant_text_for_test(root.path(), &agent_id);
    assert_eq!(
        normalize_test_text_whitespace(&persisted),
        normalize_test_text_whitespace(response_text)
    );
}
