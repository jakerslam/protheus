        let request_body = json!({ "message": message.clone() }).to_string();
        let response = handle(
            root.path(),
            "POST",
            &format!("/api/agents/{agent_id}/message"),
            request_body.as_bytes(),
            &snapshot,
        )
        .expect("message response");
        assert_eq!(response.status, 200);
        let response_text = response
            .payload
            .get("response")
            .and_then(Value::as_str)
            .unwrap_or("");
        if response_text.trim().is_empty() {
            let current = taxonomy
                .get("empty_final")
                .and_then(Value::as_u64)
                .unwrap_or(0);
            taxonomy["empty_final"] = json!(current + 1);
        }
        if response_is_deferred_execution_preamble(response_text)
            || response_is_deferred_retry_prompt(response_text)
            || workflow_response_requests_more_tooling(response_text)
        {
            let current = taxonomy
                .get("deferred_final")
                .and_then(Value::as_u64)
                .unwrap_or(0);
            taxonomy["deferred_final"] = json!(current + 1);
        }
        if response_is_no_findings_placeholder(response_text)
            || response_looks_like_tool_ack_without_findings(response_text)
        {
            let current = taxonomy
                .get("placeholder_final")
                .and_then(Value::as_u64)
                .unwrap_or(0);
            taxonomy["placeholder_final"] = json!(current + 1);
        }
        if response_is_unrelated_context_dump(&message, response_text) {
            let current = taxonomy
                .get("off_topic_final")
                .and_then(Value::as_u64)
                .unwrap_or(0);
            taxonomy["off_topic_final"] = json!(current + 1);
        }
        let tools_len = response
            .payload
            .get("tools")
            .and_then(Value::as_array)
            .map(|rows| rows.len())
            .unwrap_or(0);
        if (message == "that was just a test" || message_is_web_tooling_status_check(&message))
            && tools_len > 0
        {
            let current = taxonomy
                .get("meta_status_tool_leak")
                .and_then(Value::as_u64)
                .unwrap_or(0);
            taxonomy["meta_status_tool_leak"] = json!(current + 1);
        }
        if natural_web_intent_from_user_message(&message).is_some()
            && !message_is_web_tooling_status_check(&message)
            && tools_len == 0
        {
            let current = taxonomy
                .get("web_missing_tool_attempt")
                .and_then(Value::as_u64)
                .unwrap_or(0);
            taxonomy["web_missing_tool_attempt"] = json!(current + 1);
        }
    }

    println!("WEB_TOOLING_CONTEXT_SOAK_TAXONOMY={}", taxonomy);
    assert_eq!(taxonomy.get("empty_final").and_then(Value::as_u64), Some(0));
    assert_eq!(taxonomy.get("deferred_final").and_then(Value::as_u64), Some(0));
    assert_eq!(taxonomy.get("placeholder_final").and_then(Value::as_u64), Some(0));
    assert_eq!(taxonomy.get("off_topic_final").and_then(Value::as_u64), Some(0));
    assert_eq!(
        taxonomy
            .get("meta_status_tool_leak")
            .and_then(Value::as_u64),
        Some(0)
    );
    assert_eq!(
        taxonomy
            .get("web_missing_tool_attempt")
            .and_then(Value::as_u64),
        Some(0)
    );
}

#[test]
fn workflow_system_fallback_requires_final_stage_failure() {
    let root = governance_temp_root();
    let snapshot = governance_ok_snapshot();
    let created = handle(
        root.path(),
        "POST",
        "/api/agents",
        br#"{"name":"workflow-contract-failure-agent","role":"assistant"}"#,
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
                {"response": "Initial direct draft."},
                {"response": "I don't have usable tool findings from this turn yet. Ask me to retry with a narrower query or a specific source URL."},
                {"response": "I don't have usable tool findings from this turn yet. Ask me to retry with a narrower query or a specific source URL."}
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
    assert_eq!(
        response
            .payload
            .pointer("/response_workflow/final_llm_response/status")
            .and_then(Value::as_str),
        Some("synthesis_failed")
    );
    assert_eq!(
        response
            .payload
            .pointer("/response_finalization/workflow_system_fallback_used")
            .and_then(Value::as_bool),
        Some(true)
    );
    let response_text = response
        .payload
        .get("response")
        .and_then(Value::as_str)
        .unwrap_or("");
    assert!(!response_text.trim().is_empty(), "expected explicit fallback reply");
    assert!(
        response_text.to_ascii_lowercase().contains("response-synthesis failure")
            || response_text.to_ascii_lowercase().contains("retry"),
        "{response_text}"
    );
}

