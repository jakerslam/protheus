
#[test]
fn direct_message_safe_step_prompt_returns_workflow_metadata() {
    let root = governance_temp_root();
    let snapshot = governance_ok_snapshot();
    let parent = handle(
        root.path(),
        "POST",
        "/api/agents",
        br#"{"name":"workflow-metadata-agent","role":"operator"}"#,
        &snapshot,
    )
    .expect("parent create");
    let parent_id = clean_agent_id(
        parent
            .payload
            .get("agent_id")
            .or_else(|| parent.payload.get("id"))
            .and_then(Value::as_str)
            .unwrap_or(""),
    );
    let response = handle(
        root.path(),
        "POST",
        &format!("/api/agents/{parent_id}/message"),
        SAFE_STEP_PROMPT_BODY_JSON_040_COMBINED,
        &snapshot,
    )
    .expect("message response");
    assert_eq!(
        response
            .payload
            .pointer("/response_workflow/contract")
            .and_then(Value::as_str),
        Some("agent_workflow_library_v1")
    );
    assert_eq!(
        response
            .payload
            .pointer("/response_workflow/selected_workflow/name")
            .and_then(Value::as_str),
        Some("complex_prompt_chain_v1")
    );
    assert_eq!(
        response
            .payload
            .pointer("/response_workflow/final_llm_response/status")
            .and_then(Value::as_str),
        Some("skipped_test")
    );
}
