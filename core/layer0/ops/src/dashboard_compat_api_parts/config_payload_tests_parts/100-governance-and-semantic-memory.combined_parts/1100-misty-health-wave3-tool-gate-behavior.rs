// SRS: V12-MISTY-HEALTH-WAVE3-001

#[test]
fn misty_wave3_gate_one_is_literal_llm_controlled_yes_no() {
    let decision = workflow_turn_tool_decision_tree("access the file tooling");
    assert_eq!(
        decision
            .pointer("/gates/gate_1/question")
            .and_then(Value::as_str),
        Some("Need tools? Yes/No")
    );
    assert_eq!(
        decision
            .get("gate_1_question_type")
            .and_then(Value::as_str),
        Some("multiple_choice")
    );
    assert_eq!(
        decision
            .get("semantic_route_classifier_active")
            .and_then(Value::as_bool),
        Some(false)
    );
    assert_eq!(
        decision
            .get("info_task_route_classifier_active")
            .and_then(Value::as_bool),
        Some(false)
    );
    assert_eq!(
        decision
            .get("system_may_select_tools")
            .and_then(Value::as_bool),
        Some(false)
    );
    assert_eq!(
        decision
            .get("tool_recommendations_allowed")
            .and_then(Value::as_bool),
        Some(false)
    );
    assert_eq!(
        decision
            .get("automatic_tool_calls_allowed")
            .and_then(Value::as_bool),
        Some(false)
    );
    assert_eq!(
        decision.get("should_call_tools").and_then(Value::as_bool),
        Some(false)
    );
}

#[test]
fn misty_wave3_dry_run_tool_question_exits_without_tool_execution() {
    let root = governance_temp_root();
    let snapshot = governance_ok_snapshot();
    let created = handle(
        root.path(),
        "POST",
        "/api/agents",
        br#"{"name":"misty-wave3-dry-run-agent","role":"assistant"}"#,
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
        &json!({
            "queue": [
                {"response": "<function=workspace_analyze>{\"query\":\"workflow files\"}</function>"},
                {"response": "I would use workspace search after you approve tool use, but I will not run tools for this dry run."}
            ],
            "calls": []
        }),
    );

    let response = handle(
        root.path(),
        "POST",
        &format!("/api/agents/{agent_id}/message"),
        br#"{"message":"Dry run only: tell me which file tool you would use, but do not run tools yet."}"#,
        &snapshot,
    )
    .expect("message response");

    assert_eq!(response.status, 200);
    assert_eq!(
        response.payload.get("tools").and_then(Value::as_array).map(Vec::len),
        Some(0)
    );
    assert_eq!(
        response.payload.get("response").and_then(Value::as_str),
        Some("I would use workspace search after you approve tool use, but I will not run tools for this dry run.")
    );
    assert_eq!(
        response
            .payload
            .pointer("/response_workflow/tool_gate/should_call_tools")
            .and_then(Value::as_bool),
        Some(false)
    );
    assert!(
        response
            .payload
            .pointer("/response_workflow/stage_statuses")
            .and_then(Value::as_array)
            .map(|rows| rows.len() <= 2)
            .unwrap_or(false)
    );
}

#[test]
fn misty_wave3_web_intent_does_not_force_automatic_web_fallback() {
    let root = governance_temp_root();
    let snapshot = governance_ok_snapshot();
    let created = handle(
        root.path(),
        "POST",
        "/api/agents",
        br#"{"name":"misty-wave3-no-auto-web-agent","role":"assistant"}"#,
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
        &json!({
            "queue": [
                {"response": "I cannot use web search because tool execution is blocked right now."}
            ],
            "calls": []
        }),
    );

    let response = handle(
        root.path(),
        "POST",
        &format!("/api/agents/{agent_id}/message"),
        br#"{"message":"Compare infring to other major agentic frameworks in April 2026."}"#,
        &snapshot,
    )
    .expect("message response");

    assert_eq!(response.status, 200);
    assert_eq!(
        response.payload.get("tools").and_then(Value::as_array).map(Vec::len),
        Some(0)
    );
    assert_eq!(
        response
            .payload
            .pointer("/response_finalization/web_invariant/forced_fallback_attempted")
            .and_then(Value::as_bool),
        Some(false)
    );
    assert_eq!(
        response
            .payload
            .pointer("/response_workflow/tool_gate/automatic_tool_calls_allowed")
            .and_then(Value::as_bool),
        Some(false)
    );
}
