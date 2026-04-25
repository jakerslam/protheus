// SRS: V12-MISTY-HEALTH-WAVE5-001

#[test]
fn misty_wave5_dashboard_health_indicator_distinguishes_buckets() {
    let indicator = agent_dashboard_health_indicator(
        &json!({
            "quality_telemetry": {
                "repeated_fallback_loop_detected": 1,
                "current_turn_dominance_reject": 1
            },
            "final_llm_response": {"status": "synthesized"}
        }),
        &json!({
            "outcome": "workflow_authored|visible_response_contamination_withheld",
            "visible_response_source": "none",
            "system_chat_injection_used": false,
            "contamination_guard": {"detected": true},
            "tooling_invariant": {
                "classification": "failed",
                "failure_code": "workspace_search_error"
            },
            "web_invariant": {"classification": "healthy"}
        }),
        &json!({"contract": "turn_process_summary_v1"}),
        &json!({"enabled": true, "issue_count": 1, "chat_injection_allowed": false}),
    );

    assert_eq!(
        indicator.get("contract").and_then(Value::as_str),
        Some("agent_dashboard_health_indicator_v1")
    );
    assert_eq!(indicator.get("overall").and_then(Value::as_str), Some("degraded"));
    for bucket in ["workflow", "model", "tool", "finalization", "telemetry"] {
        assert_eq!(
            indicator
                .pointer(&format!("/buckets/{bucket}/status"))
                .and_then(Value::as_str),
            Some("degraded"),
            "{bucket} should be degraded: {indicator}"
        );
    }
    assert_eq!(
        indicator.get("chat_injection_allowed").and_then(Value::as_bool),
        Some(false)
    );
}

#[test]
fn misty_wave5_message_payload_exposes_dashboard_health_indicator() {
    let root = governance_temp_root();
    let snapshot = governance_ok_snapshot();
    let created = handle(
        root.path(),
        "POST",
        "/api/agents",
        br#"{"name":"misty-wave5-health-agent","role":"assistant"}"#,
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
        &json!({"queue": [{"response": "Hey, I am here and answering directly."}], "calls": []}),
    );

    let response = handle(
        root.path(),
        "POST",
        &format!("/api/agents/{agent_id}/message"),
        br#"{"message":"hey"}"#,
        &snapshot,
    )
    .expect("message response");

    assert_eq!(response.status, 200);
    assert_eq!(
        response
            .payload
            .pointer("/dashboard_health_indicator/contract")
            .and_then(Value::as_str),
        Some("agent_dashboard_health_indicator_v1")
    );
    assert_eq!(
        response
            .payload
            .pointer("/dashboard_health_indicator/chat_injection_allowed")
            .and_then(Value::as_bool),
        Some(false)
    );
    assert_eq!(
        response
            .payload
            .pointer("/agent_health_snapshot/dashboard_health_indicator/contract")
            .and_then(Value::as_str),
        Some("agent_dashboard_health_indicator_v1")
    );
}
