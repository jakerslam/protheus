// SRS: V12-WORKFLOW-DIRECT-ANSWER-ACK-001
fn workflow_library_allows_direct_answer_without_second_synthesis() {
    let root = governance_temp_root();
    let snapshot = governance_ok_snapshot();
    let created = handle(
        root.path(),
        "POST",
        "/api/agents",
        br#"{"name":"workflow-owned-direct-answer-agent","role":"assistant"}"#,
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
                    "response": "The workflow and tool menu are working, and I can answer directly."
                }
            ],
            "calls": []
        }),
    );
    let response = handle(
        root.path(),
        "POST",
        &format!("/api/agents/{agent_id}/message"),
        br#"{"message":"Say hello and confirm the chain is working."}"#,
        &snapshot,
    )
    .expect("message response");
    assert_eq!(response.status, 200);
    assert_eq!(
        response.payload.get("response").and_then(Value::as_str),
        Some("The workflow and tool menu are working, and I can answer directly.")
    );
    assert_eq!(
        response
            .payload
            .pointer("/response_workflow/final_llm_response/status")
            .and_then(Value::as_str),
        Some("skipped_not_required")
    );
    assert_eq!(
        response
            .payload
            .pointer("/response_finalization/workflow_system_fallback_used")
            .and_then(Value::as_bool),
        Some(false)
    );
    assert_eq!(
        response
            .payload
            .pointer("/response_workflow/selected_workflow/gate_contract")
            .and_then(Value::as_str),
        Some("tool_menu_interface_v1")
    );
    assert_eq!(
        response
            .payload
            .pointer("/response_workflow/library/default_workflow")
            .and_then(Value::as_str),
        Some("simple_conversation_v1")
    );
    assert_eq!(
        response
            .payload
            .pointer("/response_workflow/stage_statuses/0/stage")
            .and_then(Value::as_str),
        Some("gate_1_need_tool_access_menu")
    );
    assert_eq!(
        response
            .payload
            .pointer("/response_workflow/stage_statuses/0/status")
            .and_then(Value::as_str),
        Some("answered_no")
    );
    assert_eq!(
        response
            .payload
            .pointer("/response_workflow/stage_statuses/1/stage")
            .and_then(Value::as_str),
        Some("gate_6_llm_final_output")
    );
}

#[test]
