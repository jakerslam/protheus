#[test]
fn meta_control_turn_does_not_trigger_web_tool_execution() {
    let root = governance_temp_root();
    let snapshot = governance_ok_snapshot();
    let created = handle(
        root.path(),
        "POST",
        "/api/agents",
        br#"{"name":"workflow-meta-control-tool-block-agent","role":"assistant"}"#,
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
                    "response": "<function=batch_query>{\"source\":\"web\",\"query\":\"top AI agent frameworks\",\"aperture\":\"medium\"}</function>"
                },
                {
                    "response": "Acknowledged. That was just a test turn, so no web call is needed."
                },
                {
                    "response": "Acknowledged. That was just a test turn, so no web call is needed."
                }
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
                        "status": "ok",
                        "summary": "This payload should never be consumed by meta-control turns."
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
        br#"{"message":"that was just a test"}"#,
        &snapshot,
    )
    .expect("message response");
    assert_eq!(response.status, 200);
    assert_eq!(
        response
            .payload
            .pointer("/response_workflow/tool_gate/meta_control_message")
            .and_then(Value::as_bool),
        Some(true)
    );
    assert_eq!(
        response
            .payload
            .get("tools")
            .and_then(Value::as_array)
            .map(|rows| rows.len()),
        Some(0)
    );
    let response_text = response
        .payload
        .get("response")
        .and_then(Value::as_str)
        .unwrap_or("");
    assert!(!response_text.trim().is_empty(), "{response_text}");
}

#[test]
fn workflow_web_tool_failure_still_returns_final_user_response() {
    let root = governance_temp_root();
    let snapshot = governance_ok_snapshot();
    let created = handle(
        root.path(),
        "POST",
        "/api/agents",
        br#"{"name":"workflow-web-failure-final-response-agent","role":"researcher"}"#,
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
                    "response": "<function=batch_query>{\"source\":\"web\",\"query\":\"agent frameworks now\",\"aperture\":\"medium\"}</function>"
                },
                {
                    "response": "I'll get you an update on that web request."
                },
                {
                    "response": "I'll get you an update on that web request."
                }
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
                        "ok": false,
                        "status": "error",
                        "error": "request_read_failed",
                        "summary": "request_read_failed: transient provider outage"
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
        br#"{"message":"search the web for current top agent frameworks"}"#,
        &snapshot,
    )
    .expect("message response");
    assert_eq!(response.status, 200);
    let response_text = response
        .payload
        .get("response")
        .and_then(Value::as_str)
        .unwrap_or("");
    assert!(!response_text.trim().is_empty(), "{response_text}");
    assert!(!response_is_no_findings_placeholder(response_text));
    assert!(!response_is_deferred_execution_preamble(response_text));
    assert!(!response_is_deferred_retry_prompt(response_text));
    let findings_available = response
        .payload
        .pointer("/response_finalization/findings_available")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    if !findings_available {
        assert!(
            response_text.to_ascii_lowercase().contains("request_read_failed")
                || response_text.to_ascii_lowercase().contains("web_status")
                || response_text.to_ascii_lowercase().contains("error_code"),
            "{response_text}"
        );
    }
}

