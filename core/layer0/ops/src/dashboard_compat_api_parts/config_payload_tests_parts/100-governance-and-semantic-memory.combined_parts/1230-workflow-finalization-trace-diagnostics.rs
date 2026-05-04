// SRS: V13-WORKFLOW-TRACE-001

#[test]
fn workflow_visibility_diagnostics_classify_empty_llm_reply_without_system_injection() {
    let response_workflow = json!({
        "final_llm_response": {
            "status": "synthesized"
        },
        "tool_gate": {
            "needs_tool_access": false
        }
    });
    let response_finalization = json!({
        "outcome": "workflow_authored+workflow:synthesized+empty_final_response_no_system_retry+empty_visible_response_preserved_without_system_chat",
        "visible_response_source": "none",
        "system_chat_injection_used": false
    });

    let payload = workflow_visibility_payload(&response_workflow, &response_finalization);
    let diagnostics = payload
        .pointer("/finalization_diagnostics")
        .expect("finalization diagnostics");

    assert_eq!(
        diagnostics
            .pointer("/contract")
            .and_then(Value::as_str),
        Some("workflow_finalization_diagnostics_v1")
    );
    assert_eq!(
        diagnostics
            .pointer("/diagnostic_class")
            .and_then(Value::as_str),
        Some("empty_llm_visible_response_no_system_fallback")
    );
    assert_eq!(
        diagnostics
            .pointer("/empty_visible_response")
            .and_then(Value::as_bool),
        Some(true)
    );
    assert_eq!(
        diagnostics
            .pointer("/chat_injection_allowed")
            .and_then(Value::as_bool),
        Some(false)
    );
    assert_eq!(
        diagnostics
            .pointer("/system_chat_injection_used")
            .and_then(Value::as_bool),
        Some(false)
    );
    assert_eq!(
        diagnostics
            .pointer("/trace_sufficient_for_diagnosis")
            .and_then(Value::as_bool),
        Some(true)
    );
    assert!(
        payload
            .pointer("/finalization_status")
            .and_then(Value::as_str)
            .unwrap_or("")
            .contains("empty_visible_response_preserved_without_system_chat"),
        "{payload}"
    );
    assert_eq!(payload.pointer("/ui_status").and_then(Value::as_str), Some(""));
    assert_eq!(
        payload
            .pointer("/agent_process_status")
            .and_then(Value::as_str),
        Some("")
    );
}

#[test]
fn workflow_self_play_empty_reply_keeps_trace_diagnostic_from_beginning_to_end() {
    let root = governance_temp_root();
    let snapshot = governance_ok_snapshot();
    let agent_id = workflow_self_play_agent(
        root.path(),
        &snapshot,
        "workflow-self-play-empty-trace-agent",
    );

    write_json(
        &governance_test_chat_script_path(root.path()),
        &json!({
            "queue": [
                {"response": ""}
            ],
            "calls": []
        }),
    );

    let response = workflow_self_play_message(root.path(), &snapshot, &agent_id, "hey");
    assert_eq!(response.status, 200);
    assert_eq!(
        response.payload.get("response").and_then(Value::as_str),
        Some("")
    );
    assert_eq!(
        response
            .payload
            .pointer("/workflow_visibility/finalization_diagnostics/trace_sufficient_for_diagnosis")
            .and_then(Value::as_bool),
        Some(true),
        "{}",
        response.payload
    );
    assert_eq!(
        response
            .payload
            .pointer("/workflow_visibility/finalization_diagnostics/system_chat_injection_used")
            .and_then(Value::as_bool),
        Some(false),
        "{}",
        response.payload
    );
    assert_eq!(
        response
            .payload
            .pointer("/workflow_visibility/finalization_diagnostics/diagnostic_class")
            .and_then(Value::as_str),
        Some("empty_llm_visible_response_no_system_fallback"),
        "{}",
        response.payload
    );
    assert_eq!(
        response
            .payload
            .pointer("/response_quality_telemetry/prompt_echo_reject")
            .and_then(Value::as_u64),
        Some(0),
        "{}",
        response.payload
    );
    assert!(
        response
            .payload
            .pointer("/live_eval_monitor/issues")
            .and_then(Value::as_array)
            .map(|rows| rows.iter().any(|row| {
                row.pointer("/raw_event/issue_class").and_then(Value::as_str)
                    == Some("no_response")
                    || row.pointer("/raw_event/issue_class").and_then(Value::as_str)
                        == Some("empty_direct_reply")
            }))
            .unwrap_or(false),
        "{}",
        response.payload
    );
}
